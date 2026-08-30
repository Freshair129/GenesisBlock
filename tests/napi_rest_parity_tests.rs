//! NAPI ↔ REST parity contract test.
//!
//! GenesisBlockDB is "one core, two front-ends" (CLAUDE.md): the same
//! `Arc<Storage>` is wrapped as NAPI `async`/sync methods on `impl
//! GenesisDatabase` in `src/lib.rs`, and as Axum handlers under `/v1/*` in
//! `src/router.rs`. CLAUDE.md warns these two surfaces "can drift, so check
//! both" — this test makes that check automatic instead of relying on someone
//! remembering to look.
//!
//! Design: this is a plain Rust integration test that parses SOURCE TEXT — it
//! never links against the `napi` crate or the `GenesisDatabase` NAPI type, so
//! it runs identically with or without the `napi-bindings` feature:
//!
//!     cargo test --no-default-features --test napi_rest_parity_tests
//!
//! It reads `src/lib.rs` and extracts every `#[napi]`-tagged method on `impl
//! GenesisDatabase` (both `pub fn` and `pub async fn` — the impl block itself
//! is `#[cfg(feature = "napi-bindings")] #[napi]`-gated, and each method
//! inside carries its own `#[napi]` / `#[napi(factory)]` attribute directly,
//! NOT a per-method `cfg_attr` as CLAUDE.md's shorthand summary might suggest —
//! `cfg_attr` gating is used for the `#[napi(object)]` plain-data structs
//! instead). It reads `src/router.rs` and extracts every `.route("...", ...)`
//! path literal. Then `NAPI_TO_REST` below is checked against both extracted
//! sets in both directions, so drift in either direction fails the build:
//!
//!   a) a new `#[napi]` method with no table entry              -> fails
//!   b) a table entry whose REST path no longer exists           -> fails
//!   c) a stale table entry for a method that no longer exists   -> fails
//!
//! `None` entries (deliberately NAPI-only, or in `open`'s case, not a
//! REST-shaped capability at all) are asserted to each carry a `//`
//! justification comment in this file's own source (see
//! `every_none_entry_has_a_justifying_comment`).

use std::collections::HashSet;
use std::fs;

// ---------------------------------------------------------------------------
// The parity table — the single source of truth this test enforces.
// ---------------------------------------------------------------------------

