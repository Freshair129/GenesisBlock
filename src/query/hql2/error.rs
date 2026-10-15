//! Safe v2 errors: input text and storage identifiers never enter messages.
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourcePositionV2 {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct QueryErrorV2 {
    pub code: String,
    pub stage: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourcePositionV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Box<serde_json::Value>>,
}

impl QueryErrorV2 {
    pub(crate) fn new(code: &str, stage: &str, reason: &str) -> Self {
        Self {
            code: code.into(),
            stage: stage.into(),
            message: "Query could not be completed".into(),
            retryable: matches!(code, "SNAPSHOT_EXPIRED" | "INDEX_COVERAGE_TIMEOUT"),
            span: None,
            detail: Some(Box::new(serde_json::json!({"reason": reason}))),
        }
    }

    pub(crate) fn parse(source: &str, start: usize, reason: &str) -> Self {
        let mut error = Self::new("HQL_PARSE_ERROR", "parse", reason);
        let mut line = 1;
        let mut column = 1;
        let mut previous_cr = false;
        for (offset, character) in source.char_indices() {
            if offset >= start {
                break;
            }
            match character {
                '\r' => {
                    line += 1;
                    column = 1;
                }
                '\n' => {
                    if !previous_cr {
                        line += 1;
                    }
                    column = 1;
                }
                _ => column += 1,
            }
            previous_cr = character == '\r';
        }
        error.span = Some(SourcePositionV2 { line, column });
        error
    }
}

impl std::fmt::Display for QueryErrorV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for QueryErrorV2 {}
