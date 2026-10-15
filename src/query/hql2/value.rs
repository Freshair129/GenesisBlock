//! Native scalar kernel. Domain values are enabled only with proven source adapters.
use super::error::QueryErrorV2;
use crate::uee_v2::RecordRefV2;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum QueryTypeV2 {
    Null,
    Bool,
    I64,
    DecimalU64,
    F64Finite,
    Utf8,
    Json,
    Entity,
    HistoryRevision,
    ChangeEvent,
    Path,
    Score,
    Context,
    Vector {
        space_id: String,
        dimension: u16,
        scalar: VectorScalarV2,
    },
    Nullable(Box<Self>),
    List(Box<Self>),
}

#[derive(Clone, Debug)]
pub enum QueryValueV2 {
    Null,
    Bool(bool),
    I64(i64),
    DecimalU64(u64),
    F64(f64),
    Utf8(String),
    Json(Value),
    Entity(RecordRefV2),
    HistoryRevision(HistoryRevisionV2),
    ChangeEvent(ChangeEventV2),
    Path(PathValueV2),
    Vector(VectorValueV2),
    Score(ScoreValueV2),
    Context(ContextPackageV2),
    List(Vec<Self>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextEvidenceV2 {
    pub source: RecordRefV2,
    pub source_hash: String,
    pub start_scalar: u64,
    pub end_scalar: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextFragmentV2 {
    pub text: String,
    pub citation: String,
    pub evidence: ContextEvidenceV2,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextPackageV2 {
    pub rendered_context: String,
    pub token_count: u64,
    pub token_budget: u64,
    pub tokenizer_fingerprint: String,
    pub fragments: Vec<ContextFragmentV2>,
    pub omitted_refs: Vec<ContextEvidenceV2>,
    pub truncated: bool,
    pub truncation_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HistoryRevisionV2 {
    pub subject: RecordRefV2,
    pub operation: String,
    #[serde(serialize_with = "serialize_decimal_u64")]
    pub tx_from: u64,
    #[serde(serialize_with = "serialize_optional_decimal_u64")]
    pub tx_to: Option<u64>,
    pub valid_from: String,
    pub valid_to: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChangeEventV2 {
    #[serde(serialize_with = "serialize_decimal_u64")]
    pub sequence: u64,
    pub operation: String,
    pub subject: RecordRefV2,
}

fn serialize_decimal_u64<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_string())
}

fn serialize_optional_decimal_u64<S: Serializer>(
    value: &Option<u64>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => serializer.serialize_some(&value.to_string()),
        None => serializer.serialize_none(),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScoreValueV2 {
    pub value: f64,
    pub owner: RecordRefV2,
    pub source: String,
    pub scope: String,
    pub metric: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VectorScalarV2 {
    F32,
    F64,
}

impl VectorScalarV2 {
    fn name(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VectorValueV2 {
    pub space_id: String,
    pub scalar: VectorScalarV2,
    pub values: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryPathModeV2 {
    Trail,
    Simple,
    Walk,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PathValueV2 {
    pub vertices: Vec<RecordRefV2>,
    pub edges: Vec<RecordRefV2>,
    pub mode: QueryPathModeV2,
}

impl PartialEq for QueryValueV2 {
    fn eq(&self, other: &Self) -> bool {
        use QueryValueV2::*;
        match (self, other) {
            (Null, Null) => true,
            (Bool(left), Bool(right)) => left == right,
            (I64(left), I64(right)) => left == right,
            (DecimalU64(left), DecimalU64(right)) => left == right,
            (F64(left), F64(right)) => left == right,
            (Utf8(left), Utf8(right)) => left == right,
            (Context(left), Context(right)) => left == right,
            (Json(left), Json(right)) => left == right,
            (Entity(left), Entity(right)) => left == right,
            (HistoryRevision(left), HistoryRevision(right)) => left == right,
            (ChangeEvent(left), ChangeEvent(right)) => left == right,
            (Path(left), Path(right)) => left == right,
            (Vector(left), Vector(right)) => left == right,
            (Score(left), Score(right)) => left == right,
            (List(left), List(right)) => left == right,
            _ => false,
        }
    }
}

fn invalid(reason: &str) -> QueryErrorV2 {
    QueryErrorV2::new("BIND_ERROR", "bind", reason)
}

impl QueryTypeV2 {
    pub(crate) fn parse(name: &str) -> Result<Self, QueryErrorV2> {
        Self::parse_inner(name, 0)
    }

    fn parse_inner(name: &str, depth: usize) -> Result<Self, QueryErrorV2> {
        if depth > 128 || name.len() > 4096 {
            return Err(invalid("type_depth"));
        }
        Ok(match name {
            "Null" => Self::Null,
            "Bool" => Self::Bool,
            "I64" => Self::I64,
            "DecimalU64" => Self::DecimalU64,
            "F64Finite" => Self::F64Finite,
            "Utf8" => Self::Utf8,
            "Json" => Self::Json,
            "Entity" => Self::Entity,
            "HistoryRevision" => Self::HistoryRevision,
            "ChangeEvent" => Self::ChangeEvent,
            "Score" => Self::Score,
            "Context" => Self::Context,
            _ => {
                if let Some(inner) = name
                    .strip_prefix("Nullable<")
                    .and_then(|s| s.strip_suffix('>'))
                {
                    let inner = Self::parse_inner(inner, depth + 1)?;
                    if matches!(inner, Self::Nullable(_)) {
                        return Err(invalid("nested_nullable"));
                    }
                    Self::Nullable(Box::new(inner))
                } else if let Some(inner) =
                    name.strip_prefix("List<").and_then(|s| s.strip_suffix('>'))
                {
                    Self::List(Box::new(Self::parse_inner(inner, depth + 1)?))
                } else if let Some(inner) = name
                    .strip_prefix("Vector<")
                    .and_then(|s| s.strip_suffix('>'))
                {
                    let mut parts = inner.split(',');
                    let space_id = parts.next().unwrap_or_default();
                    let dimension = parts.next().unwrap_or_default();
                    let scalar = parts.next().unwrap_or_default();
                    if parts.next().is_some()
                        || space_id.is_empty()
                        || space_id.chars().any(char::is_whitespace)
                        || space_id.contains(['<', '>'])
                    {
                        return Err(invalid("vector_type"));
                    }
                    let dimension = dimension
                        .parse::<u16>()
                        .ok()
                        .filter(|value| *value > 0)
                        .ok_or_else(|| invalid("vector_type"))?;
                    let scalar = match scalar {
                        "f32" => VectorScalarV2::F32,
                        "f64" => VectorScalarV2::F64,
                        _ => return Err(invalid("vector_type")),
                    };
                    Self::Vector {
                        space_id: space_id.to_owned(),
                        dimension,
                        scalar,
                    }
                } else {
                    return Err(QueryErrorV2::new(
                        "CAPABILITY_UNSUPPORTED",
                        "bind",
                        "type_unavailable",
                    ));
                }
            }
        })
    }

    pub(crate) fn name(&self) -> String {
        match self {
            Self::Null => "Null".into(),
            Self::Bool => "Bool".into(),
            Self::I64 => "I64".into(),
            Self::DecimalU64 => "DecimalU64".into(),
            Self::F64Finite => "F64Finite".into(),
            Self::Utf8 => "Utf8".into(),
            Self::Json => "Json".into(),
            Self::Entity => "Entity".into(),
            Self::HistoryRevision => "HistoryRevision".into(),
            Self::ChangeEvent => "ChangeEvent".into(),
            Self::Path => "Path".into(),
            Self::Score => "Score".into(),
            Self::Context => "Context".into(),
            Self::Vector {
                space_id,
                dimension,
                scalar,
            } => format!("Vector<{space_id},{dimension},{}>", scalar.name()),
            Self::Nullable(t) => format!("Nullable<{}>", t.name()),
            Self::List(t) => format!("List<{}>", t.name()),
        }
    }

    pub(crate) fn nullable(&self) -> bool {
        matches!(self, Self::Nullable(_))
    }
    pub(crate) fn base(&self) -> &Self {
        if let Self::Nullable(inner) = self {
            inner
        } else {
            self
        }
    }
    pub(crate) fn as_nullable(&self) -> Self {
        if self.nullable() {
            self.clone()
        } else {
            Self::Nullable(Box::new(self.clone()))
        }
    }

    pub(crate) fn decode(&self, value: &serde_json::Value) -> Result<QueryValueV2, QueryErrorV2> {
        use serde_json::Value;
        Ok(match (self, value) {
            (Self::Null, Value::Null) => QueryValueV2::Null,
            (Self::Nullable(_), Value::Null) => QueryValueV2::Null,
            (Self::Nullable(inner), value) => return inner.decode(value),
            (Self::Bool, Value::Bool(v)) => QueryValueV2::Bool(*v),
            (Self::I64, Value::String(v)) => {
                let parsed: i64 = v.parse().map_err(|_| invalid("integer_literal"))?;
                if parsed.to_string() != *v {
                    return Err(invalid("integer_literal"));
                }
                QueryValueV2::I64(parsed)
            }
            (Self::DecimalU64, Value::String(v)) => {
                let parsed: u64 = v.parse().map_err(|_| invalid("unsigned_integer_literal"))?;
                if parsed.to_string() != *v {
                    return Err(invalid("unsigned_integer_literal"));
                }
                QueryValueV2::DecimalU64(parsed)
            }
            (Self::F64Finite, Value::Number(v)) => {
                let parsed = v
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| invalid("nonfinite_literal"))?;
                QueryValueV2::F64(parsed)
            }
            (Self::Utf8, Value::String(v)) => QueryValueV2::Utf8(v.clone()),
            (Self::Json, value) => QueryValueV2::Json(value.clone()),
            (Self::Entity, value) => QueryValueV2::Entity(
                RecordRefV2::from_value(value.clone()).map_err(|_| invalid("entity_parameter"))?,
            ),
            (Self::HistoryRevision | Self::ChangeEvent, _) => {
                return Err(unsupported_parameter("domain_parameter_unavailable"))
            }
            (Self::Score | Self::Path | Self::Context, _) => {
                return Err(unsupported_parameter("domain_parameter_unavailable"))
            }
            (Self::List(inner), Value::Array(v)) => QueryValueV2::List(
                v.iter()
                    .map(|v| inner.decode(v))
                    .collect::<Result<_, _>>()?,
            ),
            (
                Self::Vector {
                    space_id,
                    dimension,
                    scalar,
                },
                Value::Array(values),
            ) => {
                if values.len() != usize::from(*dimension) {
                    return Err(QueryErrorV2::new(
                        "COLLECTION_SPACE_MISMATCH",
                        "bind",
                        "vector_dimension",
                    ));
                }
                let values = values
                    .iter()
                    .map(|value| {
                        let value = value
                            .as_f64()
                            .filter(|value| value.is_finite())
                            .ok_or_else(|| invalid("vector_value"))?;
                        match scalar {
                            VectorScalarV2::F32 => {
                                let value = value as f32;
                                if !value.is_finite() {
                                    return Err(invalid("vector_value"));
                                }
                                Ok(f64::from(value))
                            }
                            VectorScalarV2::F64 => Ok(value),
                        }
                    })
                    .collect::<Result<Vec<_>, QueryErrorV2>>()?;
                QueryValueV2::Vector(VectorValueV2 {
                    space_id: space_id.clone(),
                    scalar: *scalar,
                    values,
                })
            }
            _ => return Err(invalid("literal_type")),
        })
    }
}

impl Serialize for QueryValueV2 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("QueryValueV2", 2)?;
        match self {
            Self::Null => {
                state.serialize_field("type", "Null")?;
                state.serialize_field("value", &Option::<bool>::None)?;
            }
            Self::Bool(v) => {
                state.serialize_field("type", "Bool")?;
                state.serialize_field("value", v)?;
            }
            Self::I64(v) => {
                state.serialize_field("type", "I64")?;
                state.serialize_field("value", &v.to_string())?;
            }
            Self::DecimalU64(v) => {
                state.serialize_field("type", "DecimalU64")?;
                state.serialize_field("value", &v.to_string())?;
            }
            Self::F64(v) => {
                if !v.is_finite() {
                    return Err(serde::ser::Error::custom("nonfinite_result"));
                }
                state.serialize_field("type", "F64Finite")?;
                state.serialize_field("value", v)?;
            }
            Self::Utf8(v) => {
                state.serialize_field("type", "Utf8")?;
                state.serialize_field("value", v)?;
            }
            Self::Json(v) => {
                state.serialize_field("type", "Json")?;
                state.serialize_field("value", v)?;
            }
            Self::Entity(v) => {
                state.serialize_field("type", "Entity")?;
                state.serialize_field("value", v)?;
            }
            Self::HistoryRevision(v) => {
                state.serialize_field("type", "HistoryRevision")?;
                state.serialize_field("value", v)?;
            }
            Self::ChangeEvent(v) => {
                state.serialize_field("type", "ChangeEvent")?;
                state.serialize_field("value", v)?;
            }
            Self::Path(v) => {
                state.serialize_field("type", "Path")?;
                state.serialize_field("value", v)?;
            }
            Self::Vector(v) => {
                state.serialize_field(
                    "type",
                    &format!(
                        "Vector<{},{},{}>",
                        v.space_id,
                        v.values.len(),
                        v.scalar.name()
                    ),
                )?;
                state.serialize_field("value", v)?;
            }
            Self::Score(v) => {
                if !v.value.is_finite() {
                    return Err(serde::ser::Error::custom("nonfinite_result"));
                }
                state.serialize_field("type", "Score")?;
                state.serialize_field("value", v)?;
            }
            Self::Context(v) => {
                state.serialize_field("type", "Context")?;
                state.serialize_field("value", v)?;
            }
            Self::List(v) => {
                state.serialize_field("type", "List")?;
                state.serialize_field("value", v)?;
            }
        }
        state.end()
    }
}

fn unsupported_parameter(reason: &str) -> QueryErrorV2 {
    QueryErrorV2::new("CAPABILITY_UNSUPPORTED", "bind", reason)
}
