//! Closed wire decoding only; no reference evaluator and no storage effects.
#![allow(dead_code)]
#[path = "../src/query/hql2/error.rs"]
mod error;
#[path = "../src/uee_v2.rs"]
mod uee_v2;
#[path = "../src/query/hql2/wire.rs"]
mod wire;

use serde_json::{json, Value};
use wire::{decode_ir_v2, Config, Expr};

fn node(id: &str, op: &str, inputs: &[&str], config: Value) -> Value {
    json!({"id":id,"op":op,"inputs":inputs,"config":config})
}

fn ir(nodes: Vec<Value>, root: &str) -> uee_v2::QueryIrV2 {
    // Intentionally bypass G0::from_value: the production decoder must validate
    // even a directly constructed/deserialized QueryIrV2.
    serde_json::from_value(json!({"contract_version":"query-ir.v2","nodes":nodes,"root":root}))
        .unwrap()
}

fn source() -> Value {
    node("s", "NodeScan", &[], json!({"as":"n"}))
}
fn field() -> Value {
    json!({"field":{"alias":"n","path":[]}})
}
fn literal() -> Value {
    json!({"literal":true,"type":"Bool"})
}
fn compact() -> Value {
    json!({"start_alias":"n","end_alias":"m","relations":["knows"],"direction":"out","min_hops":0,"max_hops":32,"mode":"trail"})
}
fn sequence() -> Value {
    json!({"form":"sequence","start":{"alias":"n","labels":[],"properties":{}},"steps":[{"edge":{"relations":[],"direction":"both","min_hops":0,"max_hops":1,"properties":{}},"node":{"alias":"m","id":{"param":"id"},"labels":["Person"],"properties":{"display name":{"literal":"x","type":"Utf8"}}}}],"mode":"simple","path_alias":"p"})
}

fn fixtures() -> Vec<(&'static str, usize, Value)> {
    vec![
        ("NodeScan", 0, json!({"as":"n","label":"Person"})),
        ("EdgeScan", 0, json!({"as":"e","relation":"knows"})),
        ("RowScan", 0, json!({"table":"people","as":"r"})),
        ("AnnotationScan", 0, json!({"as":"a"})),
        ("Values", 0, json!({"param":"rows","as":"r"})),
        (
            "HistoryScan",
            0,
            json!({"kind":"node","id":{"param":"id"},"as":"h"}),
        ),
        (
            "ChangeScan",
            0,
            json!({"after_seq":"18446744073709551615","as":"c"}),
        ),
        (
            "Match",
            0,
            json!({"pattern":compact(),"anchors":{"n":{"param":"id"}},"shortest":true}),
        ),
        ("Filter", 1, json!({"predicate":literal()})),
        (
            "Project",
            1,
            json!({"fields":[{"expression":field(),"as":"x"}]}),
        ),
        ("Distinct", 1, json!({})),
        (
            "Knn",
            1,
            json!({"entity":"n","collection":"v","query":{"param":"q"},"k":10000,"mode":"exact","as":"score"}),
        ),
        (
            "Rerank",
            1,
            json!({"entity":"n","collection":"v","query":{"param":"q"},"k":0,"fidelity":"original","as":"score"}),
        ),
        ("Expand", 1, json!({"pattern":sequence(),"optional":true})),
        (
            "AnnotationLookup",
            1,
            json!({"target":"n","as":"a","optional":false}),
        ),
        (
            "LexicalMatch",
            1,
            json!({"entity":"n","field":"body","index":"fts","query":{"param":"q"},"k":1,"as":"score"}),
        ),
        ("Join", 2, json!({"kind":"anti","condition":literal()})),
        (
            "Aggregate",
            1,
            json!({"group_by":[],"aggregates":[{"expression":{"call":"count_all","args":[]},"as":"c"}]}),
        ),
        (
            "Sort",
            1,
            json!({"keys":[{"expression":field(),"direction":"desc","nulls":"last"}]}),
        ),
        ("Take", 1, json!({"count":18446744073709551615u64})),
        ("Offset", 1, json!({"count":0})),
        ("UnionAll", 2, json!({})),
        (
            "ContextPack",
            1,
            json!({"text":field(),"evidence":field(),"tokens":1000000,"tokenizer":"unicode","as":"ctx"}),
        ),
    ]
}

