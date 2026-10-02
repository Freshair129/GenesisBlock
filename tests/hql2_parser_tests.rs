//! Parser-only tests deliberately avoid linking the storage engine.
#[path = "../src/query/hql2/ast.rs"]
mod ast;
#[path = "../src/query/hql2/error.rs"]
mod error;
#[path = "../src/query/hql2/syntax.rs"]
mod syntax;

use ast::*;
use syntax::parse_hql2;

// Isolated subprocess probes avoid interference from the parent test runner.
mod allocation_probe {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};
    pub struct Allocator;
    pub static LIVE: AtomicUsize = AtomicUsize::new(0);
    pub static PEAK: AtomicUsize = AtomicUsize::new(0);
    unsafe impl GlobalAlloc for Allocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let pointer = System.alloc(layout);
            if !pointer.is_null() {
                let size = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
                PEAK.fetch_max(size, Ordering::Relaxed);
            }
            pointer
        }
        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
            System.dealloc(pointer, layout);
        }
        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            let pointer = System.realloc(pointer, layout, size);
            if !pointer.is_null() {
                let live = LIVE.fetch_add(size, Ordering::Relaxed) + size;
                LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
                PEAK.fetch_max(live - layout.size(), Ordering::Relaxed);
            }
            pointer
        }
    }
}
#[global_allocator]
static ALLOCATOR: allocation_probe::Allocator = allocation_probe::Allocator;

#[test]
fn resource_bound_requires_pinned_grammar_and_layout_review() {
    let grammar = include_str!("../src/query/hql2/hql2.pest").replace("\r\n", "\n");
    let hash = grammar
        .trim_end()
        .bytes()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
    // Reviewed 2026-10-02: edge property objects reuse the bounded object/expression path.
    assert_eq!(
        hash, 0x2379f9d351018cb4,
        "grammar changed: re-audit lexical allocation bound"
    );
    let lock = include_str!("../Cargo.lock").replace("\r\n", "\n");
    assert!(
        lock.contains("\nname = \"pest\"\nversion = \"2.8.6\"\n"),
        "re-audit Pest queue/diagnostic allocation behavior"
    );
    assert!(std::mem::size_of::<SyntaxNode>() <= 96);
    assert!(std::mem::size_of::<Expr>() <= 256);
    assert!(std::mem::size_of::<Query>() <= 512);
}

#[test]
fn heap_reservation_covers_isolated_allocator_probes() {
    for diagnostics in ["default", "detailed"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "parser_heap_probe_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("HQL2_RESOURCE_PROBE", diagnostics)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{diagnostics}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
}

#[test]
#[ignore = "run by heap_reservation_covers_isolated_allocator_probes in a dedicated process"]
fn parser_heap_probe_child() {
    use std::sync::atomic::Ordering;
    let mode = std::env::var("HQL2_RESOURCE_PROBE").expect("isolated probe only");
    // This child process runs exactly this test. Production never changes these
    // process-global settings; exercise both possible diagnostic configurations.
    if mode == "detailed" {
        pest::set_error_detail(true);
    }
    let mut cases = vec![
        ("small", "VALUES $v AS x |> RETURN 1 AS y".into(), true),
        (
            "wide100",
            format!("VALUES $v AS x |> RETURN [{}0] AS y", "0,".repeat(100)),
            true,
        ),
        (
            "wide500",
            format!("VALUES $v AS x |> RETURN [{}0] AS y", "0,".repeat(500)),
            true,
        ),
        (
            "reject1000",
            format!("VALUES $v AS x |> RETURN [{}0] AS y", "0,".repeat(1000)),
            false,
        ),
        (
            "reject10000",
            format!("VALUES $v AS x |> RETURN [{}0] AS y", "0,".repeat(10000)),
            false,
        ),
        (
            "reject100000",
            format!("VALUES $v AS x |> RETURN [{}0] AS y", "0,".repeat(100000)),
            false,
        ),
        (
            "nested",
            format!(
                "VALUES $v AS x |> RETURN {}1{} AS y",
                "(".repeat(127),
                ")".repeat(127)
            ),
            true,
        ),
        (
            "unsupported_copy",
            format!(
                "FROM NODES AS n |> KNN n IN c USING \"{}\" TOP 1 EXACT AS h |> RETURN n",
                "x".repeat(64000)
            ),
            true,
        ),
        (
            "long_malformed",
            format!("VALUES $v AS x |> RETURN \"{}", "x".repeat(200000)),
            false,
        ),
        (
            "large_comment",
            format!("//{}\nVALUES $v AS x |> RETURN 1", "(".repeat(200000)),
            true,
        ),
        (
            "long_unicode",
            format!("VALUES $v AS x |> RETURN \"{}\" AS y", "😀".repeat(250000)),
            true,
        ),
    ];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hql2/blueprint/examples");
    for file in std::fs::read_dir(root).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_some_and(|s| s == "hql") {
            cases.push((
                "pinned_fixture",
                std::fs::read_to_string(path).unwrap(),
                true,
            ));
        }
    }
    for (label, source, accepted) in cases {
        let reservation = syntax::required_heap_bytes(&source);
        let start = allocation_probe::LIVE.load(Ordering::Relaxed);
        allocation_probe::PEAK.store(start, Ordering::Relaxed);
        let result = parse_hql2(&source);
        let peak = allocation_probe::PEAK
            .load(Ordering::Relaxed)
            .saturating_sub(start);
        assert_eq!(result.is_ok(), accepted, "{label}: {result:?}");
        match reservation {
            Ok(heap) => {
                assert!(
                    peak as u64 <= heap,
                    "{label}: peak {peak} exceeds reservation {heap}"
                );
                assert!(heap + syntax::required_stack_bytes(&source).unwrap() <= 64 * 1024 * 1024);
                if label != "pinned_fixture" {
                    println!(
                        "{mode} {label}: bytes={} peak={peak} heap={heap}",
                        source.len()
                    );
                }
            }
            Err(_) => {
                assert!(
                    peak < 64 * 1024,
                    "{label}: preflight allocated {peak} before refusal"
                );
                println!(
                    "{mode} {label}: bytes={} preflight_peak={peak}",
                    source.len()
                );
            }
        }
    }
}

