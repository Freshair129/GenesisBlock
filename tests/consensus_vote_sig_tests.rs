// Consensus vote signature verification: `submit_vote` must reject votes that
// aren't authentically signed by the claimed voter, so a caller can't forge
// votes on another peer's behalf and drive a proposal to quorum. The vote
// signature binds (proposal_id, voter_peer_id, approve). See the consensus
// section of src/lib.rs (`sign_vote` / `submit_vote`).

use genesis_block_native::{Storage, OpenOptions, NodeOutput, LogicalClock, Event};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() { fs::remove_dir_all(&p).unwrap(); }
    p
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(), page_cache_mb: Some(64), read_only: Some(false), vector_dim: Some(4),
    }).unwrap()
}

fn mk_node(id: &str) -> NodeOutput {
    NodeOutput {
        id: id.to_string(), labels: vec!["USER".to_string()], props: serde_json::json!({}),
        impact: None, embedding: None, lang: None,
        valid_from: "2026-01-01T00:00:00Z".to_string(), valid_to: None, caused_by: None,
        expires_at: None, clock: LogicalClock { time: 1, peer_id: "p".to_string() }, collection: None,
    }
}

fn propose(s: &Storage, id: &str) -> String {
    s.propose_consensus(Event::Node(mk_node(id)), vec![0u8; 64]).unwrap()
}

/// A correctly self-signed vote verifies and (with no other peers, quorum
/// threshold 0) drives the proposal to quorum.
#[test]
fn authentic_self_vote_reaches_quorum() {
    let s = open(&fresh("test_cv_ok"));
    let pid = propose(&s, "N1");
    let me = s.local_peer_id.clone();
    let sig = s.sign_vote(pid.clone(), true);
    let reached = s.submit_vote(pid, me, true, sig).unwrap();
    assert!(reached, "an authentic vote past threshold reaches quorum");
}

/// A vote for an unknown peer (no registered verifying key) is rejected — it
/// cannot be verified, so it must not count.
#[test]
fn vote_from_unknown_peer_rejected() {
    let s = open(&fresh("test_cv_unknown"));
    let pid = propose(&s, "N1");
    let r = s.submit_vote(pid, "ghost".to_string(), true, vec![0u8; 64]);
    assert!(r.is_err());
    assert!(r.unwrap_err().to_string().contains("unknown voter"));
}

/// A tampered signature (right peer, wrong bytes) fails verification.
#[test]
fn tampered_signature_rejected() {
    let s = open(&fresh("test_cv_tampered"));
    let pid = propose(&s, "N1");
    let me = s.local_peer_id.clone();
    let mut sig = s.sign_vote(pid.clone(), true);
    sig[0] ^= 0xFF; // flip a byte
    let r = s.submit_vote(pid, me, true, sig);
    assert!(r.is_err());
    assert!(r.unwrap_err().to_string().contains("invalid vote signature"));
}

/// The signature is bound to the approve/reject choice: a signature produced for
/// `approve=true` cannot be replayed as a `approve=false` vote (or vice versa).
#[test]
fn signature_is_bound_to_the_choice() {
    let s = open(&fresh("test_cv_flip"));
    let pid = propose(&s, "N1");
    let me = s.local_peer_id.clone();
    let sig_for_yes = s.sign_vote(pid.clone(), true);
    let r = s.submit_vote(pid, me, false, sig_for_yes); // claim "reject" with a "approve" signature
    assert!(r.is_err());
    assert!(r.unwrap_err().to_string().contains("invalid vote signature"));
}

/// A signature of the wrong length is a malformed-input error, not a panic.
#[test]
fn malformed_signature_rejected() {
    let s = open(&fresh("test_cv_malformed"));
    let pid = propose(&s, "N1");
    let me = s.local_peer_id.clone();
    let r = s.submit_vote(pid, me, true, vec![1, 2, 3]); // not 64 bytes
    assert!(r.is_err());
    assert!(r.unwrap_err().to_string().contains("malformed vote signature"));
}
