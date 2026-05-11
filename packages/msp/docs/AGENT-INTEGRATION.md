# Agent integration

This is the practical wire-in guide for plugging MSP into a cognitive-layer
client. The contract every client honors is documented in
`gks/concept/CONCEPT--AGENT-INTEGRATION-PATTERNS.md`; this file is the copy-paste
companion.

> **TL;DR**: every client launches `msp-mcp-server` as an MCP stdio server.
> Project is selected via `MSP_PROJECT` env var (preferred — universal across
> clients) or `.mspconfig` in the project root. Identity is read from
> `~/.msp/identity.json` (or `$MSP_HOME/identity.json`).

## Prerequisites

1. Install MSP so `msp-mcp-server` is on `PATH`:

   ```bash
   git clone https://github.com/<your-fork>/msp ~/msp
   cd ~/msp && npm ci && npm run build
   npm link               # or use absolute path to ./dist/mcp/bin.js in configs
   ```

   On systems where `npm link` is not desirable, replace `"command": "msp-mcp-server"`
   with `"command": "node"` and `"args": ["/abs/path/to/msp/dist/mcp/bin.js"]`
   in any of the snippets below.

2. (Optional) Initialize the global root so identity persists across projects:

   ```bash
   mkdir -p ~/.msp
   # First call to msp_identity_get from any client will create
   # ~/.msp/identity.json with defaults if it doesn't exist.
   ```

3. (Optional) Set `MSP_HOME` if you want the global root somewhere other than
   `~/.msp/` (e.g. shared machine with per-account roots, CI sandbox):

   ```bash
   export MSP_HOME=/var/lib/msp/$USER
   ```

## Client-specific snippets

### Claude Code

Claude Code reads MCP server config from `~/.claude/mcp.json` (user-global) or
the project's `.claude/settings.json`. The user-global form:

```jsonc
// ~/.claude/mcp.json
{
  "mcpServers": {
    "msp": {
      "command": "msp-mcp-server",
      "env": {
        "MSP_PROJECT": "default"
      }
    }
  }
}
```

Project-scoped (override per repo) — drop the same block under `mcpServers` in
`.claude/settings.json` at the repo root. Project-scoped wins for that workspace.

To switch projects without editing config, set `MSP_PROJECT` in the shell
that spawns Claude Code:

```bash
MSP_PROJECT=clinic claude
```

To point at a non-default global root:

```jsonc
{
  "mcpServers": {
    "msp": {
      "command": "msp-mcp-server",
      "env": {
        "MSP_HOME": "/Users/me/Library/Application Support/msp"
      }
    }
  }
}
```

**Verify it's live**: from inside Claude Code, run `/mcp` to list servers, or
ask the agent to call `msp_recall` with a short query. If `msp` is listed and
the call returns (even with empty results), the integration is wired.

### Gemini CLI

Gemini CLI honors MCP servers declared in its user config. The general shape
(see official Gemini CLI MCP integration docs for the exact path on your
platform):

```jsonc
{
  "mcpServers": {
    "msp": {
      "command": "msp-mcp-server",
      "env": {
        "MSP_PROJECT": "default"
      }
    }
  }
}
```

Project switching is the same as Claude Code — `MSP_PROJECT` env var either in
the config block or in the shell.

**Verify it's live**: list available tools from the Gemini CLI prompt and
confirm the `msp_*` tools appear. Run `msp_recall "test"` for a smoke test.

### Antigravity

Antigravity has built-in MCP support. The standard MCP server config shape:

```jsonc
{
  "mcpServers": {
    "msp": {
      "command": "msp-mcp-server",
      "env": {
        "MSP_PROJECT": "default"
      }
    }
  }
}
```

Place this in Antigravity's MCP configuration file (see Antigravity's official
docs for the exact path). Project switching and `MSP_HOME` overrides work the
same way.

**Verify it's live**: tool listing in Antigravity's UI should show
`msp_validate`, `msp_recall`, `msp_remember`, etc.

### Cursor

Cursor reads MCP config from `~/.cursor/mcp.json`:

```jsonc
{
  "mcpServers": {
    "msp": {
      "command": "msp-mcp-server",
      "env": {
        "MSP_PROJECT": "default"
      }
    }
  }
}
```

To switch project per workspace, override in the workspace's `.cursor/mcp.json`
(same shape, different path).

**Verify it's live**: open Cursor settings → MCP, confirm `msp` shows green /
connected. Tool calls become available via `@msp` mentions or the agent's
auto-routing.

### Codex / ChatGPT custom agents

Codex-style custom agents typically take a TOML or JSON manifest pointing at
MCP servers. The JSON form:

```jsonc
{
  "mcpServers": {
    "msp": {
      "command": "msp-mcp-server",
      "env": {
        "MSP_PROJECT": "default"
      }
    }
  }
}
```

Or the TOML form some agents prefer:

```toml
[mcpServers.msp]
command = "msp-mcp-server"

[mcpServers.msp.env]
MSP_PROJECT = "default"
```

Both shapes carry identical semantics. Pick whichever the host expects.

**Verify it's live**: per the host agent's MCP introspection — usually a
"list tools" or "describe servers" admin call.

### Custom agents (TypeScript)

Roughly 15 lines using `@modelcontextprotocol/sdk`:

```typescript
import { Client } from '@modelcontextprotocol/sdk/client/index.js'
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js'

const transport = new StdioClientTransport({
  command: 'msp-mcp-server',
  env: { ...process.env, MSP_PROJECT: 'default' },
})

const client = new Client({ name: 'my-agent', version: '0.1.0' }, { capabilities: {} })
await client.connect(transport)

const tools = await client.listTools()
console.log(tools.tools.map((t) => t.name))

const result = await client.callTool({
  name: 'msp_recall',
  arguments: { query: 'test', limit: 3 },
})
console.log(result)
```

