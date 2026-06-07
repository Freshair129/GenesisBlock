use pest::Parser;
use pest_derive::Parser;
use std::convert::TryFrom;

#[derive(Parser)]
#[grammar = "hql.pest"]
pub struct HqlParser;

#[derive(Debug, Clone)]
pub enum HqlRel {
    Physical(String),
    Inferred(String),
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
    },
    Traverse {
        seed: String,
        depth: u32,
        rel: HqlRel,
        fuzzy: bool,
        as_of: Option<String>,
    },
    Hybrid {
        target: String,
        vector: Vec<f64>,
        alpha: f64,
        fuzzy: bool,
        lang: Option<String>,
        as_of: Option<String>,
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
        let pairs = HqlParser::parse(Rule::query, query)
            .map_err(|e| format!("HQL Parse Error: {}", e))?;

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
                            _ => unreachable!("Unexpected rule in query: {:?}", inner_pair.as_rule()),
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
                    id = s[1..s.len()-1].to_string(); // strip quotes
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
                return s[1..s.len()-1].to_string(); // strip quotes
            }
        }
        "en".to_string()
    }

    fn parse_as_of(pair: pest::iterators::Pair<Rule>) -> String {
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::string_lit {
                let s = inner.as_str();
                return s[1..s.len()-1].to_string(); // strip quotes
            }
        }
        "".to_string()
    }

    fn parse_search(pair: pest::iterators::Pair<Rule>) -> Self {
        let mut target = String::new();
        let mut vector = Vec::new();
        let mut k = 5;
        let mut fuzzy = false;
        let mut lang = None;
        let mut as_of = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::target => {
                    let (id, f) = Self::parse_id_with_fuzzy(inner);
                    target = id;
                    fuzzy = f;
                }
                Rule::vector => {
                    vector = inner.into_inner()
                        .map(|n| n.as_str().parse::<f64>().unwrap_or(0.0))
                        .collect();
                }
                Rule::k => k = inner.as_str().parse::<u32>().unwrap_or(5),
                Rule::lang_spec => lang = Some(Self::parse_lang_spec(inner)),
                Rule::as_of => as_of = Some(Self::parse_as_of(inner)),
                _ => {}
            }
        }

        HqlCommand::Search { target, vector, k, fuzzy, lang, as_of }
    }

    fn parse_traverse(pair: pest::iterators::Pair<Rule>) -> Self {
        let mut seed = String::new();
        let mut depth = 1;
        let mut rel = HqlRel::Physical("ANY".to_string());
        let mut fuzzy = false;
        let mut as_of = None;

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
                _ => {}
            }
        }

        HqlCommand::Traverse { seed, depth, rel, fuzzy, as_of }
    }

    fn parse_hybrid(pair: pest::iterators::Pair<Rule>) -> Self {
        let mut target = String::new();
        let mut vector = Vec::new();
        let mut alpha = 0.5;
        let mut fuzzy = false;
        let mut lang = None;
        let mut as_of = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::target => {
                    let (id, f) = Self::parse_id_with_fuzzy(inner);
                    target = id;
                    fuzzy = f;
                }
                Rule::vector => {
                    vector = inner.into_inner()
                        .map(|n| n.as_str().parse::<f64>().unwrap_or(0.0))
                        .collect();
                }
                Rule::alpha => alpha = inner.as_str().parse::<f64>().unwrap_or(0.5),
                Rule::lang_spec => lang = Some(Self::parse_lang_spec(inner)),
                Rule::as_of => as_of = Some(Self::parse_as_of(inner)),
                _ => {}
            }
        }

        HqlCommand::Hybrid { target, vector, alpha, fuzzy, lang, as_of }
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

        HqlCommand::Context { target, tier, budget, fuzzy }
    }
}