mod unrelated_parser {
    #[derive(pest_derive::Parser)]
    #[grammar_inline = "start = { SOI ~ (\"ok\")+ ~ EOI }"]
    pub struct OtherParser;
}

#[test]
fn concurrent_public_and_unrelated_pest_parsers_do_not_share_mutated_limits() {
    use pest::Parser;
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                let ordinary = "VALUES $v AS x |> RETURN 1";
                let oversized = format!("VALUES $v AS x |> RETURN [{}0]", "0,".repeat(10000));
                for _ in 0..25 {
                    assert!(parse_hql2(ordinary).is_ok());
                    assert_eq!(
                        parse_hql2(&oversized).unwrap_err().code,
                        "QUERY_BUDGET_EXCEEDED"
                    );
                    assert!(unrelated_parser::OtherParser::parse(
                        unrelated_parser::Rule::start,
                        &"ok".repeat(3000)
                    )
                    .is_ok());
                }
            });
        }
    });
}

#[test]
fn public_parser_and_resource_helpers_share_unicode_source_cap() {
    let prefix = "VALUES $v AS x |> RETURN \"";
    let suffix = "\" AS y";
    for character in ['x', '😀'] {
        let n = 262_144 - prefix.chars().count() - suffix.chars().count();
        let at_limit = format!("{prefix}{}{suffix}", character.to_string().repeat(n));
        assert!(syntax::required_heap_bytes(&at_limit).is_ok());
        assert!(parse_hql2(&at_limit).is_ok());
        let too_long = format!("{prefix}{}{suffix}", character.to_string().repeat(n + 1));
        for result in [
            parse_hql2(&too_long).map(|_| ()),
            syntax::required_heap_bytes(&too_long).map(|_| ()),
            syntax::required_stack_bytes(&too_long).map(|_| ()),
        ] {
            assert_eq!(
                result.unwrap_err().detail.unwrap()["reason"],
                "source_length"
            );
        }
    }
}

#[test]
fn broad_sources_are_refused_before_pest_by_shared_work_profile() {
    for n in [1_000, 10_000, 100_000] {
        let source = format!("VALUES $v AS x |> RETURN [{}0] AS y", "0,".repeat(n));
        for result in [
            syntax::required_heap_bytes(&source).map(|_| ()),
            syntax::required_stack_bytes(&source).map(|_| ()),
            parse_hql2(&source).map(|_| ()),
        ] {
            assert_eq!(result.unwrap_err().code, "QUERY_BUDGET_EXCEEDED");
        }
    }
    let ordinary = "VALUES $v AS x |> RETURN [1, 2, 3] AS y";
    let heap = syntax::required_heap_bytes(ordinary).unwrap();
    assert!(heap >= 4 * 1024 * 1024);
    assert!(heap + syntax::required_stack_bytes(ordinary).unwrap() <= 64 * 1024 * 1024);
    assert!(parse_hql2(ordinary).is_ok());
}

