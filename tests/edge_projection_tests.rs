//! Edge projection (SPEC--GENESISDB-EDGE-PROJECTION).
//!
//! Before this the projection was node-only BY CONSTRUCTION: `Event::Edge` fell
//! through the `_ => {}` arm of `projection_apply_event`, so a database holding
//! 15,393 edges answered `no such table: edges`. Reports that join across
//! relationships could not be written at all.
//!
//! The load-bearing test here is not "edges appear" — it is
//! `edges_current_agrees_with_the_graph_api`. Two surfaces that disagree about
//! what exists are worse than one surface that cannot answer, and a soft-deleted
//! edge is exactly where they would drift apart without anyone noticing.

use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&path).exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
}

fn node(storage: &Storage, id: &str, label: &str) {
    storage
        .add_node(NodeInput {
            id: Some(id.to_string()),
            labels: vec![label.to_string()],
            props: Some(json!({ "name": id })),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn edge(storage: &Storage, id: &str, from: &str, to: &str, rel: &str) {
    storage
        .add_edge(EdgeInput {
            id: Some(id.to_string()),
            from: from.to_string(),
            to: to.to_string(),
            rel: rel.to_string(),
            props: Some(json!({ "via": "test" })),
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
}

/// A model with two variants, one of which carries an offer.
fn seeded(path: &str) -> Storage {
    let storage = open(path);
    node(&storage, "model:1", "ProductModel");
    node(&storage, "variant:1", "PhysicalVariant");
    node(&storage, "variant:2", "PhysicalVariant");
    node(&storage, "offer:1", "CatalogOffer");
    edge(&storage, "e1", "model:1", "variant:1", "HAS_VARIANT");
    edge(&storage, "e2", "model:1", "variant:2", "HAS_VARIANT");
    edge(&storage, "e3", "variant:1", "offer:1", "HAS_OFFER");
    storage
}

fn ids(storage: &Storage, sql: &str) -> HashSet<String> {
    storage
        .query_sql(sql, vec![])
        .unwrap()
        .into_iter()
        .map(|row| row["id"].as_str().unwrap().to_string())
        .collect()
}

fn count(storage: &Storage, sql: &str) -> i64 {
    storage.query_sql(sql, vec![]).unwrap()[0]["n"]
        .as_i64()
        .unwrap()
}

// 1. The report that motivated the whole thing: a join across relationships,
//    in one statement, which the node-only projection made impossible.
#[test]
fn a_relationship_join_is_now_expressible_in_one_statement() {
    let storage = seeded(&fresh("edgeproj_join"));

    let rows = storage
        .query_sql(
            "SELECT m.from_id AS id, count(*) AS n
               FROM edges_current m
               JOIN node_labels lm ON lm.node_u32 = m.from_u32 AND lm.label = ?1
              GROUP BY m.from_id",
            vec![json!("ProductModel")],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], json!("model:1"));
    assert_eq!(rows[0]["n"], json!(2), "the model has two variants");

    // Two hops, which is where the graph shape actually lives.
    let offers = count(
        &storage,
        "SELECT count(*) AS n
           FROM edges_current a JOIN edges_current b ON b.from_u32 = a.to_u32
           JOIN node_labels lo ON lo.node_u32 = b.to_u32 AND lo.label = 'CatalogOffer'
          WHERE a.from_id = 'model:1'",
    );
    assert_eq!(
        offers, 1,
        "model:1 reaches exactly one offer through a variant"
    );
}

// 2. THE test. A retracted edge stays in `edges` and leaves `edges_current`, and
//    `edges_current` is what agrees with `neighbors`. If these two ever diverge,
//    a caller gets a confident wrong answer with no signal that anything is off.
#[test]
fn edges_current_agrees_with_the_graph_api() {
    let path = fresh("edgeproj_agreement");
    let storage = seeded(&path);

    let neighbours_of = |s: &Storage| -> HashSet<String> {
        s.neighbors(
            "model:1".to_string(),
            genesis_block_native::NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: None,
                limit: Some(100),
            },
            false,
        )
        .unwrap()
        .into_iter()
        .map(|n| n.node.id)
        .collect()
    };

    let sql_targets = |s: &Storage| {
        ids(
            s,
            "SELECT to_id AS id FROM edges_current WHERE from_id = 'model:1'",
        )
    };

    assert_eq!(
        sql_targets(&storage),
        neighbours_of(&storage),
        "before any retraction the two surfaces must already agree"
    );

    storage.retract_edge("e2".to_string(), None).unwrap();

    // Soft delete: the row survives, so time-travel still has it.
    assert_eq!(
        count(&storage, "SELECT count(*) AS n FROM edges WHERE id = 'e2'"),
        1,
        "a retracted edge must remain in `edges` - it is a soft delete"
    );
    assert_eq!(
        count(
            &storage,
            "SELECT count(*) AS n FROM edges WHERE id = 'e2' AND valid_to IS NOT NULL"
        ),
        1,
        "and it must carry valid_to"
    );

    // ...and the current view drops it, in step with the graph API.
    let after_sql = sql_targets(&storage);
    let after_graph = neighbours_of(&storage);
    assert!(
        !after_sql.contains("variant:2"),
        "edges_current still shows a retracted edge: {after_sql:?}"
    );
    assert_eq!(
        after_sql, after_graph,
        "the SQL view and the graph API disagree about what exists now - \
         this is the failure the view exists to prevent"
    );
}

// 3. Retracting a NODE is a hard delete of its incident edges in the live view,
//    so the projection must drop them too rather than keep claiming them.
#[test]
fn retracting_a_node_removes_its_edges_from_the_projection() {
    let storage = seeded(&fresh("edgeproj_node_retract"));
    assert_eq!(count(&storage, "SELECT count(*) AS n FROM edges"), 3);

    storage.retract_node("variant:1").unwrap();

    assert_eq!(
        count(
            &storage,
            "SELECT count(*) AS n FROM edges WHERE from_id = 'variant:1' OR to_id = 'variant:1'"
        ),
        0,
        "a retracted node's incident edges must not survive in the projection"
    );
    assert_eq!(
        count(&storage, "SELECT count(*) AS n FROM edges"),
        1,
        "only model:1 -> variant:2 should remain"
    );
}

// 4. The projection is declared rebuildable from Genesis-owned state. An
//    existing database predating v4 has edges only in the live map, so opening
//    it must backfill them - otherwise every database created before this change
//    would report zero relationships forever.
#[test]
fn an_existing_database_is_backfilled_on_open() {
    let path = fresh("edgeproj_backfill");
    let storage = seeded(&path);
    assert_eq!(count(&storage, "SELECT count(*) AS n FROM edges"), 3);
    drop(storage);

    // Simulate the pre-v4 state: the table exists but was never written to.
    let reopened = open(&path);
    reopened
        .query_sql("SELECT count(*) AS n FROM edges", vec![])
        .unwrap();
    drop(reopened);
    {
        let conn = rusqlite_open(&path);
        conn.execute("DELETE FROM edges", []).unwrap();
    }

    let after = open(&path);
    assert_eq!(
        count(&after, "SELECT count(*) AS n FROM edges"),
        3,
        "opening a database whose edge projection is empty must rebuild it"
    );
    // And the rebuild is faithful, not just the right count.
    assert_eq!(
        ids(&after, "SELECT id FROM edges"),
        ["e1", "e2", "e3"]
            .iter()
            .map(|s| s.to_string())
            .collect::<HashSet<_>>()
    );
}

fn rusqlite_open(db_path: &str) -> rusqlite::Connection {
    rusqlite::Connection::open(Path::new(db_path).join("projection.sqlite")).unwrap()
}

// 5. Every column a caller would reach for actually carries its value. A
//    projection that writes NULLs would pass a row-count test and fail every
//    real query.
#[test]
fn the_projected_row_carries_the_whole_edge() {
    let storage = seeded(&fresh("edgeproj_fidelity"));

    let rows = storage
        .query_sql(
            "SELECT id, from_id, to_id, rel, props, valid_from, valid_to,
                    recorded_at, clock_time, clock_peer, from_u32, to_u32
               FROM edges WHERE id = 'e3'",
            vec![],
        )
        .unwrap();
    let r = &rows[0];
    assert_eq!(r["from_id"], json!("variant:1"));
    assert_eq!(r["to_id"], json!("offer:1"));
    assert_eq!(r["rel"], json!("HAS_OFFER"));
    assert_eq!(r["valid_to"], json!(null));
    assert!(
        r["props"].as_str().unwrap().contains("\"via\""),
        "props must survive as JSON text, got {}",
        r["props"]
    );
    assert!(!r["valid_from"].as_str().unwrap().is_empty());
    assert!(!r["recorded_at"].as_str().unwrap().is_empty());
    assert!(r["clock_time"].as_i64().unwrap() > 0);

    // The interned endpoints must join to the node tables, which is the whole
    // reason both spellings are stored.
    let joined = count(
        &storage,
        "SELECT count(*) AS n FROM edges e
           JOIN props p ON p.node_u32 = e.from_u32
          WHERE e.id = 'e3'",
    );
    assert_eq!(joined, 1, "from_u32 must join to props");
}
