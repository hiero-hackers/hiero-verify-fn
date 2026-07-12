# hiero-verify-fn

[![CI](https://github.com/hiero-hackers/hiero-verify-fn/actions/workflows/ci.yml/badge.svg)](https://github.com/hiero-hackers/hiero-verify-fn/actions/workflows/ci.yml)
[![CodeQL](https://github.com/hiero-hackers/hiero-verify-fn/actions/workflows/codeql.yml/badge.svg)](https://github.com/hiero-hackers/hiero-verify-fn/actions/workflows/codeql.yml)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/hiero-hackers/hiero-verify-fn/badge)](https://scorecard.dev/viewer/?uri=github.com/hiero-hackers/hiero-verify-fn)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

<!-- After registering at https://www.bestpractices.dev (Get Your Badge → this repo), add:
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/<ID>/badge)](https://www.bestpractices.dev/projects/<ID>)
-->

Hedera block-proof verification as a serverless function. `POST` a
block file, get back a cryptographic verdict — recomputed merkle root,
hinTS BLS threshold check, aggregate-Schnorr or WRAPS proof path, every
check itemized. The trust kernel is
[hiero-streams](https://github.com/hiero-hackers/hiero-streams-rs)'
`verify_block_proof`; this binary is ~100 lines of HTTP around it.

**The point**: proof verification portable enough to run *anywhere* —
including the smallest serverless tier. Measured on the
`CN_0_73_TSS_WRAPS` test-network blocks (same fixtures
`hiero-block-node` tests against):

| Measurement | Value |
| --- | --- |
| Binary size | **1.9 MB** (stripped, LTO) |
| Cold start → serving | **36 ms** |
| Steady RSS under load | **5.4 MiB** |
| Verify, Schnorr path | 28 ms |
| Verify, WRAPS path (settled blocks) | 45 ms |

A 128 MB function tier leaves ~96% headroom. For comparison, the same
verification (same crypto, same fixtures) through the Java
`hiero-block-node` stack runs at a ~375 MiB working set — this
deployment class is unavailable to it, which is the demo.

## API

```sh
# genesis blocks carry their own ledger-ID publication
curl -X POST --data-binary @0.blk.gz https://<fn>/verify
# any other block resolves its bootstrap from BOOTSTRAP_BLOCK
curl -X POST --data-binary @467.blk.gz https://<fn>/verify
```

```json
{
    "block_number": 467,
    "block_root": "a7986473fa0a42a55a74f04eca352ec7cb6dc37…",
    "proof_path": "wraps",
    "hints_all_passed": true,
    "suffix_all_passed": true,
    "valid": true,
    "verify_ms": 45,
    "rss_kib": 4352
}
```

Semantics mirror the library's crate-wide convention: a proof that
**fails** is a *successful* verification with `valid: false` (HTTP
200); malformed input is `400`; a block whose bootstrap cannot be
resolved is `422` with an error that names the fix. `GET /healthz`
for liveness.

## Deploy (Cloud Run, 128 MiB)

```sh
gcloud run deploy hiero-verify-fn --source . \
    --memory 128Mi --cpu 1 --allow-unauthenticated
```

The image bakes a test-network genesis (`bootstrap/`) as the default
`BOOTSTRAP_BLOCK`. To verify a different chain's blocks, replace it
with that chain's genesis block — for mainnet, block 0 of the
HIP-1056 stream once GA. Nothing else is configurable on purpose.

## Run locally

```sh
cargo build --release
BOOTSTRAP_BLOCK=bootstrap/genesis-cn-0.73-tss.blk.gz PORT=8080 \
    ./target/release/hiero-verify-fn
```

## Contributing & security

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — the one rule (verification
  logic belongs upstream), build, gates, conventions.
- [`SECURITY.md`](SECURITY.md) — how to report a vulnerability
  privately; this service accepts attacker bytes by design, so
  panic-freedom and soundness are the scope.
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) — Contributor Covenant 2.1.

## Status

**Mainnet is still pre-TSS** (checked 2026-07-12): live
`block-preview/` blocks carry a 48-byte placeholder signature, not yet
a TSS proof — POSTing one returns a `422` explaining exactly that.
What this function cryptographically verifies today are HIP-1056 TSS
proofs as produced by the consensus-node TSS test network (the same
`CN_0_73_TSS_WRAPS` vectors `hiero-block-node` itself tests against,
included under `bootstrap/`). The moment mainnet cuts over, real
mainnet blocks verify here with **zero changes** — the format is the
format. (The cutover is watched automatically by hiero-streams'
sentinel and tripwires.)

Built pre-publish against a path dependency; the Cargo.toml `TODO`
flips it to the crates.io `hiero-streams` release, which also makes
the Dockerfile self-contained. No tests of its own beyond the
library's — the crypto is differentially tested in hiero-streams
(check-for-check against `hiero-block-verifier-js`); this repo is
deliberately too thin to be wrong.
