use genesis_block_native::{
    query::hql2::QueryOutcomeV2,
    uee_v2::{ExplainV2, QueryRequestV2},
    AccessContext, NodeInput, OpenOptions, Storage,
};
use serde_json::json;
use std::{collections::BTreeMap, path::Path};
use tempfile::TempDir;

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn access() -> AccessContext {
    AccessContext {
        principal: "reader".into(),
        namespace: "default".into(),
    }
}

fn disk_tree(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path
                .file_name()
                .is_some_and(|name| name == "genesis.lock" || name == "projection.sqlite-shm")
            {
                out.insert(
                    format!("{}:size", path.file_name().unwrap().to_string_lossy()),
                    std::fs::metadata(&path)
                        .unwrap()
                        .len()
                        .to_le_bytes()
                        .to_vec(),
                );
                continue;
            }
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_string_lossy().into(),
                    std::fs::read(&path)
                        .unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
                );
            }
        }
    }

    let mut out = BTreeMap::new();
    visit(path, path, &mut out);
    out
}

#[test]
fn explain_source_plan_does_not_open_data_or_publish_a_snapshot() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    db.add_node(NodeInput {
        id: Some("doc:one".into()),
        labels: vec!["Document".into()],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
    db.flush_index();

    let frontier = db.stable_frontier();
    let files = disk_tree(dir.path());
    let mut request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"explain-node-source",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM NODES Document AS n |> RETURN n",
        "params":{}
    }))
    .unwrap();
    request.explain = Some(ExplainV2::Plan);

    let QueryOutcomeV2::Plan(plan) = db.query_v2(access(), request).unwrap() else {
        panic!("EXPLAIN must return a plan without executing its source")
    };
    assert_eq!(plan.catalog.observed_frontier, frontier);
    assert!(plan
        .plan
        .iter()
        .any(|node| node.physical_op == "AuthorizedNodeScan"));
    assert!(plan
        .plan
        .iter()
        .all(|node| node.actual.is_none() && node.estimates.is_none()));
    assert_eq!(db.stable_frontier(), frontier);

    let after = disk_tree(dir.path());
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        files.keys().collect::<Vec<_>>()
    );
    for (name, bytes) in files {
        assert_eq!(after[&name], bytes, "EXPLAIN changed {name}");
    }
}