fn request(op: &str, arity: usize, config: Value) -> uee_v2::QueryIrV2 {
    let mut nodes = Vec::new();
    if arity > 0 {
        nodes.push(source());
    }
    if arity > 1 {
        nodes.push(node("t", "NodeScan", &[], json!({"as":"m"})));
    }
    nodes.push(node("root", op, &["s", "t"][..arity], config));
    ir(nodes, "root")
}

#[test]
fn all_23_configs_decode_to_typed_owned_nodes() {
    let cases = fixtures();
    assert_eq!(cases.len(), 23);
    for (op, arity, config) in cases {
        let decoded =
            decode_ir_v2(request(op, arity, config)).unwrap_or_else(|e| panic!("{op}: {e:?}"));
        assert_eq!(decoded.root, "root");
        assert!(
            !decoded.from_hql,
            "IR may not claim the trusted lowering origin"
        );
        assert_eq!(decoded.nodes.last().unwrap().id, "root");
        assert_eq!(decoded.nodes.last().unwrap().inputs.len(), arity);
    }
    let decoded = decode_ir_v2(request("Filter", 1, json!({"predicate":literal()}))).unwrap();
    assert!(
        matches!(&decoded.nodes.last().unwrap().config,Config::Filter{predicate:Expr::Literal{ty,..}} if ty=="Bool")
    );
}

#[test]
fn every_config_rejects_unknown_and_missing_required_fields_and_wrong_arity() {
    for (op, arity, config) in fixtures() {
        let mut extra = config.clone();
        extra["hidden_override"] = json!(true);
        assert!(
            decode_ir_v2(request(op, arity, extra)).is_err(),
            "unknown {op}"
        );
        for key in config.as_object().unwrap().keys() {
            if matches!(
                (op, key.as_str()),
                ("NodeScan", "label") | ("EdgeScan", "relation")
            ) {
                continue;
            }
            let mut missing = config.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(
                decode_ir_v2(request(op, arity, missing)).is_err(),
                "missing {op}.{key}"
            );
        }
        let mut wrong = request(op, arity, config);
        wrong.nodes.last_mut().unwrap().inputs.push("root".into());
        assert!(decode_ir_v2(wrong).is_err(), "arity {op}");
    }
}

#[test]
fn expression_variants_are_closed_recursively() {
    let expressions = vec![
        literal(),
        json!({"param":"p"}),
        field(),
        json!({"binary":"eq","left":field(),"right":literal()}),
        json!({"unary":"not","arg":literal()}),
        json!({"call":"lower","args":[{"literal":"ABC","type":"Utf8"}]}),
        json!({"in":field(),"values":[literal()],"negated":false}),
    ];
    for expression in expressions {
        assert!(decode_ir_v2(request(
            "Filter",
            1,
            json!({"predicate":expression.clone()})
        ))
        .is_ok());
        let mut extra = expression.clone();
        extra["untrusted"] = json!(1);
        assert!(decode_ir_v2(request("Filter", 1, json!({"predicate":extra}))).is_err());
        for key in expression.as_object().unwrap().keys() {
            let mut missing = expression.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(
                decode_ir_v2(request("Filter", 1, json!({"predicate":missing}))).is_err(),
                "missing expression {key}"
            );
        }
    }
    for bad in [
        json!({"field":{"alias":"n","path":[],"extra":1}}),
        json!({"binary":"xor","left":literal(),"right":literal()}),
        json!({"unary":"abs","arg":literal()}),
        json!({"call":"bad.name","args":[]}),
        json!({"param":"ก"}),
        json!({"literal":null,"type":""}),
    ] {
        assert!(decode_ir_v2(request("Filter", 1, json!({"predicate":bad}))).is_err());
    }
    assert!(decode_ir_v2(request(
        "Filter",
        1,
        json!({"predicate":{"literal":null,"type":"Nullable<Bool>"}})
    ))
    .is_ok());
}

