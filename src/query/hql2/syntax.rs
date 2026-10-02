//! Full pinned HQL2 grammar frontend. No storage access or query execution.
use super::ast::*;
use super::error::QueryErrorV2;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::collections::BTreeMap;

#[derive(Parser)]
#[grammar = "query/hql2/hql2.pest"]
struct Hql2Parser;

const MAX_DEPTH: usize = 128;
const MAX_NODES: usize = 10_000;
const MAX_SOURCE_SCALARS: usize = 262_144;
const MAX_LEXICAL_UNITS: usize = 2_048;
const PARSER_PROFILE_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const PARSER_STACK_BYTES: u64 = 32 * 1024 * 1024;

struct ParserResources {
    heap: u64,
    stack: u64,
}

/// Conservative reservation for this pinned grammar/Pest implementation, not a
/// measured counter. Shared preflight bounds work BEFORE Pest builds its queue.
/// See RCA--HQL2-P8-PARSER-RESOURCE-BOUNDARY for the grammar/layout argument.
pub(crate) fn required_heap_bytes(source: &str) -> Result<u64, QueryErrorV2> {
    Ok(preflight(source)?.heap)
}

/// Additional worker stack reservation required before calling `parse_hql2`.
/// The query boundary must charge this separately from source/PEG/AST heap
/// allocations. Zero means no additional worker stack, not a zero-cost parser.
pub(crate) fn required_stack_bytes(source: &str) -> Result<u64, QueryErrorV2> {
    Ok(preflight(source)?.stack)
}

pub fn parse_hql2(source: &str) -> Result<Hql2Statement, QueryErrorV2> {
    // Public callers bypassing Storage receive the same initial 64 MiB profile.
    // Never parse first and discover the work/heap quota only from the AST.
    let resources = preflight(source)?;
    // Pest's recursive grammar frames are substantially deeper than source nesting.
    // Do not depend on an embedding caller's (possibly small) thread stack for
    // deeply nested, but legal, inputs. Ordinary queries remain on the caller.
    if resources.stack != 0 {
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .name("hql2-parser".into())
                .stack_size(PARSER_STACK_BYTES as usize)
                .spawn_scoped(scope, || parse_checked(source))
                .map_err(|_| {
                    QueryErrorV2::new("QUERY_BUDGET_EXCEEDED", "parse", "parser_stack_unavailable")
                })?
                .join()
                .map_err(|_| QueryErrorV2::parse(source, 0, "parser_failed"))?
        })
    } else {
        parse_checked(source)
    }
}

/// Lexical resource guard only; acceptance is exclusively decided by the PEG.
/// Scan iteratively before entering any recursive grammar, ignoring comments,
/// escaped strings and quoted names. NOT is iterative in the PEG as well.
fn preflight(source: &str) -> Result<ParserResources, QueryErrorV2> {
    if source.is_empty()
        || source.len() > MAX_SOURCE_SCALARS * 4
        || source.chars().take(MAX_SOURCE_SCALARS + 1).count() > MAX_SOURCE_SCALARS
    {
        return Err(QueryErrorV2::parse(source, 0, "source_length"));
    }
    let bytes = source.as_bytes();
    let mut index = 0;
    // Preflight itself must not allocate an input-sized work stack.
    let mut delimiters = [0u8; MAX_DEPTH];
    let mut depth = 0usize;
    let mut deepest = 0;
    let mut nots = 0;
    let mut units = 0usize;
    while index < bytes.len() {
        let start = index;
        let comment = bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/');
        if !bytes[index].is_ascii_whitespace() && !comment {
            units += 1;
            if units > MAX_LEXICAL_UNITS {
                return Err(QueryErrorV2::new(
                    "QUERY_BUDGET_EXCEEDED",
                    "parse",
                    "parser_work_limit",
                ));
            }
        }
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                    index += 1;
                }
            }
            b'"' | b'`' => {
                let quote = bytes[index];
                index += 1;
                while index < bytes.len() {
                    if bytes[index] == quote {
                        index += 1;
                        break;
                    }
                    if quote == b'"' && bytes[index] == b'\\' {
                        index += 1;
                    }
                    index += usize::from(index < bytes.len());
                }
                nots = 0;
            }
            b'(' | b'[' | b'{' => {
                if depth == MAX_DEPTH {
                    return Err(QueryErrorV2::parse(source, index, "syntax_depth_exceeded"));
                }
                delimiters[depth] = bytes[index];
                depth += 1;
                deepest = deepest.max(depth);
                index += 1;
                nots = 0;
            }
            b')' | b']' | b'}' => {
                let expected = match bytes[index] {
                    b')' => b'(',
                    b']' => b'[',
                    _ => b'{',
                };
                if depth == 0 || delimiters[depth - 1] != expected {
                    return Err(QueryErrorV2::parse(source, index, "unbalanced_delimiter"));
                }
                depth -= 1;
                index += 1;
                nots = 0;
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                if source[start..index].eq_ignore_ascii_case("NOT") {
                    nots += 1;
                    if nots > MAX_DEPTH {
                        return Err(QueryErrorV2::parse(
                            source,
                            start,
                            "expression_depth_exceeded",
                        ));
                    }
                    deepest = deepest.max(nots);
                } else {
                    nots = 0;
                }
            }
            b'0'..=b'9' => {
                index += 1;
                while index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
                nots = 0;
            }
            byte if byte.is_ascii_whitespace() => {
                index += 1;
            }
            _ => {
                index += 1;
                nots = 0;
            }
        }
    }
    if depth != 0 {
        return Err(QueryErrorV2::parse(
            source,
            source.len(),
            "unbalanced_delimiter",
        ));
    }
    // Retained queue: <=32 rules per lexical unit (+1 fixed), two tokens per
    // rule, <=64-byte token slot, 3x including growth overlap: 12288 bytes/unit.
    // Two CST copies at 96 bytes/node and 2x capacity receive another 12288;
    // 8192 remains for typed nodes/work/error scratch and CST growth overlap.
    // The fixed allowance covers depth/state overhead; source bytes cover text
    // copies, decoding and error-line buffers. Grammar/layout drift is tested.
    let heap = 4 * 1024 * 1024 + (units as u64 + 1) * 32_768 + source.len() as u64 * 16;
    let stack = if deepest > 8 { PARSER_STACK_BYTES } else { 0 };
    if heap + stack > PARSER_PROFILE_BYTES {
        return Err(QueryErrorV2::new(
            "QUERY_BUDGET_EXCEEDED",
            "parse",
            "parser_memory_profile",
        ));
    }
    Ok(ParserResources { heap, stack })
}

