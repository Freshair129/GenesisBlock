# Smart Connections — recommended configuration for MSP

> **Why this guide exists.** MSP delegates *all* embedding work to the
> [Smart Connections](https://github.com/brianpetro/obsidian-smart-connections)
> Obsidian plugin (see
> [`gks/concept/CONCEPT--EMBEDDING-STRATEGY.md`](../../gks/concept/CONCEPT--EMBEDDING-STRATEGY.md)
> and
> [`gks/adr/ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS.md`](../../gks/adr/ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS.md)).
> MSP never embeds. The model the plugin chooses is therefore the model the
> whole passport sees — pick once, re-index, and don't change it without a
> migration plan.

## TL;DR

| Step | Action |
|------|--------|
| 1 | Install Smart Connections from Community Plugins |
| 2 | Set embedding model to `nomic-ai/nomic-embed-text-v1.5` |
| 3 | Trigger a full re-index |
| 4 | Confirm the **Smart View** pane lists similar notes |
| 5 | (Optional) enable the plugin's HTTP bridge for MSP to query |

## 1. Install the plugin

1. Open Obsidian → **Settings → Community plugins**.
2. If you have not already, click **Turn on community plugins**.
3. Click **Browse** → search for **Smart Connections** (author: Brian Joseph
   Petro) → **Install** → **Enable**.
4. After enable, a **Smart View** ribbon icon and pane become available.

> **Vault scope.** Smart Connections indexes the whole vault by default. If
> your `gks/` tree is *part* of a larger vault, restrict the plugin via its
> **Settings → Smart Connections → Excluded folders / files** before the
> first re-index — re-indexing a 10k-note vault on first run can take 10+
> minutes and pin the GPU.

## 2. Pick the embedding model — `nomic-embed-text-v1.5`

In **Settings → Smart Connections → Embedding model**, choose:

```
nomic-ai/nomic-embed-text-v1.5
```

**Why this model:**

- 768-dim, 8192-token context, multilingual — handles English + Thai well
  enough for mixed-language atoms.
- Local by default — Smart Connections downloads ONNX weights once and runs
  them inside the Obsidian renderer process. No data leaves the machine.
- Fits the "GUI-resourced" trade in
  [`CONCEPT--EMBEDDING-STRATEGY`](../../gks/concept/CONCEPT--EMBEDDING-STRATEGY.md)
  — embed cost is absorbed by the Electron process the user is already paying
  for.
- Stable identifier — the version pin (`-v1.5`) means re-indexing on a new
  machine produces vectors a downstream pgvector adapter can still read.

> **Don't mix models.** Once you re-index, every vector under
> `.smart-connections/` is in this model's space. Switching to a different
> model later (e.g. `BAAI/bge-m3`) requires a **full re-index**; partially
> mixed vectors produce garbage similarity scores.

## 3. Re-index the vault

After saving the model choice:

1. Open the **Smart Connections** settings pane.
2. Click **Re-build all embeddings** (label varies by plugin version: also
   shown as **Force refresh** or **Embed all notes**).
3. Wait. The status bar shows progress; first-time index over a fresh
   `gks/` tree (~100 atoms) takes seconds, full vault (thousands of notes)
   takes minutes.

The plugin persists vectors under:

```
<vault>/.smart-connections/
```

This directory is plugin-private — see
[`ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS`](../../gks/adr/ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS.md)
§"Embedding storage is plugin-private". MSP **never** parses these files for
live queries (only for diagnostics).

## 4. Verify

Open any atom under `gks/concept/` and open the **Smart View** pane (ribbon
icon, or **Cmd/Ctrl-P → Smart Connections: View**). You should see a ranked
list of *other* atoms semantically related to the open one.

Quick sanity check:

- Open `gks/concept/CONCEPT--EMBEDDING-STRATEGY.md`.
- Smart View should surface
  `ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS`,
  `CONCEPT--OBSIDIAN-AS-RUNTIME`,
  and other embedding/vector atoms in the top results.

If Smart View is empty, the index isn't built — go back to step 3.

## 5. (Optional) Expose Smart Connections to MSP

For MSP's `msp_recall` to call into Smart Connections live (versus the
filesystem fallback path described in
[`ADR--MSP-OBSIDIAN-INTEGRATION`](../../gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md)),
the plugin needs to be reachable over HTTP. Today, the cleanest path is:

1. Install the **Local REST API** plugin (see
   [`obsidian-local-rest-api.md`](./obsidian-local-rest-api.md)).
2. Smart Connections's request endpoint is then probed by MSP's Obsidian
   client at startup (`createObsidianClient(...)`); when present it is used,
   when absent MSP falls back to text search and flags
   `semantic.available = false` on the result envelope.

A future companion plugin (`msp-bridge`, see roadmap M10a) will give a
versioned HTTP contract; until then, treat Smart Connections's REST surface
as best-effort.

## Troubleshooting

### Model download stalls or fails

Smart Connections fetches ONNX weights on first selection of a new model.
On slow / firewalled networks this can hang silently. Diagnostics:

- Open Obsidian's developer console (**Cmd/Ctrl-Shift-I → Console**) and
  watch for `transformers` / `onnxruntime` log lines.
- Check disk: weights for `nomic-embed-text-v1.5` are ~250 MB; cached under
  `<vault>/.smart-connections/models/` (path varies by plugin version).
- If a partial download is corrupt, delete the model cache and re-select
  the model. The plugin re-downloads.

### GPU vs CPU

The bundled `transformers.js` runtime tries WebGPU → WASM SIMD → WASM
fallback. On a recent macOS / Windows machine WebGPU is fastest; on Linux
without GPU drivers it silently drops to CPU SIMD. Either is fine for
interactive use (one query per agent turn). If embedding is the bottleneck
during re-index:

- Close other Electron apps (each holds its own GPU context).
- On Linux, `obsidian --enable-features=Vulkan` sometimes activates WebGPU
  where it would otherwise stay disabled.

### Thai (or other non-Latin) content scores poorly

`nomic-embed-text-v1.5` is multilingual but English-dominant. For
Thai-heavy vaults:

- Mix in a Thai-aware tokeniser at the *atom* level: write the title in
  English, keep the Thai content in the body. Smart Connections weights
  title + content, so a clear English title pulls similar atoms back.
- Or switch to `BAAI/bge-m3` (also supported by Smart Connections), which
  has stronger multilingual recall but ~4x larger embeddings (1024-dim,
  more disk + RAM). Trigger a **full re-index** if you switch.

### Smart View pane is empty after re-index

- Confirm the plugin is **enabled** (not just installed).
- Check the **Excluded folders** list in plugin settings — `gks/` should
  not be excluded.
- Look for `.smart-connections/embeddings*.json` (or `.ajson` in newer
  versions) in the vault root. If missing, re-index didn't run.

### "Plugin process killed" / OOM during re-index

Big vaults blow Electron's renderer heap. Mitigations:

- Restart Obsidian; re-indexing resumes incrementally.
- Set Smart Connections **Batch size** lower (settings pane).
- Re-index in chunks — exclude folders, index, swap exclusion.

## Cross-references

- [`gks/concept/CONCEPT--EMBEDDING-STRATEGY.md`](../../gks/concept/CONCEPT--EMBEDDING-STRATEGY.md) — why we delegate embedding to the plugin layer
- [`gks/adr/ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS.md`](../../gks/adr/ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS.md) — the architectural decision MSP never embeds
- [`gks/concept/CONCEPT--OBSIDIAN-AS-RUNTIME.md`](../../gks/concept/CONCEPT--OBSIDIAN-AS-RUNTIME.md) — Obsidian as MSP's runtime layer
- [`gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md`](../../gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md) — REST primary / file fallback
- [`obsidian-local-rest-api.md`](./obsidian-local-rest-api.md) — companion HTTP setup
- `msp_spec.md` §7b — embedding strategy in the spec
