use super::*;
use tempfile::TempDir;

fn budget(max_payload_bytes: u64, max_event_count: usize) -> JournalAuthorizationBudget {
    JournalAuthorizationBudget {
        max_payload_bytes,
        max_event_count,
    }
}

fn signed_empty_batch(key: &SigningKey) -> SignedEvent {
    let event = Event::Batch(Vec::new());
    let payload = canonical_event_bytes(&event).unwrap();
    let signer_peer_id =
        hex::encode(Sha256::digest(key.verifying_key().as_bytes()))[..16].to_string();
    SignedEvent {
        event,
        signature: key.sign(&payload).to_bytes().to_vec(),
        signer_peer_id,
    }
}

fn append_signed_event(storage: &Storage, signed_event: SignedEvent) {
    let (ack_tx, ack_rx) = unbounded();
    storage
        .wal_sender
        .send(WalMsg::Append(Box::new(signed_event), ack_tx))
        .unwrap();
    let seq = ack_rx.recv().unwrap().unwrap();
    storage.commit_sequence.fetch_max(seq, Ordering::SeqCst);
}

fn open_options(path: &std::path::Path, read_only: bool) -> OpenOptions {
    OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(read_only),
        vector_dim: Some(2),
        retention: None,
    }
}

#[test]
fn bounded_active_journal_read_accepts_a_not_yet_created_wal() {
    let root = TempDir::new().unwrap();
    let active_path = root.path().join("wal/active.gwal");
    let source_limit = budget(64, 1).max_uncompressed_source_bytes().unwrap();

    assert!(read_active_journal_source_bounded(
        &active_path,
        source_limit + ACTIVE_HEADER_LEN as u64,
        true,
    )
    .unwrap()
    .is_none());
    assert!(read_active_journal_source_bounded(
        &active_path,
        source_limit + ACTIVE_HEADER_LEN as u64,
        false,
    )
    .is_err());

    fs::create_dir_all(active_path.parent().unwrap()).unwrap();
    fs::write(&active_path, active_header(1)).unwrap();
    assert_eq!(
        read_active_journal_source_bounded(
            &active_path,
            source_limit + ACTIVE_HEADER_LEN as u64,
            false,
        )
        .unwrap(),
        Some(active_header(1).to_vec())
    );
}

#[test]
fn budget_counts_jsonl_segments_framed_segments_and_active_wal() {
    let signing_key = SigningKey::from_bytes(&[17; 32]);
    let signed_event = signed_empty_batch(&signing_key);
    let payload = serde_json::to_vec(&signed_event).unwrap();
    let root = TempDir::new().unwrap();
    let journal_dir = root.path().join("journal");
    let wal_dir = root.path().join("wal");
    fs::create_dir_all(&journal_dir).unwrap();
    fs::create_dir_all(&wal_dir).unwrap();

    let legacy_line = [payload.as_slice(), b"\n"].concat();
    fs::write(root.path().join("genesis-graph.wal"), &legacy_line).unwrap();
    write_segment(
        &journal_dir.join("L0.gseg"),
        SEG_KIND_LEGACY_JSONL,
        CODEC_LZ4,
        0,
        0,
        1,
        &legacy_line,
    )
    .unwrap();

    let mut history = Vec::new();
    encode_frame(&mut history, 1, &payload);
    write_segment(
        &journal_dir.join("H1.gseg"),
        SEG_KIND_HISTORY,
        CODEC_LZ4,
        1,
        1,
        1,
        &history,
    )
    .unwrap();

    let mut active = active_header(2).to_vec();
    encode_frame(&mut active, 2, &payload);
    fs::write(wal_dir.join("active.gwal"), active).unwrap();

    let counted = Storage::preflight_journal_authorization_budget(
        root.path(),
        budget(payload.len() as u64 * 4, 4),
    )
    .unwrap();
    assert_eq!(counted, 4);

    let payload_error = Storage::preflight_journal_authorization_budget(
        root.path(),
        budget(payload.len() as u64 * 4 - 1, 4),
    )
    .unwrap_err();
    assert!(payload_error
        .to_string()
        .contains("aggregate event payload"));

    let event_error = Storage::preflight_journal_authorization_budget(
        root.path(),
        budget(payload.len() as u64 * 4, 3),
    )
    .unwrap_err();
    assert!(event_error.to_string().contains("event count"));
}

