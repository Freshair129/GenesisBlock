# Security Policy

## Supported Versions

GenesisBlockDB is pre-1.0. Security fixes are applied to the latest released
version only.

| Version        | Supported          |
| -------------- | ------------------ |
| 0.1.0-beta.x   | :white_check_mark: |
| < 0.1.0-beta.1 | :x:                |

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
  `MASTER`-tier nodes. Enforcement lives in the engine, not the transport.
- **Consensus & sync** — `SignedEvent`s are ed25519-signed; the swarm identity
  keypair is persisted under the database path. Report any signature-bypass or
  key-handling weakness.
- **REST surface** (`/v1/*`) — input validation, query handling, and
  authentication boundaries.
- **Deserialization** — WAL replay and snapshot (`state.json`, `*.bin`) loading.

## Dependency Hygiene

`Cargo.lock` is committed and audited against the [RustSec](https://rustsec.org)
advisory database on every push/PR and weekly (see
`.github/workflows/security.yml`).
