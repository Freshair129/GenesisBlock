//! Independent, standard-library-only P7 ranking/context fixtures. No production evaluator.
//! Fixture contract v1: f64 originals; pretokenized BM25; scalar-count tokenizer.
//! Approved boundary: ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY, P7.
//! This is a fixture oracle, not an index, tokenizer model, ACL service or runtime.
//! Fingerprints and source hashes are explicit fixture assertions, not computed hashes.
//! Adapters retain relational payloads by unique occurrence row_key. Ranking preserves
//! bags; equal scores use lexicographic row_key. Only explicit RRF groups entities.
//! Owners here have no namespace/kind discriminator: adapters MUST constrain a
//! batch/corpus to one explicit namespace and Node kind before calling this module.
//! Other owner kinds and quantized-only vector representations are unsupported.
use std::collections::{BTreeMap, BTreeSet};

pub type Outcome<T> = Result<T, Error>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    DimensionMismatch,
    SpaceMismatch,
    NonfiniteVector,
    ZeroCosineNorm,
    NonfiniteDistance,
    OriginalUnavailable,
    OwnerUnknown,
    DuplicateRowKey,
    DuplicateDocument,
    AnalyzerMismatch,
    InvalidBm25,
    SnapshotMismatch,
    InvalidSourceRank,
    InvalidRrf,
    TokenizerUnavailable,
    InvalidEvidence,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub transaction: u64,
    pub valid: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Visibility {
    pub tx_from: u64,
    pub tx_to: Option<u64>,
    pub valid_from: i64,
    pub valid_to: Option<i64>,
    pub authorized: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    L2Squared,
    Cosine,
    NegDot,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Space {
    pub fingerprint: String,
    pub dimension: usize,
    pub metric: Metric,
}
#[derive(Clone, Debug, PartialEq)]
pub struct OriginalVector {
    pub fingerprint: String,
    pub values: Vec<f64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub id: String,
    pub revision: String,
    pub visibility: Visibility,
    pub original: Option<OriginalVector>,
    pub text: String,
    pub source_hash: String,
    pub analyzer_fingerprint: String,
    pub tokens: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct Fixture {
    pub space: Space,
    pub analyzer_fingerprint: String,
    pub documents: Vec<Document>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub value: f64,
    pub rank: usize,
    pub source: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub row_key: String,
    pub owner_id: String,
    pub revision: String,
    pub language: String,
    pub hit: Option<Hit>,
    pub source_ranks: BTreeMap<String, usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    WholeInput,
    RerankCandidates,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lineage {
    pub approximate_sources: Vec<String>,
    pub scope: Scope,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    pub snapshot: Snapshot,
    pub rows: Vec<Candidate>,
    pub lineage: Lineage,
}
#[derive(Clone, Debug)]
pub struct VectorRequest {
    pub query: OriginalVector,
    pub top: usize,
    pub source: String,
}
#[derive(Clone, Debug)]
pub struct LexicalRequest {
    pub analyzer_fingerprint: String,
    pub terms: Vec<String>,
    pub k1: f64,
    pub b: f64,
    pub top: usize,
    pub source: String,
}
#[derive(Clone, Debug)]
pub enum Stage {
    FilterLanguage(String),
    Knn(VectorRequest),
    Rerank(VectorRequest),
    Lexical(LexicalRequest),
    Take(usize),
    SourceRank(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixtureTokenizer {
    UnicodeScalarV1,
}
#[derive(Clone, Debug, Default)]
pub struct TokenizerRegistry {
    pub entries: BTreeMap<String, FixtureTokenizer>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRef {
    pub row_key: String,
    pub owner_id: String,
    pub revision: String,
    pub source_hash: String,
    pub start_scalar: usize,
    pub end_scalar: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment {
    pub evidence: EvidenceRef,
    pub text: String,
    pub citation: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackage {
    pub rendered_context: String,
    pub token_count: usize,
    pub token_budget: usize,
    pub tokenizer_fingerprint: String,
    pub diagnostic_envelope_counted: bool,
    pub fragments: Vec<Fragment>,
    pub omitted_refs: Vec<EvidenceRef>,
    pub truncated: bool,
    pub truncation_reason: Option<String>,
    pub lineage: Lineage,
}

fn visible(d: &Document, s: Snapshot) -> bool {
    let v = &d.visibility;
    v.authorized
        && v.tx_from <= s.transaction
        && v.tx_to.is_none_or(|end| s.transaction < end)
        && v.valid_from <= s.valid
        && v.valid_to.is_none_or(|end| s.valid < end)
}

fn owner<'a>(row: &Candidate, fixture: &'a Fixture) -> Outcome<&'a Document> {
    fixture
        .documents
        .iter()
        .find(|d| d.id == row.owner_id && d.revision == row.revision)
        .ok_or(Error::OwnerUnknown)
}

fn validate_keys(input: &Batch) -> Outcome<()> {
    let mut keys = BTreeSet::new();
    for row in &input.rows {
        if row.row_key.is_empty() || !keys.insert(&row.row_key) {
            return Err(Error::DuplicateRowKey);
        }
    }
    Ok(())
}

// All entry points gate owners, including pack on a batch provided by an adapter.
fn visible_batch(mut input: Batch, fixture: &Fixture) -> Outcome<Batch> {
    validate_keys(&input)?;
    let mut ids = BTreeSet::new();
    for d in &fixture.documents {
        if !ids.insert((&d.id, &d.revision)) {
            return Err(Error::DuplicateDocument);
        }
    }
    let mut rows = Vec::new();
    for row in input.rows {
        if visible(owner(&row, fixture)?, input.snapshot) {
            rows.push(row);
        }
    }
    input.rows = rows;
    Ok(input)
}

fn validate_vector(space: &Space, vector: &OriginalVector) -> Outcome<()> {
    if space.fingerprint.is_empty() || vector.fingerprint != space.fingerprint {
        return Err(Error::SpaceMismatch);
    }
    if space.dimension == 0 || vector.values.len() != space.dimension {
        return Err(Error::DimensionMismatch);
    }
    if vector.values.iter().any(|v| !v.is_finite()) {
        return Err(Error::NonfiniteVector);
    }
    if space.metric == Metric::Cosine && vector.values.iter().all(|v| *v == 0.0) {
        return Err(Error::ZeroCosineNorm);
    }
    Ok(())
}

// Neumaier compensation preserves small residuals when large products cancel.
// This is f64 arithmetic, not arbitrary precision: nonfinite intermediates fail.
fn compensated_sum(values: impl Iterator<Item = f64>) -> Outcome<f64> {
    let (mut sum, mut correction) = (0.0_f64, 0.0_f64);
    for value in values {
        let next = sum + value;
        if !value.is_finite() || !next.is_finite() {
            return Err(Error::NonfiniteDistance);
        }
        correction += if sum.abs() >= value.abs() {
            (sum - next) + value
        } else {
            (value - next) + sum
        };
        if !correction.is_finite() {
            return Err(Error::NonfiniteDistance);
        }
        sum = next;
    }
    let result = sum + correction;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(Error::NonfiniteDistance)
    }
}

/// Original f64 distance, lower is better. L2 is squared; DOT is compensated and negated.
/// Cosine scales each operand independently to avoid finite-input norm overflow.
pub fn distance(space: &Space, a: &OriginalVector, b: &OriginalVector) -> Outcome<f64> {
    validate_vector(space, a)?;
    validate_vector(space, b)?;
    let (a, b) = (&a.values, &b.values);
    let result = match space.metric {
        Metric::L2Squared => a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum(),
        Metric::NegDot => -compensated_sum(a.iter().zip(b).map(|(x, y)| x * y))?,
        Metric::Cosine => {
            let sa = a.iter().fold(0.0_f64, |s, x| s.max(x.abs()));
            let sb = b.iter().fold(0.0_f64, |s, x| s.max(x.abs()));
            let (mut dot, mut na, mut nb) = (0.0, 0.0, 0.0);
            for (x, y) in a.iter().zip(b) {
                let (x, y) = (x / sa, y / sb);
                dot += x * y;
                na += x * x;
                nb += y * y;
            }
            1.0 - (dot / na.sqrt() / nb.sqrt()).clamp(-1.0, 1.0)
        }
    };
    if result.is_finite() {
        Ok(result)
    } else {
        Err(Error::NonfiniteDistance)
    }
}

fn ranked(
    mut input: Batch,
    mut scores: Vec<(Candidate, f64)>,
    source: &str,
    top: usize,
    descending: bool,
) -> Outcome<Batch> {
    if source.is_empty() {
        return Err(Error::InvalidSourceRank);
    }
    scores.sort_by(|(a, x), (b, y)| {
        let cmp = x.partial_cmp(y).expect("validated finite scores");
        (if descending { cmp.reverse() } else { cmp }).then_with(|| a.row_key.cmp(&b.row_key))
    });
    input.rows = scores
        .into_iter()
        .take(top)
        .enumerate()
        .map(|(i, (mut row, value))| {
            row.hit = Some(Hit {
                value,
                rank: i + 1,
                source: source.into(),
            });
            row
        })
        .collect();
    Ok(input)
}

fn vector_rank(
    input: Batch,
    fixture: &Fixture,
    request: &VectorRequest,
    rerank: bool,
) -> Outcome<Batch> {
    // Query binding precedes any empty-input / TOP 0 shortcut.
    validate_vector(&fixture.space, &request.query)?;
    let mut input = visible_batch(input, fixture)?;
    let mut scores = Vec::new();
    for row in &input.rows {
        let Some(original) = owner(row, fixture)?.original.as_ref() else {
            if rerank {
                return Err(Error::OriginalUnavailable);
            }
            continue;
        };
        scores.push((
            row.clone(),
            distance(&fixture.space, &request.query, original)?,
        ));
    }
    if rerank || !input.lineage.approximate_sources.is_empty() {
        input.lineage.scope = Scope::RerankCandidates;
    }
    ranked(input, scores, &request.source, request.top, false)
}

/// Exact scan of the supplied visible input bag, without ANN candidate generation.
/// Missing-vector rows are excluded before TOP; adapters track their count if needed.
pub fn knn(input: Batch, fixture: &Fixture, request: &VectorRequest) -> Outcome<Batch> {
    vector_rank(input, fixture, request, false)
}

/// Exact originals reorder only the supplied candidates; lineage cannot be erased.
pub fn rerank(input: Batch, fixture: &Fixture, request: &VectorRequest) -> Outcome<Batch> {
    vector_rank(input, fixture, request, true)
}

/// BM25 fixture profile: natural-log positive IDF, distinct pretokenized query
/// terms, OR matching (nonmatching rows omitted), all visible revisions in corpus
/// statistics, including empty documents. No text normalization or segmentation.
pub fn lexical_rank(input: Batch, fixture: &Fixture, request: &LexicalRequest) -> Outcome<Batch> {
    if fixture.analyzer_fingerprint.is_empty()
        || request.analyzer_fingerprint != fixture.analyzer_fingerprint
    {
        return Err(Error::AnalyzerMismatch);
    }
    if !request.k1.is_finite()
        || request.k1 <= 0.0
        || !request.b.is_finite()
        || !(0.0..=1.0).contains(&request.b)
    {
        return Err(Error::InvalidBm25);
    }
    let input = visible_batch(input, fixture)?;
    let corpus: Vec<_> = fixture
        .documents
        .iter()
        .filter(|d| visible(d, input.snapshot))
        .collect();
    if corpus
        .iter()
        .any(|d| d.analyzer_fingerprint != request.analyzer_fingerprint)
    {
        return Err(Error::AnalyzerMismatch);
    }
    let terms: BTreeSet<_> = request.terms.iter().collect();
    let avg = if corpus.is_empty() {
        0.0
    } else {
        corpus.iter().map(|d| d.tokens.len() as f64).sum::<f64>() / corpus.len() as f64
    };
    let mut idfs = BTreeMap::new();
    for term in terms {
        let df = corpus.iter().filter(|d| d.tokens.contains(term)).count() as f64;
        idfs.insert(
            term,
            (1.0 + (corpus.len() as f64 - df + 0.5) / (df + 0.5)).ln(),
        );
    }
    let mut scores = Vec::new();
    for row in &input.rows {
        let d = owner(row, fixture)?;
        let mut score = 0.0;
        for (term, idf) in &idfs {
            let tf = d.tokens.iter().filter(|t| *t == *term).count() as f64;
            if tf > 0.0 {
                let length = 1.0 - request.b + request.b * (d.tokens.len() as f64 / avg);
                // Divide both sides by k1 for large finite k1. The ordinary
                // form below avoids reciprocal overflow for tiny positive k1.
                let weight = if request.k1 >= 1.0 {
                    tf * (1.0 + 1.0 / request.k1) / (tf / request.k1 + length)
                } else {
                    tf * (request.k1 + 1.0) / (tf + request.k1 * length)
                };
                score += idf * weight;
            }
        }
        if !score.is_finite() {
            return Err(Error::InvalidBm25);
        }
        if score > 0.0 {
            scores.push((row.clone(), score));
        }
    }
    ranked(input, scores, &request.source, request.top, true)
}

/// Run stages strictly in declared order. TAKE/filter preserve existing source ranks.
/// SourceRank freezes the current hit rank, not the post-filter ordinal.
pub fn execute(input: Batch, fixture: &Fixture, stages: &[Stage]) -> Outcome<Batch> {
    let mut input = visible_batch(input, fixture)?;
    for stage in stages {
        input = match stage {
            Stage::FilterLanguage(language) => {
                input.rows.retain(|r| r.language == *language);
                input
            }
            Stage::Take(n) => {
                input.rows.truncate(*n);
                input
            }
            Stage::Knn(request) => knn(input, fixture, request)?,
            Stage::Rerank(request) => rerank(input, fixture, request)?,
            Stage::Lexical(request) => lexical_rank(input, fixture, request)?,
            Stage::SourceRank(source) => {
                if source.is_empty() {
                    return Err(Error::InvalidSourceRank);
                }
                for row in &mut input.rows {
                    let hit = row.hit.as_ref().ok_or(Error::InvalidSourceRank)?;
                    if hit.source != *source
                        || hit.rank == 0
                        || row.source_ranks.contains_key(source)
                    {
                        return Err(Error::InvalidSourceRank);
                    }
                    row.source_ranks.insert(source.clone(), hit.rank);
                }
                input
            }
        };
    }
    Ok(input)
}

/// Explicit UNION-source / group-by-(id,revision) fixture fusion. Each source must
/// belong to one input, with one positive unique rank and one row per entity.
/// Ambiguous same-source duplicate entities are rejected, not silently deduped.
/// Missing-source contribution is zero; source cutoffs/ranks are never recomputed.
/// The complete representative payload comes from the minimum key for that entity.
/// One stable key cannot denote conflicting entity/language payloads across sources.
pub fn rrf(inputs: &[Batch], k: f64) -> Outcome<Batch> {
    let first = inputs.first().ok_or(Error::InvalidRrf)?;
    if !k.is_finite() || k < 0.0 {
        return Err(Error::InvalidRrf);
    }
    let mut sources = BTreeMap::new();
    let mut ranks = BTreeSet::new();
    let mut key_owners = BTreeMap::new();
    let mut groups: BTreeMap<(String, String), Candidate> = BTreeMap::new();
    let mut approximation = BTreeSet::new();
    let mut scope = Scope::WholeInput;
    for (index, input) in inputs.iter().enumerate() {
        if input.snapshot != first.snapshot {
            return Err(Error::SnapshotMismatch);
        }
        validate_keys(input)?;
        approximation.extend(input.lineage.approximate_sources.iter().cloned());
        if input.lineage.scope == Scope::RerankCandidates
            || !input.lineage.approximate_sources.is_empty()
        {
            scope = Scope::RerankCandidates;
        }
        for row in &input.rows {
            let entity = (row.owner_id.clone(), row.revision.clone());
            let payload = (entity.clone(), row.language.clone());
            if let Some(old) = key_owners.insert(row.row_key.clone(), payload.clone()) {
                if old != payload {
                    return Err(Error::DuplicateRowKey);
                }
            }
            if row.source_ranks.is_empty() {
                return Err(Error::InvalidSourceRank);
            }
            let group = groups.entry(entity).or_insert_with(|| {
                let mut copy = row.clone();
                copy.source_ranks.clear();
                copy.hit = None;
                copy
            });
            if row.row_key < group.row_key {
                let ranks = std::mem::take(&mut group.source_ranks);
                *group = row.clone();
                group.source_ranks = ranks;
                group.hit = None;
            }
            for (source, rank) in &row.source_ranks {
                if source.is_empty()
                    || *rank == 0
                    || sources.get(source).is_some_and(|i| *i != index)
                    || !ranks.insert((source.clone(), *rank))
                    || group.source_ranks.contains_key(source)
                {
                    return Err(Error::InvalidSourceRank);
                }
                sources.insert(source.clone(), index);
                group.source_ranks.insert(source.clone(), *rank);
            }
        }
    }
    let scores = groups
        .into_values()
        .map(|row| {
            // BTreeMap order makes accumulation independent of source input order.
            let score = row
                .source_ranks
                .values()
                .map(|rank| 1.0 / (k + *rank as f64))
                .sum();
            (row, score)
        })
        .collect();
    let output = Batch {
        snapshot: first.snapshot,
        rows: vec![],
        lineage: Lineage {
            approximate_sources: approximation.into_iter().collect(),
            scope,
        },
    };
    ranked(output, scores, "rrf", usize::MAX, true)
}

impl FixtureTokenizer {
    /// Test-only: one token per Unicode scalar, including whitespace/citation syntax.
    /// This is not a model tokenizer or grapheme/Thai word segmentation.
    pub fn count(self, text: &str) -> usize {
        match self {
            Self::UnicodeScalarV1 => text.chars().count(),
        }
    }
}

fn evidence(row: &Candidate, doc: &Document, start: usize, end: usize) -> EvidenceRef {
    EvidenceRef {
        row_key: row.row_key.clone(),
        owner_id: doc.id.clone(),
        revision: doc.revision.clone(),
        source_hash: doc.source_hash.clone(),
        start_scalar: start,
        end_scalar: end,
    }
}

/// Greedy ordered prefix extraction; `[n]` immediately follows each fragment,
/// with one newline between fragments. Counts the final rendered text exactly.
/// Partial suffixes and subsequent rows are omitted_refs with scalar spans.
/// No generated summaries, normalization, hidden-owner diagnostics or model calls.
pub fn pack_context(
    input: &Batch,
    fixture: &Fixture,
    registry: &TokenizerRegistry,
    tokenizer: &str,
    budget: usize,
) -> Outcome<ContextPackage> {
    let counter = *registry
        .entries
        .get(tokenizer)
        .ok_or(Error::TokenizerUnavailable)?;
    let input = visible_batch(input.clone(), fixture)?;
    let mut package = ContextPackage {
        rendered_context: String::new(),
        token_count: 0,
        token_budget: budget,
        tokenizer_fingerprint: tokenizer.into(),
        diagnostic_envelope_counted: false,
        fragments: vec![],
        omitted_refs: vec![],
        truncated: false,
        truncation_reason: None,
        lineage: input.lineage.clone(),
    };
    let mut stopped = false;
    for row in &input.rows {
        let doc = owner(row, fixture)?;
        if doc.source_hash.is_empty() || doc.revision.is_empty() {
            return Err(Error::InvalidEvidence);
        }
        let total = doc.text.chars().count();
        let citation = format!("[{}]", package.fragments.len() + 1);
        let separator = if package.fragments.is_empty() {
            ""
        } else {
            "\n"
        };
        let overhead = counter.count(separator) + counter.count(&citation);
        let remaining = budget - package.token_count;
        if stopped || remaining < overhead || (total > 0 && remaining == overhead) {
            package.omitted_refs.push(evidence(row, doc, 0, total));
            stopped = true;
            continue;
        }
        // The sole registered fixture tokenizer is additive per Unicode scalar.
        let kept = total.min(remaining - overhead);
        let text: String = doc.text.chars().take(kept).collect();
        package.rendered_context.push_str(separator);
        package.rendered_context.push_str(&text);
        package.rendered_context.push_str(&citation);
        package.token_count = counter.count(&package.rendered_context);
        package.fragments.push(Fragment {
            evidence: evidence(row, doc, 0, kept),
            text,
            citation,
        });
        if kept < total {
            package.omitted_refs.push(evidence(row, doc, kept, total));
            stopped = true;
        }
    }
    package.truncated = !package.omitted_refs.is_empty();
    if package.truncated {
        package.truncation_reason = Some("TOKEN_BUDGET".into());
    }
    Ok(package)
}