#[test]
fn scalar_bounds_and_symbols_reject_without_coercion() {
    for (op, arity, config) in fixtures() {
        for (key, value) in config.as_object().unwrap() {
            let invalid = match key.as_str() {
                "as" | "param" | "entity" | "collection" | "table" | "target" | "index"
                | "field" => Some(json!("bad.name")),
                "k" => Some(json!(10001)),
                "tokens" => Some(json!(1000001)),
                "count" => Some(json!(-1)),
                "after_seq" => Some(json!("18446744073709551616")),
                "label" | "relation" | "tokenizer" => Some(json!("")),
                _ => None,
            };
            if let Some(bad) = invalid {
                let mut copy = config.clone();
                copy[key] = bad;
                assert!(
                    decode_ir_v2(request(op, arity, copy)).is_err(),
                    "{op}.{key}"
                );
            }
            if value.is_boolean() {
                let mut copy = config.clone();
                copy[key] = json!("true");
                assert!(decode_ir_v2(request(op, arity, copy)).is_err());
            }
        }
    }
    for text in ["", "00", "+1", "-1", " 1", "1.0", "١"] {
        assert!(
            decode_ir_v2(request("ChangeScan", 0, json!({"as":"c","after_seq":text}))).is_err()
        );
    }
    for count in [json!(1.0), json!("1"), Value::Null] {
        assert!(decode_ir_v2(request("Take", 1, json!({"count":count}))).is_err());
    }
    let unicode = "ก".repeat(256);
    assert!(decode_ir_v2(request("NodeScan", 0, json!({"as":"n","label":unicode}))).is_ok());
    assert!(decode_ir_v2(request(
        "NodeScan",
        0,
        json!({"as":"n","label":"ก".repeat(257)})
    ))
    .is_err());
    assert!(decode_ir_v2(request("NodeScan", 0, json!({"as":"n","label":null}))).is_err());
}

#[test]
fn patterns_validate_all_nested_shapes_and_bounds() {
    for pattern in [compact(), sequence()] {
        assert!(decode_ir_v2(request(
            "Expand",
            1,
            json!({"pattern":pattern,"optional":false})
        ))
        .is_ok());
    }
    let mut bads = Vec::new();
    let mut p = compact();
    p["max_hops"] = json!(33);
    bads.push(p);
    let mut p = compact();
    p["min_hops"] = json!(2);
    p["max_hops"] = json!(1);
    bads.push(p);
    let mut p = compact();
    p["mode"] = json!("weighted");
    bads.push(p);
    let mut p = compact();
    p["relations"] = json!([""]);
    bads.push(p);
    let mut p = compact();
    p["edge_alias"] = Value::Null;
    bads.push(p);
    let mut p = sequence();
    p["steps"] = json!([]);
    bads.push(p);
    let mut p = sequence();
    p["steps"] = Value::Array(vec![p["steps"][0].clone(); 33]);
    bads.push(p);
    let mut p = sequence();
    p["start"]["extra"] = json!(1);
    bads.push(p);
    let mut p = sequence();
    p["start"]["id"] = Value::Null;
    bads.push(p);
    let mut p = sequence();
    p["steps"][0]["edge"]["properties"]["x"] = json!({"param":"bad.name"});
    bads.push(p);
    let mut p = sequence();
    p["steps"][0]["node"]["labels"] = json!([""]);
    bads.push(p);
    for pattern in bads {
        assert!(decode_ir_v2(request(
            "Expand",
            1,
            json!({"pattern":pattern,"optional":true})
        ))
        .is_err());
    }
}

#[test]
fn dag_rejects_duplicate_cycle_missing_orphan_invalid_id_and_version() {
    let mut cases = vec![
        ir(vec![source(), source()], "s"),
        ir(vec![source()], "missing"),
        ir(
            vec![source(), node("orphan", "NodeScan", &[], json!({"as":"m"}))],
            "s",
        ),
    ];
    cases.push(ir(
        vec![
            node("a", "Take", &["b"], json!({"count":0})),
            node("b", "Take", &["a"], json!({"count":0})),
        ],
        "a",
    ));
    cases.push(ir(
        vec![node("a", "Take", &["missing"], json!({"count":0}))],
        "a",
    ));
    cases.push(ir(
        vec![node("1bad", "NodeScan", &[], json!({"as":"n"}))],
        "1bad",
    ));
    cases.push(ir(vec![], "root"));
    for case in cases {
        assert!(decode_ir_v2(case).is_err());
    }
    let mut wrong = ir(vec![source()], "s");
    wrong.contract_version = "query-ir.v1".into();
    assert!(decode_ir_v2(wrong).is_err());
    for (key, ty) in [("bad.name", "Bool"), ("p", "")] {
        let mut wrong = ir(vec![source()], "s");
        wrong.parameter_types.insert(key.into(), ty.into());
        assert!(decode_ir_v2(wrong).is_err());
    }
}

