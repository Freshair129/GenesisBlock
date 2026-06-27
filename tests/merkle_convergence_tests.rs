// Ordered/state Merkle convergence (MARK XIV P5). The gossip Heartbeat compares
// `get_merkle_root()` to decide whether to issue a PullRequest. The root used to
// hash the WAL line-by-line in FILE order, so two peers at identical state but
// different write order diverged permanently and re-pulled forever. The root now
// digests the canonical in-memory state (nodes + edges by id + version + validity),
// which is order-independent — same state ⇒ same root.
//
// Guarded: (a) a peer that pulls another's delta ends with the SAME root (they
// converge); (b) the root is independent of the order events are applied — the key
// fix vs the old WAL-order hash; (c) divergent state still yields a different root.

use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage, SyncPeer};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: None,
    })
    .unwrap()
}

fn node(s: &Storage, id: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

fn edge(s: &Storage, id: &str, from: &str, to: &str) {
    s.add_edge(EdgeInput {
        id: Some(id.to_string()),
        from: from.to_string(),
        to: to.to_string(),
        rel: "REL".to_string(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
}

/// B knows A's key so it can accept A's signed events.
fn trust(b: &Storage, a: &Storage) {
    b.peers.insert(
        a.local_peer_id.clone(),
        SyncPeer {
            id: a.local_peer_id.clone(),
            addr: String::new(),
            last_seen: 0,
            verifying_key: a.verifying_key.to_bytes().to_vec(),
        },
    );
}

/// After B pulls A's delta, both peers report the SAME Merkle root — convergence
/// is detectable, so gossip stops issuing PullRequests.
#[test]
fn converged_peers_share_root() {
    let a = open(&fresh("test_mk_a"));
    node(&a, "X");
    node(&a, "Y");
    edge(&a, "E1", "X", "Y");

    let b = open(&fresh("test_mk_b"));
    trust(&b, &a);
    b.reconcile_state(a.events_since(0)).unwrap();

    assert_eq!(
        a.get_merkle_root(),
        b.get_merkle_root(),
        "peers at the same state share a Merkle root"
    );
}

/// The root is independent of the order events are applied. B applies A's delta in
/// order, C applies it reversed; both match A. The old WAL-order hash would make B
/// and C diverge — this is the fix.
#[test]
fn root_independent_of_apply_order() {
    let a = open(&fresh("test_mk_oa"));
    node(&a, "X");
    node(&a, "Y");
    edge(&a, "E1", "X", "Y");
    let root_a = a.get_merkle_root();

    let b = open(&fresh("test_mk_ob"));
    trust(&b, &a);
    b.reconcile_state(a.events_since(0)).unwrap();

    let c = open(&fresh("test_mk_oc"));
    trust(&c, &a);
    let mut rev = a.events_since(0);
    rev.reverse();
    c.reconcile_state(rev).unwrap();

    assert_eq!(root_a, b.get_merkle_root());
    assert_eq!(
        root_a,
        c.get_merkle_root(),
        "root is the same whether the delta is applied in order or reversed"
    );
    assert_eq!(b.get_merkle_root(), c.get_merkle_root());
}

/// Divergent state yields a different root: once B adds a node A doesn't have,
/// their roots differ (so gossip correctly detects the divergence and pulls).
#[test]
fn divergent_state_differs() {
    let a = open(&fresh("test_mk_da"));
    node(&a, "X");

    let b = open(&fresh("test_mk_db"));
    trust(&b, &a);
    b.reconcile_state(a.events_since(0)).unwrap();
    assert_eq!(a.get_merkle_root(), b.get_merkle_root(), "converged first");

    node(&b, "Z"); // B diverges with a local node
    assert_ne!(
        a.get_merkle_root(),
        b.get_merkle_root(),
        "a node only B has makes the roots differ"
    );
}

/// An empty store has the all-zero sentinel root (back-compat with the prior
/// empty-WAL behaviour).
#[test]
fn empty_state_is_zero_root() {
    let a = open(&fresh("test_mk_empty"));
    assert_eq!(a.get_merkle_root(), "0".repeat(64));
}

/// A secondary vector flips the root: `add_vector` changes no node/edge field, so
/// without vector presence in the digest its root would be unchanged — and a peer
/// missing only that vector would never be told to pull. The digest includes a
/// (collection, node) presence entry so the divergence is detectable.
#[test]
fn secondary_vector_changes_root() {
    let a = open(&fresh("test_mk_vec"));
    a.create_collection(
        "code".to_string(),
        "m".to_string(),
        4,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    node(&a, "N");
    let r1 = a.get_merkle_root();
    a.add_vector(
        "N".to_string(),
        "code".to_string(),
        vec![1.0, 0.0, 0.0, 0.0],
    )
    .unwrap();
    let r2 = a.get_merkle_root();
    assert_ne!(
        r1, r2,
        "adding a secondary vector flips the root (vector presence in the digest)"
    );
}

/// Ids containing the would-be delimiter bytes ('|', '\n') don't break the digest
/// or collide with a differently-shaped state — fields are length-prefixed, not
/// delimiter-joined, so a crafted id can't forge another entry.
#[test]
fn delimiter_laden_ids_dont_break_digest() {
    let a = open(&fresh("test_mk_inj_a"));
    node(&a, "x|y\nz"); // id carries the digest's delimiter bytes
    let root1 = a.get_merkle_root();
    assert_eq!(
        root1,
        a.get_merkle_root(),
        "root is stable for a delimiter-laden id"
    );
    assert_ne!(
        root1,
        "0".repeat(64),
        "non-empty state is not the zero root"
    );

    let b = open(&fresh("test_mk_inj_b"));
    node(&b, "x");
    node(&b, "y");
    assert_ne!(
        root1,
        b.get_merkle_root(),
        "one delimiter-laden node does not collide with two plain nodes"
    );
}
