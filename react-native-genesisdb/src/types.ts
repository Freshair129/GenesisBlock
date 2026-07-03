/**
 * Wire-format types for react-native-genesisdb.
 *
 * These fields are intentionally **snake_case**, matching the engine's raw
 * `serde_json` output (no `rename_all`) that crosses the JNI (Android) / C ABI
 * (iOS) boundary — see src/jni.rs and src/ffi.rs. That is NOT the camelCase
 * seen in the Node addon's index.d.ts (napi-rs's own binding convention,
 * unrelated to the actual wire bytes). This mirrors the precedent set by
 * genesisdb-python and genesisdb-go, which also use wire-matching field
 * names rather than translating to their host language's idiom.
 *
 * Deliberate non-choice: a generic deep camelCase<->snake_case converter was
 * considered and rejected — `NodeInput.props`/`NodeOutput.props` is an
 * opaque, caller-defined JSON value (e.g. `{ userName: "Ada" }`). A blind
 * recursive key-casing pass would silently rewrite the caller's own object
 * keys inside `props`, corrupting their data. Passing the wire shape straight
 * through avoids that trap entirely.
 */

export interface LogicalClock {
  time: number;
  peer_id: string;
}

export interface NodeInput {
  id?: string;
  labels: string[];
  props?: unknown;
  embedding?: number[];
  lang?: string;
  valid_from?: string;
  caused_by?: string;
  ttl?: number;
  collection?: string;
}

export interface NodeOutput {
  id: string;
  labels: string[];
  props: unknown;
  impact?: number;
  embedding?: number[];
  lang?: string;
  valid_from: string;
  valid_to?: string;
  caused_by?: string;
  expires_at?: string;
  clock: LogicalClock;
  collection?: string;
}

export interface EdgeOutput {
  id: string;
  from: string;
  to: string;
  rel: string;
  props: unknown;
  valid_from: string;
  valid_to?: string;
  recorded_at: string;
  superseded_by?: string;
  impact?: number;
  caused_by?: string;
  clock: LogicalClock;
}

export interface SuperNode {
  cluster_id: number;
  theme: string;
  member_count: number;
  impact: number;
  centroid: number[];
  timestamp: string;
  drift?: number;
}

export interface ContextPackage {
  nodes: NodeOutput[];
  edges: EdgeOutput[];
  super_nodes: SuperNode[];
  token_estimate: number;
  reasoning_path: string;
}

export interface NeighborOutput {
  node: NodeOutput;
  path: EdgeOutput[];
  depth: number;
  score?: number;
}

export interface HybridSearchInput {
  query_vector: number[];
  k: number;
  alpha?: number;
  lang?: string;
  as_of?: string;
  collection?: string;
  ef_search?: number;
  oversample?: number;
}

export class GenesisDBError extends Error {
  constructor(message: string, public readonly code?: string) {
    super(message);
    this.name = 'GenesisDBError';
  }
}
