//! P7 fixture contract v1 acceptance. Compile directly with rustc; no engine imports.
#[path = "support/hql2_rank_reference.rs"]
mod reference;
use reference::*;
use std::collections::BTreeMap;

fn vector(values: &[f64]) -> OriginalVector {
    OriginalVector {
        fingerprint: "space-v1".into(),
        values: values.to_vec(),
    }
}
fn doc(id: &str, x: f64, text: &str, tokens: &[&str]) -> Document {
    Document {
        id: id.into(),
        revision: "r1".into(),
        visibility: Visibility {
            tx_from: 1,
            tx_to: None,
            valid_from: 0,
            valid_to: None,
            authorized: true,
        },
        original: Some(vector(&[x, 0.0])),
        text: text.into(),
        source_hash: format!("fixture-hash-{id}"),
        analyzer_fingerprint: "pretok-v1".into(),
        tokens: tokens.iter().map(|s| (*s).into()).collect(),
    }
}
fn fixture() -> Fixture {
    Fixture {
        space: Space {
            fingerprint: "space-v1".into(),
            dimension: 2,
            metric: Metric::L2Squared,
        },
        analyzer_fingerprint: "pretok-v1".into(),
        documents: vec![
            doc("A", 0.1, "red red", &["red", "red"]),
            doc("B", 0.2, "red blue", &["red", "blue"]),
            doc("C", 0.3, "blue blue", &["blue", "blue"]),
        ],
    }
}
fn row(key: &str, id: &str, lang: &str) -> Candidate {
    Candidate {
        row_key: key.into(),
        owner_id: id.into(),
        revision: "r1".into(),
        language: lang.into(),
        hit: None,
        source_ranks: BTreeMap::new(),
    }
}
fn batch(rows: Vec<Candidate>) -> Batch {
    Batch {
        snapshot: Snapshot {
            transaction: 10,
            valid: 5,
        },
        rows,
        lineage: Lineage {
            approximate_sources: vec![],
            scope: Scope::WholeInput,
        },
    }
}
fn sample() -> Batch {
    batch(vec![
        row("a", "A", "en"),
        row("b", "B", "th"),
        row("c", "C", "th"),
    ])
}
fn request(top: usize) -> VectorRequest {
    VectorRequest {
        query: vector(&[0.0, 0.0]),
        top,
        source: "dense".into(),
    }
}
fn lexical(top: usize) -> LexicalRequest {
    LexicalRequest {
        analyzer_fingerprint: "pretok-v1".into(),
        terms: vec!["red".into()],
        k1: 1.2,
        b: 0.75,
        top,
        source: "lexical".into(),
    }
}
fn keys(b: &Batch) -> Vec<&str> {
    b.rows.iter().map(|r| r.row_key.as_str()).collect()
}
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
}
fn registry() -> TokenizerRegistry {
    TokenizerRegistry {
        entries: BTreeMap::from([(
            "fixture-unicode-scalar-v1".into(),
            FixtureTokenizer::UnicodeScalarV1,
        )]),
    }
}
fn pack(b: &Batch, f: &Fixture, budget: usize) -> ContextPackage {
    pack_context(b, f, &registry(), "fixture-unicode-scalar-v1", budget).unwrap()
}

#[test]
fn filter_cutoff_order_is_observable_in_composed_pipeline() {
    let after = execute(
        sample(),
        &fixture(),
        &[Stage::Knn(request(2)), Stage::FilterLanguage("th".into())],
    )
    .unwrap();
    let before = execute(
        sample(),
        &fixture(),
        &[Stage::FilterLanguage("th".into()), Stage::Knn(request(2))],
    )
    .unwrap();
    assert_eq!(keys(&after), ["b"]);
    assert_eq!(keys(&before), ["b", "c"]);
    close(before.rows[0].hit.as_ref().unwrap().value, 0.04);
    close(before.rows[1].hit.as_ref().unwrap().value, 0.09);
}

