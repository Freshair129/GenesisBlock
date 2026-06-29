use pest::Parser;
use pest_derive::Parser;
use std::convert::TryFrom;

#[derive(Parser)]
#[grammar = "query/hql.pest"]
pub struct HqlParser;

#[derive(Debug, Clone)]
pub enum HqlRel {
    Physical(String),
    Inferred(String),
}

/// A queryable field in WHERE / ORDER BY / RETURN clauses.
/// `Prop(key)` reaches into a node's `props` object; the rest are top-level.
#[derive(Debug, Clone, PartialEq)]
pub enum HqlField {
    Id,
    Label,
    Score,
    Depth,
    Prop(String),
}

impl HqlField {
    /// The output key used when this field is projected via RETURN
    /// (`prop.text` -> `"text"`, `score` -> `"score"`, ...).
    pub fn output_key(&self) -> String {
        match self {
            HqlField::Id => "id".to_string(),
            HqlField::Label => "label".to_string(),
            HqlField::Score => "score".to_string(),
            HqlField::Depth => "depth".to_string(),
            HqlField::Prop(k) => k.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HqlOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
    StartsWith,
}

#[derive(Debug, Clone)]
pub enum HqlValue {
    Str(String),
    Num(f64),
}

#[derive(Debug, Clone)]
pub struct HqlPredicate {
    pub field: HqlField,
    pub op: HqlOp,
    pub value: HqlValue,
}

/// RETURN projection: `All` is `RETURN *` (keep the full NeighborOutput shape);
/// `Fields` reshapes each hit into a flat object of the named fields.
#[derive(Debug, Clone)]
pub enum HqlReturn {
    All,
    Fields(Vec<HqlField>),
}

/// Optional post-process clauses applied (in this order) to a command's result
/// list: WHERE filter -> ORDER BY -> LIMIT -> RETURN projection. An absent
/// `ret` means no RETURN clause (full NeighborOutput list, unchanged behavior).
#[derive(Debug, Clone, Default)]
pub struct HqlClauses {
    pub where_preds: Vec<HqlPredicate>,
    pub order_by: Option<(HqlField, bool)>, // (field, descending)
    pub limit: Option<usize>,
    pub ret: Option<HqlReturn>,
}

#[derive(Debug, Clone)]
pub enum HqlCommand {
    Search {
        target: String,
        vector: Vec<f64>,
        k: u32,
        fuzzy: bool,
        lang: Option<String>,
        as_of: Option<String>,
        collection: Option<String>,
        clauses: HqlClauses,
    },
    Traverse {
        seed: String,
        depth: u32,
        rel: HqlRel,
        fuzzy: bool,
        as_of: Option<String>,
        clauses: HqlClauses,
    },
    Hybrid {
        target: String,
        vector: Vec<f64>,
        alpha: f64,
        fuzzy: bool,
        lang: Option<String>,
        as_of: Option<String>,
        collection: Option<String>,
        clauses: HqlClauses,
    },
    Context {
        target: String,
        tier: String,
        budget: Option<u32>,
        fuzzy: bool,
    },
}

impl TryFrom<&str> for HqlCommand {
    type Error = String;

    fn try_from(query: &str) -> Result<Self, Self::Error> {
        let pairs =
            HqlParser::parse(Rule::query, query).map_err(|e| format!("HQL Parse Error: {}", e))?;

        for pair in pairs {
            match pair.as_rule() {
                Rule::query => {
                    for inner_pair in pair.into_inner() {
                        match inner_pair.as_rule() {
                            Rule::search => return Ok(Self::parse_search(inner_pair)),
                            Rule::traverse => return Ok(Self::parse_traverse(inner_pair)),
                            Rule::hybrid => return Ok(Self::parse_hybrid(inner_pair)),
                            Rule::context => return Ok(Self::parse_context(inner_pair)),
                            Rule::EOI => continue,
                            _ => {
                                unreachable!("Unexpected rule in query: {:?}", inner_pair.as_rule())
                            }
                        }
                    }
                }
                _ => continue,
            }
        }

        Err("No valid HQL command found".to_string())
    }
}

impl HqlCommand {
    fn parse_id_with_fuzzy(pair: pest::iterators::Pair<Rule>) -> (String, bool) {
        let mut id = String::new();
        let mut fuzzy = false;
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::fuzzy_prefix => fuzzy = true,
                Rule::identifier => id = inner.as_str().to_string(),
                Rule::string_lit => {
                    let s = inner.as_str();
                    id = s[1..s.len() - 1].to_string(); // strip quotes
                }
                _ => {}
            }
        }
        (id, fuzzy)
    }