/// `(napi_method_name, Some(rest_path) | None)`.
///
/// Built from the real current surface (verified against `src/lib.rs` /
/// `src/router.rs` at the time this test was authored — see PR description for
/// the audit). Every `None` below carries a `//` comment justifying why that
/// method is intentionally NAPI-only (or, for `open`, why it has no
/// REST-shaped analogue at all).
const NAPI_TO_REST: &[(&str, Option<&str>)] = &[
    // Constructor — the REST server opens its own `Storage` once at startup
    // (see `src/main.rs`), so there is no per-request REST analogue to "open".
    ("open", None),
    ("bulk_add_nodes", Some("/v1/bulk/nodes")),
    ("bulk_add_edges", Some("/v1/bulk/edges")),
    // rebuild_index_handler calls `Storage::rebuild_index_parallel` directly.
    ("rebuild_index_parallel", Some("/v1/bulk/rebuild")),
    ("add_node", Some("/v1/node/add")),
    ("add_edge", Some("/v1/edge/add")),
    (
        "register_relational_schema",
        Some("/v1/relational/schema/register"),
    ),
    (
        "get_relational_schema",
        Some("/v1/relational/schema/:namespace"),
    ),
    ("apply_relational_batch", Some("/v1/relational/mutate")),
    // Compatibility-only embedded API; REST intentionally requires the U2 batch envelope.
    ("apply_relational_rows", None),
    // Compatibility-only embedded API; REST intentionally permits named queries only.
    ("query_relational", None),
    // Deliberately in-process only, for now. Network-reachable SQL is a
    // different class of exposure from an embedded call: the surface can read
    // the whole projection, including every node's props, with no per-caller
    // separation (SPEC--GENESISDB-READONLY-SQL-SURFACE §5, §8). Exposing it
    // over REST is a separate decision to take once the embedded surface has
    // been used in anger, not a step to skip quietly.
    ("query_sql", None),
    ("execute_named_query", Some("/v1/relational/query")),
    ("commit_transaction", Some("/v1/transaction/commit")),
    ("stable_frontier", Some("/v1/frontier")),
    // WP-1.2 frontier split (ADR D2.3): txn frontier rides the same REST route
    // (the `/v1/frontier` body now returns {frame, txn}).
    ("txn_frontier", Some("/v1/frontier")),
    // WP-1.3: history horizon rides the same frontier surface on REST.
    ("history_horizon", Some("/v1/frontier")),
    // WP-2.1: tx-time version chain + resolve-at-commit.
    ("node_versions", Some("/v1/node/versions")),
    ("supersede_node", Some("/v1/node/supersede")),
    ("retract_edge", Some("/v1/edge/retract")),
    // GRL tiered retrieval (target_id/tier/budget/fuzzy). NOT the same as
    // `/v1/reason/context`, which calls `Storage::get_ranked_context`
    // (HybridSearchInput-based) — the two routes coexist deliberately.
    ("retrieve_context", Some("/v1/context/retrieve")),
    ("execute_hql", Some("/v1/query/hql")),
    ("execute_query_ir", Some("/v1/query/ir")),
    ("query_ir_capabilities", Some("/v1/query/ir/capabilities")),
    ("hybrid_search", Some("/v1/search/hybrid")),
    // No direct REST route; graph traversal is reachable via HQL TRAVERSE
    // through /v1/query/hql (see tests/rest_api_tests.rs::test_hql_traverse_finds_neighbor).
    ("neighbors", None),
    // Ops-only manual snapshot trigger; deliberately not remote-triggerable
    // over HTTP (no admin/ops surface exists on the REST API for this yet).
    ("save_state", None),
    // Ops-only WAL compaction trigger; same reasoning as save_state.
    ("compact", None),
    ("create_collection", Some("/v1/collection/create")),
    ("list_collections", Some("/v1/collections")),
    ("add_vector", Some("/v1/vector/add")),
    // Test/ops-only sync helper for HNSW async-indexing read-your-writes
    // (ADR--GENESISDB-ASYNC-INDEXING); not meant to be a network-triggerable op.
    ("flush_index", None),
    // Folded into ExtendedStatus.index_lag: status_handler calls
    // `storage.index_lag()` directly rather than through its own route.
    ("index_lag", Some("/v1/status")),
    // Embedded-SDK-only tuning knob (per-language centroid), no REST route.
    ("set_language_centroid", None),
    // Embedded-SDK-only tuning knob (HNSW ef_construction/ef_search), no REST route.
    ("set_index_params", None),
    // Composed server-side into /v1/insight/rebuild together with
    // generate_meta_graph (insight_rebuild_handler calls both); no route of its own.
    ("detect_communities", None),
    ("calculate_structural_gaps", Some("/v1/insight/gaps")),
    // Composed into /v1/insight/rebuild together with detect_communities; no
    // route of its own (see previous comment).
    ("generate_meta_graph", None),
    ("get_meta_history", Some("/v1/insight/drift/:cluster_id")),
    // CRDT anti-entropy sink — peer-to-peer sync internals, not part of the
    // row-data HTTP surface (mirrors events_since below).
    ("reconcile_state", None),
    // CRDT anti-entropy source; pairs with reconcile_state, same reasoning.
    ("events_since", None),
    ("semantic_verify", Some("/v1/consensus/verify")),
    ("propose_consensus", Some("/v1/consensus/propose")),
    ("submit_vote", Some("/v1/consensus/vote")),
    ("sign_vote", Some("/v1/consensus/sign-vote")),
    // Folded into SwarmStatus.peer_id: swarm_status_handler reads
    // `storage.local_peer_id` directly (same data, not via this getter).
    ("get_local_peer_id", Some("/v1/swarm/status")),
    // Folded into SwarmStatus.logical_clock (same route as above).
    ("get_logical_clock", Some("/v1/swarm/status")),
    // Folded into SwarmStatus.merkle_root (same route as peer_id/clock).
    ("get_merkle_root", Some("/v1/swarm/status")),
    // version_handler returns schema_version from `crate::SCHEMA_VERSION` directly.
    ("schema_version_sync", Some("/v1/version")),
    // version_handler returns version from `crate::ENGINE_VERSION` directly.
    ("version_sync", Some("/v1/version")),
    ("status_sync", Some("/v1/status")),
];