fn parse_checked(source: &str) -> Result<Hql2Statement, QueryErrorV2> {
    let root = Hql2Parser::parse(Rule::start, source)
        .map_err(|error| {
            let offset = match error.location {
                pest::error::InputLocation::Pos(offset)
                | pest::error::InputLocation::Span((offset, _)) => offset,
            };
            QueryErrorV2::parse(source, offset, "invalid_syntax")
        })?
        .next()
        .ok_or_else(|| QueryErrorV2::parse(source, 0, "missing_statement"))?;
    validate_tree(source, root.clone())?;
    let statement = root
        .clone()
        .into_inner()
        .find(|p| p.as_rule() != Rule::EOI)
        .ok_or_else(|| QueryErrorV2::parse(source, 0, "missing_statement"))?;
    let kind = match statement.as_rule() {
        Rule::query => StatementKind::Read,
        Rule::explain => {
            if statement
                .clone()
                .into_inner()
                .any(|p| p.as_rule() == Rule::k_analyze)
            {
                StatementKind::AnalyzeRead
            } else {
                StatementKind::Explain
            }
        }
        Rule::create_type
        | Rule::create_table
        | Rule::create_collection
        | Rule::create_index
        | Rule::drop_index => StatementKind::Ddl,
        Rule::admin => StatementKind::Admin,
        _ => StatementKind::Mutation,
    };
    let query = match kind {
        StatementKind::Read => Some(query(source, statement.clone())?),
        StatementKind::Explain | StatementKind::AnalyzeRead => Some(query(
            source,
            child(source, statement.clone(), Rule::query)?,
        )?),
        _ => None,
    };
    Ok(Hql2Statement {
        source: source.into(),
        span: span(&statement),
        kind,
        query,
        syntax: syntax_tree(root),
    })
}

fn span(pair: &Pair<'_, Rule>) -> Span {
    Span {
        start_byte: pair.as_span().start(),
        end_byte: pair.as_span().end(),
    }
}

fn at<T>(pair: &Pair<'_, Rule>, value: T) -> Spanned<T> {
    Spanned {
        span: span(pair),
        value,
    }
}

fn invalid(source: &str, pair: &Pair<'_, Rule>, reason: &str) -> QueryErrorV2 {
    QueryErrorV2::parse(source, pair.as_span().start(), reason)
}

fn child<'s>(
    source: &str,
    pair: Pair<'s, Rule>,
    rule: Rule,
) -> Result<Pair<'s, Rule>, QueryErrorV2> {
    pair.clone()
        .into_inner()
        .find(|p| p.as_rule() == rule)
        .ok_or_else(|| invalid(source, &pair, "invalid_syntax_tree"))
}

fn decode_string(source: &str, pair: &Pair<'_, Rule>) -> Result<String, QueryErrorV2> {
    serde_json::from_str(pair.as_str()).map_err(|_| invalid(source, pair, "invalid_string_literal"))
}

fn name(pair: Pair<'_, Rule>) -> Name {
    let text = pair.as_str();
    at(
        &pair,
        text.strip_prefix('`')
            .and_then(|s| s.strip_suffix('`'))
            .unwrap_or(text)
            .to_string(),
    )
}

fn number(source: &str, pair: &Pair<'_, Rule>) -> Result<ExprKind, QueryErrorV2> {
    if pair.as_str().contains(['.', 'e', 'E']) {
        let value = pair
            .as_str()
            .parse::<f64>()
            .map_err(|_| invalid(source, pair, "invalid_number"))?;
        if !value.is_finite() {
            return Err(invalid(source, pair, "nonfinite_literal"));
        }
        Ok(ExprKind::F64(value))
    } else {
        pair.as_str()
            .parse::<i64>()
            .map(ExprKind::I64)
            .map_err(|_| invalid(source, pair, "integer_literal_overflow"))
    }
}

fn unsigned(source: &str, pair: Pair<'_, Rule>) -> Result<Unsigned, QueryErrorV2> {
    let part = pair
        .clone()
        .into_inner()
        .next()
        .ok_or_else(|| invalid(source, &pair, "invalid_unsigned"))?;
    let value = if part.as_rule() == Rule::parameter {
        UnsignedKind::Parameter(part.as_str()[1..].into())
    } else {
        UnsignedKind::Literal(
            part.as_str()
                .parse()
                .map_err(|_| invalid(source, &part, "unsigned_literal_overflow"))?,
        )
    };
    Ok(at(&pair, value))
}