#[test]
fn stack_reservation_matches_preflight_without_parsing() {
    assert_eq!(
        syntax::required_stack_bytes("VALUES $v AS x |> RETURN 1").unwrap(),
        0
    );
    assert_eq!(
        syntax::required_stack_bytes("VALUES $v AS x |> RETURN \"((((((((((((\"").unwrap(),
        0
    );
    let nested = format!(
        "VALUES $v AS x |> RETURN {}1{}",
        "(".repeat(9),
        ")".repeat(9)
    );
    assert_eq!(
        syntax::required_stack_bytes(&nested).unwrap(),
        32 * 1024 * 1024
    );
    let unary = format!("VALUES $v AS x |> RETURN {}true", "NOT ".repeat(9));
    assert_eq!(
        syntax::required_stack_bytes(&unary).unwrap(),
        32 * 1024 * 1024
    );
    assert!(syntax::required_stack_bytes(&"(".repeat(129)).is_err());
}

#[test]
fn pinned_positive_manifest_is_accepted() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hql2/blueprint");
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/hql2/blueprint/tests/example-manifest.json"
    ))
    .unwrap();
    let examples = manifest["examples"].as_array().unwrap();
    assert_eq!(examples.len(), 35);
    for entry in examples {
        let path = entry["file"].as_str().unwrap();
        let source = std::fs::read_to_string(root.join(path)).unwrap();
        assert!(
            parse_hql2(&source).is_ok(),
            "{path}: {:?}",
            parse_hql2(&source)
        );
    }
}

#[test]
fn pinned_negative_manifest_is_rejected() {
    let examples: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/hql2/blueprint/tests/negative-syntax.json"
    ))
    .unwrap();
    let examples = examples.as_array().unwrap();
    assert_eq!(examples.len(), 6);
    for entry in examples {
        let error = parse_hql2(entry["query"].as_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "HQL_PARSE_ERROR", "{}", entry["name"]);
        assert_eq!(error.stage, "parse");
    }
}

#[test]
fn scalar_values_ast_preserves_types_precedence_and_parameter_identity() {
    let source = "VALUES $rows AS r |> FILTER NOT r.x = null OR true AND false |> PROJECT r.x + 2 * 3 AS n |> ORDER BY n DESC NULLS LAST |> TAKE $limit |> SKIP 0 |> RETURN n";
    let statement = parse_hql2(source).unwrap();
    assert_eq!(statement.kind, StatementKind::Read);
    let query = statement.query.unwrap();
    assert!(
        matches!(&query.source.value, SourceKind::Values { parameter, alias }
        if parameter.value == "rows" && alias.value == "r")
    );
    let StageKind::Project(fields) = &query.stages[1].value else {
        panic!()
    };
    let SelectKind::Expression { expression, alias } = &fields[0].value else {
        panic!()
    };
    assert_eq!(alias.as_ref().unwrap().value, "n");
    let ExprKind::Binary {
        op: BinaryOp::Add,
        right,
        ..
    } = &expression.value
    else {
        panic!()
    };
    assert!(matches!(
        right.value,
        ExprKind::Binary {
            op: BinaryOp::Multiply,
            ..
        }
    ));
    assert!(matches!(&query.stages[3].value, StageKind::Take(value)
        if matches!(&value.value, UnsignedKind::Parameter(name) if name == "limit")));
}

#[test]
fn statement_classes_are_explicit() {
    for (source, kind) in [
        ("VALUES $v AS x |> RETURN x", StatementKind::Read),
        ("EXPLAIN VALUES $v AS x |> RETURN x", StatementKind::Explain),
        (
            "EXPLAIN ANALYZE VALUES $v AS x |> RETURN x",
            StatementKind::AnalyzeRead,
        ),
        ("APPLY $batch", StatementKind::Mutation),
        ("CREATE TABLE t SCHEMA $s", StatementKind::Ddl),
        ("ANALYZE INDEX idx", StatementKind::Admin),
    ] {
        assert_eq!(parse_hql2(source).unwrap().kind, kind);
    }
    assert!(parse_hql2("EXPLAIN ANALYZE UPSERT NODE \"x\" LABELS [] PROPS {}").is_err());
}

