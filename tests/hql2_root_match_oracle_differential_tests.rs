#[path = "support/hql2_pipeline_reference.rs"]
mod reference;

use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2},
    uee_v2::QueryRequestV2,
    AccessContext, EdgeInput, NodeInput, OpenOptions, Storage,
};
use reference::{graph, rank, Environment, Plan};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tempfile::TempDir;

fn entity(kind: graph::Kind, id: &str) -> graph::EntityRef {
    graph::EntityRef {
        namespace: "default".into(),
        kind,
        id: id.into(),
        revision: "r1".into(),
    }
}

fn record(kind: graph::Kind, id: &str) -> graph::Revision {
    graph::Revision {
        entity: entity(kind, id),
        transaction: graph::Interval {
            start: 1,
            end: None,
        },
        valid: graph::Interval {
            start: 0,
            end: None,
        },
        retracted: false,
        fields: BTreeMap::new(),
        data: graph::RecordData::Plain,
    }
}

fn oracle_environment() -> Environment {
    let mut revisions = ["a", "b", "c", "d"]
        .into_iter()
        .map(|id| record(graph::Kind::Node, id))
        .collect::<Vec<_>>();
    for (id, from, to, relation) in [
        ("ab-1", "a", "b", "LINK"),
        ("ab-2", "a", "b", "LINK"),
        ("bc", "b", "c", "LINK"),
        ("ad", "a", "d", "OTHER"),
    ] {
        let mut edge = record(graph::Kind::Edge, id);
        edge.data = graph::RecordData::Edge {
            source: entity(graph::Kind::Node, from).identity(),
            target: entity(graph::Kind::Node, to).identity(),
            relation: relation.into(),
        };
        revisions.push(edge);
    }
    let catalog = graph::Catalog {
        frontier: 10,
        history: [graph::Kind::Node, graph::Kind::Edge]
            .into_iter()
            .map(|kind| {
                (
                    kind,
                    graph::HistoryCapability {
                        horizon: 0,
                        available: true,
                    },
                )
            })
            .collect(),
        revisions,
    };
    let view = graph::View {
        namespace: "default".into(),
        transaction: 10,
        valid_at: 5,
        permissions: graph::Permissions {
            read: catalog
                .revisions
                .iter()
                .map(|revision| revision.entity.identity())
                .collect(),
            annotation_body: BTreeSet::new(),
        },
    };
    Environment {
        catalog,
        view,
        ranking: rank::Fixture {
            space: rank::Space {
                fingerprint: "unused".into(),
                dimension: 1,
                metric: rank::Metric::L2Squared,
            },
            analyzer_fingerprint: "unused".into(),
            documents: vec![],
        },
        tokenizers: rank::TokenizerRegistry::default(),
        limits: graph::Limits::default(),
    }
}

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

fn add_node(storage: &Storage, id: &str) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: vec!["Node".into()],
            props: None,
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn add_edge(storage: &Storage, id: &str, from: &str, to: &str, relation: &str) {
    storage
        .add_edge(EdgeInput {
            id: Some(id.into()),
            from: from.into(),
            to: to.into(),
            rel: relation.into(),
            props: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
}

fn production_rows(storage: &Storage) -> Vec<(String, String, String)> {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"root-match-p7-oracle",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default MATCH (a)-[e:LINK]->(b) AS p WALK |> RETURN a.id AS from, b.id AS to, e.id AS edge",
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage
        .query_v2(
            AccessContext {
                principal: "graph-reader".into(),
                namespace: "default".into(),
            },
            request,
        )
        .unwrap()
    else {
        panic!("HQL root MATCH must return rows")
    };
    result
        .rows
        .into_iter()
        .map(|row| {
            let QueryValueV2::Utf8(from) = &row["from"] else {
                panic!("from.id must be UTF-8")
            };
            let QueryValueV2::Utf8(to) = &row["to"] else {
                panic!("to.id must be UTF-8")
            };
            let QueryValueV2::Utf8(edge) = &row["edge"] else {
                panic!("edge.id must be UTF-8")
            };
            (from.clone(), to.clone(), edge.clone())
        })
        .collect()
}

fn oracle_rows() -> Vec<(String, String, String)> {
    let expansion = graph::Expand {
        start_alias: "a".into(),
        segments: vec![graph::Segment {
            end_alias: "b".into(),
            edge_alias: Some("e".into()),
            direction: graph::Direction::Out,
            relations: BTreeSet::from(["LINK".into()]),
            min_hops: 1,
            max_hops: 1,
            node_predicate: graph::Predicate::default(),
            edge_predicate: graph::Predicate::default(),
        }],
        path_alias: Some("p".into()),
        mode: graph::PathMode::Walk,
        optional: false,
        shortest: false,
    };
    reference::execute(
        &oracle_environment(),
        &Plan::Match {
            start: "a".into(),
            predicate: graph::Predicate::default(),
            expansion,
        },
    )
    .unwrap()
    .rows
    .into_iter()
    .map(|row| {
        let entity_id = |alias: &str| match &row[alias] {
            reference::Value::Graph(graph::Binding::Entity(entity)) => entity.id.clone(),
            _ => panic!("{alias} must bind an entity"),
        };
        (entity_id("a"), entity_id("b"), entity_id("e"))
    })
    .collect()
}

fn bag(rows: Vec<(String, String, String)>) -> BTreeMap<(String, String, String), usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        *counts.entry(row).or_insert(0) += 1;
    }
    counts
}

#[test]
fn root_match_hql_matches_independent_p7_bag_with_parallel_edges() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    for id in ["a", "b", "c", "d"] {
        add_node(&storage, id);
    }
    for (id, from, to, relation) in [
        ("ab-1", "a", "b", "LINK"),
        ("ab-2", "a", "b", "LINK"),
        ("bc", "b", "c", "LINK"),
        ("ad", "a", "d", "OTHER"),
    ] {
        add_edge(&storage, id, from, to, relation);
    }

    let expected = bag(oracle_rows());
    let actual = bag(production_rows(&storage));
    assert_eq!(expected, actual);
    assert_eq!(actual.values().sum::<usize>(), 3);
}