#[test]
fn duplicate_payloads_count_toward_k_with_stable_row_keys() {
    let input = batch(vec![
        row("z", "A", "en"),
        row("a", "A", "en"),
        row("b", "B", "th"),
    ]);
    let result = execute(input, &fixture(), &[Stage::Knn(request(2))]).unwrap();
    assert_eq!(keys(&result), ["a", "z"]);
    assert_eq!(
        result
            .rows
            .iter()
            .map(|r| r.owner_id.as_str())
            .collect::<Vec<_>>(),
        ["A", "A"]
    );
    assert_eq!(result.rows[1].hit.as_ref().unwrap().rank, 2);
}

#[test]
fn stable_ties_do_not_depend_on_fixture_input_order() {
    for order in [["z", "a", "m"], ["m", "z", "a"], ["a", "m", "z"]] {
        let input = batch(order.iter().map(|k| row(k, "A", "en")).collect());
        assert_eq!(
            keys(&execute(input, &fixture(), &[Stage::Knn(request(3))]).unwrap()),
            ["a", "m", "z"]
        );
    }
}

#[test]
fn invalid_vectors_are_rejected_even_with_empty_input_and_top_zero() {
    let mut f = fixture();
    f.space.metric = Metric::Cosine;
    for (v, error) in [
        (vec![0.0, 0.0], Error::ZeroCosineNorm),
        (vec![1.0], Error::DimensionMismatch),
        (vec![f64::NAN, 1.0], Error::NonfiniteVector),
        (vec![f64::INFINITY, 1.0], Error::NonfiniteVector),
    ] {
        let q = VectorRequest {
            query: vector(&v),
            ..request(0)
        };
        assert_eq!(execute(batch(vec![]), &f, &[Stage::Knn(q)]), Err(error));
    }
}

#[test]
fn fingerprints_bind_query_and_each_original() {
    let mut q = request(0);
    q.query.fingerprint = "other-space".into();
    assert_eq!(
        execute(batch(vec![]), &fixture(), &[Stage::Knn(q)]),
        Err(Error::SpaceMismatch)
    );
    let mut f = fixture();
    f.documents[0].original.as_mut().unwrap().fingerprint = "other-space".into();
    assert_eq!(
        execute(sample(), &f, &[Stage::Rerank(request(3))]),
        Err(Error::SpaceMismatch)
    );
}

#[test]
fn finite_huge_and_subnormal_cosine_inputs_do_not_overflow() {
    let space = Space {
        metric: Metric::Cosine,
        ..fixture().space
    };
    close(
        distance(&space, &vector(&[1e308, 1e308]), &vector(&[1e308, -1e308])).unwrap(),
        1.0,
    );
    close(
        distance(&space, &vector(&[1e308, 1e308]), &vector(&[1e308, 1e308])).unwrap(),
        0.0,
    );
    close(
        distance(&space, &vector(&[5e-324, 0.0]), &vector(&[-5e-324, 0.0])).unwrap(),
        2.0,
    );
}

#[test]
fn l2_and_negative_dot_have_explicit_conventions_and_overflow_errors() {
    let mut space = fixture().space;
    close(
        distance(&space, &vector(&[1.0, 2.0]), &vector(&[4.0, 6.0])).unwrap(),
        25.0,
    );
    assert_eq!(
        distance(&space, &vector(&[1e308, 0.0]), &vector(&[-1e308, 0.0])),
        Err(Error::NonfiniteDistance)
    );
    space.metric = Metric::NegDot;
    close(
        distance(&space, &vector(&[1.0, 2.0]), &vector(&[4.0, 6.0])).unwrap(),
        -16.0,
    );
}

#[test]
fn knn_removes_missing_vector_rows_but_rerank_requires_originals() {
    let mut f = fixture();
    f.documents[0].original = None;
    let input = batch(vec![
        row("a1", "A", "en"),
        row("a2", "A", "en"),
        row("b", "B", "th"),
    ]);
    let ranked = knn(input.clone(), &f, &request(2)).unwrap();
    assert_eq!(keys(&ranked), ["b"]);
    close(ranked.rows[0].hit.as_ref().unwrap().value, 0.04);
    assert_eq!(ranked.rows[0].hit.as_ref().unwrap().rank, 1);
    assert_eq!(
        rerank(input, &f, &request(2)),
        Err(Error::OriginalUnavailable)
    );
    f.documents[1].original = None;
    assert!(knn(
        batch(vec![row("a", "A", "en"), row("b", "B", "th")]),
        &f,
        &request(2)
    )
    .unwrap()
    .rows
    .is_empty());
    let invalid = VectorRequest {
        query: vector(&[f64::NAN, 0.0]),
        ..request(0)
    };
    assert_eq!(
        knn(batch(vec![row("a", "A", "en")]), &f, &invalid),
        Err(Error::NonfiniteVector)
    );
}