#[test]
fn topological_order_is_deterministic_and_preserves_ordered_duplicate_inputs() {
    let a = node("a", "NodeScan", &[], json!({"as":"a"}));
    let z = node("z", "NodeScan", &[], json!({"as":"z"}));
    let root = node(
        "root",
        "Join",
        &["z", "a"],
        json!({"kind":"left","condition":literal()}),
    );
    let first = decode_ir_v2(ir(vec![root.clone(), z.clone(), a.clone()], "root")).unwrap();
    let second = decode_ir_v2(ir(vec![a, z, root], "root")).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first
            .nodes
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "z", "root"]
    );
    assert_eq!(first.nodes[2].inputs, vec!["z", "a"]);
    assert!(decode_ir_v2(ir(
        vec![source(), node("root", "UnionAll", &["s", "s"], json!({}))],
        "root"
    ))
    .is_ok());
}

#[test]
fn dag_depth_is_longest_path_not_visit_order_and_limit_is_inclusive() {
    for depth in [128, 129] {
        let mut nodes = vec![source()];
        let mut previous = "s".to_string();
        for i in 1..depth {
            let id = format!("n{i}");
            nodes.push(node(&id, "Take", &[&previous], json!({"count":0})));
            previous = id;
        }
        assert_eq!(decode_ir_v2(ir(nodes, &previous)).is_ok(), depth == 128);
    }
    let mut nodes = vec![source()];
    let mut previous = "s".to_string();
    for i in 1..128 {
        let id = format!("n{i}");
        nodes.push(node(&id, "UnionAll", &["s", &previous], json!({})));
        previous = id;
    }
    nodes.push(node("root", "UnionAll", &["s", &previous], json!({})));
    assert!(decode_ir_v2(ir(nodes, "root")).is_err());
}

#[test]
fn node_limit_rejects_ten_thousand_and_one_before_traversal() {
    let nodes = (0..10001)
        .map(|i| node(&format!("n{i}"), "NodeScan", &[], json!({"as":"n"})))
        .collect();
    assert!(decode_ir_v2(ir(nodes, "n0")).is_err());
}

#[test]
fn expression_depth_is_checked_before_recursive_deserialization() {
    for depth in [128, 129] {
        let mut expression = literal();
        for _ in 1..depth {
            expression = json!({"unary":"not","arg":expression});
        }
        assert_eq!(
            decode_ir_v2(request("Filter", 1, json!({"predicate":expression}))).is_ok(),
            depth == 128
        );
    }
}

#[test]
fn expression_order_and_all_discriminators_survive_decoding() {
    for op in [
        "eq",
        "ne",
        "lt",
        "le",
        "gt",
        "ge",
        "and",
        "or",
        "add",
        "sub",
        "mul",
        "div",
        "rem",
        "contains",
        "startswith",
    ] {
        let decoded = decode_ir_v2(request(
            "Filter",
            1,
            json!({"predicate":{"binary":op,"left":{"param":"left"},"right":{"param":"right"}}}),
        ))
        .unwrap();
        let Config::Filter {
            predicate: Expr::Binary { left, right, .. },
        } = &decoded.nodes.last().unwrap().config
        else {
            panic!("binary shape")
        };
        assert!(matches!(&**left,Expr::Param{param} if param=="left"));
        assert!(matches!(&**right,Expr::Param{param} if param=="right"));
    }
    for op in ["not", "neg", "is_null", "is_not_null"] {
        assert!(decode_ir_v2(request(
            "Filter",
            1,
            json!({"predicate":{"unary":op,"arg":literal()}})
        ))
        .is_ok());
    }
    let decoded=decode_ir_v2(request("Project",1,json!({"fields":[{"as":"x","expression":{"call":"f","args":[{"param":"a"},{"in":{"param":"b"},"values":[{"param":"c"},{"param":"d"}],"negated":true},{"param":"e"}]}}]}))).unwrap();
    let Config::Project { fields } = &decoded.nodes.last().unwrap().config else {
        panic!("project shape")
    };
    let Expr::Call { args, .. } = &fields[0].expression else {
        panic!("call shape")
    };
    assert_eq!(args.len(), 3);
    assert!(matches!(&args[0],Expr::Param{param} if param=="a"));
    assert!(matches!(&args[2],Expr::Param{param} if param=="e"));
    let Expr::In {
        expression,
        values,
        negated,
    } = &args[1]
    else {
        panic!("in shape")
    };
    assert!(*negated);
    assert!(matches!(&**expression,Expr::Param{param} if param=="b"));
    assert!(matches!(&values[0],Expr::Param{param} if param=="c"));
    assert!(matches!(&values[1],Expr::Param{param} if param=="d"));
}