// ---------------------------------------------------------------------------
// Extraction helpers — pure text parsing, no napi linkage.
// ---------------------------------------------------------------------------

fn read_lib_rs() -> String {
    fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("failed to read src/lib.rs")
}

fn read_router_rs() -> String {
    fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/router.rs"))
        .expect("failed to read src/router.rs")
}

fn read_self() -> String {
    fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/napi_rest_parity_tests.rs"
    ))
    .expect("failed to read this test's own source")
}

/// Slice out the `impl GenesisDatabase { ... }` block by brace-depth counting
/// from the first `{` after `marker`. Assumes no unbalanced `{`/`}` inside
/// string literals or comments within the block (true of the current source —
/// method bodies here are plain `tokio::task::spawn_blocking` calls with no
/// string literals containing braces).
fn extract_impl_block<'a>(src: &'a str, marker: &str) -> &'a str {
    let start = src
        .find(marker)
        .unwrap_or_else(|| panic!("marker {marker:?} not found in src/lib.rs"));
    let brace_start = src[start..]
        .find('{')
        .map(|i| start + i)
        .expect("no opening brace after marker");
    let bytes = src.as_bytes();
    let mut depth = 0i32;
    let mut end = brace_start;
    for (i, &b) in bytes[brace_start..].iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = brace_start + i;
                    break;
                }
            }
            _ => {}
        }
    }
    assert!(
        end > brace_start,
        "impl block never closed (unbalanced braces?)"
    );
    &src[brace_start..=end]
}

