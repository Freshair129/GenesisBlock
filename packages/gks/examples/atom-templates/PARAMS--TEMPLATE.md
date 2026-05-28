---
id: PARAMS--TEMPLATE
tier: genesis                  # safety | master | genesis | process
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2                     # 0 | 1 | 2 | 3 | 4 | 5 | 6
type: params
status: active                  # stub | raw | draft | active | stable | deprecated | superseded | partial
vault_id: <YOUR-PROJECT>
title: <One-line business parameters summary>
tags: [config, constants]
domain: <domain-name>
crosslinks:
  used_by: []                   # FEAT-- / ALGO-- / MOD-- that use these parameters
  references: []                # CONCEPT-- / ADR-- that justify these values
---

# PARAMS — <Title>

## Purpose

Describe the business rules or configuration context. Why do these parameters exist?
(e.g., "Tier limits for SaaS pricing plans", "Default timeout values for third-party integrations")

## Values / Thresholds

Use a table to define business-meaningful numbers, threshold lists, or configuration properties.

| Parameter Name | Value / Threshold | Description | Override Logic |
|---|---|---|---|
| `MAX_FREE_USERS` | 5 | Limit of free users per organization | Can be overridden per tenant in DB |
| `DEFAULT_TIMEOUT_MS` | 5000 | Baseline timeout for external APIs | Environment variable `API_TIMEOUT` |

## Data Shape / Schema (Optional)

If these parameters are passed as a structured object, define the JSON schema or TypeScript interface here.

```typescript
interface PricingParams {
  maxFreeUsers: number;
  defaultTimeoutMs: number;
}
```

## Update Policy

Who can change these parameters? Do they require a redeploy, or are they hot-reloaded from a database?

- **Storage:** Hardcoded / Database / Remote Config / Env Vars
- **Update Frequency:** Rarely / Monthly / Runtime