To switch projects, change `MSP_PROJECT` in the `env` map. To override the
global root, add `MSP_HOME` to the same map.

**Verify it's live**: the `listTools()` call returns ~16 tools whose names
start with `msp_`.

### Custom agents (Python)

Equivalent shape using the official MCP Python SDK:

```python
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
import os

params = StdioServerParameters(
    command="msp-mcp-server",
    env={**os.environ, "MSP_PROJECT": "default"},
)

async with stdio_client(params) as (read, write):
    async with ClientSession(read, write) as session:
        await session.initialize()
        tools = await session.list_tools()
        print([t.name for t in tools.tools])

        result = await session.call_tool("msp_recall", {"query": "test", "limit": 3})
        print(result)
```

Project switching and `MSP_HOME` work identically — they're env-var driven, the
SDK is just a transport.

**Verify it's live**: `list_tools()` returns the `msp_*` surface.

## Switching projects

The portable answer is `MSP_PROJECT` env var. It works in every shape:

- MCP-native clients: under `"env"` in the config block.
- MCP-bridged clients: in the env map passed to `StdioClientTransport` /
  `StdioServerParameters`.
- Shell-wrapped: `export MSP_PROJECT=...` before the wrapped command.

For project-bound switching (one repo always uses one project), drop a
`.mspconfig` at the repo root:

```yaml
# .mspconfig
project: clinic
```

`.mspconfig` is searched in cwd and ancestors; the nearest one wins. CLI flag
or env var overrides it.

> **Status note**: the named-project registry (`~/.msp/projects.yaml`) is
> aspirational per `CONCEPT--NAMED-PROJECT-REGISTRY`. Until it ships, the
> `MSP_PROJECT` value is treated as a directory namespace under
> `.brain/msp/projects/<ns>/`. The wire-in pattern doesn't change once the
> registry lands.

## Verifying the integration is live

For any client, the smoke test is identical:

1. Confirm tool discovery — the agent / client should list ~16 tools whose
   names start with `msp_`.
2. Call `msp_recall` with a trivial query (`{ "query": "test" }`). Empty
   results are fine; an error is not.
3. Optional: call `msp_identity_get` and confirm the response includes the
   identity block (defaults if `~/.msp/identity.json` was just created).

If any of the above fails, common causes:

- `msp-mcp-server` not on `PATH` → use absolute path in `command`.
- Wrong cwd → MSP defaults to `process.cwd()`; pass `--root=<abs>` via `args`
  if the client launches the bin from a directory you don't control.
- Stale build → `cd ~/msp && npm run build` after pulling.
- Project namespace doesn't exist on disk — first `msp_remember` creates it,
  but a query before any writes against an empty namespace returns `[]`,
  not an error.

## Known caveats

### Windows path quirks

- `~/.msp/` on Windows resolves to `%USERPROFILE%\.msp\` (e.g.
  `C:\Users\me\.msp\`). MSP uses `os.homedir()`, which is correct on
  Windows; do not hardcode forward slashes in `MSP_HOME`.
- MCP server config files on Windows may need backslash-escaping in JSON
  (`"C:\\Users\\me\\.msp"`).
- `npm link` on Windows requires admin shell or a manually-created shim;
  vendoring with absolute paths to `dist/mcp/bin.js` is often simpler.

### `MSP_HOME` on shared machines

- If `~/` is shared (some lab setups, classroom VMs), set per-user `MSP_HOME`
  in the shell rc to keep identity / projects separate. Do not let two users
  share `~/.msp/` — identity merges last-write-wins, which is wrong for
  multi-tenant.
- For CI: set `MSP_HOME=$RUNNER_TEMP/msp` to get a fresh global root per
  pipeline run.

### Process lifecycle

- Some clients keep the MCP server alive for the whole session; others spawn
  per-call. Both work. Per-call is slower (cold start ~100ms) but wholly
  stateless from the client's perspective.
- The MSP server has no idle timeout of its own — it lives as long as the
  client's stdio pipe is open.

### Permission gates

- Clients with permission models (Claude Code, Antigravity) prompt on first
  call to a `msp_*` tool. Approve once at the user-global level if you want
  silent operation across projects.
- Tool-name allow-lists, if your client supports them, should at minimum
  include `msp_recall`, `msp_remember`, `msp_identity_get` for read-side
  agents and add `msp_candidate`, `msp_session_append`, `msp_episode_append`
  for write-side agents.

## What MSP does NOT provide

Per `CONCEPT--AGENT-AGNOSTIC` ("What MSP does not own"):

- **No slash-command framework.** `/msp-recall` is a Claude Code convention,
  not an MSP feature. Each client maps tools to UX in its own way.
- **No agent identity beyond `identity.json`.** The `Identity` schema
  (`name`, `role`, `voice`, `preferences`, `guardrails`) is universal. There
  is no MSP concept of "agent type", "tier", "stakes-level" — those belong to
  the consuming project (e.g. EVA's cognitive-layer spec).
- **No `MSP-IMP-` / `MSP-TSK-` / `MSP-WKT-` process IDs.** These are EVA
  project artifacts (per GKS `docs/MSP_RELATIONSHIP.md`); MSP does not own
  them and clients should not try to read them through MSP tools.
- **No multi-agent orchestration.** Spawning subagents, routing between
  agents, fan-out / fan-in patterns — all client concerns.
- **No model selection.** MSP is a memory/passport layer; the LLM the agent
  is talking to is selected by the client (or the agent's own config), not
  by MSP.

If a client team needs any of the above, build it in the client's own layer
and have it consume the MSP MCP surface — don't try to push it down into MSP.
