# GenesisBlock Mobile (MARK XVI Phase A)

A Tauri v2 app that embeds the **GenesisBlockDB engine in-process** (no server, no
network) with an Obsidian-style **graph view** (sigma.js) and a **GRL retriever**
panel. The engine is the same `genesis-block-native` crate as desktop, compiled with
`--no-default-features --features mobile`. See [`docs/SPEC--MOBILE-SDK.md`](../docs/SPEC--MOBILE-SDK.md).

## Layout

```
genesisblock-mobile/
├── src/                     # React + TypeScript frontend
│   ├── lib/api.ts           # typed Tauri invoke() wrappers + engine types
│   └── components/
│       ├── GraphView.tsx    # sigma.js WebGL graph (node color = governance tier)
│       └── RetrieverPanel.tsx  # HQL CONTEXT → H0–H6 tier cards
└── src-tauri/               # Rust shell
    ├── src/commands.rs      # 7 Tauri commands over Arc<Storage> (spawn_blocking)
    └── src/lib.rs           # opens DB at app_data_dir()/genesisdb, manages state
```

The graph and the retriever share one data source: `retrieve_context` returns a
`ContextPackage { nodes, edges, ... }` — the subgraph to render. Tapping a node
re-retrieves its context (the Obsidian "click to expand" loop).

## Run (desktop dev — fastest iteration)

```bash
cd genesisblock-mobile
npm install
npm run tauri dev          # Tauri shell + Vite; needs system WebView (WebView2 on Windows)
```

> The frontend cannot run standalone in a browser — every `api.ts` call uses Tauri
> `invoke`, which only resolves inside the Tauri runtime.

## Run (mobile)

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi   # Android (works on Windows)
npm run tauri android init && npm run tauri android dev           # needs Android SDK + NDK

rustup target add aarch64-apple-ios aarch64-apple-ios-sim         # iOS (macOS + Xcode ONLY)
npm run tauri ios init && npm run tauri ios dev
```

> **iOS requires macOS + Xcode** — it cannot be built on the Windows dev host. Cross-compile
> linking for both platforms is gated by `.github/workflows/mobile-build.yml`.

## Commands (`src-tauri/src/commands.rs`)

| Command | Engine call | Returns |
|---|---|---|
| `add_node` | `Storage::add_node` | `NodeOutput` |
| `search` | `hybrid_search` | `NeighborOutput[]` |
| `execute_hql` | `execute_hql` | JSON |
| `retrieve_context` | `retrieve_context` | `ContextPackage` (graph + context) |
| `neighbors` | `neighbors` | `NeighborOutput[]` (local-graph expansion) |
| `flush_index` | `flush_index` | — |
| `get_status` | `status_sync` | `DatabaseStatus` |

## Notes

- `src-tauri/icons/` are **throwaway placeholders** so the Rust crate type-checks.
  Regenerate real icons with `npx tauri icon <source.png>` before bundling.
- This is a standalone crate (not in the root cargo workspace); the root
  `cargo build` / CI does not compile it. Build it from this directory.