#[test]
fn nested_pattern_and_call_depth_limits_do_not_count_json_wrappers() {
    for depth in [128, 129] {
        let mut expression = literal();
        for _ in 1..depth {
            expression = json!({"call":"f","args":[expression]});
        }
        let mut pattern = sequence();
        pattern["steps"][0]["node"]["id"] = expression;
        assert_eq!(
            decode_ir_v2(request(
                "Expand",
                1,
                json!({"pattern":pattern,"optional":true})
            ))
            .is_ok(),
            depth == 128
        );
    }
}

#[test]
fn all_nested_closed_objects_reject_extras_and_required_omissions() {
    // Map containers (anchors/properties) are intentionally open; the values
    // inside them, named expressions, field refs and pattern components are not.
    let fixtures = vec![
        (
            "Project",
            1,
            json!({"fields":[{"expression":field(),"as":"x"}]}),
            vec![
                "/fields/0",
                "/fields/0/expression",
                "/fields/0/expression/field",
            ],
        ),
        (
            "Sort",
            1,
            json!({"keys":[{"expression":field(),"direction":"asc","nulls":"first"}]}),
            vec!["/keys/0"],
        ),
        (
            "Aggregate",
            1,
            json!({"group_by":[{"expression":field(),"as":"g"}],"aggregates":[{"expression":{"call":"count","args":[field()]},"as":"c"}]}),
            vec![
                "/group_by/0",
                "/aggregates/0",
                "/aggregates/0/expression",
                "/aggregates/0/expression/args/0",
            ],
        ),
        (
            "Expand",
            1,
            json!({"pattern":sequence(),"optional":false}),
            vec![
                "/pattern",
                "/pattern/start",
                "/pattern/steps/0",
                "/pattern/steps/0/edge",
                "/pattern/steps/0/node",
                "/pattern/steps/0/node/id",
                "/pattern/steps/0/node/properties/display name",
            ],
        ),
    ];
    for (op, arity, config, paths) in fixtures {
        for path in paths {
            let mut extra = config.clone();
            extra.pointer_mut(path).unwrap()["unknown"] = json!(true);
            assert!(
                decode_ir_v2(request(op, arity, extra)).is_err(),
                "extra {op}{path}"
            );
            for key in config.pointer(path).unwrap().as_object().unwrap().keys() {
                if matches!(key.as_str(), "id" | "path_alias") {
                    continue;
                }
                let mut missing = config.clone();
                missing
                    .pointer_mut(path)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(key);
                assert!(
                    decode_ir_v2(request(op, arity, missing)).is_err(),
                    "missing {op}{path}/{key}"
                );
            }
        }
    }
    for (op, arity, config) in [
        ("Project", 1, json!({"fields":[]})),
        ("Sort", 1, json!({"keys":[]})),
        ("Aggregate", 1, json!({"group_by":[],"aggregates":[]})),
        (
            "Match",
            0,
            json!({"pattern":compact(),"anchors":{"bad.name":literal()},"shortest":false}),
        ),
    ] {
        assert!(decode_ir_v2(request(op, arity, config)).is_err());
    }
}

#[test]
fn ten_thousand_reachable_nodes_are_allowed() {
    let mut nodes = Vec::new();
    // Balanced union DAG: 5000 leaves + 4999 unions + one Take = 10000;
    // unlike a chain, this distinguishes the node limit from the depth limit.
    let mut layer = Vec::new();
    for i in 0..5000 {
        let id = format!("s{i}");
        nodes.push(node(&id, "NodeScan", &[], json!({"as":"n"})));
        layer.push(id);
    }
    let mut serial = 0;
    while layer.len() > 1 {
        let mut next = Vec::new();
        for chunk in layer.chunks(2) {
            if chunk.len() == 1 {
                next.push(chunk[0].clone());
                continue;
            }
            let id = format!("u{serial}");
            serial += 1;
            nodes.push(node(&id, "UnionAll", &[&chunk[0], &chunk[1]], json!({})));
            next.push(id);
        }
        layer = next;
    }
    nodes.push(node("root", "Take", &[&layer[0]], json!({"count":0})));
    assert_eq!(nodes.len(), 10000);
    assert_eq!(decode_ir_v2(ir(nodes, "root")).unwrap().nodes.len(), 10000);
}