#[test]
fn visible_originals_are_required_for_exact_rerank() {
    let mut f = fixture();
    f.documents[0].original = None;
    assert_eq!(
        execute(sample(), &f, &[Stage::Rerank(request(3))]),
        Err(Error::OriginalUnavailable)
    );
    f.documents[0].visibility.authorized = false;
    assert_eq!(
        keys(&execute(sample(), &f, &[Stage::Rerank(request(3))]).unwrap()),
        ["b", "c"]
    );
}

#[test]
fn snapshot_visibility_is_half_open_and_gates_owners_before_ranking() {
    let mut f = fixture();
    f.documents[0].visibility.tx_to = Some(10);
    f.documents[1].visibility.valid_to = Some(5);
    f.documents[2].visibility.tx_from = 10;
    f.documents[2].visibility.valid_from = 5;
    assert_eq!(
        keys(&execute(sample(), &f, &[Stage::Knn(request(3))]).unwrap()),
        ["c"]
    );
    f.documents[2].visibility.tx_from = 11;
    assert!(execute(sample(), &f, &[Stage::Knn(request(3))])
        .unwrap()
        .rows
        .is_empty());
}

#[test]
fn revision_owner_is_not_substituted_and_bad_keys_fail() {
    let mut input = sample();
    input.rows[0].revision = "missing".into();
    assert_eq!(
        execute(input, &fixture(), &[Stage::Knn(request(1))]),
        Err(Error::OwnerUnknown)
    );
    assert_eq!(
        execute(
            batch(vec![row("same", "A", "en"), row("same", "A", "en")]),
            &fixture(),
            &[]
        ),
        Err(Error::DuplicateRowKey)
    );
    let mut f = fixture();
    f.documents.push(f.documents[0].clone());
    assert_eq!(execute(sample(), &f, &[]), Err(Error::DuplicateDocument));
}

#[test]
fn rerank_keeps_approximation_lineage_through_filter_take_and_context() {
    let mut input = sample();
    input.rows.reverse();
    input.lineage.approximate_sources = vec!["fixture-ann-v1".into()];
    let result = execute(
        input,
        &fixture(),
        &[
            Stage::FilterLanguage("th".into()),
            Stage::Rerank(request(2)),
            Stage::Take(1),
        ],
    )
    .unwrap();
    assert_eq!(keys(&result), ["b"]);
    assert_eq!(
        result.lineage,
        Lineage {
            approximate_sources: vec!["fixture-ann-v1".into()],
            scope: Scope::RerankCandidates
        }
    );
    assert_eq!(pack(&result, &fixture(), 100).lineage, result.lineage);
    let exact = execute(sample(), &fixture(), &[Stage::Knn(request(1))]).unwrap();
    assert_eq!(exact.lineage.scope, Scope::WholeInput);
    assert!(exact.lineage.approximate_sources.is_empty());
    let reranked = execute(
        exact,
        &fixture(),
        &[Stage::Rerank(request(1)), Stage::Knn(request(1))],
    )
    .unwrap();
    assert_eq!(reranked.lineage.scope, Scope::RerankCandidates);
}

#[test]
fn bm25_tiny_visible_corpus_matches_hand_computed_formula() {
    let result = execute(sample(), &fixture(), &[Stage::Lexical(lexical(3))]).unwrap();
    assert_eq!(keys(&result), ["a", "b"]);
    // N=3, df(red)=2, average length=2; IDF=ln(1+(3-2+.5)/(2+.5)).
    close(
        result.rows[0].hit.as_ref().unwrap().value,
        (1.6_f64).ln() * 4.4 / 3.2,
    );
    close(result.rows[1].hit.as_ref().unwrap().value, (1.6_f64).ln());
    assert_eq!(result.rows[1].hit.as_ref().unwrap().source, "lexical");
}

