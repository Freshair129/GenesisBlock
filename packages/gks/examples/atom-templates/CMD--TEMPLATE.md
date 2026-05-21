---
id: CMD--TEMPLATE
tier: genesis
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2
type: cmd
status: draft
vault_id: <YOUR-PROJECT>
title: <One-line command summary>
tags: [cli, internal-tool, command]
domain: <domain-name>
crosslinks:
  references: []                # SKILL-- that uses this command
  enforces: []                  # PROTO-- this command validates
linked_symbols:
  - {"file": "package.json", "symbol": "scripts.<script-name>"}
---

# CMD — <Title>

## Execution

- **Command:** `npm run <script-name>` | `<binary-name> <args>`
- **Runtime:** <e.g. Node.js 22 | Python 3.12 | Shell>

## Arguments & Flags

| Flag | Type | Required | Description |
|---|---|---|---|
| `--help` | boolean | No | Show usage |
| `-v` | boolean | No | Verbose mode |
| `[input]` | string | Yes | Input file/path |

## Environment Variables

- `GEMINI_API_KEY`: <Description>
- `DEBUG`: <Description>

## Expected Outcome

- **Success:** <What happens on success (e.g. index generated)>
- **Failure:** <Common error codes and their meaning>

## Security & Access

- **Write Permission:** <Does it modify the filesystem?>
- **Network Access:** <Does it make external calls?>

## Usage Example

```bash
npm run msp:index -- --root ./gks
```