#[test]
fn spans_are_half_open_utf8_and_errors_use_scalar_columns() {
    let source = "// ไทย\r\nVALUES $v AS `ค่า` |> RETURN \"😀\" AS `ผล`";
    let parsed = parse_hql2(source).unwrap();
    let mut pending = vec![&parsed.syntax];
    while let Some(node) = pending.pop() {
        assert!(source
            .get(node.span.start_byte..node.span.end_byte)
            .is_some());
        for child in &node.children {
            assert!(child.span.start_byte >= node.span.start_byte);
            assert!(child.span.end_byte <= node.span.end_byte);
            pending.push(child);
        }
    }
    let query = parsed.query.unwrap();
    let SelectKind::Expression { expression, .. } = &query.returning[0].value else {
        panic!()
    };
    assert_eq!(
        &source[expression.span.start_byte..expression.span.end_byte],
        "\"😀\""
    );
    let invalid = "// ไทย\r\nVALUES $v AS `ค่า` |> RETURN \"😀\"; @";
    let error = parse_hql2(invalid).unwrap_err();
    let position = error.span.unwrap();
    assert_eq!(position.line, 2);
    assert_eq!(
        position.column,
        invalid
            .lines()
            .nth(1)
            .unwrap()
            .find('@')
            .map(|byte| invalid.lines().nth(1).unwrap()[..byte].chars().count() + 1)
            .unwrap()
    );
}

#[test]
fn numeric_boundaries_and_malformed_literals() {
    for literal in [
        "9223372036854775807",
        "-9223372036854775808",
        "+1",
        "1e3",
        ".5",
        "1.",
    ] {
        assert!(
            parse_hql2(&format!("VALUES $v AS x |> RETURN {literal}")).is_ok(),
            "{literal}"
        );
    }
    for literal in [
        "9223372036854775808",
        "-9223372036854775809",
        "1e999",
        "1e",
        "1.2.3",
        "\"\\uD800\"",
        "\"\\x20\"",
    ] {
        assert!(
            parse_hql2(&format!("VALUES $v AS x |> RETURN {literal}")).is_err(),
            "{literal}"
        );
    }
    assert!(parse_hql2("VALUES $v AS x |> TAKE 18446744073709551615 |> RETURN x").is_ok());
    assert!(parse_hql2("VALUES $v AS x |> TAKE 18446744073709551616 |> RETURN x").is_err());
    assert!(parse_hql2("VALUES $v AS x |> TAKE 01 |> RETURN x").is_err());
}

#[test]
fn depth_preflight_handles_delimiters_not_chains_and_comments() {
    let nested = format!(
        "VALUES $v AS x |> RETURN {}1{}",
        "(".repeat(129),
        ")".repeat(129)
    );
    assert!(parse_hql2(&nested).is_err());
    let nots = format!("VALUES $v AS x |> RETURN {}true", "NOT ".repeat(129));
    assert!(parse_hql2(&nots).is_err());
    let deep_binary = format!("VALUES $v AS x |> RETURN {}1", "1 + ".repeat(129));
    assert!(parse_hql2(&deep_binary).is_err());
    let deep_boolean = format!("VALUES $v AS x |> RETURN {}true", "true AND ".repeat(129));
    assert!(parse_hql2(&deep_boolean).is_err());
    assert!(parse_hql2("VALUES $v AS x |> RETURN \"((((//not syntax\" // [ { NOT\n;").is_ok());
}

#[test]
fn keywords_have_boundaries_and_trailing_statements_reject() {
    for source in [
        "VALUES $v AS x |> RETURN truefalse",
        "VALUES $v AS x |> RETURN nothing",
    ] {
        // Names containing keyword prefixes are identifiers, never split literals.
        let q = parse_hql2(source).unwrap().query.unwrap();
        let SelectKind::Expression { expression, .. } = &q.returning[0].value else {
            panic!()
        };
        assert!(matches!(expression.value, ExprKind::Field(_)));
    }
    for source in [
        "VALUESx $v AS x |> RETURN x",
        "VALUES $v AS x |> RETURN x;;",
        "VALUES $v AS x |> RETURN x; SHOW INDEXES",
        "",
        "// only comment",
    ] {
        assert!(parse_hql2(source).is_err(), "{source}");
    }
}