#[test]
fn bm25_statistics_use_visible_corpus_not_filtered_candidate_bag() {
    let mut f = fixture();
    let mut hidden = doc("hidden", 0.0, "red", &["red"]);
    hidden.visibility.authorized = false;
    hidden.analyzer_fingerprint = "inaccessible".into();
    f.documents.push(hidden);
    let mut future = doc("future", 0.0, "red", &["red"]);
    future.visibility.tx_from = 11;
    f.documents.push(future);
    let input = batch(vec![row("z", "B", "th"), row("a", "B", "th")]);
    let result = execute(input, &f, &[Stage::Lexical(lexical(5))]).unwrap();
    assert_eq!(keys(&result), ["a", "z"]);
    close(result.rows[0].hit.as_ref().unwrap().value, 1.6_f64.ln());
    f.documents[2].visibility.valid_to = Some(5);
    let result = execute(sample(), &f, &[Stage::Lexical(lexical(5))]).unwrap();
    close(result.rows[1].hit.as_ref().unwrap().value, 1.2_f64.ln());
}

#[test]
fn bm25_length_normalization_query_terms_and_cutoff_are_explicit() {
    let mut f = fixture();
    f.documents[0].tokens = vec!["red".into()];
    f.documents[1].tokens = vec!["red".into(), "blue".into(), "blue".into()];
    f.documents.truncate(2);
    let input = batch(vec![row("a", "A", "en"), row("b", "B", "th")]);
    let mut q = lexical(2);
    q.terms.push("red".into()); // Fixture profile uses distinct query terms.
    let result = execute(input.clone(), &f, &[Stage::Lexical(q)]).unwrap();
    close(
        result.rows[0].hit.as_ref().unwrap().value,
        1.2_f64.ln() * 2.2 / 1.75,
    );
    close(
        result.rows[1].hit.as_ref().unwrap().value,
        1.2_f64.ln() * 2.2 / 2.65,
    );
    let post = execute(
        input.clone(),
        &f,
        &[
            Stage::Lexical(lexical(1)),
            Stage::FilterLanguage("th".into()),
        ],
    )
    .unwrap();
    let pre = execute(
        input,
        &f,
        &[
            Stage::FilterLanguage("th".into()),
            Stage::Lexical(lexical(1)),
        ],
    )
    .unwrap();
    assert!(post.rows.is_empty());
    assert_eq!(keys(&pre), ["b"]);
}

#[test]
fn lexical_profile_errors_and_empty_corpus_are_explicit() {
    let mut q = lexical(0);
    q.analyzer_fingerprint = "wrong".into();
    assert_eq!(
        execute(batch(vec![]), &fixture(), &[Stage::Lexical(q)]),
        Err(Error::AnalyzerMismatch)
    );
    let mut q = lexical(1);
    q.k1 = f64::NAN;
    assert_eq!(
        execute(batch(vec![]), &fixture(), &[Stage::Lexical(q)]),
        Err(Error::InvalidBm25)
    );
    let mut f = fixture();
    f.documents[2].analyzer_fingerprint = "wrong".into();
    assert_eq!(
        execute(sample(), &f, &[Stage::Lexical(lexical(1))]),
        Err(Error::AnalyzerMismatch)
    );
    f.documents.clear();
    assert!(execute(batch(vec![]), &f, &[Stage::Lexical(lexical(1))])
        .unwrap()
        .rows
        .is_empty());
}

#[test]
fn canonical_thai_combining_marks_and_emoji_are_never_normalized() {
    let text = "กำกํา🙂";
    let mut f = fixture();
    f.documents = vec![doc("A", 0.1, text, &["กำ", "🙂"])];
    let mut q = lexical(2);
    q.terms = vec!["กำ".into()];
    let result = execute(
        batch(vec![row("a", "A", "th")]),
        &f,
        &[Stage::Lexical(q.clone())],
    )
    .unwrap();
    assert_eq!(keys(&result), ["a"]);
    assert_eq!(pack(&result, &f, 100).rendered_context, "กำกํา🙂[1]");
    q.terms = vec!["กํา".into()];
    assert!(execute(result, &f, &[Stage::Lexical(q)])
        .unwrap()
        .rows
        .is_empty());
}