fn validate_tree(source: &str, root: Pair<'_, Rule>) -> Result<(), QueryErrorV2> {
    let mut pending = vec![(root, false)];
    let mut count = 0;
    while let Some((node, within_expression)) = pending.pop() {
        count += 1;
        if count > MAX_NODES {
            return Err(invalid(source, &node, "syntax_nodes_exceeded"));
        }
        match node.as_rule() {
            Rule::number => {
                number(source, &node)?;
            }
            Rule::uint => {
                node.as_str()
                    .parse::<u64>()
                    .map_err(|_| invalid(source, &node, "unsigned_literal_overflow"))?;
            }
            Rule::string => {
                decode_string(source, &node)?;
            }
            Rule::expr if !within_expression => {
                expression(source, node.clone())?;
            }
            _ => {}
        }
        let within_expression = within_expression || node.as_rule() == Rule::expr;
        pending.extend(node.into_inner().map(|p| (p, within_expression)));
    }
    Ok(())
}

/// Build the lossless production tree without recursive traversal.
fn syntax_tree(root: Pair<'_, Rule>) -> SyntaxNode {
    enum Work<'s> {
        Visit(Pair<'s, Rule>),
        Finish(SyntaxNode, usize),
    }
    let mut work = vec![Work::Visit(root)];
    let mut built = Vec::new();
    while let Some(next) = work.pop() {
        match next {
            Work::Visit(pair) => {
                let mut node = SyntaxNode {
                    kind: pair.as_rule(),
                    span: span(&pair),
                    text: None,
                    children: Vec::new(),
                };
                let children: Vec<_> = pair.clone().into_inner().collect();
                if children.is_empty() {
                    node.text = Some(pair.as_str().into());
                }
                work.push(Work::Finish(node, children.len()));
                work.extend(children.into_iter().rev().map(Work::Visit));
            }
            Work::Finish(mut node, count) => {
                node.children = built.split_off(built.len() - count);
                built.push(node);
            }
        }
    }
    built
        .pop()
        .expect("one grammar root always produces one syntax root")
}

fn query(source: &str, pair: Pair<'_, Rule>) -> Result<Query, QueryErrorV2> {
    let query_span = span(&pair);
    let mut scope = None;
    let body = if pair.as_rule() == Rule::query {
        if let Some(item) = pair
            .clone()
            .into_inner()
            .find(|p| p.as_rule() == Rule::scope)
        {
            let namespace = name(child(source, item.clone(), Rule::name)?);
            let mut tx = None;
            let mut valid_at = None;
            for selector in item.clone().into_inner() {
                match selector.as_rule() {
                    Rule::tx_selector => {
                        tx = Some(unsigned(source, child(source, selector, Rule::unsigned)?)?);
                    }
                    Rule::valid_selector => {
                        let text = child(source, selector, Rule::string)?;
                        valid_at = Some(at(&text, decode_string(source, &text)?));
                    }
                    _ => {}
                }
            }
            scope = Some(Scope {
                span: span(&item),
                namespace,
                tx,
                valid_at,
            });
        }
        child(source, pair, Rule::query_body)?
    } else {
        pair
    };
    let mut parts = body.clone().into_inner();
    let first = parts
        .next()
        .ok_or_else(|| invalid(source, &body, "missing_source"))?;
    let source_node = source_node(source, first)?;
    let mut stages = Vec::new();
    let mut returning = Vec::new();
    for part in parts {
        if part.as_rule() == Rule::return_stage {
            returning = select_list(source, child(source, part, Rule::select_list)?)?;
        } else {
            stages.push(stage(source, part)?);
        }
    }
    Ok(Query {
        span: query_span,
        scope,
        source: source_node,
        stages,
        returning,
    })
}

struct ParsedPatternNode {
    alias: Name,
    id: Option<Expr>,
    labels: Vec<Name>,
    properties: BTreeMap<String, Expr>,
}

