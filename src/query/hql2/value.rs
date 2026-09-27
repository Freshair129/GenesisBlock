//! Native scalar kernel. Domain values are enabled only with proven source adapters.
use super::error::QueryErrorV2;
use serde::{ser::SerializeStruct, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum QueryTypeV2 {
    Bool,
    I64,
    F64Finite,
    Utf8,
    Nullable(Box<Self>),
    List(Box<Self>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum QueryValueV2 {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    Utf8(String),
    List(Vec<Self>),
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
            "Bool" => Self::Bool,
            "I64" => Self::I64,
            "F64Finite" => Self::F64Finite,
            "Utf8" => Self::Utf8,
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
            Self::Bool => "Bool".into(),
            Self::I64 => "I64".into(),
            Self::F64Finite => "F64Finite".into(),
            Self::Utf8 => "Utf8".into(),
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
            (Self::F64Finite, Value::Number(v)) => {
                let parsed = v
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| invalid("nonfinite_literal"))?;
                QueryValueV2::F64(parsed)
            }
            (Self::Utf8, Value::String(v)) => QueryValueV2::Utf8(v.clone()),
            (Self::List(inner), Value::Array(v)) => QueryValueV2::List(
                v.iter()
                    .map(|v| inner.decode(v))
                    .collect::<Result<_, _>>()?,
            ),
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
            Self::List(v) => {
                state.serialize_field("type", "List")?;
                state.serialize_field("value", v)?;
            }
        }
        state.end()
    }
}