#[test]
fn source_cutoffs_rrf_and_context_are_actual_stage_composition() {
    let dense = execute(
        sample(),
        &fixture(),
        &[Stage::Knn(request(2)), Stage::SourceRank("dense".into())],
    )
    .unwrap();
    let lex = execute(
        sample(),
        &fixture(),
        &[
            Stage::FilterLanguage("th".into()),
            Stage::Lexical(lexical(1)),
            Stage::SourceRank("lexical".into()),
        ],
    )
    .unwrap();
    let fused = rrf(&[dense, lex], 60.0).unwrap();
    assert_eq!(keys(&fused), ["b", "a"]);
    close(
        fused.rows[0].hit.as_ref().unwrap().value,
        1.0 / 62.0 + 1.0 / 61.0,
    );
    close(fused.rows[1].hit.as_ref().unwrap().value, 1.0 / 61.0);
    assert_eq!(
        fused.rows[0].source_ranks,
        BTreeMap::from([("dense".into(), 2), ("lexical".into(), 1)])
    );
    let final_rows = execute(fused, &fixture(), &[Stage::Take(1)]).unwrap();
    assert_eq!(
        pack(&final_rows, &fixture(), 11).rendered_context,
        "red blue[1]"
    );
}

#[test]
fn source_rank_survives_cutoff_without_renumbering() {
    let result = execute(
        sample(),
        &fixture(),
        &[
            Stage::Knn(request(3)),
            Stage::SourceRank("dense".into()),
            Stage::FilterLanguage("th".into()),
            Stage::Take(1),
        ],
    )
    .unwrap();
    assert_eq!(result.rows[0].source_ranks["dense"], 2);
    close(
        rrf(&[result], 0.0).unwrap().rows[0]
            .hit
            .as_ref()
            .unwrap()
            .value,
        0.5,
    );
}

#[test]
fn rrf_rejects_mixed_snapshots_repeated_source_and_invalid_parameters() {
    let a = execute(
        sample(),
        &fixture(),
        &[Stage::Knn(request(2)), Stage::SourceRank("dense".into())],
    )
    .unwrap();
    let mut b = a.clone();
    b.snapshot.transaction += 1;
    assert_eq!(rrf(&[a.clone(), b], 60.0), Err(Error::SnapshotMismatch));
    assert_eq!(
        rrf(&[a.clone(), a.clone()], 60.0),
        Err(Error::InvalidSourceRank)
    );
    assert_eq!(rrf(&[a], -1.0), Err(Error::InvalidRrf));
    assert_eq!(rrf(&[], 60.0), Err(Error::InvalidRrf));
}

#[test]
fn context_budget_counts_citations_and_separators_exactly() {
    let mut f = fixture();
    f.documents[0].text = "ก🙂ข".into();
    f.documents[1].text = "ไทย".into();
    let input = execute(sample(), &f, &[Stage::Knn(request(2))]).unwrap();
    let p = pack(&input, &f, 11);
    assert_eq!(p.rendered_context, "ก🙂ข[1]\nไ[2]");
    assert_eq!(p.token_count, 11);
    assert_eq!(p.token_budget, 11);
    assert!(!p.diagnostic_envelope_counted);
    assert!(p.truncated);
    assert_eq!(p.truncation_reason.as_deref(), Some("TOKEN_BUDGET"));
    assert_eq!(p.fragments[0].evidence.start_scalar, 0);
    assert_eq!(p.fragments[0].evidence.end_scalar, 3);
    assert_eq!(p.fragments[1].text, "ไ");
    assert_eq!(p.fragments[1].evidence.end_scalar, 1);
    assert_eq!(p.fragments[1].evidence.source_hash, "fixture-hash-B");
    assert_eq!(
        p.omitted_refs,
        vec![EvidenceRef {
            row_key: "b".into(),
            owner_id: "B".into(),
            revision: "r1".into(),
            source_hash: "fixture-hash-B".into(),
            start_scalar: 1,
            end_scalar: 3
        }]
    );
}