#[test]
fn journal_preflight_rechecks_limits_after_count_source_changes() {
    let root = TempDir::new().unwrap();
    let signing_key = SigningKey::from_bytes(&[19; 32]);
    let signed_event = signed_empty_batch(&signing_key);
    let payload = serde_json::to_vec(&signed_event).unwrap();
    let wal_dir = root.path().join("wal");
    fs::create_dir_all(&wal_dir).unwrap();
    let active_path = wal_dir.join("active.gwal");
    let mut active = active_header(1).to_vec();
    encode_frame(&mut active, 1, &payload);
    fs::write(&active_path, active).unwrap();

    let budget = budget(payload.len() as u64, 1);
    assert_eq!(
        Storage::preflight_journal_authorization_budget(root.path(), budget).unwrap(),
        1
    );

    let source_limit = budget.max_uncompressed_source_bytes().unwrap();
    FileOpenOptions::new()
        .write(true)
        .open(&active_path)
        .unwrap()
        .set_len(ACTIVE_HEADER_LEN as u64 + source_limit + 1)
        .unwrap();
    let error = Storage::preflight_journal(root.path(), Some(2), Some(budget)).unwrap_err();
    assert!(error.to_string().contains("active WAL is"));
    assert!(!error.to_string().contains("unsupported active header"));
}

#[test]
fn journal_preflight_rejects_record_growth_within_source_byte_bound() {
    let root = TempDir::new().unwrap();
    let signing_key = SigningKey::from_bytes(&[29; 32]);
    let signed_event = signed_empty_batch(&signing_key);
    let payload = serde_json::to_vec(&signed_event).unwrap();
    let wal_dir = root.path().join("wal");
    fs::create_dir_all(&wal_dir).unwrap();
    let active_path = wal_dir.join("active.gwal");
    let mut active = active_header(1).to_vec();
    encode_frame(&mut active, 1, &payload);
    fs::write(&active_path, active).unwrap();

    let budget = budget(payload.len() as u64 * 2 + FRAME_HEADER_LEN as u64, 1);
    assert_eq!(
        Storage::preflight_journal_authorization_budget(root.path(), budget).unwrap(),
        1
    );

    let mut grown_active = active_header(1).to_vec();
    encode_frame(&mut grown_active, 1, &payload);
    encode_frame(&mut grown_active, 2, &payload);
    assert_eq!(
        grown_active.len() as u64 - ACTIVE_HEADER_LEN as u64,
        budget.max_uncompressed_source_bytes().unwrap()
    );
    fs::write(&active_path, grown_active).unwrap();

    let error = Storage::preflight_journal(root.path(), Some(2), Some(budget)).unwrap_err();
    assert!(error.to_string().contains("journal event count exceeds 1"));
    assert!(!error.to_string().contains("active WAL is"));
}

