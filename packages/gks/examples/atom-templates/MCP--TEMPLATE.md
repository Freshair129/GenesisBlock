---
id: MCP--TEMPLATE
tier: genesis                  # safety | master | genesis | process
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2                     # 0 | 1 | 2 | 3 | 4 | 5 | 6
type: mcp
status: active                  # stub | raw | draft | active | stable | deprecated | superseded | partial
vault_id: <YOUR-PROJECT>
title: <One-line tool summary>
tags: [mcp, tool, rpc]
domain: agent-infrastructure
crosslinks:
  references: []                # API-- hub this tool belongs to
  enforces: []                  # GUARD-- for permission/rate-limiting
linked_symbols:
  - {"file": "<path-to-handler>", "symbol": "<handler-function>"}
---

# MCP — <Tool Name>

## Tool Specification

- **Name:** `<snake_case_name>` (Must match MCP client expectations)
- **Description:** <Prompt-friendly description for the Agent>

## Arguments (JSON Schema)

```json
{
  "type": "object",
  "properties": {
    "arg1": { "type": "string", "description": "..." },
    "arg2": { "type": "number", "description": "..." }
  },
  "required": ["arg1"]
}
```

## Response Shape

- **Mime-type:** `application/json` | `text/plain`
- **Data:** <Description of the returned content>

## Security & Constraints

- **Auth Required:** <Yes/No>
- **Rate Limit:** <e.g. 60 requests/min>
- **Side-effects:** <Read-only | Mutates filesystem | External network>

## Example Interaction

> **Call:** `{"name": "...", "arguments": {"arg1": "value"}}`
> **Response:** `{"content": [{"type": "text", "text": "..."}]}`
