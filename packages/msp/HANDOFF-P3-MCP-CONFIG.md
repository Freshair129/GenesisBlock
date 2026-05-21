# HANDOFF P3 — MCP server configuration

> Build status: ✅ `npm install` + `npm run build` complete. `dist/mcp/bin.js` exists. The 11 MSP tools are ready to expose.

## Your config file

Detected on this machine: **`C:\Users\freshair\AppData\Roaming\Claude\claude_desktop_config.json`** (Claude Desktop on Windows).

Current contents have only a `preferences` block — no `mcpServers` yet.

## Snippet to add

Merge this `mcpServers` key into the config:

```jsonc
{
  "mcpServers": {
    "msp": {
      "command": "node",
      "args": [
        "G:\\msp\\dist\\mcp\\bin.js",
        "--root=G:\\msp"
      ],
      "env": {
        "OBSIDIAN_URL": "https://127.0.0.1:27124",
        "OBSIDIAN_API_KEY": "<paste-from-Obsidian-Local-REST-API-plugin>",
        "MSP_LLM_PROVIDER": "ollama"
      }
    }
  }
}
```

Notes on the choices:

- **`command: node` + absolute path to `dist/mcp/bin.js`** instead of `npx msp-mcp-server`. The HANDOFF.md template uses `npx`, but on this Windows setup the package isn't published to npm and `node` against the built bin is more direct + less fragile.
- **`--root=G:\\msp`** points at the repo so MSP can read `gks/`, `.brain/`, and write to `.brain/msp/projects/<ns>/`.
- **`OBSIDIAN_API_KEY`** is optional. Leave it as the placeholder if you don't run Obsidian — MSP will fall back to filesystem-only mode for `msp_recall`. To enable: install Obsidian's "Local REST API" plugin, copy the key from its settings.
- **`MSP_LLM_PROVIDER=ollama`** assumes you have Ollama running locally. To use mock instead, set to `mock`.

## Final merged file (ready to write)

If you want the full file as it would look after the merge, here it is:

```json
{
  "preferences": {
    "localAgentModeTrustedFolders": [
      "G:\\cognitive_system"
    ],
    "allowAllBrowserActions": true,
    "coworkScheduledTasksEnabled": true,
    "ccdScheduledTasksEnabled": true,
    "sidebarMode": "task",
    "bypassPermissionsModeEnabled": true,
    "autoPermissionsModeEnabled": true,
    "dockBounceEnabled": true,
    "coworkWebSearchEnabled": true,
    "keepAwakeEnabled": true,
    "coworkOnboardingResumeStep": null,
    "chicagoEnabled": true,
    "ccAutoArchiveOnPrClose": true
  },
  "mcpServers": {
    "msp": {
      "command": "node",
      "args": [
        "G:\\msp\\dist\\mcp\\bin.js",
        "--root=G:\\msp"
      ],
      "env": {
        "OBSIDIAN_URL": "https://127.0.0.1:27124",
        "OBSIDIAN_API_KEY": "<paste-from-Obsidian-Local-REST-API-plugin>",
        "MSP_LLM_PROVIDER": "ollama"
      }
    }
  }
}
```

## After applying

1. Restart Claude Desktop.
2. The 11 MSP tools should appear in the tool list:
   `msp_validate`, `msp_propose`, `msp_run_task`, `msp_session_append`, `msp_episode_append`, `msp_backlinks_rebuild`, `msp_recall`, `msp_remember`, `msp_compress`, `msp_identity_get`, `msp_identity_set`.

## Optional plugins

- **Obsidian + Local REST API plugin** — enables `msp_recall`'s REST text-search path
- **Smart Connections plugin** (configured to use `nomic-embed-text-v1.5`) — human-side semantic browse in Obsidian

Setup guides under [`examples/setup/`](./examples/setup/).