#[test]
fn very_deep_constructed_json_is_rejected_and_disposed_without_recursion() {
    // Build directly: even serde_json's external parser recursion limit is not
    // trusted for an API accepting already-owned QueryIrV2 values.
    let mut value = Value::Null;
    for _ in 0..20000 {
        value = Value::Array(vec![value]);
    }
    let mut raw = ir(vec![source()], "s");
    raw.nodes[0].config.insert("extra".into(), value);
    let error = decode_ir_v2(raw).unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.stage, "bind");
    assert!(error.span.is_none());
    assert_eq!(
        error.detail.as_deref(),
        Some(&json!({"reason":"json_depth"}))
    );
}

#[test]
fn schema_examples_and_origin_boundary_remain_distinct_from_binding() {
    let pinned: uee_v2::QueryIrV2 = serde_json::from_str(include_str!(
        "fixtures/hql2/blueprint/examples/query-ir.json"
    ))
    .unwrap();
    assert!(decode_ir_v2(pinned).is_ok());
    let mut value = json!({"contract_version":"query-ir.v2","nodes":[source()],"root":"s","parameter_types":{"p":"Bool"}});
    let decoded = decode_ir_v2(serde_json::from_value(value.clone()).unwrap()).unwrap();
    assert_eq!(decoded.parameter_types["p"], "Bool");
    assert!(!decoded.from_hql);
    value["from_hql"] = json!(true);
    assert!(serde_json::from_value::<uee_v2::QueryIrV2>(value).is_err());
    // Decoding is not semantic binding: this valid closed shape is retained for
    // the binder to reject without invoking any unregistered function.
    assert!(decode_ir_v2(request(
        "Filter",
        1,
        json!({"predicate":{"call":"unregistered","args":[]}})
    ))
    .is_ok());
}

#[test]
fn all_closed_enumerations_match_the_wire_spelling() {
    for kind in ["node", "edge", "row", "annotation", "artifact"] {
        assert!(decode_ir_v2(request(
            "HistoryScan",
            0,
            json!({"kind":kind,"id":literal(),"as":"h"})
        ))
        .is_ok());
    }
    for direction in ["out", "in", "both"] {
        for mode in ["trail", "simple", "walk"] {
            let mut p = compact();
            p["direction"] = json!(direction);
            p["mode"] = json!(mode);
            assert!(
                decode_ir_v2(request("Expand", 1, json!({"pattern":p,"optional":false}))).is_ok()
            );
        }
    }
    for mode in ["exact", "approx"] {
        assert!(decode_ir_v2(request(
            "Knn",
            1,
            json!({"entity":"n","collection":"v","query":{"param":"q"},"k":0,"mode":mode,"as":"s"})
        ))
        .is_ok());
    }
    for kind in ["inner", "left", "semi", "anti"] {
        assert!(decode_ir_v2(request(
            "Join",
            2,
            json!({"kind":kind,"condition":literal()})
        ))
        .is_ok());
    }
    for direction in ["asc", "desc"] {
        for nulls in ["first", "last"] {
            assert!(decode_ir_v2(request(
                "Sort",
                1,
                json!({"keys":[{"expression":field(),"direction":direction,"nulls":nulls}]})
            ))
            .is_ok());
        }
    }
    for (op, arity, config) in fixtures() {
        for key in ["mode", "fidelity", "kind"] {
            if config.get(key).is_some() {
                for wrong in [json!("UNKNOWN"), Value::Null, json!(0)] {
                    let mut c = config.clone();
                    c[key] = wrong;
                    assert!(
                        decode_ir_v2(request(op, arity, c)).is_err(),
                        "enum {op}.{key}"
                    );
                }
            }
        }
    }
}

#[test]
fn bounded_literal_json_does_not_impersonate_expression_nodes() {
    let mut payload = json!({"unary":"untrusted_json_property","arg":false});
    for _ in 0..400 {
        payload = Value::Array(vec![payload]);
    }
    let mut raw = request("Filter", 1, json!({"predicate":literal()}));
    raw.nodes
        .last_mut()
        .unwrap()
        .config
        .insert("predicate".into(), json!({"literal":payload,"type":"Json"}));
    let decoded = decode_ir_v2(raw).unwrap();
    assert!(
        matches!(&decoded.nodes.last().unwrap().config,Config::Filter{predicate:Expr::Literal{ty,..}} if ty=="Json")
    );
}
