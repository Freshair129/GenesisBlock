// Typed Tauri `invoke` wrappers for the GenesisBlock Mobile backend.
//
// IMPORTANT: import `invoke` from `@tauri-apps/api/core` (Tauri v2 path),
// NOT the v1 `@tauri-apps/api/tauri`.
//
// `invoke` only resolves inside the Tauri runtime (WebView). Calling these
// functions in a plain browser (`vite dev` without `tauri dev`) will reject —
// the app is not standalone-runnable; it must be driven by the Tauri shell.
import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Engine types — mirror the Rust types in `src/lib.rs` exactly.
// ---------------------------------------------------------------------------

export interface LogicalClock {
  time: number;
  peer_id: string;
}

export interface NodeOutput {
  id: string;
  labels: string[];
  props: any;
  impact?: number | null;
  lang?: string | null;
  valid_from: string;
  valid_to?: string | null;
  caused_by?: string | null;
  clock: LogicalClock;
  collection?: string | null;
}

export interface EdgeOutput {
  id: string;
  from: string;
  to: string;
  rel: string;
  props: any;
  valid_from: string;
  valid_to?: string | null;
  impact?: number | null;
}

export interface NeighborOutput {
  node: NodeOutput;
  path: EdgeOutput[];
  depth: number;
}

export interface ContextPackage {
  nodes: NodeOutput[];
  edges: EdgeOutput[];
  super_nodes: any[];
  token_estimate: number;
  reasoning_path: string;
}

export interface NodeInput {
  id?: string | null;
  labels: string[];
  props?: any;
  embedding?: number[] | null;
  lang?: string | null;
  valid_from?: string | null;
  caused_by?: string | null;
  ttl?: number | null;
  collection?: string | null;
}

export interface HybridSearchInput {
  query_vector: number[];
  k: number;
  alpha?: number | null;
  lang?: string | null;
  as_of?: string | null;
  collection?: string | null;
  ef_search?: number | null;
}

export interface DatabaseStatus {
  open: boolean;
  read_only: boolean;
  page_cache_mb: number;
}

// ---------------------------------------------------------------------------
// Command wrappers.
//
// Args are camelCase on the JS side; Tauri converts them to snake_case before
// they reach the Rust `#[tauri::command]` handlers.
// ---------------------------------------------------------------------------

/** Add a node. Vector indexing is async — call `flushIndex()` for read-your-write. */
export const addNode = (input: NodeInput) =>
  invoke<NodeOutput>("add_node", { input });

/** Hybrid (vector + graph) search scoped to a collection. */
export const search = (input: HybridSearchInput) =>
  invoke<NeighborOutput[]>("search", { input });

/** Execute a raw HQL command (SEARCH / TRAVERSE / MATCH / HYBRID / CONTEXT). */
export const executeHql = (query: string) =>
  invoke<any>("execute_hql", { query });

/**
 * Graph Retrieval Layer: build a tiered context package around `targetId`.
 * Primary graph + context source for the app.
 */
export const retrieveContext = (
  targetId: string,
  tier: string,
  budget: number | null,
  fuzzy: boolean,
) =>
  invoke<ContextPackage>("retrieve_context", {
    targetId,
    tier,
    budget,
    fuzzy,
  });

/** Local-graph expansion: neighbors of `seed` out to `depth`. */
export const neighbors = (seed: string, depth: number) =>
  invoke<NeighborOutput[]>("neighbors", { seed, depth });

/** Drain the async HNSW indexing queue (read-your-write). */
export const flushIndex = () => invoke<void>("flush_index");

/** Engine status / diagnostics. */
export const getStatus = () => invoke<DatabaseStatus>("get_status");
