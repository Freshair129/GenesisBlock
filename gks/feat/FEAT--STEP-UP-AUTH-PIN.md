---
id: FEAT--STEP-UP-AUTH-PIN
phase: 2
type: feat
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Step-up auth — PIN provider, the minimal StepUpProvider implementation
tags:
  - msp
  - ucf
  - feat
  - auth
  - step-up
crosslinks:
  references:
    - CONCEPT--STEP-UP-AUTH
    - CONCEPT--ABAC-POLICY-ENGINE
    - FEAT--POLICY-DECISION-POINT
created_at: 2026-05-14T19:42:04.304+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: Feature spec
attributes:
  domain: feat
---

# FEAT — Step-up auth: PIN provider

> The minimal concrete `StepUpProvider` from `[[CONCEPT--STEP-UP-AUTH]]` — PIN re-entry, the homelab / single-user tier. Establishes the provider interface that Passkey and signed-token providers will later implement.

## User-facing behaviour

The PDP (`[[FEAT--POLICY-DECISION-POINT]]`) emits `advice: ['request-step-up-auth']` on a deny. The PEP intercepts that advice and drives a `StepUpProvider`. This FEAT ships the interface plus the PIN implementation:

```ts
interface StepUpProvider {
  id: string
  methods: StepUpMethod[]
  challenge(subject: Subject, action_hash: string): Promise<Challenge>
  verify(challenge_id: string, response: VerifyResponse): Promise<VerifyResult>
}

// The MVP provider
class PinProvider implements StepUpProvider {
  id = 'pin'
  methods = ['pin']
  // challenge() → { challenge_id, nonce, expires_at }
  // verify()    → checks PIN hash, nonce unseen, not expired, action_hash matches
}
```

Behaviour contract:

- **Triggered by policy, not hardcoded**: the PEP calls `challenge()` only when a `Decision` carries `advice: ['request-step-up-auth']`.
- **Action-bound**: `challenge()` takes an `action_hash` (hash of the action payload + nonce + timestamp). `verify()` rejects a response whose `action_hash` does not match — a captured PIN proof cannot be replayed on a different action.
- **TTL-bounded**: challenges expire (default 300s for high-sensitivity, per `[[CONCEPT--STEP-UP-AUTH]]`). Expired challenges fail `verify()`.
- **Replay-defended**: nonces are stored server-side; a verified nonce cannot be reused.
- **On success**: the Subject's `last_step_up_at` and `last_step_up_method` attributes update; the caller retries the original action and the PDP now permits it (a policy rule tests `last_step_up_at` against the action's TTL).
- **PIN storage**: hashed (Argon2id), never plaintext; configured per `MSP_HOME` identity.
- **Audit**: both the initial deny (with reasoning) and the subsequent step-up + permit are logged with `challenge_id` and method.

CLI / MCP surface:

```sh
msp-auth set-pin                 # set or rotate the local PIN (prompts, never echoes)
```

The HTTP PEP prompts via the web UI; the MCP PEP — for `risk: high` tools — dispatches an out-of-band confirmation (working assumption per spec §14 OQ-2).

## Verification

- A policy emitting `request-step-up-auth` causes the PEP to defer the action and issue a `challenge()`.
- `verify()` with the correct PIN + matching `action_hash` + unexpired + unseen-nonce succeeds; the action then proceeds.
- `verify()` rejects: wrong PIN, expired challenge, reused nonce, mismatched `action_hash`.
- After success, `last_step_up_at` is updated and an identical action within TTL is permitted without re-challenge.
- PIN is stored Argon2id-hashed; the plaintext never touches disk or logs.
- Audit log shows the deny→challenge→verify→permit sequence with `challenge_id`.

## Out of scope

- **PasskeyProvider** (WebAuthn) and **SignedTokenProvider** — they implement the same `StepUpProvider` interface in later FEATs; this FEAT only establishes the interface + PIN.
- Primary authentication (login) — spec §14 OQ-3; this FEAT is step-up only.
- The MCP out-of-band confirmation channel implementation — spec §14 OQ-2; this FEAT assumes it exists for `risk: high` tools.
- Risk scoring (device fingerprint, anomaly detection) — future.
- The PDP and the policy that triggers step-up — `[[FEAT--POLICY-DECISION-POINT]]` and policy content.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §8 — step-up mechanism table, flow, TTL guidance.
- `[[CONCEPT--STEP-UP-AUTH]]` — the concept this FEAT partially implements (PIN tier).
- `[[CONCEPT--ABAC-POLICY-ENGINE]]` — emits the `request-step-up-auth` advice.
- `[[FEAT--POLICY-DECISION-POINT]]` — the PDP whose advice drives this provider.
