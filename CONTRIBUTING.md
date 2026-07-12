# Contributing to hiero-verify-fn

Thanks for your interest. This repo is **deliberately thin**: ~150
lines of HTTP around `hiero-streams-rs`. That
thinness is the design — please keep it.

## The one rule

**Verification logic belongs upstream.** If a change touches what is
verified or how, it goes to
[hiero-streams](https://github.com/hiero-hackers/hiero-streams-rs),
where the differential tests live. This repo only owns the HTTP
mapping: routes, status codes, the report shape, and deployment.

## Getting started

Prerequisites: Rust (stable), `protoc` (`brew install protobuf` /
`apt install protobuf-compiler`), and — until the crates.io dependency
lands — a sibling checkout of `hiero-streams-rs`.

```sh
cargo build --release
BOOTSTRAP_BLOCK=bootstrap/genesis-cn-0.73-tss.blk.gz PORT=8080 \
    ./target/release/hiero-verify-fn
curl -X POST --data-binary @bootstrap/genesis-cn-0.73-tss.blk.gz \
    http://127.0.0.1:8080/verify   # expect "valid": true
```

Enable the local hooks once per clone:

```sh
git config core.hooksPath .githooks
```

## Gates

CI runs `cargo fmt --check`, `cargo clippy -D warnings`, a release
build, the smoke test above (plus garbage-in → 400), and `cargo deny`.
All must be green.

## Conventions

- **HTTP semantics are the API**: a failing proof is HTTP 200 with
  `valid: false`; malformed input is 400; unresolvable bootstrap is
  422 with an error naming the fix. Changing these is a breaking
  change and reviewed as one.
- The measured numbers in the README (binary size, RSS, cold start)
  are claims — re-measure and update them in any PR that could move
  them.
- Commits require a DCO sign-off (`git commit -s`).

## Security

Do not open public issues for vulnerabilities — see
[`SECURITY.md`](SECURITY.md).