#[test]
fn context_zero_budget_and_marker_overhead_omit_whole_refs() {
    let mut f = fixture();
    f.documents[0].text = "🙂".into();
    let input = batch(vec![row("a", "A", "th")]);
    for budget in 0..4 {
        let p = pack(&input, &f, budget);
        assert_eq!(p.rendered_context, "");
        assert_eq!(p.token_count, 0);
        assert!(p.truncated);
        assert_eq!(p.omitted_refs[0].end_scalar, 1);
    }
    let full = pack(&input, &f, 4);
    assert_eq!(full.rendered_context, "🙂[1]");
    assert!(!full.truncated);
    assert!(full.omitted_refs.is_empty());
}

#[test]
fn context_unknown_tokenizer_rejects_even_empty_input() {
    assert_eq!(
        pack_context(
            &batch(vec![]),
            &fixture(),
            &registry(),
            "model-tokenizer",
            0
        ),
        Err(Error::TokenizerUnavailable)
    );
    let empty = pack(&batch(vec![]), &fixture(), 0);
    assert_eq!(empty.token_count, 0);
    assert!(!empty.truncated);
}

#[test]
fn context_validates_revision_visibility_and_source_hash() {
    let mut f = fixture();
    f.documents[0].visibility.authorized = false;
    let p = pack(&sample(), &f, 100);
    assert_eq!(p.rendered_context, "red blue[1]\nblue blue[2]");
    assert!(p.omitted_refs.is_empty()); // Hidden owners never enter diagnostics.
    f.documents[1].source_hash.clear();
    assert_eq!(
        pack_context(&sample(), &f, &registry(), "fixture-unicode-scalar-v1", 100),
        Err(Error::InvalidEvidence)
    );
}

#[test]
fn context_property_budgets_preserve_prefixes_and_duplicate_rows() {
    let mut f = fixture();
    f.documents[0].text = "ก้🙂".into();
    let input = batch(vec![row("a1", "A", "th"), row("a2", "A", "th")]);
    for budget in 0..20 {
        let p = pack(&input, &f, budget);
        assert!(p.token_count <= budget);
        assert_eq!(p.token_count, p.rendered_context.chars().count());
        for frag in &p.fragments {
            assert!(f.documents[0].text.starts_with(&frag.text));
        }
    }
    let p = pack(&input, &f, 13);
    assert_eq!(p.rendered_context, "ก้🙂[1]\nก้🙂[2]");
    assert_eq!(p.fragments.len(), 2);
    assert!(!p.truncated);
}

#[test]
fn standalone_adapter_entrypoints_compose_and_top_zero_is_valid() {
    let f = fixture();
    let ranked = knn(sample(), &f, &request(2)).unwrap();
    let ranked = rerank(ranked, &f, &request(2)).unwrap();
    let ranked = lexical_rank(ranked, &f, &lexical(1)).unwrap();
    assert_eq!(keys(&ranked), ["a"]);
    assert_eq!(pack(&ranked, &f, 10).rendered_context, "red red[1]");
    assert_eq!(ranked.lineage.scope, Scope::RerankCandidates);
    assert!(knn(sample(), &f, &request(0)).unwrap().rows.is_empty());
    assert!(lexical_rank(sample(), &f, &lexical(0))
        .unwrap()
        .rows
        .is_empty());
    let mut bad = request(0);
    bad.query.fingerprint = "wrong".into();
    assert_eq!(rerank(batch(vec![]), &f, &bad), Err(Error::SpaceMismatch));
}

#[test]
fn invalid_visible_originals_are_checked_before_zero_cutoff() {
    for (values, error) in [
        (vec![1.0], Error::DimensionMismatch),
        (vec![1.0, f64::NEG_INFINITY], Error::NonfiniteVector),
    ] {
        let mut f = fixture();
        f.documents[0].original = Some(vector(&values));
        assert_eq!(knn(sample(), &f, &request(0)), Err(error));
    }
    let mut f = fixture();
    f.space.metric = Metric::Cosine;
    f.documents[0].original = Some(vector(&[0.0, 0.0]));
    let q = VectorRequest {
        query: vector(&[1.0, 0.0]),
        ..request(0)
    };
    assert_eq!(rerank(sample(), &f, &q), Err(Error::ZeroCosineNorm));
}

