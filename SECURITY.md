# Security Policy

## Supported versions

`hiero-verify-fn` is pre-1.0. Security fixes are applied to the latest
`0.x` release; there is no back-porting to earlier `0.x` versions.

| Version | Supported |
| ------- | --------- |
| latest `0.x` | ✅ |
| older | ❌ |

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Report privately through GitHub's
[private vulnerability reporting](https://github.com/hiero-hackers/hiero-verify-fn/security/advisories/new)
("Report a vulnerability" under the repository's **Security** tab). If
you cannot use that, open a minimal public issue asking for a private
contact channel — without any exploit detail — and a maintainer will
respond.

We aim to acknowledge a report within a few days and to agree on a
coordinated disclosure timeline before any public detail is shared.

## Scope

This service accepts **attacker-controlled bytes over unauthenticated
HTTP by design** — that is its job. In scope:

- **Crashes or panics on any request body.** The verification core
  (`hiero-streams`) is contract-bound to return errors, never panic,
  on arbitrary input (fuzzed and robustness-tested upstream); this
  wrapper must preserve that property end to end.
- **Verification soundness** — any input reported `valid: true` whose
  proof the network did not actually produce. The cryptography lives
  in [hiero-streams](https://github.com/hiero-hackers/hiero-streams-rs);
  soundness issues found here should generally be reported there, but
  reports to either repo will be routed correctly.
- **Resource exhaustion** — the request-body cap (64 MB) and the
  library's decompression ceiling (1 GiB) are the intended limits;
  inputs that evade them or otherwise pin memory/CPU beyond one
  request's work are vulnerabilities.

Out of scope: the service holds no keys and signs nothing; there is no
authentication surface to bypass (deployments choosing
`--allow-unauthenticated` are rate-limited by their platform, which is
the operator's concern).

## Disclosure

Fixed vulnerabilities are disclosed in the release notes and, where a
CVE applies, via a GitHub Security Advisory once a fixed version is
available.
