use pest::Parser;
use pest_derive::Parser;
use std::convert::TryFrom;

#[derive(Parser)]
#[grammar = "hql.pest"]
pub struct HqlParser;

#[derive(Debug, Clone)]
pub enum HqlCommand {
    Search {
        target: String,
        vector: Vec<f64>,
        k: u32,
        fuzzy: bool,
    },
    Traverse {
        seed: String,
        depth: u32,
        rel: String,
        fuzzy: bool,
    },
    Hybrid {
        target: String,
        vector: Vec<f64>,
        alpha: f64,
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
                _ => {}
            }
        }
        (id, fuzzy)
    }

    fn parse_search(pair: pest::iterators::Pair<Rule>) -> Self {
        let mut target = String::new();
        let mut vector = Vec::new();
        let mut k = 5;
        let mut fuzzy = false;

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
                _ => {}
            }
        }

        HqlCommand::Search { target, vector, k, fuzzy }
    }

    fn parse_traverse(pair: pest::iterators::Pair<Rule>) -> Self {
        let mut seed = String::new();
        let mut depth = 1;
        let mut rel = String::new();
        let mut fuzzy = false;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::seed => {
                    let (id, f) = Self::parse_id_with_fuzzy(inner);
                    seed = id;
                    fuzzy = f;
                }
                Rule::depth => depth = inner.as_str().parse::<u32>().unwrap_or(1),
                Rule::rel => rel = inner.as_str().trim().to_string(),
                _ => {}
            }
        }

        HqlCommand::Traverse { seed, depth, rel, fuzzy }
    }

    fn parse_hybrid(pair: pest::iterators::Pair<Rule>) -> Self {
        let mut target = String::new();
        let mut vector = Vec::new();
        let mut alpha = 0.5;
        let mut fuzzy = false;

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
                _ => {}
            }
        }

        HqlCommand::Hybrid { target, vector, alpha, fuzzy }
    }
}