#[test]
fn rrf_ties_lineage_and_branch_order_are_deterministic() {
    let f = fixture();
    let a = execute(
        batch(vec![row("z", "A", "en")]),
        &f,
        &[Stage::Knn(request(1)), Stage::SourceRank("dense".into())],
    )
    .unwrap();
    let mut b = execute(
        batch(vec![row("a", "B", "th")]),
        &f,
        &[
            Stage::Lexical(lexical(1)),
            Stage::SourceRank("lexical".into()),
        ],
    )
    .unwrap();
    b.lineage.approximate_sources = vec!["ann-fixture".into()];
    let forward = rrf(&[a.clone(), b.clone()], 60.0).unwrap();
    assert_eq!(forward, rrf(&[b, a], 60.0).unwrap());
    assert_eq!(keys(&forward), ["a", "z"]);
    assert_eq!(forward.lineage.scope, Scope::RerankCandidates);
    assert_eq!(forward.lineage.approximate_sources, ["ann-fixture"]);
    assert_eq!(pack(&forward, &f, 100).lineage, forward.lineage);
}

#[test]
fn unsupported_same_source_rrf_duplicates_fail_without_silent_deduplication() {
    let ranked = execute(
        batch(vec![row("a1", "A", "en"), row("a2", "A", "en")]),
        &fixture(),
        &[Stage::Knn(request(2)), Stage::SourceRank("dense".into())],
    )
    .unwrap();
    assert_eq!(ranked.rows.len(), 2);
    assert_eq!(rrf(&[ranked], 60.0), Err(Error::InvalidSourceRank));
    assert_eq!(rrf(&[sample()], 60.0), Err(Error::InvalidSourceRank));
}

#[test]
fn multi_digit_citation_overhead_is_counted_at_boundary() {
    let mut f = fixture();
    f.documents[0].text = "🙂".into();
    let input = batch((0..10).map(|i| row(&format!("r{i}"), "A", "th")).collect());
    let complete = pack(&input, &f, 50);
    assert_eq!(
        complete.rendered_context,
        "🙂[1]\n🙂[2]\n🙂[3]\n🙂[4]\n🙂[5]\n🙂[6]\n🙂[7]\n🙂[8]\n🙂[9]\n🙂[10]"
    );
    assert_eq!(complete.token_count, 50);
    assert!(!complete.truncated);
    let short = pack(&input, &f, 49);
    assert_eq!(short.fragments.len(), 9);
    assert_eq!(short.token_count, 44);
    assert_eq!(short.omitted_refs[0].row_key, "r9");
    assert_eq!(short.omitted_refs[0].start_scalar, 0);
}

#[test]
fn bm25_empty_terms_and_documents_have_no_nan_or_phantom_matches() {
    let mut f = fixture();
    for d in &mut f.documents {
        d.tokens.clear();
    }
    assert!(lexical_rank(sample(), &f, &lexical(3))
        .unwrap()
        .rows
        .is_empty());
    let mut q = lexical(3);
    q.terms.clear();
    assert!(lexical_rank(sample(), &fixture(), &q)
        .unwrap()
        .rows
        .is_empty());
    for (k1, b) in [(0.0, 0.75), (1.2, -0.1), (1.2, 1.1), (f64::INFINITY, 0.75)] {
        let q = LexicalRequest {
            k1,
            b,
            ..lexical(3)
        };
        assert_eq!(lexical_rank(batch(vec![]), &f, &q), Err(Error::InvalidBm25));
    }
}

#[test]
fn negdot_cancellation_keeps_unit_residual_in_all_permutations() {
    let space = Space {
        dimension: 3,
        metric: Metric::NegDot,
        ..fixture().space
    };
    let query = vector(&[1.0, 1.0, 1.0]);
    // Three placements of the unit residual, and both orders of the large terms.
    for values in [
        [1e16, 1.0, -1e16],
        [1.0, 1e16, -1e16],
        [1e16, -1e16, 1.0],
        [-1e16, 1.0, 1e16],
        [1.0, -1e16, 1e16],
        [-1e16, 1e16, 1.0],
    ] {
        assert_eq!(
            distance(&space, &query, &vector(&values)),
            Ok(-1.0),
            "{values:?}"
        );
    }
}