    fn parse_lang_spec(pair: pest::iterators::Pair<Rule>) -> String {
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::string_lit {
                let s = inner.as_str();
                return s[1..s.len() - 1].to_string(); // strip quotes
            }
        }
        "en".to_string()
    }

    fn parse_as_of(pair: pest::iterators::Pair<Rule>) -> String {
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::string_lit {
                let s = inner.as_str();
                return s[1..s.len() - 1].to_string(); // strip quotes
            }
        }
        "".to_string()
    }

    fn parse_string_lit(s: &str) -> String {
        // strip surrounding double quotes
        if s.len() >= 2 {
            s[1..s.len() - 1].to_string()
        } else {
            s.to_string()
        }
    }

    fn parse_predicate(pair: pest::iterators::Pair<Rule>) -> HqlPredicate {
        let mut field = HqlField::Id;
        let mut op = HqlOp::Eq;
        let mut value = HqlValue::Str(String::new());
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::field => field = Self::field_from_pair(inner),
                Rule::op => {
                    op = match inner.as_str().to_uppercase().as_str() {
                        "=" => HqlOp::Eq,
                        "!=" => HqlOp::Ne,
                        "<" => HqlOp::Lt,
                        "<=" => HqlOp::Le,
                        ">" => HqlOp::Gt,
                        ">=" => HqlOp::Ge,
                        "CONTAINS" => HqlOp::Contains,
                        "STARTSWITH" => HqlOp::StartsWith,
                        _ => HqlOp::Eq,
                    }
                }
                Rule::filter_value => {
                    let v = inner.into_inner().next();
                    if let Some(vp) = v {
                        match vp.as_rule() {
                            Rule::string_lit => {
                                value = HqlValue::Str(Self::parse_string_lit(vp.as_str()))
                            }
                            Rule::number => {
                                value = HqlValue::Num(vp.as_str().parse::<f64>().unwrap_or(0.0))
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        HqlPredicate { field, op, value }
    }

    /// Resolve a `field` rule pair into an `HqlField`.
    fn field_from_pair(pair: pest::iterators::Pair<Rule>) -> HqlField {
        if let Some(inner) = pair.clone().into_inner().next() {
            if inner.as_rule() == Rule::prop_field {
                let s = inner.as_str();
                return HqlField::Prop(s["prop.".len()..].to_string());
            }
        }
        // Bare keyword field (id/label/score/depth) — `field` has no inner token
        // for these, so read its own matched text.
        match pair.as_str().to_lowercase().as_str() {
            "id" => HqlField::Id,
            "label" => HqlField::Label,
            "score" => HqlField::Score,
            "depth" => HqlField::Depth,
            other => HqlField::Prop(other.to_string()),
        }
    }

    fn parse_clauses(pair: pest::iterators::Pair<Rule>) -> HqlClauses {
        let mut c = HqlClauses::default();
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::where_clause => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::predicate {
                            c.where_preds.push(Self::parse_predicate(p));
                        }
                    }
                }
                Rule::order_clause => {
                    let mut f = HqlField::Score;
                    let mut desc = false;
                    for p in inner.into_inner() {
                        match p.as_rule() {
                            Rule::field => f = Self::field_from_pair(p),
                            Rule::order_dir => desc = p.as_str().eq_ignore_ascii_case("DESC"),
                            _ => {}
                        }
                    }
                    c.order_by = Some((f, desc));
                }
                Rule::limit_clause => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::limit_n {
                            // Saturate on overflow rather than silently dropping the
                            // clause: an absurdly large LIMIT means "no practical cap"
                            // (truncate(usize::MAX) is a no-op), never "no LIMIT at all".
                            c.limit = Some(p.as_str().parse::<usize>().unwrap_or(usize::MAX));
                        }
                    }
                }
                Rule::return_clause => {
                    let mut fields = Vec::new();
                    let mut all = false;
                    for p in inner.into_inner() {
                        match p.as_rule() {
                            Rule::return_all => all = true,
                            Rule::field => fields.push(Self::field_from_pair(p)),
                            _ => {}
                        }
                    }
                    c.ret = Some(if all {
                        HqlReturn::All
                    } else {
                        HqlReturn::Fields(fields)
                    });
                }
                _ => {}
            }
        }
        c
    }

    fn parse_collection_spec(pair: pest::iterators::Pair<Rule>) -> String {
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => return inner.as_str().to_string(),
                Rule::string_lit => {
                    let s = inner.as_str();
                    return s[1..s.len() - 1].to_string(); // strip quotes
                }
                _ => {}
            }
        }
        String::new()
    }

    fn parse_search(pair: pest::iterators::Pair<Rule>) -> Self {
        let mut target = String::new();
        let mut vector = Vec::new();
        let mut k = 5;
        let mut fuzzy = false;
        let mut lang = None;
        let mut as_of = None;
        let mut collection = None;
        let mut clauses = HqlClauses::default();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::target => {
                    let (id, f) = Self::parse_id_with_fuzzy(inner);
                    target = id;
                    fuzzy = f;
                }
                Rule::vector => {
                    vector = inner
                        .into_inner()
                        .map(|n| n.as_str().parse::<f64>().unwrap_or(0.0))
                        .collect();
                }
                Rule::k => k = inner.as_str().parse::<u32>().unwrap_or(5),
                Rule::collection_spec => collection = Some(Self::parse_collection_spec(inner)),
                Rule::lang_spec => lang = Some(Self::parse_lang_spec(inner)),
                Rule::as_of => as_of = Some(Self::parse_as_of(inner)),
                Rule::clauses => clauses = Self::parse_clauses(inner),
                _ => {}
            }
        }

        HqlCommand::Search {
            target,
            vector,
            k,
            fuzzy,
            lang,
            as_of,
            collection,
            clauses,
        }
    }

    fn parse_traverse(pair: pest::iterators::Pair<Rule>) -> Self {
        let mut seed = String::new();
        let mut depth = 1;
        let mut rel = HqlRel::Physical("ANY".to_string());
        let mut fuzzy = false;
        let mut as_of = None;
        let mut clauses = HqlClauses::default();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::seed => {
                    let (id, f) = Self::parse_id_with_fuzzy(inner);
                    seed = id;
                    fuzzy = f;
                }
                Rule::depth => depth = inner.as_str().parse::<u32>().unwrap_or(1),
                Rule::rel => {
                    for r in inner.into_inner() {
                        match r.as_rule() {
                            Rule::rel_type => {
                                rel = HqlRel::Physical(r.as_str().to_string());
                            }
                            Rule::infer_rel => {
                                for inner_r in r.into_inner() {
                                    if inner_r.as_rule() == Rule::identifier {
                                        rel = HqlRel::Inferred(inner_r.as_str().to_string());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Rule::as_of => as_of = Some(Self::parse_as_of(inner)),
                Rule::clauses => clauses = Self::parse_clauses(inner),
                _ => {}
            }
        }

        HqlCommand::Traverse {
            seed,
            depth,
            rel,
            fuzzy,
            as_of,
            clauses,
        }
    }

    fn parse_hybrid(pair: pest::iterators::Pair<Rule>) -> Self {
        let mut target = String::new();
        let mut vector = Vec::new();
        let mut alpha = 0.5;
        let mut fuzzy = false;
        let mut lang = None;
        let mut as_of = None;
        let mut collection = None;
        let mut clauses = HqlClauses::default();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::target => {
                    let (id, f) = Self::parse_id_with_fuzzy(inner);
                    target = id;
                    fuzzy = f;
                }
                Rule::vector => {
                    vector = inner
                        .into_inner()
                        .map(|n| n.as_str().parse::<f64>().unwrap_or(0.0))
                        .collect();
                }
                Rule::alpha => alpha = inner.as_str().parse::<f64>().unwrap_or(0.5),
                Rule::collection_spec => collection = Some(Self::parse_collection_spec(inner)),
                Rule::lang_spec => lang = Some(Self::parse_lang_spec(inner)),
                Rule::as_of => as_of = Some(Self::parse_as_of(inner)),
                Rule::clauses => clauses = Self::parse_clauses(inner),
                _ => {}
            }
        }

        HqlCommand::Hybrid {
            target,
            vector,
            alpha,
            fuzzy,
            lang,
            as_of,
            collection,
            clauses,
        }
    }

    fn parse_context(pair: pest::iterators::Pair<Rule>) -> Self {
        let mut target = String::new();
        let mut tier = "H1".to_string();
        let mut budget = None;
        let mut fuzzy = false;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::target => {
                    let (id, f) = Self::parse_id_with_fuzzy(inner);
                    target = id;
                    fuzzy = f;
                }
                Rule::tier => tier = inner.as_str().to_string(),
                Rule::budget => budget = Some(inner.as_str().parse::<u32>().unwrap_or(32000)),
                _ => {}
            }
        }

        HqlCommand::Context {
            target,
            tier,
            budget,
            fuzzy,
        }
    }
}