fn pattern_node(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Option<ParsedPatternNode>, QueryErrorV2> {
    let mut alias = None;
    let mut label = None;
    let mut id = None;
    let mut properties = BTreeMap::new();
    for part in pair.clone().into_inner() {
        match part.as_rule() {
            Rule::name => {
                let is_label = source[..part.as_span().start()].trim_end().ends_with(':');
                if is_label {
                    if label.is_some() {
                        return Err(invalid(source, &part, "duplicate_pattern_label"));
                    }
                    label = Some(name(part));
                } else if alias.is_none() {
                    alias = Some(name(part));
                } else {
                    return Err(invalid(source, &part, "pattern_node_shape"));
                }
            }
            Rule::object => {
                for field in part.into_inner() {
                    if field.as_rule() != Rule::pair {
                        continue;
                    }
                    let key_pair = field
                        .clone()
                        .into_inner()
                        .next()
                        .ok_or_else(|| invalid(source, &field, "missing_object_key"))?;
                    let key = if key_pair.as_rule() == Rule::string {
                        decode_string(source, &key_pair)?
                    } else {
                        key_pair.as_str().to_owned()
                    };
                    let value_pair = child(source, field.clone(), Rule::expr)?;
                    let value = expression(source, value_pair)?.0;
                    if key == "id" {
                        if id.replace(value).is_some() {
                            return Err(invalid(source, &field, "duplicate_pattern_id"));
                        }
                    } else if properties.insert(key, value).is_some() {
                        return Err(invalid(source, &field, "duplicate_pattern_property"));
                    }
                }
            }
            _ => {}
        }
    }
    let Some(alias) = alias else {
        return Ok(None);
    };
    Ok(Some(ParsedPatternNode {
        alias,
        id,
        labels: label.into_iter().collect(),
        properties,
    }))
}

type ExpandedEdgeV2 = (
    Option<Name>,
    Vec<Name>,
    BTreeMap<String, Expr>,
    GraphDirection,
    u32,
    u32,
);

fn expand_edge(
    source: &str,
    pattern: &Pair<'_, Rule>,
    pair: Pair<'_, Rule>,
) -> Result<ExpandedEdgeV2, QueryErrorV2> {
    let raw_edge = pair.as_str().trim_start();
    let direction = if raw_edge.starts_with("<-") {
        GraphDirection::In
    } else if raw_edge.ends_with("->") {
        GraphDirection::Out
    } else {
        GraphDirection::Both
    };
    let mut edge_alias = None;
    let mut relations = Vec::new();
    let mut properties = BTreeMap::new();
    let mut min_hops = 1;
    let mut max_hops = 1;
    if let Some(detail) = pair
        .into_inner()
        .find(|part| part.as_rule() == Rule::edge_detail)
    {
        for item in detail.into_inner() {
            match item.as_rule() {
                Rule::name if edge_alias.is_none() => edge_alias = Some(name(item)),
                Rule::relation_list => {
                    relations.extend(
                        item.into_inner()
                            .filter(|part| part.as_rule() == Rule::name)
                            .map(name),
                    );
                }
                Rule::object => {
                    for field in item.into_inner() {
                        if field.as_rule() != Rule::pair {
                            continue;
                        }
                        let key_pair = field
                            .clone()
                            .into_inner()
                            .next()
                            .ok_or_else(|| invalid(source, &field, "missing_object_key"))?;
                        let key = if key_pair.as_rule() == Rule::string {
                            decode_string(source, &key_pair)?
                        } else {
                            key_pair.as_str().to_owned()
                        };
                        let value =
                            expression(source, child(source, field.clone(), Rule::expr)?)?.0;
                        if properties.insert(key, value).is_some() {
                            return Err(invalid(source, &field, "duplicate_pattern_property"));
                        }
                    }
                }
                Rule::hop_range => {
                    let bounds = item
                        .into_inner()
                        .map(|part| {
                            part.as_str()
                                .parse::<u32>()
                                .map_err(|_| invalid(source, &part, "hop_range_overflow"))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if bounds.len() != 2 {
                        return Err(invalid(source, pattern, "hop_range_shape"));
                    }
                    min_hops = bounds[0];
                    max_hops = bounds[1];
                }
                _ => {}
            }
        }
    }
    if min_hops > max_hops || max_hops > 32 {
        return Err(invalid(source, pattern, "hop_bounds"));
    }
    Ok((
        edge_alias, relations, properties, direction, min_hops, max_hops,
    ))
}

fn expand_stage(source: &str, pair: Pair<'_, Rule>) -> Result<Option<StageKind>, QueryErrorV2> {
    let pattern = child(source, pair.clone(), Rule::pattern)?;
    let parts: Vec<_> = pattern.clone().into_inner().collect();
    if parts.len() < 3 || parts.len() % 2 == 0 || parts.len() > 65 {
        return Ok(None);
    }
    if parts.iter().enumerate().any(|(index, part)| {
        part.as_rule()
            != if index % 2 == 0 {
                Rule::node_pattern
            } else {
                Rule::edge_pattern
            }
    }) {
        return Ok(None);
    }
    let Some(start_node) = pattern_node(source, parts[0].clone())? else {
        return Ok(None);
    };
    let mut path_alias = None;
    let mut mode = GraphPathMode::Trail;
    for item in pair.clone().into_inner() {
        match item.as_rule() {
            Rule::name => path_alias = Some(name(item)),
            Rule::path_mode => {
                mode = match item.as_str().to_ascii_lowercase().as_str() {
                    "simple" => GraphPathMode::Simple,
                    "walk" => GraphPathMode::Walk,
                    _ => GraphPathMode::Trail,
                }
            }
            _ => {}
        }
    }
    let optional = pair
        .clone()
        .into_inner()
        .any(|part| part.as_rule() == Rule::k_optional);
    let mut steps = Vec::with_capacity((parts.len() - 1) / 2);
    for index in (1..parts.len()).step_by(2) {
        let Some(end_node) = pattern_node(source, parts[index + 1].clone())? else {
            return Ok(None);
        };
        let (edge_alias, relations, edge_properties, direction, min_hops, max_hops) =
            expand_edge(source, &pattern, parts[index].clone())?;
        steps.push(GraphSequenceStep {
            end_alias: end_node.alias,
            node_id: end_node.id,
            node_labels: end_node.labels,
            node_properties: end_node.properties,
            edge_alias,
            edge_properties,
            relations,
            direction,
            min_hops,
            max_hops,
        });
    }
    let constrained = start_node.id.is_some()
        || !start_node.labels.is_empty()
        || !start_node.properties.is_empty()
        || steps.iter().any(|step| {
            step.node_id.is_some()
                || !step.node_labels.is_empty()
                || !step.node_properties.is_empty()
                || !step.edge_properties.is_empty()
        });
    if steps.len() > 1 || constrained {
        return Ok(Some(StageKind::ExpandSequence {
            start_alias: start_node.alias,
            start_id: start_node.id,
            start_labels: start_node.labels,
            start_properties: start_node.properties,
            steps,
            mode,
            path_alias,
            optional,
        }));
    }
    let step = steps
        .pop()
        .ok_or_else(|| invalid(source, &pattern, "empty_pattern"))?;
    Ok(Some(StageKind::Expand {
        start_alias: start_node.alias,
        end_alias: step.end_alias,
        edge_alias: step.edge_alias,
        relations: step.relations,
        direction: step.direction,
        min_hops: step.min_hops,
        max_hops: step.max_hops,
        mode,
        path_alias,
        optional,
    }))
}

fn match_source(source: &str, pair: Pair<'_, Rule>) -> Result<Option<SourceKind>, QueryErrorV2> {
    let pattern = child(source, pair.clone(), Rule::pattern)?;
    let parts: Vec<_> = pattern.clone().into_inner().collect();
    if parts.len() < 3 || parts.len() % 2 == 0 || parts.len() > 65 {
        return Ok(None);
    }
    if parts.iter().enumerate().any(|(index, part)| {
        part.as_rule()
            != if index % 2 == 0 {
                Rule::node_pattern
            } else {
                Rule::edge_pattern
            }
    }) {
        return Ok(None);
    }
    let Some(start_node) = pattern_node(source, parts[0].clone())? else {
        return Ok(None);
    };
    let mut steps = Vec::with_capacity((parts.len() - 1) / 2);
    for index in (1..parts.len()).step_by(2) {
        let Some(end_node) = pattern_node(source, parts[index + 1].clone())? else {
            return Ok(None);
        };
        let (edge_alias, relations, edge_properties, direction, min_hops, max_hops) =
            expand_edge(source, &pattern, parts[index].clone())?;
        steps.push(GraphSequenceStep {
            end_alias: end_node.alias,
            node_id: end_node.id,
            node_labels: end_node.labels,
            node_properties: end_node.properties,
            edge_alias,
            edge_properties,
            relations,
            direction,
            min_hops,
            max_hops,
        });
    }
    let mut path_alias = None;
    let mut mode = GraphPathMode::Trail;
    let mut shortest = false;
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::name => path_alias = Some(name(item)),
            Rule::path_mode => {
                mode = match item.as_str().to_ascii_lowercase().as_str() {
                    "simple" => GraphPathMode::Simple,
                    "walk" => GraphPathMode::Walk,
                    _ => GraphPathMode::Trail,
                }
            }
            Rule::k_shortest => shortest = true,
            _ => {}
        }
    }
    Ok(Some(SourceKind::Match {
        start_alias: start_node.alias,
        start_id: start_node.id,
        start_labels: start_node.labels,
        start_properties: start_node.properties,
        steps,
        mode,
        path_alias,
        shortest,
    }))
}

fn source_node(source: &str, pair: Pair<'_, Rule>) -> Result<Source, QueryErrorV2> {
    let mut names: Vec<_> = pair
        .clone()
        .into_inner()
        .filter(|p| p.as_rule() == Rule::name)
        .map(name)
        .collect();
    let value = match pair.as_rule() {
        Rule::values_source => {
            let param = child(source, pair.clone(), Rule::parameter)?;
            SourceKind::Values {
                parameter: at(&param, param.as_str()[1..].into()),
                alias: names.remove(0),
            }
        }
        Rule::node_source | Rule::edge_source => {
            let alias = names
                .pop()
                .ok_or_else(|| invalid(source, &pair, "missing_alias"))?;
            if pair.as_rule() == Rule::node_source {
                SourceKind::Nodes {
                    label: names.pop(),
                    alias,
                }
            } else {
                SourceKind::Edges {
                    relation: names.pop(),
                    alias,
                }
            }
        }
        Rule::row_source => SourceKind::Rows {
            table: names.remove(0),
            alias: names.remove(0),
        },
        Rule::ann_source => SourceKind::Annotations {
            alias: names.remove(0),
        },
        Rule::history_source => {
            let kind_pair = child(source, pair.clone(), Rule::kind)?;
            let kind = kind_pair
                .clone()
                .into_inner()
                .next()
                .ok_or_else(|| invalid(source, &kind_pair, "missing_history_kind"))?;
            let id_pair = child(source, pair.clone(), Rule::id_value)?;
            let id_atom = id_pair
                .clone()
                .into_inner()
                .next()
                .ok_or_else(|| invalid(source, &id_pair, "missing_history_id"))?;
            let id = match id_atom.as_rule() {
                Rule::string => at(&id_atom, ExprKind::Utf8(decode_string(source, &id_atom)?)),
                Rule::parameter => at(&id_atom, ExprKind::Parameter(id_atom.as_str()[1..].into())),
                _ => return Err(invalid(source, &id_atom, "invalid_history_id")),
            };
            SourceKind::History {
                kind: at(&kind_pair, kind.as_str().to_ascii_lowercase()),
                id,
                alias: names.remove(0),
            }
        }
        Rule::changes_source => SourceKind::Changes {
            after_seq: unsigned(source, child(source, pair.clone(), Rule::unsigned)?)?,
            alias: names.remove(0),
        },
        Rule::match_source => match match_source(source, pair.clone())? {
            Some(source) => source,
            None => SourceKind::Unsupported(syntax_tree(pair.clone())),
        },
        Rule::union_source => {
            let mut bodies = pair
                .clone()
                .into_inner()
                .filter(|p| p.as_rule() == Rule::query_body);
            let left = bodies
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_union_input"))?;
            let right = bodies
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_union_input"))?;
            SourceKind::UnionAll {
                left: Box::new(query(source, left)?),
                right: Box::new(query(source, right)?),
            }
        }
        _ => SourceKind::Unsupported(syntax_tree(pair.clone())),
    };
    Ok(at(&pair, value))
}

fn stage(source: &str, pair: Pair<'_, Rule>) -> Result<Stage, QueryErrorV2> {
    let value = match pair.as_rule() {
        Rule::filter_stage => {
            StageKind::Filter(expression(source, child(source, pair.clone(), Rule::expr)?)?.0)
        }
        Rule::project_stage => StageKind::Project(select_list(
            source,
            child(source, pair.clone(), Rule::select_list)?,
        )?),
        Rule::distinct_stage => StageKind::Distinct,
        Rule::join_stage => {
            let mut names = pair
                .clone()
                .into_inner()
                .filter(|part| part.as_rule() == Rule::name)
                .map(name);
            let table = names
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_join_table"))?;
            let alias = names
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_join_alias"))?;
            let kind = match pair
                .clone()
                .into_inner()
                .find(|part| part.as_rule() == Rule::join_kind)
                .and_then(|part| part.into_inner().next())
                .map(|part| part.as_rule())
            {
                None | Some(Rule::k_inner) => JoinKind::Inner,
                Some(Rule::k_left) => JoinKind::Left,
                Some(Rule::k_semi) => JoinKind::Semi,
                Some(Rule::k_anti) => JoinKind::Anti,
                _ => return Err(invalid(source, &pair, "invalid_join_kind")),
            };
            StageKind::Join {
                table,
                alias,
                kind,
                condition: expression(source, child(source, pair.clone(), Rule::expr)?)?.0,
            }
        }
        Rule::ann_stage => {
            let mut names = pair
                .clone()
                .into_inner()
                .filter(|p| p.as_rule() == Rule::name)
                .map(name);
            let target = names
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_annotation_target"))?;
            let alias = names
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_annotation_alias"))?;
            StageKind::AnnotationLookup {
                target,
                alias,
                optional: pair
                    .clone()
                    .into_inner()
                    .any(|p| p.as_rule() == Rule::k_optional),
            }
        }
        Rule::knn_stage => {
            let mut names = pair
                .clone()
                .into_inner()
                .filter(|p| p.as_rule() == Rule::name)
                .map(name);
            let entity = names
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_knn_entity"))?;
            let collection = names
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_knn_collection"))?;
            let alias = names
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_knn_alias"))?;
            let mode = child(source, pair.clone(), Rule::mode)?
                .into_inner()
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_knn_mode"))?;
            StageKind::Knn {
                entity,
                collection,
                query: expression(source, child(source, pair.clone(), Rule::expr)?)?.0,
                k: unsigned(source, child(source, pair.clone(), Rule::unsigned)?)?,
                mode: if mode.as_rule() == Rule::k_exact {
                    VectorSearchMode::Exact
                } else {
                    VectorSearchMode::Approx
                },
                alias,
            }
        }
        Rule::rerank_stage => {
            let mut names = pair
                .clone()
                .into_inner()
                .filter(|p| p.as_rule() == Rule::name)
                .map(name);
            let entity = names
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_rerank_entity"))?;
            let collection = names
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_rerank_collection"))?;
            let alias = names
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_rerank_alias"))?;
            StageKind::Rerank {
                entity,
                collection,
                query: expression(source, child(source, pair.clone(), Rule::expr)?)?.0,
                k: unsigned(source, child(source, pair.clone(), Rule::unsigned)?)?,
                alias,
            }
        }
        Rule::lexical_stage => {
            let names = pair
                .clone()
                .into_inner()
                .filter(|part| part.as_rule() == Rule::name)
                .map(name)
                .collect::<Vec<_>>();
            if names.len() != 4 {
                return Err(invalid(source, &pair, "lexical_stage_shape"));
            }
            StageKind::LexicalMatch {
                entity: names[0].clone(),
                field: names[1].clone(),
                index: names[2].clone(),
                alias: names[3].clone(),
                query: expression(source, child(source, pair.clone(), Rule::expr)?)?.0,
                k: unsigned(source, child(source, pair.clone(), Rule::unsigned)?)?,
            }
        }
        Rule::pack_stage => {
            let tokenizer_pair = child(source, pair.clone(), Rule::string)?;
            StageKind::ContextPack {
                text: expression(source, child(source, pair.clone(), Rule::expr)?)?.0,
                evidence: expression(
                    source,
                    pair.clone()
                        .into_inner()
                        .filter(|part| part.as_rule() == Rule::expr)
                        .nth(1)
                        .ok_or_else(|| invalid(source, &pair, "context_evidence"))?,
                )?
                .0,
                tokens: unsigned(source, child(source, pair.clone(), Rule::unsigned)?)?,
                tokenizer: at(&tokenizer_pair, decode_string(source, &tokenizer_pair)?),
                alias: name(child(source, pair.clone(), Rule::name)?),
            }
        }
        Rule::expand_stage => match expand_stage(source, pair.clone())? {
            Some(stage) => stage,
            None => StageKind::Unsupported(syntax_tree(pair.clone())),
        },
        Rule::group_stage | Rule::aggregate_stage => {
            let mut lists = pair
                .clone()
                .into_inner()
                .filter(|p| p.as_rule() == Rule::select_list);
            let first = lists
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_select_list"))?;
            let (group_by, aggregates) = if pair.as_rule() == Rule::group_stage {
                let second = lists
                    .next()
                    .ok_or_else(|| invalid(source, &pair, "missing_select_list"))?;
                (select_list(source, first)?, select_list(source, second)?)
            } else {
                (Vec::new(), select_list(source, first)?)
            };
            StageKind::Aggregate {
                group_by,
                aggregates,
            }
        }
        Rule::order_stage => {
            let mut items = Vec::new();
            for item in pair
                .clone()
                .into_inner()
                .filter(|p| p.as_rule() == Rule::order_item)
            {
                let expression = expression(source, child(source, item.clone(), Rule::expr)?)?.0;
                let mut direction = None;
                let mut nulls = None;
                for modifier in item.clone().into_inner() {
                    match modifier.as_rule() {
                        Rule::sort_direction => {
                            direction = Some(at(
                                &modifier,
                                if modifier
                                    .clone()
                                    .into_inner()
                                    .any(|p| p.as_rule() == Rule::k_desc)
                                {
                                    SortDirection::Desc
                                } else {
                                    SortDirection::Asc
                                },
                            ))
                        }
                        Rule::null_order => {
                            nulls = Some(at(
                                &modifier,
                                if modifier
                                    .clone()
                                    .into_inner()
                                    .any(|p| p.as_rule() == Rule::k_first)
                                {
                                    NullOrder::First
                                } else {
                                    NullOrder::Last
                                },
                            ))
                        }
                        _ => {}
                    }
                }
                items.push(OrderItem {
                    span: span(&item),
                    expression,
                    direction,
                    nulls,
                });
            }
            StageKind::Order(items)
        }
        Rule::take_stage => StageKind::Take(unsigned(
            source,
            child(source, pair.clone(), Rule::unsigned)?,
        )?),
        Rule::skip_stage => StageKind::Skip(unsigned(
            source,
            child(source, pair.clone(), Rule::unsigned)?,
        )?),
        _ => StageKind::Unsupported(syntax_tree(pair.clone())),
    };
    Ok(at(&pair, value))
}

fn select_list(source: &str, pair: Pair<'_, Rule>) -> Result<Vec<SelectItem>, QueryErrorV2> {
    pair.into_inner()
        .map(|item| {
            let value = if item.clone().into_inner().any(|p| p.as_rule() == Rule::star) {
                SelectKind::Star
            } else {
                let expression = expression(source, child(source, item.clone(), Rule::expr)?)?.0;
                let alias = item
                    .clone()
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::name)
                    .map(name);
                SelectKind::Expression { expression, alias }
            };
            Ok(at(&item, value))
        })
        .collect()
}

