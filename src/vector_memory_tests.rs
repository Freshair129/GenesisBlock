use super::*;

struct BoundedWriter {
    bytes: Vec<u8>,
    fail_after: Option<usize>,
}

impl std::io::Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        assert!(bytes.len() <= 8192, "snapshot attempted an unbounded write");
        if self.fail_after.is_some_and(|n| self.bytes.len() >= n) {
            return Err(std::io::Error::other("injected disk error"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn streamed_arena_bytes_match_independent_little_endian_encoding() {
    let floats: Vec<f32> = (0..5000).map(|n| f32::from_bits(0x3f00_0000 + n)).collect();
    let words: Vec<u64> = (0..5000).map(|n| 0x8123_4567_89ab_cdef ^ n).collect();
    let halves: Vec<u16> = (0..5000).map(|n| n as u16).collect();
    let bytes: Vec<u8> = (0..17000).map(|n| n as u8).collect();
    let cases = [
        (
            ArenaStore::F32(floats.clone()),
            floats
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        ),
        (
            ArenaStore::U8 {
                data: bytes.clone(),
                scale: 1.0,
                bias: 0.0,
            },
            bytes,
        ),
        (
            ArenaStore::Binary {
                data: words.clone(),
                dim: 64,
                n: words.len(),
            },
            words.iter().flat_map(|v| v.to_le_bytes()).collect(),
        ),
        (
            ArenaStore::F16(halves.clone()),
            halves.iter().flat_map(|v| v.to_le_bytes()).collect(),
        ),
    ];
    for (arena, expected) in cases {
        let mut writer = BoundedWriter {
            bytes: vec![],
            fail_after: None,
        };
        arena.write_bytes(&mut writer).unwrap();
        assert_eq!(writer.bytes, expected);
        let mut writer = BoundedWriter {
            bytes: vec![],
            fail_after: Some(8192),
        };
        assert!(arena.write_bytes(&mut writer).is_err());
    }
    for q in [Quant::None, Quant::ScalarU8, Quant::Binary, Quant::F16] {
        let mut bytes = Vec::new();
        ArenaStore::new(q, 65).write_bytes(&mut bytes).unwrap();
        assert!(bytes.is_empty());
    }
}

#[test]
fn arena_allocated_bytes_include_capacity_in_every_mode() {
    assert_eq!(
        ArenaStore::F32(Vec::with_capacity(31)).allocated_bytes(),
        31 * 4
    );
    assert_eq!(
        ArenaStore::U8 {
            data: Vec::with_capacity(31),
            scale: 1.0,
            bias: 0.0
        }
        .allocated_bytes(),
        31
    );
    assert_eq!(
        ArenaStore::Binary {
            data: Vec::with_capacity(31),
            dim: 65,
            n: 0
        }
        .allocated_bytes(),
        31 * 8
    );
    assert_eq!(
        ArenaStore::F16(Vec::with_capacity(31)).allocated_bytes(),
        31 * 2
    );
}

#[test]
fn rebuild_preserves_native_codes_and_f16_rounding_including_binary_tail() {
    for quant in [Quant::None, Quant::ScalarU8, Quant::Binary, Quant::F16] {
        let coll = VectorCollection::new(
            "test".into(),
            "m".into(),
            65,
            Metric::L2,
            quant,
            None,
            false,
            None,
            true,
            false,
        );
        let mut expected = ArenaStore::new(quant, 65);
        for id in 0..3 {
            let row: Vec<f32> = (0..65)
                .map(|i| (i as f32 - 32.0) * 0.01 + id as f32 * 0.001)
                .collect();
            expected.push_f32(&row, None);
            coll.stage(id, &row, "en".into(), id as u64 + 1).unwrap();
        }
        coll.rehydrate(100);
        let index = coll.hnsw.read();
        match index.as_ref().unwrap() {
            VecIndex::F32(h) | VecIndex::F16(h) => {
                for point in h.get_point_indexation() {
                    assert_eq!(
                        point.get_v(),
                        expected.f32_at(point.get_origin_id() * 65, 65)
                    );
                }
            }
            VecIndex::U8(h) => {
                if let ArenaStore::U8 { data, .. } = &expected {
                    for point in h.get_point_indexation() {
                        let start = point.get_origin_id() * 65;
                        assert_eq!(point.get_v(), &data[start..start + 65]);
                    }
                }
            }
            VecIndex::Binary(h) => {
                if let ArenaStore::Binary { data, .. } = &expected {
                    for point in h.get_point_indexation() {
                        let start = point.get_origin_id() * 2;
                        assert_eq!(point.get_v(), &data[start..start + 2]);
                    }
                }
            }
        }
        assert_eq!(index.as_ref().unwrap().point_count(), 3);
    }
}

#[test]
fn rebuild_preserves_calibrated_sq8_and_centered_binary_codes_without_rerank() {
    for quant in [Quant::ScalarU8, Quant::Binary] {
        let coll = VectorCollection::new(
            "test".into(),
            "m".into(),
            65,
            Metric::L2,
            quant,
            None,
            false,
            None,
            true,
            false,
        );
        let mut expected = ArenaStore::new(quant, 65);
        let center = vec![0.123; 65];
        if quant == Quant::ScalarU8 {
            *coll.sq8_scale.write() = Some((317.25, 104.75));
            coll.arena.write().set_sq8_scale(317.25, 104.75);
            expected.set_sq8_scale(317.25, 104.75);
        } else {
            *coll.bq_center.write() = Some(center.clone());
        }
        for id in 0..4 {
            let row: Vec<f32> = (0..65)
                .map(|i| (i as f32 - 32.0) * 0.01 + id as f32 * 0.03)
                .collect();
            expected.push_f32(&row, Some(&center));
            coll.stage(id, &row, "en".into(), id as u64 + 1).unwrap();
        }
        coll.rehydrate(100);
        let index = coll.hnsw.read();
        match (index.as_ref().unwrap(), expected) {
            (VecIndex::U8(h), ArenaStore::U8 { data, .. }) => {
                for point in h.get_point_indexation() {
                    let start = point.get_origin_id() * 65;
                    assert_eq!(point.get_v(), &data[start..start + 65]);
                }
            }
            (VecIndex::Binary(h), ArenaStore::Binary { data, .. }) => {
                for point in h.get_point_indexation() {
                    let start = point.get_origin_id() * 2;
                    assert_eq!(point.get_v(), &data[start..start + 2]);
                }
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn checkpoint_proof_maps_share_one_source_and_keep_signed_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(8),
        retention: Some("full".into()),
    })
    .unwrap();
    storage
        .bulk_add_nodes(
            (0..6)
                .map(|id| NodeInput {
                    id: Some(format!("proof-{id}")),
                    labels: vec!["USER".into()],
                    props: None,
                    embedding: Some(vec![1.0; 8]),
                    lang: None,
                    valid_from: None,
                    caused_by: None,
                    ttl: None,
                    collection: None,
                })
                .collect(),
        )
        .unwrap();
    storage.flush_index();
    let proofs = storage.compacted_base_materialization_proofs().unwrap();
    assert_eq!(proofs.nodes.len(), 6);
    assert_eq!(proofs.vectors.len(), 6);
    let first = proofs.nodes.get("proof-0").unwrap();
    let source_bytes = serde_json::to_vec::<SignedEvent>(first).unwrap();
    let owned: SignedEvent = serde_json::from_slice(&source_bytes).unwrap();
    let mut node = match &owned.event {
        Event::Transaction(tx) => tx.nodes[0].clone(),
        _ => panic!("expected one revision-bound bulk transaction"),
    };
    node.embedding = None;
    let event = Event::Node(node);
    let from_map = storage
        .sign_compacted_base_materialization(&event, Some(first))
        .unwrap();
    let from_owned = storage
        .sign_compacted_base_materialization(&event, Some(&owned))
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&from_map).unwrap(),
        serde_json::to_vec(&from_owned).unwrap()
    );
    let mut cache = CompactionProofBytesCache::default();
    let encoded = storage
        .sign_compacted_base_materialization_cached(&event, Some(first), &mut cache)
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&from_map).unwrap(),
        serde_json::to_vec(&encoded).unwrap(),
        "cached proof envelope/signature must be byte-identical"
    );
    let prepared = cache.prepare(first).unwrap().unwrap();
    let mismatch = storage
        .sign_compacted_base_materialization_impl(&event, Some(&owned), Some(prepared))
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&from_owned).unwrap(),
        serde_json::to_vec(&mismatch).unwrap(),
        "mismatched cache reference must fall back to actual proof bytes"
    );
    let mut addresses = std::collections::HashSet::new();
    for proof in proofs.nodes.values().chain(proofs.vectors.values()) {
        assert!(storage.verify_raw_event_signature(proof));
        assert_eq!(
            serde_json::to_vec::<SignedEvent>(proof).unwrap(),
            source_bytes
        );
        addresses.insert(&proof.event as *const Event);
    }
    assert_eq!(
        addresses.len(),
        1,
        "all leaf lookups must share their immutable source transaction"
    );
}

