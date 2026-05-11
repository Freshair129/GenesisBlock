# Obsidian Local REST API — setup for MSP

> **Why this guide exists.** MSP's Obsidian client
> ([`ADR--MSP-OBSIDIAN-INTEGRATION`](../../gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md))
> talks to a running Obsidian via the
> [Local REST API](https://github.com/coddingtonbear/obsidian-local-rest-api)
> plugin as its **primary** path. When the plugin is reachable, MSP gets
> live vault search, file reads, and the active-file pointer. When it
> isn't, MSP falls back to filesystem-only mode (no semantic recall).

## TL;DR

| Step | Action |
|------|--------|
| 1 | Install the **Local REST API** plugin |
| 2 | Generate / copy the API key |
| 3 | Set `OBSIDIAN_HOST` and `OBSIDIAN_API_KEY` in your shell or `.env` |
| 4 | Verify with a `curl` probe |

## 1. Install the plugin

1. **Settings → Community plugins → Browse** → search **Local REST API**
   (author: Adam Coddington) → **Install** → **Enable**.
2. After enable, the plugin's settings pane shows:
   - HTTPS port (default `27124`)
   - HTTP port (default `27123`, off by default — leave it off)
   - **API Key** — a 64-char hex string
   - Self-signed certificate fingerprint

## 2. Copy the API key

In **Settings → Local REST API**, click the **eye icon** beside the API
key to reveal it, then **Copy**. The key looks like:

```
0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

> Treat this key like an SSH private key. Anyone with it + reachability
> to `127.0.0.1:27124` can read and modify your vault.

## 3. Tell MSP where to find it

MSP looks at, in order:

```
$OBSIDIAN_API_KEY              env var (preferred)
~/.config/msp/obsidian.key     fallback file
```

Pick one. For a per-project setup, use a `.env` next to `package.json`:

```dotenv
# .env  (gitignored — never commit this file)
OBSIDIAN_HOST=https://127.0.0.1:27124
OBSIDIAN_API_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

For a per-user setup, write the key once to disk:

```sh
mkdir -p ~/.config/msp
umask 077
printf '%s\n' '<paste-your-key-here>' > ~/.config/msp/obsidian.key
```

> The repo's `.gitignore` already excludes `.env` and `.brain/`. The key
> file under `~/.config/msp/` is outside the repo entirely.

### Optional knobs

| Variable | Default | Meaning |
|----------|---------|---------|
| `OBSIDIAN_HOST` | `https://127.0.0.1:27124` | Plugin's HTTPS endpoint |
| `OBSIDIAN_API_KEY` | *(unset)* | API key from step 2 |
| `OBSIDIAN_INSECURE` | `false` | Accept the plugin's self-signed cert. MSP only honours this for `127.0.0.1` / `localhost` (see [`ADR--MSP-OBSIDIAN-INTEGRATION`](../../gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md) §"TLS"). |

## 4. Verify

The plugin uses a self-signed certificate by default, so `curl` needs `-k`
(or `--insecure`). The endpoint is bound to `127.0.0.1` only — it won't
respond on a LAN address even if you try.

```sh
curl -k https://127.0.0.1:27124/
```

Expected response — a JSON manifest:

```json
{
  "status": "OK",
  "manifest": {
    "id": "obsidian-local-rest-api",
    "name": "Local REST API",
    "version": "...",
    ...
  },
  "service": "Obsidian Local REST API",
  "authenticated": false
}
```

`"authenticated": false` is normal for `/` — the root endpoint is the
unauthenticated probe MSP uses to detect that the plugin is alive. For
authenticated endpoints, send the bearer token:

```sh
curl -k -H "Authorization: Bearer $OBSIDIAN_API_KEY" \
  https://127.0.0.1:27124/vault/
```

Expected — a JSON listing of top-level vault entries.

If `curl` reports `Connection refused`:

- Obsidian is not running, **or**
- the plugin is installed but disabled, **or**
- the port is different (check **Settings → Local REST API → Encrypted (HTTPS) Server Port**).

If `curl` reports a TLS error without `-k`, that's expected — the cert is
self-signed. MSP handles this internally for localhost only; nothing to
configure.

## What MSP uses this for

Per [`ADR--MSP-OBSIDIAN-INTEGRATION`](../../gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md):

| Capability | Endpoint (subject to plugin version) |
|------------|--------------------------------------|
| Vault text/tag search | `/search/simple/` |
| File read by path | `/vault/<path>` |
| Active file pointer | `/active/` |
| Smart Connections bridge | probed; if present, used for semantic recall |

MSP wraps these behind a single client interface — callers check
`client.mode === 'rest'` before requesting semantic features. When the
plugin is unreachable, `client.mode === 'filesystem'` and MSP works
directly off `gks/<type>/*.md` + `gks/00_index/atomic_index.jsonl`.

## Troubleshooting

### Obsidian crashes on plugin enable

Some Linux distros ship Electron versions that conflict with the plugin's
Node TLS module. Workaround: restart Obsidian; if it still crashes,
disable other community plugins one-by-one to isolate.

### `Connection refused` even though Obsidian is running

- The plugin only binds when **enabled**, not just installed.
- Some firewalls (Little Snitch, ufw with strict outbound) reject
  `127.0.0.1:27124`. Add an explicit allow rule for loopback.
- Windows Defender may flag the plugin's self-signed cert; click **Allow**.

### Self-signed cert keeps failing in MSP

MSP's HTTP client accepts the self-signed cert **only for `127.0.0.1`
or `localhost`**. If you point `OBSIDIAN_HOST` at a remote IP / hostname,
the bypass is intentionally disabled. For dev tunneling (e.g. forwarding
the plugin port over SSH), keep the host as `127.0.0.1` on the MSP side.

### Want to test without the API key?

You can't. The plugin's authenticated endpoints (everything except `/`)
require the bearer token. If you've lost the key, regenerate it in the
plugin settings — note that any client (including MSP) needs the new
value.

## Cross-references

- [`gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md`](../../gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md) — REST primary / file fallback design
- [`gks/concept/CONCEPT--OBSIDIAN-AS-RUNTIME.md`](../../gks/concept/CONCEPT--OBSIDIAN-AS-RUNTIME.md) — Obsidian as MSP's runtime
- [`smart-connections-config.md`](./smart-connections-config.md) — embeddings on top of this REST setup
- `msp_spec.md` §7a — Obsidian-as-runtime in the spec