#[test]
fn boolean_literals_do_not_absorb_whitespace_or_qualified_names() {
    let query = parse_hql2("VALUES $v AS x |> RETURN true AS t, false AS f, null AS n, true.value, false . value, `null`, truefalse")
        .unwrap().query.unwrap();
    let exprs: Vec<_> = query
        .returning
        .iter()
        .map(|s| match &s.value {
            SelectKind::Expression { expression, .. } => expression,
            _ => panic!(),
        })
        .collect();
    assert!(matches!(exprs[0].value, ExprKind::Bool(true)));
    assert!(matches!(exprs[1].value, ExprKind::Bool(false)));
    assert!(matches!(exprs[2].value, ExprKind::Null));
    for expr in &exprs[3..] {
        assert!(matches!(expr.value, ExprKind::Field(_)));
    }
}

#[test]
fn legal_depth_128_is_accepted_even_with_small_caller_stack() {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let nested = format!(
                "VALUES $v AS x |> RETURN {}1{}",
                "(".repeat(127),
                ")".repeat(127)
            );
            let parsed = parse_hql2(&nested);
            assert!(parsed.is_ok());
            drop(parsed);
            let too_deep = format!(
                "VALUES $v AS x |> RETURN {}1{}",
                "(".repeat(128),
                ")".repeat(128)
            );
            assert!(parse_hql2(&too_deep).is_err());
            let unary = format!("VALUES $v AS x |> RETURN {}true", "NOT ".repeat(127));
            assert!(parse_hql2(&unary).is_ok());
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn remaining_grammar_families_are_parsed_without_claiming_execution() {
    for source in [
        "FROM EDGES DEPENDS AS e |> RETURN e",
        "FROM EDGES AS e |> RETURN e",
        "HISTORY ARTIFACT $id AS h |> RETURN h",
        "CHANGES SINCE $seq AS c |> RETURN c",
        "MATCH (a)<-[e:R|S*0..2]-(b)--(c) AS p WALK |> RETURN p",
        "MATCH ()-[]->(:Label) |> OPTIONAL EXPAND (a)--(b) SIMPLE |> RETURN *",
        "VALUES $v AS x |> FILTER x IS NOT NULL |> FILTER x NOT IN [1,2] |> FILTER x BETWEEN 1 AND 3 |> RETURN lower(x), f(), {key: [1, true], \"ไทย\": $p}",
        "CREATE EDGE TYPE R SCHEMA $s",
        "CREATE ANNOTATION TYPE Review SCHEMA $s",
        "CREATE UNIQUE INDEX i ON EDGE R (a,b) USING BTREE INCLUDE (c,d) WHERE a >= 1 WITH $c",
        "CREATE COLLECTION c DIM 3 METRIC L2_SQUARED MODEL \"m\" QUANT SQ8",
        "CREATE COLLECTION c DIM 3 METRIC NEG_DOT MODEL \"m\" QUANT BQ",
        "DROP INDEX i",
        "UPSERT EDGE $id FROM $a TO $b TYPE \"R\" PROPS {x: 1} VALID FROM $start TO null",
        "UPSERT ROW t KEY {id: 1} VALUES {v: 2} VALID FROM $start TO $end",
        "UPSERT VECTOR ON NODE $id IN c USING [1,2,3]",
        "ANNOTATE ARTIFACT $id WITH $annotation",
        "RETRACT ANNOTATION $id",
        "SHOW CAPABILITIES", "SHOW COLLECTIONS", "SHOW SCHEMAS",
        "REBUILD INDEX i", "CHECK DATABASE", "CHECKPOINT", "COMPACT",
    ] { assert!(parse_hql2(source).is_ok(), "{source}: {:?}", parse_hql2(source)); }
}

#[test]
fn scalar_operators_have_structural_not_string_split_representation() {
    let q = parse_hql2("VALUES $v AS x |> RETURN x BETWEEN 1 AND 2, x NOT IN [1], x IS NOT NULL, 8 / 2 % 3 - 1, NOT x = 1 OR x <> 2 AND x CONTAINS \"a\"")
        .unwrap().query.unwrap();
    let kinds: Vec<_> = q
        .returning
        .iter()
        .map(|s| match &s.value {
            SelectKind::Expression { expression, .. } => &expression.value,
            _ => panic!(),
        })
        .collect();
    assert!(matches!(kinds[0], ExprKind::Between { .. }));
    assert!(matches!(kinds[1], ExprKind::In { negated: true, .. }));
    assert!(matches!(kinds[2], ExprKind::IsNull { negated: true, .. }));
    assert!(matches!(
        kinds[3],
        ExprKind::Binary {
            op: BinaryOp::Subtract,
            ..
        }
    ));
    assert!(matches!(
        kinds[4],
        ExprKind::Binary {
            op: BinaryOp::Or,
            ..
        }
    ));
}