fn checked(
    source: &str,
    pair: &Pair<'_, Rule>,
    value: ExprKind,
    depth: usize,
) -> Result<(Expr, usize), QueryErrorV2> {
    if depth > MAX_DEPTH {
        return Err(invalid(source, pair, "expression_depth_exceeded"));
    }
    Ok((at(pair, value), depth))
}

fn binary_operator(pair: &Pair<'_, Rule>) -> Option<BinaryOp> {
    Some(match pair.as_str().to_ascii_uppercase().as_str() {
        "OR" => BinaryOp::Or,
        "AND" => BinaryOp::And,
        "=" => BinaryOp::Equal,
        "!=" | "<>" => BinaryOp::NotEqual,
        "<" => BinaryOp::Less,
        "<=" => BinaryOp::LessEqual,
        ">" => BinaryOp::Greater,
        ">=" => BinaryOp::GreaterEqual,
        "CONTAINS" => BinaryOp::Contains,
        "STARTSWITH" => BinaryOp::StartsWith,
        "+" => BinaryOp::Add,
        "-" => BinaryOp::Subtract,
        "*" => BinaryOp::Multiply,
        "/" => BinaryOp::Divide,
        "%" => BinaryOp::Remainder,
        _ => return None,
    })
}

fn expression(source: &str, pair: Pair<'_, Rule>) -> Result<(Expr, usize), QueryErrorV2> {
    match pair.as_rule() {
        Rule::expr => expression(
            source,
            pair.clone()
                .into_inner()
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_expression"))?,
        ),
        Rule::or_expr | Rule::and_expr | Rule::sum_expr | Rule::term => {
            let mut parts = pair.clone().into_inner();
            let first = parts
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_expression"))?;
            let (mut left, mut depth) = expression(source, first)?;
            while let Some(op) = parts.next() {
                let right = parts
                    .next()
                    .ok_or_else(|| invalid(source, &pair, "missing_operand"))?;
                let (right, right_depth) = expression(source, right)?;
                depth = depth.max(right_depth) + 1;
                if depth > MAX_DEPTH {
                    return Err(invalid(source, &op, "expression_depth_exceeded"));
                }
                let node_span = Span {
                    start_byte: left.span.start_byte,
                    end_byte: right.span.end_byte,
                };
                left = Spanned {
                    span: node_span,
                    value: ExprKind::Binary {
                        op: binary_operator(&op)
                            .ok_or_else(|| invalid(source, &op, "invalid_operator"))?,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                };
            }
            Ok((left, depth))
        }
        Rule::not_expr => {
            let mut parts: Vec<_> = pair.clone().into_inner().collect();
            let comparison = parts
                .pop()
                .ok_or_else(|| invalid(source, &pair, "missing_expression"))?;
            let (mut value, mut depth) = expression(source, comparison)?;
            for op in parts.into_iter().rev() {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(invalid(source, &op, "expression_depth_exceeded"));
                }
                let node_span = Span {
                    start_byte: op.as_span().start(),
                    end_byte: value.span.end_byte,
                };
                value = Spanned {
                    span: node_span,
                    value: ExprKind::Not(Box::new(value)),
                };
            }
            Ok((value, depth))
        }
        Rule::comparison => {
            let mut parts = pair.clone().into_inner();
            let first = parts
                .next()
                .ok_or_else(|| invalid(source, &pair, "missing_expression"))?;
            let (left, left_depth) = expression(source, first)?;
            let Some(operator) = parts.next() else {
                return Ok((left, left_depth));
            };
            let (value, depth) = match operator.as_rule() {
                Rule::is_null => (
                    ExprKind::IsNull {
                        expression: Box::new(left),
                        negated: operator.into_inner().any(|p| p.as_rule() == Rule::k_not),
                    },
                    left_depth + 1,
                ),
                Rule::in_array => {
                    let negated = operator
                        .clone()
                        .into_inner()
                        .any(|p| p.as_rule() == Rule::k_not);
                    let (values, depth) =
                        expression(source, child(source, operator, Rule::array)?)?;
                    (
                        ExprKind::In {
                            expression: Box::new(left),
                            values: Box::new(values),
                            negated,
                        },
                        left_depth.max(depth) + 1,
                    )
                }
                Rule::between => {
                    let mut bounds = operator
                        .into_inner()
                        .filter(|p| p.as_rule() == Rule::sum_expr);
                    let low = bounds
                        .next()
                        .ok_or_else(|| invalid(source, &pair, "missing_operand"))?;
                    let high = bounds
                        .next()
                        .ok_or_else(|| invalid(source, &pair, "missing_operand"))?;
                    let (low, ld) = expression(source, low)?;
                    let (high, hd) = expression(source, high)?;
                    (
                        ExprKind::Between {
                            expression: Box::new(left),
                            low: Box::new(low),
                            high: Box::new(high),
                        },
                        left_depth.max(ld).max(hd) + 1,
                    )
                }
                _ => {
                    let right = parts
                        .next()
                        .ok_or_else(|| invalid(source, &pair, "missing_operand"))?;
                    let (right, depth) = expression(source, right)?;
                    (
                        ExprKind::Binary {
                            op: binary_operator(&operator)
                                .ok_or_else(|| invalid(source, &operator, "invalid_operator"))?,
                            left: Box::new(left),
                            right: Box::new(right),
                        },
                        left_depth.max(depth) + 1,
                    )
                }
            };
            checked(source, &pair, value, depth)
        }
        Rule::group => {
            let (inner, depth) = expression(source, child(source, pair.clone(), Rule::expr)?)?;
            checked(source, &pair, ExprKind::Group(Box::new(inner)), depth + 1)
        }
        Rule::function => {
            let function_name = name(child(source, pair.clone(), Rule::name)?);
            let mut arguments = Vec::new();
            let mut depth = 1;
            let mut star = None;
            if let Some(args) = pair
                .clone()
                .into_inner()
                .find(|p| p.as_rule() == Rule::function_args)
            {
                for arg in args.into_inner() {
                    if arg.as_rule() == Rule::star {
                        star = Some(span(&arg));
                    } else {
                        let (value, d) = expression(source, arg)?;
                        depth = depth.max(d + 1);
                        arguments.push(value);
                    }
                }
            }
            checked(
                source,
                &pair,
                ExprKind::Call {
                    name: function_name,
                    arguments,
                    star,
                },
                depth,
            )
        }
        Rule::array => {
            let mut values = Vec::new();
            let mut depth = 1;
            for expr in pair.clone().into_inner() {
                let (value, d) = expression(source, expr)?;
                depth = depth.max(d + 1);
                values.push(value);
            }
            checked(source, &pair, ExprKind::List(values), depth)
        }
        Rule::object => {
            let mut values = Vec::new();
            let mut depth = 1;
            for field in pair.clone().into_inner() {
                let key = field
                    .clone()
                    .into_inner()
                    .next()
                    .ok_or_else(|| invalid(source, &field, "missing_object_key"))?;
                let key = if key.as_rule() == Rule::string {
                    at(&key, decode_string(source, &key)?)
                } else {
                    name(key)
                };
                let (value, d) = expression(source, child(source, field.clone(), Rule::expr)?)?;
                depth = depth.max(d + 1);
                values.push(ObjectField {
                    span: span(&field),
                    key,
                    expression: value,
                });
            }
            checked(source, &pair, ExprKind::Object(values), depth)
        }
        Rule::field_ref => checked(
            source,
            &pair,
            ExprKind::Field(pair.clone().into_inner().map(name).collect()),
            1,
        ),
        Rule::parameter => checked(
            source,
            &pair,
            ExprKind::Parameter(pair.as_str()[1..].into()),
            1,
        ),
        Rule::string => checked(
            source,
            &pair,
            ExprKind::Utf8(decode_string(source, &pair)?),
            1,
        ),
        Rule::number => checked(source, &pair, number(source, &pair)?, 1),
        Rule::boolean => checked(
            source,
            &pair,
            ExprKind::Bool(
                pair.clone()
                    .into_inner()
                    .any(|p| p.as_rule() == Rule::k_true),
            ),
            1,
        ),
        Rule::null => checked(source, &pair, ExprKind::Null, 1),
        _ => Err(invalid(source, &pair, "invalid_expression_tree")),
    }
}
