use genesis_block_native::{
    query::hql2::QueryOutcomeV2,
    uee_v2::{ExplainV2, QueryRequestV2},
    AccessContext, NodeInput, OpenOptions, RelationalColumn, RelationalColumnType,
    RelationalSchemaPackage, RelationalTable, Storage,
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

#[test]
fn analyze_does_not_attribute_pre_execution_source_scan_time_to_operator() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    db.add_node(NodeInput {
        id: Some("doc:source".into()),
        labels: vec!["Document".into()],
        props: Some(json!({"name":"source"})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"g4-source-scan-analyze",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM NODES Document AS n |> RETURN n",
        "params":{},
        "explain":"analyze"
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = db.query_v2(access(), request).unwrap() else {
        panic!("ANALYZE must return rows")
    };
    let plan = result.explain.unwrap();
    let source_scan = plan
        .plan
        .iter()
        .find(|node| node.physical_op == "AuthorizedNodeScan")
        .expect("plan contains the authorized node source scan");
    let actual = serde_json::to_value(source_scan.actual.as_ref().unwrap()).unwrap();
    assert_eq!(actual["elapsed_ns"]["measurement"]["status"], "unknown");
    assert_eq!(
        actual["elapsed_ns"]["measurement"]["reason"],
        "not_instrumented"
    );
    assert_eq!(actual["rows_out"]["measurement"]["status"], "measured");
    assert_eq!(actual["rows_out"]["measurement"]["value"], 1);
    assert_eq!(actual["rows_in"]["measurement"]["value"], 0);
}

#[test]
fn explain_edge_row_and_annotation_sources_preserve_frontier_and_fixture_files() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    db.register_relational_schema(RelationalSchemaPackage {
        namespace: "default".into(),
        schema_version: 1,
        previous_version: None,
        package_id: uuid::Uuid::new_v4().to_string(),
        schema_hash: String::new(),
        tables: vec![RelationalTable {
            name: "records".into(),
            columns: vec![RelationalColumn::required("id", RelationalColumnType::Text)],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
        named_queries: vec![],
    })
    .unwrap();
    for (source, physical) in [
        ("EDGES LINK", "AuthorizedEdgeScan"),
        ("TABLE records", "AuthorizedRowScan"),
        ("ANNOTATIONS", "AuthorizedAnnotationScan"),
    ] {
        let frontier = db.stable_frontier();
        let before = disk_tree(dir.path());
        let request: QueryRequestV2 = serde_json::from_value(json!({
            "contract_version":"genesis.api.v2","request_id":"explain-source-matrix",
            "namespace":"default","language_version":"hql.v2",
            "hql":format!("USE default FROM {source} AS x |> RETURN x"),"params":{},"explain":"plan"
        }))
        .unwrap();
        let QueryOutcomeV2::Plan(plan) = db.query_v2(access(), request).unwrap() else {
            panic!("plan")
        };
        assert!(plan.plan.iter().any(|node| node.physical_op == physical));
        assert!(plan
            .plan
            .iter()
            .all(|node| node.actual.is_none() && node.estimates.is_none()));
        assert_eq!(plan.catalog.observed_frontier, frontier);
        assert_eq!(db.stable_frontier(), frontier);
        assert_eq!(disk_tree(dir.path()), before, "{source}");
    }
}

#[test]
fn explain_history_and_change_sources_do_not_publish_or_write() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    db.add_node(NodeInput {
        id: Some("doc:history".into()),
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
    let selected = db.stable_frontier();
    for (source, op, config) in [
        (
            "HISTORY NODE \"doc:history\"",
            "HistoryScan",
            json!({"kind":"node","id":{"type":"Utf8","literal":"doc:history"},"as":"x"}),
        ),
        (
            "CHANGES SINCE 0",
            "ChangeScan",
            json!({"after_seq":"0","as":"x"}),
        ),
    ] {
        let hql:QueryRequestV2=serde_json::from_value(json!({
            "contract_version":"genesis.api.v2","request_id":"history-plan-hql",
            "namespace":"default","language_version":"hql.v2",
            "hql":format!("USE default AT TX {selected} {source} AS x |> RETURN x"),"params":{},"explain":"plan"
        })).unwrap();
        let ir:QueryRequestV2=serde_json::from_value(json!({
            "contract_version":"genesis.api.v2","request_id":"history-plan-ir",
            "namespace":"default","temporal":{"tx_as_of":selected.to_string()},
            "ir":{"contract_version":"query-ir.v2","nodes":[{"id":"scan","op":op,"inputs":[],"config":config}],"root":"scan","parameter_types":{}},
            "params":{},"explain":"plan"
        })).unwrap();
        for request in [hql, ir] {
            let before = disk_tree(dir.path());
            let frontier = db.stable_frontier();
            let QueryOutcomeV2::Plan(plan) = db.query_v2(access(), request).unwrap() else {
                panic!("plan")
            };
            assert!(plan
                .plan
                .iter()
                .all(|node| node.actual.is_none() && node.estimates.is_none()));
            assert!(plan
                .plan
                .iter()
                .any(|node| node.physical_op == format!("Authorized{op}")));
            assert_eq!(db.stable_frontier(), frontier);
            assert_eq!(disk_tree(dir.path()), before);
        }
    }
}

#[test]
fn explain_lexical_and_context_do_not_publish_or_write() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    db.add_node(NodeInput {
        id: Some("doc:text".into()),
        labels: vec!["Document".into()],
        props: Some(json!({"body":"red é猫😀"})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
    let selected = db.stable_frontier();
    for (op,physical,suffix,config) in [
        ("LexicalMatch","ExactLexicalScan","LEXICAL n FIELD body QUERY \"red\" USING INDEX `unicode-whitespace-bm25-v1` TOP 1 AS score |> RETURN n, score",json!({"entity":"n","field":"body","index":"unicode-whitespace-bm25-v1","query":{"literal":"red","type":"Utf8"},"k":1,"as":"score"})),
        ("ContextPack","ContextPack","PACK CONTEXT TEXT prop(n, \"body\") EVIDENCE n TOKENS 5 TOKENIZER \"unicode-scalar-v1\" AS ctx |> RETURN ctx",json!({"text":{"call":"prop","args":[{"field":{"alias":"n","path":[]}},{"literal":"body","type":"Utf8"}]},"evidence":{"field":{"alias":"n","path":[]}},"tokens":5,"tokenizer":"unicode-scalar-v1","as":"ctx"}))
    ] {
        let hql:QueryRequestV2=serde_json::from_value(json!({"contract_version":"genesis.api.v2","request_id":"text-plan-hql","namespace":"default","language_version":"hql.v2","hql":format!("USE default AT TX {selected} FROM NODES AS n |> {suffix}"),"params":{},"explain":"plan"})).unwrap();
        let ir:QueryRequestV2=serde_json::from_value(json!({"contract_version":"genesis.api.v2","request_id":"text-plan-ir","namespace":"default","temporal":{"tx_as_of":selected.to_string()},"ir":{"contract_version":"query-ir.v2","nodes":[{"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"n"}},{"id":"text","op":op,"inputs":["scan"],"config":config}],"root":"text","parameter_types":{}},"params":{},"explain":"plan"})).unwrap();
        for request in [hql,ir] {
            let files=disk_tree(dir.path());let frontier=db.stable_frontier();
            let QueryOutcomeV2::Plan(plan)=db.query_v2(access(),request).unwrap() else {panic!("plan")};
            assert!(plan.plan.iter().any(|n|n.physical_op==physical));
            assert!(plan.plan.iter().all(|n|n.actual.is_none() && n.estimates.is_none()));
            assert_eq!(db.stable_frontier(),frontier);assert_eq!(disk_tree(dir.path()),files);
        }
    }
}