#[test]
fn negdot_cancellation_does_not_reverse_knn_or_rerank_winner() {
    let mut f = fixture();
    f.space.dimension = 3;
    f.space.metric = Metric::NegDot;
    f.documents.truncate(2);
    f.documents[0].original = Some(vector(&[1e16, 1.0, -1e16]));
    f.documents[1].original = Some(vector(&[0.0, 0.5, 0.0]));
    let q = VectorRequest {
        query: vector(&[1.0, 1.0, 1.0]),
        ..request(1)
    };
    let input = batch(vec![row("b", "B", "th"), row("a", "A", "en")]);
    for stage in [Stage::Knn(q.clone()), Stage::Rerank(q)] {
        let result = execute(input.clone(), &f, &[stage, Stage::Take(1)]).unwrap();
        assert_eq!(keys(&result), ["a"]);
        assert_eq!(result.rows[0].hit.as_ref().unwrap().value, -1.0);
    }
}

#[test]
fn huge_finite_negdot_query_is_valid_without_evaluating_self_dot() {
    let mut f = fixture();
    f.space.metric = Metric::NegDot;
    let q = VectorRequest {
        query: vector(&[1e308, 0.0]),
        ..request(1)
    };
    assert_eq!(
        distance(&f.space, &q.query, &q.query),
        Err(Error::NonfiniteDistance)
    );
    assert!(knn(batch(vec![]), &f, &q).unwrap().rows.is_empty());
    assert!(rerank(batch(vec![]), &f, &q).unwrap().rows.is_empty());
    f.documents[0].original = Some(vector(&[1e-308, 0.0]));
    let result = knn(batch(vec![row("a", "A", "en")]), &f, &q).unwrap();
    close(result.rows[0].hit.as_ref().unwrap().value, -1.0);
}

#[test]
fn bm25_finite_extreme_k1_retains_matching_row_and_expected_score() {
    let mut f = fixture();
    f.documents = vec![
        doc("A", 0.1, "red x x x", &["red", "x", "x", "x"]),
        doc("B", 0.2, "", &[]),
    ];
    let q = LexicalRequest {
        k1: f64::MAX,
        b: 1.0,
        ..lexical(1)
    };
    let result = lexical_rank(batch(vec![row("a", "A", "en")]), &f, &q).unwrap();
    assert_eq!(keys(&result), ["a"]);
    // N=2, df=1, tf=1, dl/avgdl=2: ln(2)*(k1+1)/(2*k1+1).
    close(
        result.rows[0].hit.as_ref().unwrap().value,
        std::f64::consts::LN_2 / 2.0,
    );
}

#[test]
fn rrf_representative_payload_and_downstream_filter_are_branch_order_independent() {
    let mut dense = row("z", "A", "en");
    dense.source_ranks.insert("dense".into(), 1);
    let mut lexical = row("a", "A", "th");
    lexical.source_ranks.insert("lexical".into(), 1);
    let dense = batch(vec![dense]);
    let lexical = batch(vec![lexical]);
    let forward = rrf(&[dense.clone(), lexical.clone()], 60.0).unwrap();
    let reverse = rrf(&[lexical, dense], 60.0).unwrap();
    assert_eq!(forward, reverse);
    assert_eq!(forward.rows[0].row_key, "a");
    assert_eq!(forward.rows[0].language, "th");
    assert_eq!(
        forward.rows[0].source_ranks,
        BTreeMap::from([("dense".into(), 1), ("lexical".into(), 1)])
    );
    close(forward.rows[0].hit.as_ref().unwrap().value, 2.0 / 61.0);
    for fused in [forward, reverse] {
        assert_eq!(
            keys(&execute(fused, &fixture(), &[Stage::FilterLanguage("th".into())]).unwrap()),
            ["a"]
        );
    }
}

#[test]
fn rrf_conflicting_payloads_for_the_same_row_key_are_rejected() {
    let mut dense = row("same", "A", "en");
    dense.source_ranks.insert("dense".into(), 1);
    let mut lexical = row("same", "A", "th");
    lexical.source_ranks.insert("lexical".into(), 1);
    for rows in [[dense.clone(), lexical.clone()], [lexical, dense]] {
        assert_eq!(
            rrf(&rows.map(|r| batch(vec![r])), 60.0),
            Err(Error::DuplicateRowKey)
        );
    }
}
