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

// --- Cypher-style graph pattern matching (ADR--GENESISDB-HQL-CYPHER-PATTERNS) ---

/// Which index a hop follows and which endpoint is the "far" node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternDirection {
    Out,  // (a)-[r]->(b): follow outgoing edges, far = the other endpoint
    In,   // (a)<-[r]-(b): follow incoming edges
    Both, // (a)-[r]-(b): either direction
}

/// A node in a pattern: `(var? :Label? {k:v,...}?)`. Every part is optional;
/// `()` matches any node. Inline `props` are exact-equality constraints.
#[derive(Debug, Clone, Default)]
pub struct NodePattern {
    pub var: Option<String>,
    pub label: Option<String>,
    pub props: Vec<(String, HqlValue)>,
}

/// An edge in a pattern: direction + optional variable + optional `:Type`.
#[derive(Debug, Clone)]
pub struct EdgePattern {
    pub var: Option<String>,
    pub rel_type: Option<String>,
    pub direction: PatternDirection,
}

/// A linear path pattern: an anchor node then zero+ `(edge, node)` hops.
#[derive(Debug, Clone)]
pub struct GraphPattern {
    pub start: NodePattern,
    pub hops: Vec<(EdgePattern, NodePattern)>,
}

/// A variable-qualified clause field: `a` (`field` None → whole bound entity)
/// or `a.id` / `a.label` / `a.prop.<key>` (reusing `HqlField`).
#[derive(Debug, Clone)]
pub struct QualField {
    pub var: String,
    pub field: Option<HqlField>,
}

