//! JavaScript DTOs are separate from authenticated storage/wire metadata.
//!
//! Keep declarations explicit: napi's object parser must see concrete Option
//! types, not opaque macro type fragments, to recognize omitted properties.

use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};
use napi_derive::napi;

#[napi(object, js_name = "NodeOutput")]
pub struct NodeApiDto {
    pub id: String,
    pub labels: Vec<String>,
    pub props: serde_json::Value,
    pub embedding: Option<Vec<f64>>,
    pub lang: Option<String>,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub caused_by: Option<String>,
    pub expires_at: Option<String>,
    pub clock: super::LogicalClock,
    /// Vector collection containing this node's embedding; omitted means default.
    pub collection: Option<String>,
}

#[napi(object, js_name = "EdgeOutput")]
pub struct EdgeApiDto {
    pub id: String,
    pub from: String,
    pub to: String,
    pub rel: String,
    pub props: serde_json::Value,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub recorded_at: String,
    pub superseded_by: Option<String>,
    pub caused_by: Option<String>,
    pub clock: super::LogicalClock,
}

#[napi(object, js_name = "EdgeInput")]
pub struct EdgeInputApiDto {
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    pub rel: String,
    pub props: Option<serde_json::Value>,
    pub valid_from: Option<String>,
    pub supersede: Option<bool>,
    #[napi(skip_typescript)]
    pub impact: Option<f64>,
    pub caused_by: Option<String>,
}

#[napi(object, js_name = "HybridSearchInput")]
pub struct HybridSearchApiDto {
    pub query_vector: Vec<f64>,
    pub k: u32,
    #[napi(skip_typescript)]
    pub alpha: Option<f64>,
    pub lang: Option<String>,
    pub as_of: Option<String>,
    /// Vector collection to search. Defaults to `default`.
    pub collection: Option<String>,
    /// Per-query HNSW candidate breadth.
    pub ef_search: Option<u32>,
    /// Per-query exact rerank over-fetch multiplier.
    pub oversample: Option<u32>,
}

macro_rules! delegate_napi {
    ($dto:ident, $core:ident, $js_name:literal,
     $prepare:expr, {$($extra:ident: $default:expr),*},
     [$($field:ident),* $(,)?]) => {
        impl napi::bindgen_prelude::TypeName for super::$core {
            fn type_name() -> &'static str { $js_name }
            fn value_type() -> napi::ValueType { napi::ValueType::Object }
        }
        impl napi::bindgen_prelude::ValidateNapiValue for super::$core {}

        impl FromNapiValue for super::$core {
            unsafe fn from_napi_value(
                env: napi::sys::napi_env,
                value: napi::sys::napi_value,
            ) -> napi::Result<Self> {
                let dto = unsafe { <$dto as FromNapiValue>::from_napi_value(env, value)? };
                Ok(Self { $($field: dto.$field,)* $($extra: $default,)* })
            }
        }

        impl ToNapiValue for super::$core {
            unsafe fn to_napi_value(
                env: napi::sys::napi_env,
                value: Self,
            ) -> napi::Result<napi::sys::napi_value> {
                let value = ($prepare)(value)?;
                let dto = $dto { $($field: value.$field,)* };
                unsafe { <$dto as ToNapiValue>::to_napi_value(env, dto) }
            }
        }
    };
}

delegate_napi!(NodeApiDto, NodeOutput, "NodeOutput",
    |mut value: super::NodeOutput| {
        super::migrate_legacy_node_impact(&mut value)?;
        Ok::<_, napi::Error>(value)
    },
    {legacy_wire_impact: super::LegacyImpactWire::Absent},
    [id, labels, props, embedding, lang, valid_from, valid_to, caused_by,
     expires_at, clock, collection]
);

delegate_napi!(EdgeApiDto, EdgeOutput, "EdgeOutput",
    |mut value: super::EdgeOutput| {
        super::migrate_legacy_edge_impact(&mut value)?;
        Ok::<_, napi::Error>(value)
    },
    {legacy_wire_impact: super::LegacyImpactWire::Absent},
    [id, from, to, rel, props, valid_from, valid_to, recorded_at,
     superseded_by, caused_by, clock]
);

delegate_napi!(
    EdgeInputApiDto,
    EdgeInput,
    "EdgeInput",
    Ok::<_, napi::Error>,
    {},
    [id, from, to, rel, props, valid_from, supersede, impact, caused_by]
);

delegate_napi!(
    HybridSearchApiDto,
    HybridSearchInput,
    "HybridSearchInput",
    Ok::<_, napi::Error>,
    {},
    [
        query_vector,
        k,
        alpha,
        lang,
        as_of,
        collection,
        ef_search,
        oversample
    ]
);
