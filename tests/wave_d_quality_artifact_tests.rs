use serde_json::Value;

#[test]
fn wave_d_quality_artifact_keeps_per_index_rows() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../docs/AUDIT--WAVE-D-QUALITY-2026-09-08.json"
    ))
    .unwrap();
    assert_eq!(artifact["artifact"], "genesisdb.wave-d-quality.v1");
    assert!(artifact["pass"].is_boolean());
    let rows = artifact["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 48);
    assert!(rows.iter().all(|row| {
        row["index_id"].is_string()
            && row["exact_filtered_recall"].is_number()
            && row["eligibility_shortfall"].is_number()
            && row["latency_ms"]["p50"].is_number()
            && row["latency_ms"]["p95"].is_number()
            && row["latency_ms"]["p99"].is_number()
            && row["memory"]["arena_resident_bytes"].is_number()
            && row["memory"]["sidecar_disk_bytes"].is_number()
            && row["pass"].is_boolean()
    }));
    assert!(rows
        .iter()
        .all(|row| row["eligibility_shortfall"].as_u64() == Some(0)));
}
