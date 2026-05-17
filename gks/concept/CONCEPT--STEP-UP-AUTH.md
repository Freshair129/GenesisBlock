---
id: CONCEPT--STEP-UP-AUTH
phase: 1
type: concept
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Step-up authentication — defense-in-depth for sensitive actions
tags: &a1
  - msp
  - ucf
  - concept
  - auth
  - step-up
  - security
crosslinks: &a2
  references:
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
    - CONCEPT--ABAC-POLICY-ENGINE
    - CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT
created_at: 2026-05-13T17:22:02.427+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--STEP-UP-AUTH
  phase: 1
  type: concept
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Step-up authentication — defense-in-depth for sensitive actions
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-13T17:22:02.427+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--STEP-UP-AUTH
    phase: 1
    type: concept
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Step-up authentication — defense-in-depth for sensitive actions
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-13T17:22:02.427+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      domain: concept
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# CONCEPT — Step-up authentication

> A second authentication gate placed **between primary login and sensitive actions**, scoped to a short TTL and (optionally) bound to the action's prompt. Not a replacement for login — a complement that limits blast radius when the outer session is compromised, idle, or shared.
>
> Spec section: `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §8.

## Problem

A shared agent inherits the worst property of long-lived sessions: anyone with physical or network access to a logged-in client can issue **any** action the primary identity permits. If the agent has access to sensitive data (medical records, financial transactions, IP, customer PII), forgetting to log out — or losing a device — exposes everything.

Primary auth (UI login, API token) cannot be tightened arbitrarily without ruining UX. Asking for password on every action is unworkable. But blanket trust between login and logout is also unworkable when actions vary in sensitivity by orders of magnitude.

A graduated, action-scoped re-auth is the standard answer (online banking has used this since the 1990s). MSP needs an equivalent that:

1. Fires only on actions the policy marks as high-impact.
2. Has UX cost proportional to the action's risk.
3. Binds (where possible) to the action's content so a captured proof cannot be replayed elsewhere.
4. Works over **both** HTTP (browser session) and MCP (stdio with no browser).

## Hypothesis

Step-up authentication is **triggered by policy obligation**, not by hardcoded action lists. When the PDP evaluates a request and the Resource's sensitivity or the Action's impact warrants step-up, the decision carries `advice: ['request-step-up-auth']` and the PEP defers execution until step-up completes within a bounded TTL.

The step-up **mechanism** is pluggable: PIN re-entry (low security), TOTP (medium), Passkey/WebAuthn signature (high), signed-prompt token (bound to content). Vault config selects the minimum acceptable mechanism per sensitivity tier.

Last-step-up timestamp is an **attribute on the Subject**; policies test it against the action's TTL.

## Scope

In:

- Trigger model: PDP-driven via `advice: ['request-step-up-auth']` + `ttl: N seconds`.
- Subject attribute `last_step_up_at: timestamp` + `last_step_up_method: pin | totp | passkey | signed-token`.
- StepUpProvider interface:
  ```ts
  interface StepUpProvider {
    id: string
    methods: StepUpMethod[]
    challenge(subject: Subject, action_hash: string): Promise<Challenge>
    verify(challenge_id: string, response: VerifyResponse): Promise<VerifyResult>
  }
  ```
- Built-in providers:
  - **PinProvider** — local / single-user / homelab — re-enter PIN.
  - **PasskeyProvider** — WebAuthn signature over `prompt_hash + nonce + ts`. Phishing-resistant.
  - **SignedTokenProvider** — caller signs request payload with a paired key (HMAC-ES256 or Ed25519). Bound to prompt; works on MCP stdio when paired with an out-of-band confirmation channel.
- Replay defense: nonces stored server-side with TTL; once verified, a challenge cannot be reused.
- Out-of-band step-up for MCP: notification dispatch to a paired web/mobile session; MCP call blocks until ack or TTL expires.

Out:

- Primary authentication (login). Owned by the auth provider chosen per spec §14 OQ-3. Working assumption: minimal in-house PIN + Passkey, or accept reverse-proxy auth headers.
- Role / identity provisioning. Step-up does not change who the Subject is — it merely re-proves they are still that Subject.
- Risk scoring (device fingerprint, IP reputation, behavioural anomaly). Possible advice signals later; not in v1.

## When step-up fires

The policy decides, but typical rules look like:

```yaml
- id: phi-needs-recent-step-up
  match: { resource.attributes.classification: phi, action: expose-to-llm }
  condition: |
    subject.attributes.auth_level >= 3
    AND context.time - subject.attributes.last_step_up_at <= 5min
  effect: permit
  on_deny:
    advice: ['request-step-up-auth']
    advice_params: { ttl: 300, min_method: passkey }
```

A `delete`-class action on a shared namespace, a cross-namespace promote, or any `expose-to-llm` on `restricted` Resources are common triggers.

## Why bind to the prompt

A step-up proof that authorises "the next action" is **replayable**: an attacker can swap the action while keeping the proof valid. Binding the proof to a **hash of the action payload** (prompt content + nonce + timestamp) prevents this:

- Captured Passkey signature is useless on a different prompt.
- Signed-token providers always include `prompt_hash` in the signed payload.

This is essential for any deployment where prompts cross trust boundaries (web → server, agent → LLM provider).

## MCP step-up channel

MCP stdio is local-trust by design — no browser, no biometric. Three viable channels (open question §14 OQ-2, working assumption: per-tool risk class):

1. **Per-tool risk class** — MCP tools annotated `risk: low | medium | high`. `low` tools never trigger step-up; `medium` may require recent step-up; `high` always triggers an out-of-band confirmation. Lowest UX friction.
2. **Pre-signed token bundles** — a CLI command issues N step-up tokens; MCP tools consume them; user re-signs when exhausted. Smooth in scripts; less responsive to real-time anomaly.
3. **Out-of-band confirmation** — MCP server pushes a challenge to a paired web/mobile session; MCP call blocks until ack. Highest security; UX cost on every high-risk call.

MVP ships option (1) with option (3) for `risk: high` only. Option (2) considered for batch/automation use cases.

## Verification

- Policy with `advice: ['request-step-up-auth']` correctly defers execution.
- `verify()` rejects expired challenges and replayed nonces.
- Passkey signature includes `prompt_hash`; alternate prompt with same signature is rejected.
- Audit log records both the initial deny (with reasoning) and the subsequent permit (with step-up method + challenge id).
- MCP `risk: high` tool triggers out-of-band notification and the MCP call blocks until confirmation or TTL.

## Out of scope

- Primary authentication mechanism.
- Multi-factor enrolment UX (per-provider).
- Adaptive step-up that escalates method by risk score — possible future enhancement.
- Cross-organisation federation of step-up proofs.

## Source

- `packages/msp/docs/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §8 — full mechanism table, TTL recommendations, flow diagram.
- Open question §14 OQ-2 — MCP step-up channel; working assumption recorded.
- WebAuthn (W3C) — Passkey standard.
- FIDO2 / CTAP2 — hardware-backed credential standard.
- `[[CONCEPT--ABAC-POLICY-ENGINE]]` — how policy triggers step-up.
- `[[CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT]]` — `last_step_up_at` lives in Subject.attributes.

## Connections
- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]

