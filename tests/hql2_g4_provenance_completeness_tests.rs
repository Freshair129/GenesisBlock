use genesis_block_native::{
    query::hql2::{QueryOutcomeV2, QueryResultV2},
    uee_v2::{AnnotationPutMutationV2, ExplainV2, QueryRequestV2},
    AccessContext, EdgeInput, GenerationInfo, NodeInput, OpenOptions, Storage,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
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
fn actor() -> AccessContext {
    AccessContext {
        principal: "annotation-lookup-reader".into(),
        namespace: "default".into(),
    }
}
fn database_id(storage: &Storage) -> String {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-lookup-db-id",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{"id":"values","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}}],
            "root":"values",
            "parameter_types":{"xs":"List<I64>"}
        },
        "params":{"xs":{"type":"List<I64>","value":[]}}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("values query must execute")
    };
    result.snapshot.database_id
}
fn frozen_ref(path: &Path, database_id: &str, id: &str) -> serde_json::Value {
    let connection = rusqlite::Connection::open(path.join("projection.sqlite")).unwrap();
    let revision: String = connection
        .query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE database_id=?1 AND namespace='default' AND kind='node'
               AND record_id=?2 AND tx_to IS NULL",
            rusqlite::params![database_id, id],
            |row| row.get(0),
        )
        .unwrap();
    json!({
        "ref": {
            "database_id":database_id,
            "namespace":"default",
            "kind":"node",
            "id":id,
            "revision":revision
        },
        "binding":"frozen",
        "selector":{"type":"whole"}
    })
}
fn put_annotation(
    storage: &Storage,
    id: &str,
    targets: Vec<serde_json::Value>,
    evidence: Vec<serde_json::Value>,
) {
    storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: json!({
                    "id":id,
                    "namespace":"default",
                    "kind":"review",
                    "targets":targets,
                    "evidence":evidence,
                    "body":{"type":"text","text":"Reviewed."},
                    "author":"asserted-reviewer",
                    "created_at":"2026-09-22T00:00:00Z",
                    "valid_from":"2026-09-22T00:00:00Z",
                    "valid_to":null
                }),
                expected_revision: None,
                valid: None,
            },
        )
        .unwrap();
}
fn add_node(storage: &Storage, id: &str, embedding: Option<Vec<f64>>) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: vec!["Document".into()],
            props: None,
            embedding,
            lang: Some("en".into()),
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: Some("default".into()),
        })
        .unwrap();
}
fn space_id_for(storage: &Storage, name: &str) -> String {
    let collection = storage
        .list_collections()
        .into_iter()
        .find(|collection| collection.name == name)
        .unwrap();
    let tuple = (
        collection.name,
        collection.model,
        u16::try_from(collection.dim).unwrap(),
        collection.metric,
        collection.quant,
    );
    let mut hasher = Sha256::new();
    hasher.update(b"genesis.hql2.vector-space.v1:");
    hasher.update(serde_json::to_vec(&tuple).unwrap());
    hex::encode(hasher.finalize())
}
fn vector_param(space_id: &str) -> Value {
    json!({
        "type":"vector",
        "space_id":space_id,
        "values":[0.0, 0.0]
    })
}
fn add_edge(storage: &Storage, id: &str, from: &str, to: &str, rel: &str) {
    storage
        .add_edge(EdgeInput {
            id: Some(id.into()),
            from: from.into(),
            to: to.into(),
            rel: rel.into(),
            props: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
}

const VALID: &str = "2026-10-03T00:00:00Z";
fn field(alias: &str) -> Value {
    json!({"field":{"alias":alias,"path":[]}})
}
fn requests(space: &str, rank: Option<&str>, empty: bool) -> [QueryRequestV2; 2] {
    let label = if empty { "Absent" } else { "Document" };
    let mut nodes =
        vec![json!({"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"d","label":label}})];
    let mut last = "scan";
    let mut stages = String::new();
    if let Some(op) = rank {
        let mut config =
            json!({"entity":"d","collection":"default","query":{"param":"q"},"k":1,"as":"hit"});
        let stage = if op == "Knn" {
            config["mode"] = json!("exact");
            "KNN"
        } else {
            config["fidelity"] = json!("original");
            "RERANK"
        };
        nodes.push(json!({"id":"rank","op":op,"inputs":[last],"config":config}));
        last = "rank";
        stages+=&format!(" |> {stage} d IN default USING $q TOP 1 EXACT AS hit |> EXPAND (d)-[e:R]->(b) AS p TRAIL");
        nodes.push(json!({"id":"expand","op":"Expand","inputs":[last],"config":{"optional":false,"pattern":{
   "form":"sequence","start":{"alias":"d","labels":[],"properties":{}},"steps":[{"edge":{"alias":"e","relations":["R"],"direction":"out","min_hops":1,"max_hops":1,"properties":{}},"node":{"alias":"b","labels":[],"properties":{}}}],"mode":"trail","path_alias":"p"}}}));
        last = "expand";
    }
    let target = if rank.is_some() { "b" } else { "d" };
    nodes.push(json!({"id":"lookup","op":"AnnotationLookup","inputs":[last],"config":{"target":target,"as":"a","optional":false}}));
    let mut fields = vec![
        json!({"as":"entity","expression":field(target)}),
        json!({"as":"annotation","expression":field("a")}),
    ];
    let mut returns = format!("{target} AS entity, a AS annotation");
    if rank.is_some() {
        fields.push(json!({"as":"score","expression":field("hit")}));
        returns += ", hit AS score";
    }
    nodes.push(
        json!({"id":"project","op":"Project","inputs":["lookup"],"config":{"fields":fields}}),
    );
    let mut ir = json!({"contract_version":"genesis.api.v2","request_id":"metadata-ir","namespace":"default","ir":{"contract_version":"query-ir.v2","nodes":nodes,"root":"project","parameter_types":{}},"params":{}});
    let mut hql = json!({"contract_version":"genesis.api.v2","request_id":"metadata-hql","namespace":"default","language_version":"hql.v2","hql":format!("USE default FROM NODES {label} AS d{stages} |> ANNOTATIONS OF {target} AS a |> RETURN {returns}"),"params":{}});
    if rank.is_some() {
        ir["ir"]["parameter_types"]["q"] = json!(format!("Vector<{space},2,f64>"));
        ir["params"]["q"] = vector_param(space);
        hql["params"]["q"] = vector_param(space);
    }
    [
        serde_json::from_value(hql).unwrap(),
        serde_json::from_value(ir).unwrap(),
    ]
}
fn qualify(result: &QueryResultV2, generation: &GenerationInfo, db_id: &str, rank: Option<&str>) {
    let actual = serde_json::to_value(result).unwrap();
    assert_eq!(
        actual["semantics"],
        json!({"candidate_search":"exact","distance_fidelity":if rank.is_some(){"original"}else{"not_applicable"},"index_coverage":"not_applicable","scope":if rank==Some("Rerank"){"rerank_candidates"}else{"whole_input"}})
    );
    assert_eq!(
        actual["completeness"],
        json!({"status":"complete","reason":null,"eligible_count_known":true})
    );
    assert_eq!(actual["index_frontiers"], json!({}));
    assert_eq!(actual["cursor"], Value::Null);
    assert_eq!(
        actual["snapshot"],
        json!({"database_id":db_id,"tx":generation.wal_frontier.to_string(),"valid_at":"2026-10-03T00:00:00+00:00","catalog_generation":generation.generation_id.to_string(),"policy_version":generation.acl_revision.to_string()})
    );
}
#[test]
fn paired_frontends_preserve_exact_metadata_and_revision_ownership() {
    for rank in [None, Some("Knn"), Some("Rerank")] {
        let dir = TempDir::new().unwrap();
        let db = open(dir.path());
        add_node(&db, "seed", Some(vec![0.0, 0.0]));
        add_node(&db, "target", Some(vec![1.0, 0.0]));
        add_edge(&db, "first", "seed", "target", "R");
        add_edge(&db, "second", "seed", "target", "R");
        let id = database_id(&db);
        let target = frozen_ref(dir.path(), &id, "target");
        let seed = frozen_ref(dir.path(), &id, "seed");
        put_annotation(
            &db,
            "review:target",
            vec![target.clone()],
            vec![seed.clone()],
        );
        let annotation=rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap().query_row("SELECT revision_id FROM hql2_record_revisions WHERE record_id='review:target' AND kind='annotation' AND tx_to IS NULL",[],|r|r.get::<_,String>(0)).unwrap();
        let space = space_id_for(&db, "default");
        let generation = db.publish_generation().unwrap();
        for empty in [false, true] {
            for mut request in requests(&space, rank, empty) {
                request.temporal = Some(
                    serde_json::from_value(
                        json!({"valid_at":VALID,"tx_as_of":generation.wal_frontier.to_string()}),
                    )
                    .unwrap(),
                );
                for mode in [None, Some(ExplainV2::Analyze)] {
                    request.explain = mode;
                    let QueryOutcomeV2::Rows(result) =
                        db.query_v2(actor(), request.clone()).unwrap()
                    else {
                        panic!("rows")
                    };
                    qualify(&result, &generation, &id, rank);
                    if !empty {
                        let mut limited = request.clone();
                        limited.budget =
                            Some(serde_json::from_value(json!({"max_result_bytes":1})).unwrap());
                        assert_eq!(
                            db.query_v2(actor(), limited).unwrap_err().code,
                            "QUERY_BUDGET_EXCEEDED"
                        );
                    }

                    assert_eq!(
                        result.rows.len(),
                        if empty {
                            0
                        } else if rank.is_some() {
                            2
                        } else {
                            1
                        }
                    );
                    for row in result.rows {
                        let entity = serde_json::to_value(&row["entity"]).unwrap();
                        assert_eq!(entity["value"], target["ref"]);
                        let a = serde_json::to_value(&row["annotation"]).unwrap();
                        assert_eq!(a["value"]["id"], "review:target");
                        assert_eq!(a["value"]["revision"], annotation);
                        if rank.is_some() {
                            let score = serde_json::to_value(&row["score"]).unwrap();
                            assert_eq!(score["value"]["owner"], seed["ref"]);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn denied_frontends_never_publish_success_completeness() {
    use genesis_block_native::{
        AccessAction, AccessGrant, AccessPolicy, AccessPolicyMode, AccessResource, PolicyAdminActor,
    };
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    db.replace_access_policy(
        PolicyAdminActor {
            access: AccessContext {
                principal: "local-owner".into(),
                namespace: "default".into(),
            },
        },
        0,
        AccessPolicy {
            revision: 1,
            mode: AccessPolicyMode::Enforced,
            grants: vec![AccessGrant {
                principal: "local-owner".into(),
                action: AccessAction::ManagePolicy,
                resource: AccessResource::Namespace("default".into()),
            }],
        },
    )
    .unwrap();
    let frontier = db.stable_frontier();
    for mut request in requests("unused", None, true) {
        for mode in [None, Some(ExplainV2::Analyze)] {
            request.explain = mode;
            let error = db.query_v2(actor(), request.clone()).unwrap_err();
            assert_eq!(error.code, "FORBIDDEN");
            assert_eq!(error.stage, "authorize");
            assert_eq!(db.stable_frontier(), frontier);
        }
    }
}