impl QualField {
    /// Output key when projected via RETURN: `a` (bare) or `a.id` / `a.<propkey>`.
    pub fn output_key(&self) -> String {
        match &self.field {
            None => self.var.clone(),
            Some(HqlField::Id) => format!("{}.id", self.var),
            Some(HqlField::Label) => format!("{}.label", self.var),
            Some(HqlField::Score) => format!("{}.score", self.var),
            Some(HqlField::Depth) => format!("{}.depth", self.var),
            Some(HqlField::Prop(k)) => format!("{}.{}", self.var, k),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PatternPredicate {
    pub field: QualField,
    pub op: HqlOp,
    pub value: HqlValue,
}

#[derive(Debug, Clone)]
pub enum PatternReturn {
    All,
    Fields(Vec<QualField>),
}

#[derive(Debug, Clone, Default)]
pub struct PatternClauses {
    pub where_preds: Vec<PatternPredicate>,
    pub order_by: Option<(QualField, bool)>, // (field, descending)
    pub limit: Option<usize>,
    pub ret: Option<PatternReturn>,
}

#[derive(Debug, Clone)]
pub enum HqlCommand {
    Search {
        target: String,
        vector: Option<Vec<f64>>,
        k: u32,
        ef_search: Option<u32>,
        oversample: Option<u32>,
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
        rels: Option<Vec<String>>,
        direction: Option<String>,
        fuzzy: bool,
        as_of: Option<String>,
        clauses: HqlClauses,
    },
    Hybrid {
        target: String,
        vector: Option<Vec<f64>>,
        alpha: f64,
        k: u32,
        ef_search: Option<u32>,
        oversample: Option<u32>,
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
    MatchPattern {
        pattern: GraphPattern,
        as_of: Option<String>,
        clauses: PatternClauses,
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
                            Rule::search => return Self::parse_search(inner_pair),
                            Rule::traverse => return Self::parse_traverse(inner_pair),
                            Rule::hybrid => return Self::parse_hybrid(inner_pair),
                            Rule::match_pattern => return Self::parse_match_pattern(inner_pair),
                            Rule::context => return Self::parse_context(inner_pair),
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
    fn parse_f64(value: &str, field: &str) -> Result<f64, String> {
        let parsed = value
            .parse::<f64>()
            .map_err(|_| format!("HQL Parse Error: {field} value out of range: {value}"))?;
        if !parsed.is_finite() {
            return Err(format!(
                "HQL Parse Error: {field} value out of range: {value}"
            ));
        }
        Ok(parsed)
    }

    fn parse_u32(value: &str, field: &str) -> Result<u32, String> {
        value
            .parse::<u32>()
            .map_err(|_| format!("HQL Parse Error: {field} value out of range: {value}"))
    }

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

    fn parse_predicate(pair: pest::iterators::Pair<Rule>) -> Result<HqlPredicate, String> {
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
                                value =
                                    HqlValue::Num(Self::parse_f64(vp.as_str(), "filter number")?)
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(HqlPredicate { field, op, value })
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

    fn parse_clauses(pair: pest::iterators::Pair<Rule>) -> Result<HqlClauses, String> {
        let mut c = HqlClauses::default();
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::where_clause => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::predicate {
                            c.where_preds.push(Self::parse_predicate(p)?);
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
        Ok(c)
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

    fn parse_vector(pair: pest::iterators::Pair<Rule>) -> Result<Vec<f64>, String> {
        pair.into_inner()
            .map(|n| Self::parse_f64(n.as_str(), "vector component"))
            .collect()
    }

    fn parse_search(pair: pest::iterators::Pair<Rule>) -> Result<Self, String> {
        let mut target = String::new();
        let mut vector = None;
        let mut k = 5;
        let mut ef_search = None;
        let mut oversample = None;
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
                Rule::similar_clause => {
                    if let Some(v) = inner.into_inner().find(|p| p.as_rule() == Rule::vector) {
                        vector = Some(Self::parse_vector(v)?);
                    }
                }
                Rule::k => k = Self::parse_u32(inner.as_str(), "K")?,
                Rule::ef_spec => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::ef {
                            ef_search = Some(Self::parse_u32(p.as_str(), "EF")?);
                        }
                    }
                }
                Rule::oversample_spec => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::oversample {
                            oversample = Some(Self::parse_u32(p.as_str(), "OVERSAMPLE")?);
                        }
                    }
                }
                Rule::collection_spec => collection = Some(Self::parse_collection_spec(inner)),
                Rule::lang_spec => lang = Some(Self::parse_lang_spec(inner)),
                Rule::as_of => as_of = Some(Self::parse_as_of(inner)),
                Rule::clauses => clauses = Self::parse_clauses(inner)?,
                _ => {}
            }
        }

        Ok(HqlCommand::Search {
            target,
            vector,
            k,
            ef_search,
            oversample,
            fuzzy,
            lang,
            as_of,
            collection,
            clauses,
        })
    }

    fn parse_traverse(pair: pest::iterators::Pair<Rule>) -> Result<Self, String> {
        let mut seed = String::new();
        let mut depth = 1;
        let mut rel = HqlRel::Physical("ANY".to_string());
        let mut rels = None;
        let mut direction = None;
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
                Rule::depth => depth = Self::parse_u32(inner.as_str(), "DEPTH")?,
                Rule::rel => {
                    for r in inner.into_inner() {
                        match r.as_rule() {
                            Rule::rel_type => {
                                let names: Vec<String> = r
                                    .into_inner()
                                    .filter(|p| p.as_rule() == Rule::rel_name)
                                    .map(|p| p.as_str().to_string())
                                    .collect();
                                if let Some(first) = names.first() {
                                    rel = HqlRel::Physical(first.clone());
                                }
                                if names.len() > 1 {
                                    rels = Some(names);
                                }
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
                Rule::direction_spec => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::direction {
                            direction = Some(p.as_str().to_ascii_lowercase());
                        }
                    }
                }
                Rule::as_of => as_of = Some(Self::parse_as_of(inner)),
                Rule::clauses => clauses = Self::parse_clauses(inner)?,
                _ => {}
            }
        }

        Ok(HqlCommand::Traverse {
            seed,
            depth,
            rel,
            rels,
            direction,
            fuzzy,
            as_of,
            clauses,
        })
    }

    fn parse_hybrid(pair: pest::iterators::Pair<Rule>) -> Result<Self, String> {
        let mut target = String::new();
        let mut vector = None;
        let mut alpha = 0.5;
        let mut k = 10;
        let mut ef_search = None;
        let mut oversample = None;
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
                Rule::similar_clause => {
                    if let Some(v) = inner.into_inner().find(|p| p.as_rule() == Rule::vector) {
                        vector = Some(Self::parse_vector(v)?);
                    }
                }
                Rule::alpha => alpha = Self::parse_f64(inner.as_str(), "ALPHA")?,
                Rule::k => k = Self::parse_u32(inner.as_str(), "K")?,
                Rule::ef_spec => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::ef {
                            ef_search = Some(Self::parse_u32(p.as_str(), "EF")?);
                        }
                    }
                }
                Rule::oversample_spec => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::oversample {
                            oversample = Some(Self::parse_u32(p.as_str(), "OVERSAMPLE")?);
                        }
                    }
                }
                Rule::collection_spec => collection = Some(Self::parse_collection_spec(inner)),
                Rule::lang_spec => lang = Some(Self::parse_lang_spec(inner)),
                Rule::as_of => as_of = Some(Self::parse_as_of(inner)),
                Rule::clauses => clauses = Self::parse_clauses(inner)?,
                _ => {}
            }
        }

        Ok(HqlCommand::Hybrid {
            target,
            vector,
            alpha,
            k,
            ef_search,
            oversample,
            fuzzy,
            lang,
            as_of,
            collection,
            clauses,
        })
    }

    fn parse_context(pair: pest::iterators::Pair<Rule>) -> Result<Self, String> {
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
                Rule::budget => budget = Some(Self::parse_u32(inner.as_str(), "BUDGET")?),
                _ => {}
            }
        }

        Ok(HqlCommand::Context {
            target,
            tier,
            budget,
            fuzzy,
        })
    }

    // --- Cypher pattern parsing (ADR--GENESISDB-HQL-CYPHER-PATTERNS) ---

    fn op_from_str(s: &str) -> HqlOp {
        match s.to_uppercase().as_str() {
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

    /// Parse a `filter_value` (`string_lit | number`) rule into an `HqlValue`.
    fn parse_filter_value(pair: pest::iterators::Pair<Rule>) -> Result<HqlValue, String> {
        if let Some(vp) = pair.into_inner().next() {
            match vp.as_rule() {
                Rule::string_lit => Ok(HqlValue::Str(Self::parse_string_lit(vp.as_str()))),
                Rule::number => Ok(HqlValue::Num(Self::parse_f64(
                    vp.as_str(),
                    "filter number",
                )?)),
                _ => Ok(HqlValue::Str(String::new())),
            }
        } else {
            Ok(HqlValue::Str(String::new()))
        }
    }

    /// `pat_label` = `":" ~ identifier` — return the identifier text.
    fn parse_pat_label(pair: pest::iterators::Pair<Rule>) -> String {
        pair.into_inner()
            .find(|p| p.as_rule() == Rule::identifier)
            .map(|p| p.as_str().to_string())
            .unwrap_or_default()
    }

    fn parse_pat_props(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<(String, HqlValue)>, String> {
        let mut props = Vec::new();
        for pp in pair.into_inner() {
            if pp.as_rule() == Rule::prop_pair {
                let mut key = String::new();
                let mut val = HqlValue::Str(String::new());
                for inner in pp.into_inner() {
                    match inner.as_rule() {
                        Rule::identifier => key = inner.as_str().to_string(),
                        Rule::filter_value => val = Self::parse_filter_value(inner)?,
                        _ => {}
                    }
                }
                props.push((key, val));
            }
        }
        Ok(props)
    }

    fn parse_node_pattern(pair: pest::iterators::Pair<Rule>) -> Result<NodePattern, String> {
        let mut np = NodePattern::default();
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::pat_var => np.var = Some(inner.as_str().to_string()),
                Rule::pat_label => np.label = Some(Self::parse_pat_label(inner)),
                Rule::pat_props => np.props = Self::parse_pat_props(inner)?,
                _ => {}
            }
        }
        Ok(np)
    }

    fn parse_edge_pattern(pair: pest::iterators::Pair<Rule>) -> EdgePattern {
        // edge_pattern = { edge_in | edge_out | edge_both }; the inner rule
        // carries the direction and an optional rel_detail.
        let mut var = None;
        let mut rel_type = None;
        let mut direction = PatternDirection::Out;
        if let Some(dir_pair) = pair.into_inner().next() {
            direction = match dir_pair.as_rule() {
                Rule::edge_in => PatternDirection::In,
                Rule::edge_out => PatternDirection::Out,
                Rule::edge_both => PatternDirection::Both,
                _ => PatternDirection::Out,
            };
            for d in dir_pair.into_inner() {
                if d.as_rule() == Rule::rel_detail {
                    for r in d.into_inner() {
                        match r.as_rule() {
                            Rule::pat_var => var = Some(r.as_str().to_string()),
                            Rule::pat_label => rel_type = Some(Self::parse_pat_label(r)),
                            _ => {}
                        }
                    }
                }
            }
        }
        EdgePattern {
            var,
            rel_type,
            direction,
        }
    }

    fn parse_hop(pair: pest::iterators::Pair<Rule>) -> Result<(EdgePattern, NodePattern), String> {
        let mut edge = EdgePattern {
            var: None,
            rel_type: None,
            direction: PatternDirection::Out,
        };
        let mut node = NodePattern::default();
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::edge_pattern => edge = Self::parse_edge_pattern(inner),
                Rule::node_pattern => node = Self::parse_node_pattern(inner)?,
                _ => {}
            }
        }
        Ok((edge, node))
    }

    fn parse_graph_pattern(pair: pest::iterators::Pair<Rule>) -> Result<GraphPattern, String> {
        let mut start = NodePattern::default();
        let mut hops = Vec::new();
        let mut seen_anchor = false;
        for inner in pair.into_inner() {
            match inner.as_rule() {
                // The only direct `node_pattern` child is the anchor; hop nodes
                // are nested inside their `hop`.
                Rule::node_pattern if !seen_anchor => {
                    start = Self::parse_node_pattern(inner)?;
                    seen_anchor = true;
                }
                Rule::hop => hops.push(Self::parse_hop(inner)?),
                _ => {}
            }
        }
        Ok(GraphPattern { start, hops })
    }

    /// `qual_tail` = `prop_field | id | label` → an `HqlField`.
    fn qual_tail_to_field(pair: pest::iterators::Pair<Rule>) -> HqlField {
        if let Some(inner) = pair.clone().into_inner().next() {
            if inner.as_rule() == Rule::prop_field {
                let s = inner.as_str();
                return HqlField::Prop(s["prop.".len()..].to_string());
            }
        }
        match pair.as_str().to_lowercase().as_str() {
            "id" => HqlField::Id,
            "label" => HqlField::Label,
            other => HqlField::Prop(other.to_string()),
        }
    }

    fn parse_qual_field(pair: pest::iterators::Pair<Rule>) -> QualField {
        let mut var = String::new();
        let mut field = None;
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => var = inner.as_str().to_string(),
                Rule::qual_tail => field = Some(Self::qual_tail_to_field(inner)),
                _ => {}
            }
        }
        QualField { var, field }
    }

    fn parse_pat_predicate(pair: pest::iterators::Pair<Rule>) -> Result<PatternPredicate, String> {
        let mut field = QualField {
            var: String::new(),
            field: None,
        };
        let mut op = HqlOp::Eq;
        let mut value = HqlValue::Str(String::new());
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::qual_field => field = Self::parse_qual_field(inner),
                Rule::op => op = Self::op_from_str(inner.as_str()),
                Rule::filter_value => value = Self::parse_filter_value(inner)?,
                _ => {}
            }
        }
        Ok(PatternPredicate { field, op, value })
    }

    fn parse_pat_clauses(pair: pest::iterators::Pair<Rule>) -> Result<PatternClauses, String> {
        let mut c = PatternClauses::default();
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::pat_where => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::pat_predicate {
                            c.where_preds.push(Self::parse_pat_predicate(p)?);
                        }
                    }
                }
                Rule::pat_order => {
                    let mut f = None;
                    let mut desc = false;
                    for p in inner.into_inner() {
                        match p.as_rule() {
                            Rule::qual_field => f = Some(Self::parse_qual_field(p)),
                            Rule::order_dir => desc = p.as_str().eq_ignore_ascii_case("DESC"),
                            _ => {}
                        }
                    }
                    if let Some(f) = f {
                        c.order_by = Some((f, desc));
                    }
                }
                Rule::limit_clause => {
                    for p in inner.into_inner() {
                        if p.as_rule() == Rule::limit_n {
                            c.limit = Some(p.as_str().parse::<usize>().unwrap_or(usize::MAX));
                        }
                    }
                }
                Rule::pat_return => {
                    let mut fields = Vec::new();
                    let mut all = false;
                    for p in inner.into_inner() {
                        match p.as_rule() {
                            Rule::return_all => all = true,
                            Rule::qual_field => fields.push(Self::parse_qual_field(p)),
                            _ => {}
                        }
                    }
                    c.ret = Some(if all {
                        PatternReturn::All
                    } else {
                        PatternReturn::Fields(fields)
                    });
                }
                _ => {}
            }
        }
        Ok(c)
    }

    fn parse_match_pattern(pair: pest::iterators::Pair<Rule>) -> Result<Self, String> {
        let mut pattern = GraphPattern {
            start: NodePattern::default(),
            hops: Vec::new(),
        };
        let mut as_of = None;
        let mut clauses = PatternClauses::default();
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::graph_pattern => pattern = Self::parse_graph_pattern(inner)?,
                Rule::as_of => as_of = Some(Self::parse_as_of(inner)),
                Rule::pat_clauses => clauses = Self::parse_pat_clauses(inner)?,
                _ => {}
            }
        }
        Ok(HqlCommand::MatchPattern {
            pattern,
            as_of,
            clauses,
        })
    }
}
