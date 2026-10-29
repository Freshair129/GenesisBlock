# Security Policy

## Supported Versions

GenesisBlockDB is pre-1.0. Only the latest published stable release is
supported for security fixes; older releases and prereleases are unsupported.

A security fix is shipped only when the corresponding release assets and
registry artifacts have been rebuilt from the fixed revision and verified.
Merging a fix to `main` does not update already-published binaries or images.

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

Report privately via GitHub's [private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
on this repository (Security → Report a vulnerability), or email the maintainer.

Please include:

- A description of the vulnerability and its impact.
- Steps to reproduce (a minimal proof of concept if possible).
- Affected version(s) and platform.

You can expect an initial acknowledgement within **5 business days**. We will
keep you informed as we triage and develop a fix, and will credit you in the
release notes unless you prefer to remain anonymous.

## Scope

Security-relevant areas of the engine:

- **Governance tiers** — external agents must not be able to create or mutate
  `MASTER`-tier nodes. Local direct-write guards live in the engine. Remote
  reconciliation checks signed membership and `masterAuthority` at the shared
  ingress boundary. See the accepted peer-membership authorization ADR.
- **Consensus & sync** — `SignedEvent` values are ed25519-signed and the swarm
  identity keypair is persisted under the database path. Gossip requires an
  operator-pinned roster, signed packets, a matching roster digest, and the
  configured source endpoint. Votes and quorum use only roster members; vote
  signatures bind to the complete signed event under the proposal ID.
- **REST surface** (`/v1/*`) — input validation, query handling, and
  authentication boundaries.
- **Deserialization** — WAL replay and snapshot (`state.json`, `*.bin`) loading.

## Dependency Hygiene

All committed Rust lockfiles — `Cargo.lock`,
`studio/src-tauri/Cargo.lock`, and
`genesisblock-mobile/src-tauri/Cargo.lock` — are audited against the
[RustSec](https://rustsec.org) advisory database on every push/PR and weekly (see
`.github/workflows/security.yml`).