#[test]
fn source_length_guard_precedes_active_wal_read_and_max_frame_fits_overhead() {
    let root = TempDir::new().unwrap();
    let active_path = root.path().join("wal/active.gwal");
    fs::create_dir_all(active_path.parent().unwrap()).unwrap();
    let test_budget = budget(1, 1);
    let source_limit = test_budget.max_uncompressed_source_bytes().unwrap();
    File::create(&active_path)
        .unwrap()
        .set_len(ACTIVE_HEADER_LEN as u64 + source_limit + 1)
        .unwrap();

    let error =
        Storage::preflight_journal_authorization_budget(root.path(), test_budget).unwrap_err();
    assert!(error.to_string().contains("active WAL is"));
    assert!(!error.to_string().contains("active header"));

    let legacy_root = TempDir::new().unwrap();
    let legacy_path = legacy_root.path().join("genesis-graph.wal");
    File::create(&legacy_path)
        .unwrap()
        .set_len(source_limit + 1)
        .unwrap();
    let error = Storage::preflight_journal_authorization_budget(legacy_root.path(), test_budget)
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("uncompressed journal sources exceed"));
    assert!(!error.to_string().contains("invalid legacy event"));

    let production_source_limit = JournalAuthorizationBudget::DEFAULT
        .max_uncompressed_source_bytes()
        .unwrap();
    let one_max_frame_body = MAX_JOURNAL_FRAME_PAYLOAD_BYTES as u64 + FRAME_HEADER_LEN as u64;
    let one_max_raw_segment_file = one_max_frame_body + (SEG_HEADER_LEN + SEG_FOOTER_LEN) as u64;
    let one_segment_file_limit = production_source_limit
        + (SEG_HEADER_LEN + SEG_FOOTER_LEN) as u64
        + MAX_AUTH_SEGMENT_CODEC_OVERHEAD_BYTES;
    assert!(one_max_frame_body <= production_source_limit);
    assert!(one_max_raw_segment_file <= one_segment_file_limit);
    assert!(MAX_JOURNAL_SEGMENT_BODY_BYTES > production_source_limit);

    let segment_root = TempDir::new().unwrap();
    let journal_dir = segment_root.path().join("journal");
    fs::create_dir_all(&journal_dir).unwrap();
    let segment_path = journal_dir.join("H1.gseg");
    let file_len = one_max_raw_segment_file;
    let mut file = File::create(&segment_path).unwrap();
    file.set_len(file_len).unwrap();
    let mut header = [0u8; SEG_HEADER_LEN];
    header[0..4].copy_from_slice(&SEG_MAGIC);
    header[4..6].copy_from_slice(&JOURNAL_FORMAT_VERSION.to_le_bytes());
    header[6] = SEG_KIND_HISTORY;
    header[7] = CODEC_NONE;
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&header).unwrap();
    let mut footer = [0u8; SEG_FOOTER_LEN];
    footer[32..40].copy_from_slice(&one_max_frame_body.to_le_bytes());
    file.seek(SeekFrom::Start(file_len - SEG_FOOTER_LEN as u64))
        .unwrap();
    file.write_all(&footer).unwrap();
    file.sync_all().unwrap();
    drop(file);
    assert!(Storage::preflight_journal_authorization_source_bounds(
        segment_root.path(),
        JournalAuthorizationBudget::DEFAULT
    )
    .is_ok());

    footer[32..40].copy_from_slice(&MAX_JOURNAL_SEGMENT_BODY_BYTES.to_le_bytes());
    let mut file = FileOpenOptions::new()
        .write(true)
        .open(&segment_path)
        .unwrap();
    file.seek(SeekFrom::Start(file_len - SEG_FOOTER_LEN as u64))
        .unwrap();
    file.write_all(&footer).unwrap();
    file.sync_all().unwrap();
    let oversized_segment = Storage::preflight_journal_authorization_source_bounds(
        segment_root.path(),
        JournalAuthorizationBudget::DEFAULT,
    )
    .unwrap_err();
    assert!(oversized_segment.to_string().contains("segment declares"));

    let bounded_segment_root = TempDir::new().unwrap();
    let bounded_journal_dir = bounded_segment_root.path().join("journal");
    fs::create_dir_all(&bounded_journal_dir).unwrap();
    let bounded_segment_limit = test_budget.max_uncompressed_source_bytes().unwrap()
        + (SEG_HEADER_LEN + SEG_FOOTER_LEN) as u64
        + MAX_AUTH_SEGMENT_CODEC_OVERHEAD_BYTES;
    File::create(bounded_journal_dir.join("H1.gseg"))
        .unwrap()
        .set_len(bounded_segment_limit + 1)
        .unwrap();
    let oversized_file = Storage::preflight_journal_authorization_source_bounds(
        bounded_segment_root.path(),
        test_budget,
    )
    .unwrap_err();
    assert!(oversized_file
        .to_string()
        .contains("per-segment authorization read limit"));
}

