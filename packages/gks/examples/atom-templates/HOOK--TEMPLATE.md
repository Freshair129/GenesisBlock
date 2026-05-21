---
id: HOOK--TEMPLATE
tier: genesis
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2
type: hook
status: draft
vault_id: <YOUR-PROJECT>
title: <One-line hook summary>
tags: [event-driven, hook]
domain: <domain-name>
crosslinks:
  references: []                # ADR-- defining the event architecture
  enforces: []                  # GUARD-- for hook payload validation
linked_symbols:
  - {"file": "<path-to-listener>", "symbol": "<listener-function>"}
---

# HOOK — <Title>

## Triggering Event

- **Source:** <e.g. GitHub Webhook / Redis PubSub / Internal System Event>
- **Type:** <e.g. `push` | `user_created` | `pre-commit`>

## Payload Schema (JSON Schema)

```json
{
  "type": "object",
  "properties": {
    "event_id": { "type": "string" },
    "timestamp": { "type": "string", "format": "date-time" },
    "data": { "type": "object" }
  },
  "required": ["event_id", "timestamp"]
}
```

## Action / Behaviour

1. <What happens when this hook is triggered?>
2. <Step 1 of the processing logic>
3. <Step 2 ...>

## Error Handling

- **Retry Policy:** <e.g. Exponential backoff (3 retries)>
- **Dead Letter Queue:** <Yes/No>
- **Alerting:** <Reference to RUNBOOK-- if it fails>

## Security

- **Signature Verification:** <Yes/No (e.g. HMAC-SHA256)>
- **Source Filtering:** <Allow-list of IPs or Domains>
