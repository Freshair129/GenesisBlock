use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
use serde_json::json;
use tempfile::TempDir;

fn storage() -> (Storage, TempDir) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap();
    (storage, dir)
}

fn node(id: &str, props: Option<serde_json::Value>) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec!["ENTITY".to_string()],
        props,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

#[test]
fn query_ir_context_returns_bounded_packet() {
    let (storage, _dir) = storage();
    storage.add_node(node("ctx-src", None)).unwrap();
    storage.add_node(node("ctx-dst", None)).unwrap();
    storage
        .add_edge(EdgeInput {
            id: Some("ctx-edge".to_string()),
            from: "ctx-src".to_string(),
            to: "ctx-dst".to_string(),
            rel: "KNOWS".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    let response = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "ctx-001",
            "operation": {
                "kind": "context",
                "target_id": "ctx-src",
                "tier": "H1",
                "budget": 512,
                "fuzzy": false
            }
        }))
        .unwrap();

    assert_eq!(response["operation_kind"], "context");
    assert_eq!(response["data"]["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(response["data"]["edges"][0]["id"], "ctx-edge");
    assert_eq!(response["data"]["coverage"]["hops_requested"], 1);
    assert_eq!(response["data"]["coverage"]["truncated"], false);
    assert!(response["meta"]["budget"].is_object());
}

#[test]
fn query_ir_context_rejects_unsupported_seed_and_temporal_modes() {
    let (storage, _dir) = storage();
    storage.add_node(node("ctx-target", None)).unwrap();

    let vector_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "ctx-vector",
            "operation": {
                "kind": "context",
                "query_vector": [1.0, 0.0],
                "tier": "H0"
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(vector_error.starts_with("QUERY_CAPABILITY_UNSUPPORTED:"));

    let temporal_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "ctx-temporal",
            "temporal": { "valid_at": "2026-01-01T00:00:00Z" },
            "operation": {
                "kind": "context",
                "target_id": "ctx-target",
                "tier": "H0"
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(temporal_error.starts_with("QUERY_CAPABILITY_UNSUPPORTED:"));
}

#[test]
fn query_ir_context_rejects_unknown_target_and_namespace_scope() {
    let (storage, _dir) = storage();

    let target_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "ctx-missing",
            "operation": {
                "kind": "context",
                "target_id": "missing",
                "tier": "H0"
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(target_error.starts_with("QUERY_TARGET_NOT_FOUND:"));

    let namespace_error = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "ctx-namespace",
            "namespace": "tenant-a",
            "operation": {
                "kind": "context",
                "target_id": "missing",
                "tier": "H0"
            }
        }))
        .unwrap_err()
        .to_string();
    assert!(namespace_error.starts_with("QUERY_CAPABILITY_UNSUPPORTED:"));
}

#[test]
fn query_ir_context_reports_compression_warning() {
    let (storage, _dir) = storage();
    storage
        .add_node(node(
            "ctx-large",
            Some(json!({ "payload": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" })),
        ))
        .unwrap();

    let response = storage
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "ctx-budget",
            "operation": {
                "kind": "context",
                "target_id": "ctx-large",
                "tier": "H0",
                "budget": 1
            }
        }))
        .unwrap();

    assert_eq!(response["data"]["coverage"]["truncated"], true);
    assert!(response["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning == "context_truncated"));
}

#[test]
fn query_ir_capabilities_disclose_wave_e_boundaries() {
    let (storage, _dir) = storage();
    let capabilities = storage.query_ir_capabilities();

    assert_eq!(capabilities["operations"]["context"], "implemented");
    assert_eq!(
        capabilities["operation_details"]["context"]["target_id"],
        "implemented"
    );
    assert_eq!(
        capabilities["operation_details"]["context"]["query_vector"],
        "unsupported"
    );
    assert_eq!(
        capabilities["operation_details"]["search"]["filters"],
        "unsupported"
    );
    assert_eq!(
        capabilities["operation_details"]["search"]["lexical"],
        "planned"
    );
}
