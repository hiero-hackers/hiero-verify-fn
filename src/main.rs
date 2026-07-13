//! Hedera block-proof verification as a serverless function.
//!
//! The whole service: `POST /verify` with a block file as the body
//! (`.blk` or `.blk.gz`, auto-detected) returns a verification report —
//! recomputed merkle root, proof path, every cryptographic check, and
//! the verdict. The trust kernel is `hiero-streams`' `verify_block_proof`
//! (hinTS BLS threshold + aggregate Schnorr / WRAPS Groth16+KZG); this
//! binary is just HTTP around it.
//!
//! Sized for the smallest serverless tiers (128 MB): single-threaded
//! runtime, ~8 MiB working set, no state. Non-genesis blocks need the
//! ledger-ID publication from the genesis block — bake it into the
//! image and point `BOOTSTRAP_BLOCK` at it.
//!
//!   PORT             listen port (Cloud Run contract; default 8080)
//!   BOOTSTRAP_BLOCK  path to the chain's genesis block (.blk[.gz])
use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{body::Bytes, Json, Router};
use hiero_streams::{extract_proof_material, resolve_bootstrap, verify_block_proof};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::timeout::TimeoutLayer;

/// Per-request ceiling — a verify is ~40 ms, so this only fires on a
/// pathological input. Cloud Run backstops at 300 s; this is the inner belt.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Blocks are single-digit MB today; leave headroom for GA-era blocks.
const MAX_BODY: usize = 64 * 1024 * 1024;

struct App {
    /// Genesis block bytes for bootstrap resolution (`BOOTSTRAP_BLOCK`).
    genesis: Option<Vec<u8>>,
}

#[derive(Serialize)]
struct Report {
    block_number: u64,
    block_root: String,
    proof_path: &'static str,
    hints_all_passed: bool,
    suffix_all_passed: bool,
    valid: bool,
    verify_ms: u64,
    /// This process's resident set right now — the footprint claim,
    /// self-reported by every response.
    rss_kib: Option<u64>,
}

#[derive(Serialize)]
struct Problem {
    error: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let genesis = match std::env::var("BOOTSTRAP_BLOCK") {
        Ok(path) => Some(std::fs::read(&path).unwrap_or_else(|e| {
            eprintln!("BOOTSTRAP_BLOCK {path}: {e}");
            std::process::exit(1);
        })),
        Err(_) => None,
    };
    let app = Arc::new(App { genesis });

    let router = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/verify", post(verify))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        // Outermost, so it catches a panic from anywhere inside: the
        // library is fuzzed not to panic, but a public endpoint should
        // return a 500, never a dropped connection.
        .layer(CatchPanicLayer::new())
        .with_state(app);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind");
    eprintln!("hiero-verify-fn listening on :{port}");
    axum::serve(listener, router).await.expect("serve");
}

async fn verify(
    State(app): State<Arc<App>>,
    body: Bytes,
) -> Result<Json<Report>, (StatusCode, Json<Problem>)> {
    let t0 = Instant::now();

    // Malformed input is 400/422 — distinct from a proof that FAILS,
    // which is a successful verification with valid=false (the same
    // Ok(false)-vs-Err split the library holds crate-wide).
    let material =
        extract_proof_material(&body).map_err(|e| problem(StatusCode::BAD_REQUEST, e))?;

    // Pre-TSS blocks (mainnet preview as of 2026-07: a 48-byte legacy
    // placeholder, SHA-384 of the root) carry NO cryptographic proof —
    // there is nothing to verify, which is different from failing.
    // Teach that instead of erroring on an "unrecognized suffix".
    if matches!(material.layout.path, hiero_streams::ProofPath::Unknown) {
        let signature_bytes = material.layout.hints_verification_key.len()
            + material.layout.hints_signature.len()
            + material.layout.suffix.len();
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(Problem {
                error: format!(
                    "block {} carries a {signature_bytes}-byte pre-TSS placeholder \
                     signature, not a TSS proof — this stream has not cut over to \
                     HIP-1056 TSS proofs yet, so there is no cryptographic proof to \
                     verify. This endpoint verifies it automatically once TSS lands.",
                    material.block_number
                ),
            }),
        ));
    }

    let bootstrap = resolve_bootstrap(
        &material,
        app.genesis.as_deref(),
        "deploy with BOOTSTRAP_BLOCK pointing at the chain's genesis block",
    )
    .map_err(|e| problem(StatusCode::UNPROCESSABLE_ENTITY, e))?;
    let v = verify_block_proof(&material, &bootstrap)
        .map_err(|e| problem(StatusCode::UNPROCESSABLE_ENTITY, e))?;

    Ok(Json(Report {
        block_number: v.block_number,
        block_root: hex::encode(material.block_root),
        proof_path: if v.wraps.is_some() {
            "wraps"
        } else {
            "aggregate-schnorr"
        },
        hints_all_passed: v.hints.all_passed(),
        suffix_all_passed: v
            .schnorr
            .as_ref()
            .map(|s| s.valid)
            .or_else(|| v.wraps.as_ref().map(|w| w.all_passed()))
            .unwrap_or(false),
        valid: v.valid(),
        verify_ms: t0.elapsed().as_millis() as u64,
        rss_kib: current_rss_kib(),
    }))
}

fn problem(status: StatusCode, error: impl std::fmt::Display) -> (StatusCode, Json<Problem>) {
    (
        status,
        Json(Problem {
            error: error.to_string(),
        }),
    )
}

/// Resident set of this process in KiB (Linux: /proc; macOS: ps).
fn current_rss_kib() -> Option<u64> {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        return status
            .lines()
            .find(|l| l.starts_with("VmRSS:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse().ok());
    }
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    String::from_utf8(out.stdout).ok()?.trim().parse().ok()
}