thread_local! {
    static REVISION_BINDING_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(super) fn record_revision_binding_visit() {
    REVISION_BINDING_VISITS.with(|visits| visits.set(visits.get() + 1));
}

#[test]
fn revision_binding_work_is_linear_and_tampering_stays_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(8),
        retention: Some("full".into()),
    })
    .unwrap();
    storage
        .execute_batch(BatchInput {
            nodes: (0..64)
                .map(|id| NodeInput {
                    id: Some(format!("binding-{id}")),
                    labels: vec!["USER".into()],
                    props: Some(serde_json::json!({"ordinal": id})),
                    embedding: Some(vec![0.125; 8]),
                    lang: None,
                    valid_from: None,
                    caused_by: None,
                    ttl: None,
                    collection: None,
                })
                .collect(),
            edges: vec![EdgeInput {
                id: Some("binding-edge".into()),
                from: "binding-0".into(),
                to: "binding-1".into(),
                rel: "test".into(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
                caused_by: None,
            }],
        })
        .unwrap();
    storage.flush_index();
    let proofs = storage.compacted_base_materialization_proofs().unwrap();
    let source = &proofs.nodes["binding-0"].event;
    assert!(storage.verify_raw_event_signature(&proofs.nodes["binding-0"]));
    let Event::Transaction(original) = source else {
        panic!("transaction proof required")
    };
    let mut reversed = original.clone();
    reversed
        .record_revision_transaction
        .as_mut()
        .unwrap()
        .mutations
        .reverse();
    assert!(Storage::revision_transaction_binds_graph(
        &Event::Transaction(reversed.clone())
    ));
    storage.validate_graph_revision_binding(&reversed).unwrap();
    for kind in 0..6 {
        let mut changed = original.clone();
        let mutations = &mut changed
            .record_revision_transaction
            .as_mut()
            .unwrap()
            .mutations;
        match kind {
            0 => {
                mutations.pop();
            }
            1 => mutations.push(mutations[0].clone()),
            2 => mutations[0].namespace = "foreign".into(),
            3 => mutations[0].operation = uee_v2::RevisionOperationV1::Retract,
            4 => mutations[0].payload = serde_json::json!({"forged": true}),
            5 => changed.nodes[0].props = serde_json::json!({"forged": true}),
            _ => unreachable!(),
        }
        assert!(
            !Storage::revision_transaction_binds_graph(&Event::Transaction(changed.clone())),
            "tamper case {kind}"
        );
        assert!(
            storage.validate_graph_revision_binding(&changed).is_err(),
            "live binding tamper case {kind}"
        );
    }
    // Distinct revision UUIDs pass envelope validation. Duplicate identity
    // counts must include other namespaces, while first() preserves source order.
    let mut duplicate = original
        .record_revision_transaction
        .as_ref()
        .unwrap()
        .clone();
    let node_positions: Vec<_> = duplicate
        .mutations
        .iter()
        .enumerate()
        .filter(|(_, mutation)| mutation.kind == uee_v2::RecordKindV2::Node)
        .map(|(position, _)| position)
        .collect();
    let first = node_positions[0];
    let second = node_positions[1];
    let mut foreign = duplicate.mutations[first].clone();
    foreign.namespace = "foreign".into();
    foreign.revision_id = Uuid::new_v4().to_string();
    duplicate.mutations[second] = foreign;
    duplicate.validate().unwrap();
    let id = duplicate.mutations[first].id.clone();
    let index = RevisionBindingIndex::new(&duplicate.mutations);
    assert!(index.unique(&uee_v2::RecordKindV2::Node, &id).is_none());
    assert_eq!(
        index
            .first(&uee_v2::RecordKindV2::Node, &id)
            .unwrap()
            .namespace,
        "default"
    );
    duplicate.mutations.swap(first, second);
    let index = RevisionBindingIndex::new(&duplicate.mutations);
    assert_eq!(
        index
            .first(&uee_v2::RecordKindV2::Node, &id)
            .unwrap()
            .namespace,
        "foreign"
    );
    let small = vec![
        duplicate.mutations[first].clone(),
        duplicate.mutations[second].clone(),
    ];
    let index = RevisionBindingIndex::new(&small);
    assert!(index.unique(&uee_v2::RecordKindV2::Node, &id).is_none());
    assert_eq!(
        index
            .first(&uee_v2::RecordKindV2::Node, &id)
            .unwrap()
            .namespace,
        "foreign"
    );

    REVISION_BINDING_VISITS.with(|visits| visits.set(0));
    assert!(Storage::revision_transaction_binds_graph(source));
    let visits = REVISION_BINDING_VISITS.with(std::cell::Cell::get);
    let mutations = original
        .record_revision_transaction
        .as_ref()
        .unwrap()
        .mutations
        .len();
    assert!(visits <= mutations * 8,
        "whole-source binding must index mutations once, not rescan per leaf: {visits} probes for {mutations} mutations");
}