#[test]
fn membership_open_budget_fails_before_replay_or_projection_creation() {
    let root = TempDir::new().unwrap();
    let signing_key = SigningKey::from_bytes(&[23; 32]);
    fs::write(root.path().join("identity.bin"), signing_key.to_bytes()).unwrap();
    let wal_dir = root.path().join("wal");
    fs::create_dir_all(&wal_dir).unwrap();
    let mut active = active_header(1).to_vec();
    let payload = serde_json::to_vec(&signed_empty_batch(&signing_key)).unwrap();
    encode_frame(&mut active, 1, &payload);
    encode_frame(&mut active, 2, &payload);
    let active_path = wal_dir.join("active.gwal");
    fs::write(&active_path, &active).unwrap();

    let membership = PeerMembershipOptions {
        local_gossip_addr: "127.0.0.1:31871".into(),
        local_master_authority: Some(true),
        peers: Vec::new(),
    };
    let result = Storage::open_internal_with_budget(
        open_options(root.path(), false),
        None,
        Some(membership),
        Some(JournalAuthorizationContext::MembershipRecovery),
        budget(1024 * 1024, 1),
    );
    let error = match result {
        Ok(storage) => {
            drop(storage);
            panic!("over-budget membership open unexpectedly succeeded")
        }
        Err(error) => error,
    };

    assert!(error.to_string().contains("RECOVERY_AUTH_REQUIRED"));
    assert!(error.to_string().contains("journal event count"));
    assert!(!root.path().join("projection.sqlite").exists());
    assert!(!root.path().join("state.json").exists());
    assert_eq!(fs::read(active_path).unwrap(), active);
}

#[test]
fn late_membership_budget_failure_rolls_back_staged_roster() {
    let root = TempDir::new().unwrap();
    let mut storage = Storage::open(open_options(root.path(), false)).unwrap();
    append_signed_event(&storage, storage.sign_event(&Event::Batch(Vec::new())));
    storage.journal_authorization_budget = budget(1, 100_000);

    let remote_key = SigningKey::from_bytes(&[31; 32]);
    let verifying_key = remote_key.verifying_key();
    let peer_id = hex::encode(Sha256::digest(verifying_key.as_bytes()))[..16].to_string();
    let error = storage
        .configure_peer_membership(
            vec![AuthorizedPeerConfig {
                peer_id,
                public_key_hex: hex::encode(verifying_key.as_bytes()),
                gossip_addr: "127.0.0.1:31872".into(),
                master_authority: false,
            }],
            "127.0.0.1:31873",
            false,
        )
        .unwrap_err();

    assert!(error.to_string().contains("RECOVERY_AUTH_REQUIRED"));
    assert!(error.to_string().contains("aggregate event payload"));
    assert!(storage.authorized_members.is_empty());
    assert!(!storage.peer_membership_configured.load(Ordering::SeqCst));
    assert!(storage.membership_digest.read().is_empty());
    assert!(storage.local_master_authority.load(Ordering::SeqCst));
}

#[test]
fn backup_restore_budget_failure_does_not_publish_target() {
    let source_root = TempDir::new().unwrap();
    let source = Storage::open(open_options(source_root.path(), false)).unwrap();
    append_signed_event(&source, source.sign_event(&Event::Batch(Vec::new())));

    let output = TempDir::new().unwrap();
    let bundle_path = output.path().join("over-budget.genesis");
    let export = source
        .export_backup(BackupExportRequest {
            destination: bundle_path.clone(),
        })
        .unwrap();
    drop(source);

    let target = output.path().join("restored");
    let result = Storage::restore_backup_internal_with_budget(
        BackupRestoreRequest {
            bundle_path,
            target_root: target.clone(),
            expected_sha256: export.sha256,
        },
        None,
        budget(1, 100_000),
    );
    let error = match result {
        Ok(_) => panic!("over-budget backup restore unexpectedly succeeded"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("BACKUP_AUTH_REQUIRED"));
    assert!(error.to_string().contains("aggregate event payload"));
    assert!(!target.exists());
    assert!(fs::read_dir(output.path()).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".genesis-restore-")));
}