/// Extract a `fn` name from a line containing `fn <name>(`.
fn extract_fn_name(line: &str) -> Option<String> {
    let idx = line.find("fn ")?;
    let name: String = line[idx + 3..]
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Within an `impl` block's source text, collect the name of every method
/// whose immediately preceding non-comment, non-blank line is a `#[napi...]`
/// attribute (`#[napi]`, `#[napi(factory)]`, etc.) — this covers both `pub
/// fn` and `pub async fn` items, since real `#[napi]` methods on
/// `GenesisDatabase` are a mix of both.
fn extract_napi_methods(block: &str) -> Vec<String> {
    let mut methods = Vec::new();
    let mut pending_napi = false;
    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[napi") {
            pending_napi = true;
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with("//") {
            // Doc comments / blank lines between the attribute and the fn
            // signature don't cancel the pending marker.
            continue;
        }
        if pending_napi {
            if let Some(name) = extract_fn_name(trimmed) {
                methods.push(name);
            }
            // Whether or not this line turned out to be a fn signature, the
            // attribute has now been "consumed" by the next real line.
            pending_napi = false;
        }
    }
    methods
}

/// Extract every `.route("<path>", ...)` path literal from router source text.
fn extract_routes(src: &str) -> HashSet<String> {
    let mut routes = HashSet::new();
    let needle = ".route(";
    let mut search_from = 0usize;
    while let Some(rel) = src[search_from..].find(needle) {
        let after = search_from + rel + needle.len();
        let rest = &src[after..];
        let rest_trimmed = rest.trim_start();
        if let Some(quote_rest) = rest_trimmed.strip_prefix('"') {
            if let Some(end) = quote_rest.find('"') {
                routes.insert(quote_rest[..end].to_string());
            }
        }
        search_from = after;
    }
    routes
}

fn extracted_napi_methods() -> Vec<String> {
    let lib_src = read_lib_rs();
    // `impl GenesisDatabase {` is unique in src/lib.rs — the struct definition
    // just above it is `pub struct GenesisDatabase {`, a different literal.
    let block_owned;
    let methods = {
        // Work around the borrow: extract_impl_block borrows lib_src, so we
        // must finish using it (collect owned Strings) before lib_src drops.
        block_owned = extract_impl_block(&lib_src, "impl GenesisDatabase {").to_string();
        extract_napi_methods(&block_owned)
    };
    methods
}

// ---------------------------------------------------------------------------
// Sanity checks on the extraction itself (fail loudly if parsing breaks
// instead of silently returning an empty/wrong set that would make the real
// assertions vacuously pass).
// ---------------------------------------------------------------------------

#[test]
fn extraction_sanity_finds_known_methods() {
    let methods = extracted_napi_methods();
    for known in ["bulk_add_nodes", "add_node", "execute_hql", "status_sync"] {
        assert!(
            methods.iter().any(|m| m == known),
            "extraction must find known NAPI method {known:?}; found: {methods:?}"
        );
    }

    let router_src = read_router_rs();
    let routes = extract_routes(&router_src);
    for known in ["/v1/node/add", "/v1/status", "/v1/batch"] {
        assert!(
            routes.contains(known),
            "extraction must find known route {known:?}; found: {routes:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// The three drift-catching assertions.
// ---------------------------------------------------------------------------

/// (a) Every extracted NAPI method appears in the table — a new `#[napi]`
/// method landing without a declared REST story fails this.
#[test]
fn every_napi_method_is_in_the_table() {
    let methods = extracted_napi_methods();
    let table_keys: HashSet<&str> = NAPI_TO_REST.iter().map(|(k, _)| *k).collect();

    let mut missing: Vec<&String> = methods
        .iter()
        .filter(|m| !table_keys.contains(m.as_str()))
        .collect();
    missing.sort();
    missing.dedup();
    assert!(
        missing.is_empty(),
        "NAPI method(s) with no NAPI_TO_REST entry (add a REST mapping or a \
         documented `None`): {missing:?}"
    );
}

/// (b) Every `Some(path)` in the table exists in the extracted route set —
/// a route rename/removal without updating the table fails this.
#[test]
fn every_table_rest_path_still_exists_as_a_route() {
    let router_src = read_router_rs();
    let routes = extract_routes(&router_src);

    let mut missing: Vec<&str> = NAPI_TO_REST
        .iter()
        .filter_map(|(_, v)| *v)
        .filter(|p| !routes.contains(*p))
        .collect();
    missing.sort();
    missing.dedup();
    assert!(
        missing.is_empty(),
        "NAPI_TO_REST references REST path(s) that no longer exist in \
         src/router.rs: {missing:?}"
    );
}

/// (c) Every table key exists in the extracted NAPI set — a stale entry left
/// behind after a method is renamed/removed fails this.
#[test]
fn every_table_key_is_a_real_napi_method() {
    let methods: HashSet<String> = extracted_napi_methods().into_iter().collect();

    let mut stale: Vec<&str> = NAPI_TO_REST
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| !methods.contains(*k))
        .collect();
    stale.sort();
    stale.dedup();
    assert!(
        stale.is_empty(),
        "NAPI_TO_REST has stale entries no longer present as #[napi] methods \
         on impl GenesisDatabase: {stale:?}"
    );
}

/// Every `None` mapping in the table must carry a `//` justification comment
/// in this file's own source — either inline on the same line, or as a
/// comment on the line immediately above the tuple (the style used
/// throughout the table above) — so "intentionally NAPI-only" is never
/// silently asserted without a reason on record.
#[test]
fn every_none_entry_has_a_justifying_comment() {
    let this_src = read_self();
    let table_start = this_src
        .find("const NAPI_TO_REST")
        .expect("NAPI_TO_REST table not found in this file");
    let table_end = table_start
        + this_src[table_start..]
            .find("];")
            .expect("end of NAPI_TO_REST table (`];`) not found");
    let table_src = &this_src[table_start..table_end];
    let lines: Vec<&str> = table_src.lines().collect();

    let mut unjustified = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.contains(", None)") {
            let has_inline_comment = line.contains("//");
            let has_preceding_comment = i > 0 && lines[i - 1].trim_start().starts_with("//");
            if !has_inline_comment && !has_preceding_comment {
                unjustified.push((i, line.to_string()));
            }
        }
    }
    assert!(
        unjustified.is_empty(),
        "NAPI_TO_REST line(s) map to None without a `//` justification \
         comment (inline or on the preceding line): {unjustified:?}"
    );
}

// ---------------------------------------------------------------------------
// NAPI <-> index.d.ts parity.
//
// `index.d.ts` and `index.js` are napi-generated but COMMITTED, so a fresh
// clone and every registry consumer can `require()` the package before any
// local build. `.gitignore` says in as many words that they must be kept in
// sync with src/lib.rs by hand, and nothing enforced it: PR #160 added
// `#[napi] pub async fn query_sql` and very nearly shipped without `querySql`
// in the declarations, which would have left a TypeScript consumer of the
// published package unable to see that the method exists. It was caught by
// reading the file, which is not a mechanism.
//
// Why this is a source-text test and not a "regenerate and diff" CI job, which
// is the obvious shape and the one the C header gate uses:
//
//   index.d.ts is NOT purely generated. `QueryIrRequest`, `QueryIrResponse`,
//   `QueryIrCapabilities` and friends are hand-written interfaces, and
//   `executeQueryIr` / `queryIrCapabilities` carry hand-written signatures that
//   refer to them. napi cannot generate any of it: the Rust types are plain
//   serde structs (`QueryIrOperation` is an enum with struct variants, which
//   `#[napi(object)]` cannot express), and the NAPI methods take and return
//   `serde_json::Value`, which napi renders as `any`. A regenerate-and-diff
//   gate would therefore go red on a CLEAN tree, and the way to make it green
//   would be to commit a regeneration that DELETES those declarations - turning
//   a freshness gate into the cause of the drift it exists to catch.
//
// So this compares NAMES, in both directions, and deliberately says nothing
// about signatures. What it catches: a NAPI method, top-level function or
// `#[napi(object)]` struct that is not declared, and a declaration left behind
// for something that no longer exists. What it does NOT catch, stated plainly
// rather than left to be discovered: a signature that drifts while the name
// stays - a parameter added, a type changed. Closing that gap means generating,
// and generating stays unsafe until the QueryIr declarations are either emitted
// by napi or moved out of the generated file.
// ---------------------------------------------------------------------------

fn read_index_dts() -> String {
    fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/index.d.ts"))
        .expect("failed to read index.d.ts")
}

/// napi's default JS name for a Rust item: `add_node` -> `addNode`.
fn to_camel_case(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut upper_next = false;
    for c in snake.chars() {
        if c == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(c.to_uppercase());
            upper_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// The body of `export declare class GenesisDatabase { ... }`.
fn dts_class_body(dts: &str) -> &str {
    let marker = "export declare class GenesisDatabase {";
    let start = dts
        .find(marker)
        .expect("`export declare class GenesisDatabase {` not found in index.d.ts")
        + marker.len();
    let end = dts[start..]
        .find("\n}")
        .map(|i| start + i)
        .expect("GenesisDatabase class block never closed in index.d.ts");
    &dts[start..end]
}

/// Method names declared on the class. Members are two-space indented; doc
/// comment lines and blank lines are skipped.
fn dts_declared_methods(dts: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    for line in dts_class_body(dts).lines() {
        let Some(rest) = line.strip_prefix("  ") else {
            continue;
        };
        if rest.starts_with(' ') || rest.starts_with('*') || rest.starts_with('/') {
            continue;
        }
        // `static open(opts: OpenOptions): GenesisDatabase` -> `open`
        let rest = rest.strip_prefix("static ").unwrap_or(rest);
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() && rest[name.len()..].starts_with('(') {
            names.insert(name);
        }
    }
    names
}

/// Names declared as `export declare function <name>(`.
fn dts_declared_functions(dts: &str) -> HashSet<String> {
    dts.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("export declare function ")?;
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

/// Names declared as `export interface <name> {`.
fn dts_declared_interfaces(dts: &str) -> HashSet<String> {
    dts.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("export interface ")?;
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

/// Every `#[napi]` free function in src/lib.rs - one whose attribute sits at
/// column 0, outside any impl block.
fn extract_napi_free_functions(src: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut pending = false;
    for line in src.lines() {
        if line.starts_with("#[napi") {
            pending = true;
            continue;
        }
        if pending {
            if line.starts_with("pub fn ") || line.starts_with("pub async fn ") {
                if let Some(name) = extract_fn_name(line) {
                    names.push(name);
                }
            }
            pending = false;
        }
    }
    names
}

/// Struct names carrying `#[napi(object)]`, however that attribute is gated -
/// they are written `#[cfg_attr(feature = "napi-bindings", napi(object))]` here
/// so the core still builds with the bindings off.
fn extract_napi_object_structs(src: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut pending = false;
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.contains("napi(object)") {
            pending = true;
            continue;
        }
        if trimmed.starts_with("#[") || trimmed.starts_with("//") || trimmed.is_empty() {
            continue;
        }
        if pending {
            if let Some(rest) = trimmed.strip_prefix("pub struct ") {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    names.push(name);
                }
            }
            pending = false;
        }
    }
    names
}

// Sanity first: if any extractor silently returns nothing, every assertion
// below passes vacuously and the gate is decoration.
#[test]
fn dts_extraction_sanity() {
    let dts = read_index_dts();
    let lib = read_lib_rs();

    let methods = dts_declared_methods(&dts);
    let functions = dts_declared_functions(&dts);
    let interfaces = dts_declared_interfaces(&dts);
    let structs = extract_napi_object_structs(&lib);
    let free = extract_napi_free_functions(&lib);

    assert!(
        methods.len() > 40,
        "expected the GenesisDatabase class to declare many methods, found {}: {methods:?}",
        methods.len()
    );
    for known in ["open", "addNode", "executeHql", "queryRelational"] {
        assert!(methods.contains(known), "class parsing missed {known:?}");
    }
    for known in ["engineNameSync", "versionSync"] {
        assert!(
            functions.contains(known),
            "function parsing missed {known:?}"
        );
    }
    for known in ["NodeInput", "EdgeOutput", "OpenOptions"] {
        assert!(
            interfaces.contains(known),
            "interface parsing missed {known:?}"
        );
    }
    for known in ["NodeInput", "OpenOptions"] {
        assert!(
            structs.iter().any(|s| s == known),
            "napi(object) struct parsing missed {known:?}; found: {structs:?}"
        );
    }
    assert!(
        free.iter().any(|f| f == "version_sync"),
        "free-function parsing missed version_sync; found: {free:?}"
    );

    assert_eq!(
        to_camel_case("query_sql"),
        "querySql",
        "camelCase conversion is what maps Rust names onto declared ones"
    );
    assert_eq!(to_camel_case("open"), "open");
}

#[test]
fn every_napi_method_is_declared_in_index_dts() {
    let dts = read_index_dts();
    let declared = dts_declared_methods(&dts);

    let missing: Vec<String> = extracted_napi_methods()
        .iter()
        .map(|m| to_camel_case(m))
        .filter(|m| !declared.contains(m))
        .collect();

    assert!(
        missing.is_empty(),
        "NAPI method(s) on GenesisDatabase with no declaration in index.d.ts: {missing:?}. \
         Consumers of the published package would not see these methods exist. Add them by \
         hand - do NOT regenerate index.d.ts, which deletes the hand-written QueryIr* \
         declarations (see the note above this test)."
    );
}

#[test]
fn every_declared_method_still_exists_in_the_rust_source() {
    let dts = read_index_dts();
    let live: HashSet<String> = extracted_napi_methods()
        .iter()
        .map(|m| to_camel_case(m))
        .collect();

    let stale: Vec<String> = dts_declared_methods(&dts)
        .into_iter()
        .filter(|m| !live.contains(m))
        .collect();

    assert!(
        stale.is_empty(),
        "index.d.ts declares method(s) that no longer exist as `#[napi]` on \
         GenesisDatabase: {stale:?}. A declaration for a method that is gone is exactly \
         the runtime TypeError the type checker promises cannot happen."
    );
}

#[test]
fn every_napi_free_function_is_declared_in_index_dts() {
    let dts = read_index_dts();
    let declared = dts_declared_functions(&dts);
    let lib = read_lib_rs();

    let missing: Vec<String> = extract_napi_free_functions(&lib)
        .iter()
        .map(|f| to_camel_case(f))
        .filter(|f| !declared.contains(f))
        .collect();

    assert!(
        missing.is_empty(),
        "`#[napi]` free function(s) with no declaration in index.d.ts: {missing:?}"
    );
}

#[test]
fn every_napi_object_struct_is_declared_in_index_dts() {
    let dts = read_index_dts();
    let declared = dts_declared_interfaces(&dts);
    let lib = read_lib_rs();

    let missing: Vec<String> = extract_napi_object_structs(&lib)
        .into_iter()
        .filter(|s| !declared.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "`#[napi(object)]` struct(s) with no `export interface` in index.d.ts: {missing:?}. \
         A method signature mentioning one of these would reference a type the consumer's \
         compiler cannot resolve."
    );
}