thread_local! {
    static SIGNATURE_VERIFICATIONS: std::cell::RefCell<HashMap<[u8; 32], usize>> = std::cell::RefCell::new(HashMap::new());
}

pub(super) fn record_signature_verification(
    key: &VerifyingKey,
    payload: &[u8],
    signature: &Signature,
) {
    let mut hash = Sha256::new();
    hash.update(key.to_bytes());
    hash.update(signature.to_bytes());
    hash.update(payload);
    let fingerprint: [u8; 32] = hash.finalize().into();
    SIGNATURE_VERIFICATIONS
        .with(|counts| *counts.borrow_mut().entry(fingerprint).or_default() += 1);
}

#[test]
fn checkpoint_reuses_identical_signature_math_within_one_operation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(8),
        retention: Some("full".into()),
    })
    .unwrap();
    for id in 0..3 {
        storage
            .add_node(NodeInput {
                id: Some(format!("signature-{id}")),
                labels: vec!["USER".into()],
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
    SIGNATURE_VERIFICATIONS.with(|counts| counts.borrow_mut().clear());
    let (bytes, count) = storage.build_compacted_wal().unwrap();
    assert!(!bytes.is_empty() && count >= 6);
    let max_repeats =
        SIGNATURE_VERIFICATIONS.with(|counts| *counts.borrow().values().max().unwrap());
    assert_eq!(
        max_repeats, 1,
        "identical immutable key/message/signature math is repeated inside one checkpoint"
    );
}

#[test]
fn signature_math_cache_is_exact_bounded_and_operation_local() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let other_key = SigningKey::from_bytes(&[8; 32]);
    let payload = b"checkpoint source";
    let signature = key.sign(payload);
    let verifying = key.verifying_key();
    {
        let _scope = SignatureVerificationScope::enter();
        SIGNATURE_VERIFICATIONS.with(|counts| counts.borrow_mut().clear());
        assert!(verify_signature_math(&verifying, payload, &signature));
        assert!(verify_signature_math(&verifying, payload, &signature));
        assert_eq!(
            SIGNATURE_VERIFICATIONS.with(|counts| counts.borrow().values().sum::<usize>()),
            1
        );
        assert!(!verify_signature_math(
            &other_key.verifying_key(),
            payload,
            &signature
        ));
        assert!(!verify_signature_math(
            &verifying,
            b"forged payload",
            &signature
        ));
        assert!(!verify_signature_math(
            &verifying,
            payload,
            &key.sign(b"other")
        ));
        // Simulate a digest collision: equal lookup key, different full message.
        let identity = (
            verifying.to_bytes(),
            signature.to_bytes(),
            Sha256::digest(payload).into(),
        );
        SIGNATURE_VERIFICATION_CACHE.with(|slot| {
            let mut slot = slot.borrow_mut();
            let cache = slot.as_mut().unwrap();
            cache.entries.clear();
            cache.payload_bytes = 0;
            cache.remember(identity, b"different message", false);
        });
        assert!(verify_signature_math(&verifying, payload, &signature));
        {
            let _nested = SignatureVerificationScope::enter();
        }
        assert!(SIGNATURE_VERIFICATION_CACHE.with(|slot| slot.borrow().is_some()));
        std::thread::spawn(|| {
            assert!(SIGNATURE_VERIFICATION_CACHE.with(|slot| slot.borrow().is_none()))
        })
        .join()
        .unwrap();
    }
    assert!(SIGNATURE_VERIFICATION_CACHE.with(|slot| slot.borrow().is_none()));
    SIGNATURE_VERIFICATIONS.with(|counts| counts.borrow_mut().clear());
    assert!(verify_signature_math(&verifying, payload, &signature));
    assert!(verify_signature_math(&verifying, payload, &signature));
    assert_eq!(
        SIGNATURE_VERIFICATIONS.with(|counts| counts.borrow().values().sum::<usize>()),
        2
    );
    let _ = std::panic::catch_unwind(|| {
        let _scope = SignatureVerificationScope::enter();
        panic!("scope unwind");
    });
    assert!(SIGNATURE_VERIFICATION_CACHE.with(|slot| slot.borrow().is_none()));

    let mut cache = SignatureVerificationCache::default();
    for id in 0..=SIGNATURE_CACHE_MAX_ENTRIES {
        let mut hash = [0; 32];
        hash[..8].copy_from_slice(&(id as u64).to_le_bytes());
        cache.remember(([0; 32], [0; 64], hash), &[], false);
    }
    assert_eq!(cache.entries.len(), SIGNATURE_CACHE_MAX_ENTRIES);
    let mut cache = SignatureVerificationCache::default();
    let large = vec![0; SIGNATURE_CACHE_MAX_PAYLOAD_BYTES / 2 + 1];
    cache.remember(([0; 32], [0; 64], [0; 32]), &large, false);
    cache.remember(([0; 32], [0; 64], [1; 32]), &large, false);
    assert_eq!(cache.entries.len(), 1);
    assert_eq!(cache.payload_bytes, large.len());
    assert!(cache.payload_bytes <= SIGNATURE_CACHE_MAX_PAYLOAD_BYTES);
}

#[test]
fn reopen_verification_stays_fresh_and_checkpoint_cache_is_released() {
    let dir = tempfile::tempdir().unwrap();
    let opts = OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(8),
        retention: Some("full".into()),
    };
    {
        let storage = Storage::open(opts.clone()).unwrap();
        storage
            .bulk_add_nodes(
                (0..3)
                    .map(|id| NodeInput {
                        id: Some(format!("reopen-signature-{id}")),
                        labels: vec!["USER".into()],
                        props: None,
                        embedding: Some(vec![0.125; 8]),
                        lang: None,
                        valid_from: None,
                        caused_by: None,
                        ttl: None,
                        collection: None,
                    })
                    .collect(),
            )
            .unwrap();
        storage.flush_index();
        storage.compact().unwrap();
        storage.save_state().unwrap();
    }
    SIGNATURE_VERIFICATIONS.with(|counts| counts.borrow_mut().clear());
    let storage = Storage::open(opts).unwrap();
    assert_eq!(storage.nodes.len(), 3);
    assert!(SIGNATURE_VERIFICATION_CACHE.with(|slot| slot.borrow().is_none()));
    let repeats = SIGNATURE_VERIFICATIONS.with(|counts| *counts.borrow().values().max().unwrap());
    assert_eq!(
        repeats, 1,
        "open repeats identical signature math across authorization/recovery scans"
    );
}

#[test]
#[ignore = "manual CPU phase diagnostic; no timing acceptance threshold"]
fn profile_large_batch_compaction_phases() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some("full".into()),
    })
    .unwrap();
    for id in ["a", "b"] {
        storage
            .add_node(NodeInput {
                id: Some(id.into()),
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
    let start = Instant::now();
    storage
        .execute_batch(BatchInput {
            nodes: vec![],
            edges: (0..1000)
                .map(|id| EdgeInput {
                    id: Some(format!("phase-{id}")),
                    from: "a".into(),
                    to: "b".into(),
                    rel: "test".into(),
                    props: None,
                    valid_from: None,
                    supersede: None,
                    impact: None,
                    caused_by: None,
                })
                .collect(),
        })
        .unwrap();
    eprintln!(
        "{}",
        serde_json::json!({"phase":"ingest", "seconds":start.elapsed().as_secs_f64()})
    );
    let _scope = SignatureVerificationScope::enter();
    let start = Instant::now();
    let proofs = storage.compacted_base_materialization_proofs().unwrap();
    eprintln!(
        "{}",
        serde_json::json!({"phase":"proof_collection", "seconds":start.elapsed().as_secs_f64()})
    );
    let proof = &proofs.edges["phase-0"];
    let edge = storage
        .edges
        .get(&Storage::edge_key("phase-0"))
        .unwrap()
        .clone();
    let event = Event::Edge(edge);
    let signing_payload =
        compacted_base_event_payload(&storage.local_peer_id, &event, proof, 2).unwrap();
    let mut encoded = CompactionProofBytesCache::default();
    for phase in [
        "raw_signature",
        "graph_binding",
        "leaf_matching",
        "signing",
        "signing_cached",
        "signing_math",
        "sha512_hash",
    ] {
        let start = Instant::now();
        for _ in 0..4 {
            match phase {
                "raw_signature" => assert!(storage.verify_raw_event_signature(proof)),
                "graph_binding" => assert!(Storage::revision_transaction_binds_graph(&proof.event)),
                "leaf_matching" => {
                    assert!(storage.compacted_base_proof_matches_event(&event, &proof.event))
                }
                "signing_math" => {
                    std::hint::black_box(storage.signing_key.sign(&signing_payload));
                }
                "sha512_hash" => {
                    std::hint::black_box(sha2::Sha512::digest(&signing_payload));
                }
                "signing_cached" => {
                    storage
                        .sign_compacted_base_materialization_cached(
                            &event,
                            Some(proof),
                            &mut encoded,
                        )
                        .unwrap();
                }
                "signing" => {
                    storage
                        .sign_compacted_base_materialization(&event, Some(proof))
                        .unwrap();
                }
                _ => unreachable!(),
            }
        }
        eprintln!(
            "{}",
            serde_json::json!({"phase":phase,"iterations":4,"seconds":start.elapsed().as_secs_f64()})
        );
    }
}

thread_local! {
    static SOURCE_ENCODINGS: std::cell::RefCell<HashMap<[u8; 32], usize>> = std::cell::RefCell::new(HashMap::new());
}

pub(super) fn record_source_encoding(bytes: &[u8]) {
    let key: [u8; 32] = Sha256::digest(bytes).into();
    SOURCE_ENCODINGS.with(|counts| *counts.borrow_mut().entry(key).or_default() += 1);
}

#[test]
fn checkpoint_shared_source_encoding_work_is_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(8),
        retention: Some("full".into()),
    })
    .unwrap();
    storage
        .bulk_add_nodes(
            (0..64)
                .map(|id| NodeInput {
                    id: Some(format!("encoding-{id}")),
                    labels: vec![],
                    props: None,
                    embedding: None,
                    lang: None,
                    valid_from: None,
                    caused_by: None,
                    ttl: None,
                    collection: None,
                })
                .collect(),
        )
        .unwrap();
    SOURCE_ENCODINGS.with(|counts| counts.borrow_mut().clear());
    storage.build_compacted_wal().unwrap();
    let max = SOURCE_ENCODINGS.with(|counts| *counts.borrow().values().max().unwrap());
    assert!(
        max <= 8,
        "source canonicalization must be bounded per source, not per leaf: {max}"
    );
}

#[test]
fn cached_compaction_bytes_preserve_all_graph_envelopes_and_bounds() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(8),
        retention: Some("full".into()),
    })
    .unwrap();
    storage
        .execute_batch(BatchInput {
            nodes: (0..2)
                .map(|id| NodeInput {
                    id: Some(format!("wire-{id}")),
                    labels: vec!["USER".into()],
                    props: Some(serde_json::json!({"text":"quote \" \n ภาษาไทย", "fraction":1.25})),
                    embedding: Some(vec![0.125; 8]),
                    lang: None,
                    valid_from: None,
                    caused_by: None,
                    ttl: None,
                    collection: None,
                })
                .collect(),
            edges: vec![EdgeInput {
                id: Some("wire-edge".into()),
                from: "wire-0".into(),
                to: "wire-1".into(),
                rel: "test".into(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
                caused_by: None,
            }],
        })
        .unwrap();
    storage.flush_index();
    let (bytes, _) = storage.build_compacted_wal().unwrap();
    let rows: Vec<_> = bytes
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<SignedEvent>(line).unwrap())
        .filter(|signed| signed.signature.starts_with(COMPACTED_BASE_EVENT_V2_MAGIC))
        .map(|signed| {
            let (_, envelope) = decode_compacted_envelope::<CompactedBaseEventEnvelope>(
                &signed.signature,
                COMPACTED_BASE_EVENT_MAGIC,
                COMPACTED_BASE_EVENT_V2_MAGIC,
                COMPACTED_BASE_EVENT_MAX_BYTES,
            )
            .unwrap();
            (signed, envelope)
        })
        .collect();
    let mut kinds = HashSet::new();
    let mut cache = CompactionProofBytesCache::default();
    for (signed, envelope) in &rows {
        kinds.insert(match signed.event {
            Event::Node(_) => "node",
            Event::Edge(_) => "edge",
            Event::VectorMaterialized(_) => "vector",
            _ => panic!("unexpected base event"),
        });
        let owned = storage
            .sign_compacted_base_materialization(&signed.event, Some(&envelope.proof))
            .unwrap();
        let cached = storage
            .sign_compacted_base_materialization_cached(
                &signed.event,
                Some(&envelope.proof),
                &mut cache,
            )
            .unwrap();
        assert_eq!(
            serde_json::to_vec(&owned).unwrap(),
            serde_json::to_vec(&cached).unwrap()
        );
        assert_eq!(owned.signature, signed.signature);
        assert!(storage.verify_event_signature(&cached));
        let json = serde_json::to_vec(envelope).unwrap();
        assert!(encode_compacted_envelope_json(
            COMPACTED_BASE_EVENT_V2_MAGIC,
            &json,
            json.len() - 1
        )
        .is_err());
    }
    assert_eq!(kinds, HashSet::from(["node", "edge", "vector"]));
    assert!(cache.bytes <= COMPACTION_ENCODING_CACHE_MAX_BYTES);
    let mut tiny = rows[0].1.proof.clone();
    tiny.event = Event::Node(
        match &rows
            .iter()
            .find(|(signed, _)| matches!(signed.event, Event::Node(_)))
            .unwrap()
            .0
            .event
        {
            Event::Node(node) => node.clone(),
            _ => unreachable!(),
        },
    );
    tiny.signature.clear();
    let sources = vec![tiny.clone(); COMPACTION_ENCODING_CACHE_MAX_ENTRIES + 1];
    let mut bounded = CompactionProofBytesCache::default();
    for source in &sources {
        let _ = bounded.prepare(source).unwrap();
    }
    assert_eq!(bounded.entries.len(), COMPACTION_ENCODING_CACHE_MAX_ENTRIES);
    assert!(bounded.bytes <= COMPACTION_ENCODING_CACHE_MAX_BYTES);
    let mut huge = tiny;
    if let Event::Node(node) = &mut huge.event {
        node.props =
            serde_json::json!({"padding":"x".repeat(COMPACTION_ENCODING_CACHE_MAX_BYTES/2)});
    }
    let mut bounded = CompactionProofBytesCache::default();
    assert!(bounded.prepare(&huge).unwrap().is_none());
    assert!(bounded.entries.is_empty());
    assert_eq!(bounded.bytes, 0);
    SOURCE_ENCODINGS.with(|counts| counts.borrow_mut().clear());
    assert!(bounded.prepare(&huge).unwrap().is_none());
    assert!(
        SOURCE_ENCODINGS.with(|counts| counts.borrow().is_empty()),
        "a refused cache admission must not reserialize the source on every leaf"
    );
    let mut full = CompactionProofBytesCache {
        bytes: COMPACTION_ENCODING_CACHE_MAX_BYTES,
        ..Default::default()
    };
    let (signed, envelope) = &rows[0];
    let fallback = storage
        .sign_compacted_base_materialization_cached(&signed.event, Some(&envelope.proof), &mut full)
        .unwrap();
    assert_eq!(fallback.signature, signed.signature);
    assert!(full.entries.is_empty());
}
