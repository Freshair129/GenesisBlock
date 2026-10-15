//! Shared, additive machine-readable errors for REST and embedded APIs.
//!
//! Legacy `Storage` and NAPI methods keep their existing result signatures.
//! New contract methods return `ApiErrorEnvelopeV1` directly rather than
//! trying to classify their legacy diagnostic strings.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

pub const API_ERROR_CONTRACT_V1: &str = "genesis.api.errors.v1";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCategory {
    Validation,
    Capability,
    Query,
    Conflict,
    Durability,
    Integrity,
    Internal,
    NotFound,
    Authorization,
    RateLimit,
    Unavailable,
}

impl ApiErrorCategory {
    pub const fn stable_code(self) -> &'static str {
        match self {
            Self::Validation => "GENESIS_VALIDATION_ERROR",
            Self::Capability => "GENESIS_CAPABILITY_UNSUPPORTED",
            Self::Query => "GENESIS_QUERY_ERROR",
            Self::Conflict => "GENESIS_CONFLICT",
            Self::Durability => "GENESIS_DURABILITY_ERROR",
            Self::Integrity => "GENESIS_INTEGRITY_ERROR",
            Self::Internal => "GENESIS_INTERNAL_ERROR",
            Self::NotFound => "GENESIS_NOT_FOUND",
            Self::Authorization => "GENESIS_UNAUTHORIZED",
            Self::RateLimit => "GENESIS_RATE_LIMITED",
            Self::Unavailable => "GENESIS_UNAVAILABLE",
        }
    }

    /// Convert the structured HQL2 code, never its message or display text.
    pub fn from_query_error_code(code: &str) -> Self {
        match code {
            "CAPABILITY_UNSUPPORTED" | "COLLECTION_SPACE_MISMATCH" => Self::Capability,
            "HQL_PARSE_ERROR" | "BIND_ERROR" | "VERSION_UNSUPPORTED" => Self::Validation,
            "QUERY_BUDGET_EXCEEDED" => Self::Query,
            "DATA_CORRUPTION" => Self::Integrity,
            "AUTH_REQUIRED" | "FORBIDDEN" => Self::Authorization,
            "SNAPSHOT_EXPIRED" | "INDEX_COVERAGE_TIMEOUT" => Self::Unavailable,
            "RECOVERY_REQUIRED" | "COMMIT_OUTCOME_UNKNOWN" | "DURABLE_COMMIT_APPLY_FAILED" => {
                Self::Durability
            }
            "CONFLICT" | "TRANSACTION_CONFLICT" => Self::Conflict,
            "NOT_FOUND" => Self::NotFound,
            "INTERNAL_ERROR" | "ENCODE_ERROR" => Self::Internal,
            _ => Self::Query,
        }
    }
}

/// Stable envelope fields. `status` is present on REST responses and omitted
/// from embedded NAPI results, where no HTTP status exists.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ApiErrorEnvelopeV1 {
    pub category: ApiErrorCategory,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    /// QueryErrorV2's structured pre-envelope code, exposed as a migration alias.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_code: Option<String>,
}

impl ApiErrorEnvelopeV1 {
    pub fn new(category: ApiErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            code: category.stable_code().into(),
            message: message.into(),
            status: None,
            details: None,
            legacy_code: None,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCategory::Validation, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCategory::Conflict, message)
    }

    pub fn durability(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCategory::Durability, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCategory::Internal, message)
    }

    pub fn from_query_error(error: &crate::query::hql2::QueryErrorV2) -> Self {
        let category = ApiErrorCategory::from_query_error_code(&error.code);
        Self {
            category,
            code: category.stable_code().into(),
            message: error.message.clone(),
            status: None,
            details: error.detail.as_deref().cloned(),
            legacy_code: Some(error.code.clone()),
        }
    }
}

// Unknown legacy Storage errors retain their diagnostic as an internal error.
// Route-specific classifications must be explicit; message text is never parsed.
impl From<super::Error> for ApiErrorEnvelopeV1 {
    fn from(error: super::Error) -> Self {
        Self::internal(error.to_string())
    }
}

impl fmt::Display for ApiErrorEnvelopeV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ApiErrorEnvelopeV1 {}

pub type CoreResult<T> = std::result::Result<T, ApiErrorEnvelopeV1>;

pub fn success_result_value(data: impl Serialize) -> Value {
    serde_json::json!({
        "contract_version": API_ERROR_CONTRACT_V1,
        "ok": true,
        "data": data,
    })
}

/// Serialize the payload of a query outcome without changing the engine enum's
/// public traits or adding a synthetic variant wrapper to the JSON contract.
pub fn query_success_result_value(outcome: crate::query::hql2::QueryOutcomeV2) -> Value {
    let data = match outcome {
        crate::query::hql2::QueryOutcomeV2::Rows(result) => serde_json::to_value(result),
        crate::query::hql2::QueryOutcomeV2::Plan(plan) => serde_json::to_value(plan),
    };
    match data {
        Ok(data) => success_result_value(data),
        Err(error) => error_result_value(ApiErrorEnvelopeV1::internal(error.to_string())),
    }
}

pub fn error_result_value(error: ApiErrorEnvelopeV1) -> Value {
    let legacy_code = error.legacy_code.clone();
    let details = error.details.clone();
    let message = error.message.clone();
    let mut value = serde_json::json!({
        "contract_version": API_ERROR_CONTRACT_V1,
        "ok": false,
        "error": error,
        "message": message,
    });
    if let Some(code) = legacy_code {
        value["code"] = serde_json::json!(code);
    }
    if let Some(details) = details {
        value["details"] = details;
    }
    value
}

pub fn query_error_result_value(error: &crate::query::hql2::QueryErrorV2) -> Value {
    error_result_value(ApiErrorEnvelopeV1::from_query_error(error))
}

pub fn parse_query_v2_input(
    access_json: &str,
    request_json: &str,
) -> CoreResult<(crate::AccessContext, crate::uee_v2::QueryRequestV2)> {
    let access = serde_json::from_str::<crate::AccessContext>(access_json)
        .map_err(|error| ApiErrorEnvelopeV1::validation(format!("access context JSON: {error}")))?;
    let request = serde_json::from_str::<crate::uee_v2::QueryRequestV2>(request_json)
        .map_err(|error| ApiErrorEnvelopeV1::validation(format!("query request JSON: {error}")))?;
    Ok((access, request))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::hql2::result::{CatalogStampV2, ExplainResultV2};
    use crate::query::hql2::{QueryErrorV2, QueryOutcomeV2};

    #[test]
    fn stable_codes_are_one_to_one_with_categories() {
        let cases = [
            (
                ApiErrorCategory::Validation,
                "validation",
                "GENESIS_VALIDATION_ERROR",
            ),
            (
                ApiErrorCategory::Capability,
                "capability",
                "GENESIS_CAPABILITY_UNSUPPORTED",
            ),
            (ApiErrorCategory::Query, "query", "GENESIS_QUERY_ERROR"),
            (ApiErrorCategory::Conflict, "conflict", "GENESIS_CONFLICT"),
            (
                ApiErrorCategory::Durability,
                "durability",
                "GENESIS_DURABILITY_ERROR",
            ),
            (
                ApiErrorCategory::Integrity,
                "integrity",
                "GENESIS_INTEGRITY_ERROR",
            ),
        ];

        for (category, serialized_category, code) in cases {
            let value = serde_json::to_value(ApiErrorEnvelopeV1::new(category, "detail")).unwrap();
            assert_eq!(value["category"], serialized_category);
            assert_eq!(value["code"], code);
            assert_eq!(value["message"], "detail");
        }
    }

    #[test]
    fn napi_result_helper_serializes_the_six_category_contract() {
        let cases = [
            (
                ApiErrorCategory::Validation,
                "validation",
                "GENESIS_VALIDATION_ERROR",
            ),
            (
                ApiErrorCategory::Capability,
                "capability",
                "GENESIS_CAPABILITY_UNSUPPORTED",
            ),
            (ApiErrorCategory::Query, "query", "GENESIS_QUERY_ERROR"),
            (ApiErrorCategory::Conflict, "conflict", "GENESIS_CONFLICT"),
            (
                ApiErrorCategory::Durability,
                "durability",
                "GENESIS_DURABILITY_ERROR",
            ),
            (
                ApiErrorCategory::Integrity,
                "integrity",
                "GENESIS_INTEGRITY_ERROR",
            ),
        ];

        for (category, serialized_category, code) in cases {
            let value = error_result_value(ApiErrorEnvelopeV1::new(category, "diagnostic"));
            let json: Value = serde_json::from_slice(&serde_json::to_vec(&value).unwrap()).unwrap();
            assert_eq!(json["contract_version"], API_ERROR_CONTRACT_V1);
            assert_eq!(json["ok"], false);
            assert_eq!(json["error"]["category"], serialized_category);
            assert_eq!(json["error"]["code"], code);
            assert_eq!(json["error"]["message"], "diagnostic");
        }
    }

    #[test]
    fn query_error_mapping_uses_structured_codes() {
        let cases = [
            ("BIND_ERROR", ApiErrorCategory::Validation),
            ("CAPABILITY_UNSUPPORTED", ApiErrorCategory::Capability),
            ("QUERY_BUDGET_EXCEEDED", ApiErrorCategory::Query),
            ("DATA_CORRUPTION", ApiErrorCategory::Integrity),
            ("FORBIDDEN", ApiErrorCategory::Authorization),
            ("CONFLICT", ApiErrorCategory::Conflict),
            ("DURABLE_COMMIT_APPLY_FAILED", ApiErrorCategory::Durability),
            ("ENCODE_ERROR", ApiErrorCategory::Internal),
        ];

        for (legacy_code, expected_category) in cases {
            let query_error = QueryErrorV2::new(legacy_code, "test", "safe_reason");
            let envelope = ApiErrorEnvelopeV1::from_query_error(&query_error);
            assert_eq!(envelope.category, expected_category);
            assert_eq!(envelope.code, expected_category.stable_code());
            assert_eq!(envelope.message, query_error.message);
            assert_eq!(envelope.details, query_error.detail.as_deref().cloned());
        }
    }

    #[test]
    fn query_contract_json_decode_errors_are_validation_envelopes() {
        let error = parse_query_v2_input("{}", "{}").unwrap_err();
        assert_eq!(error.category, ApiErrorCategory::Validation);
        assert_eq!(error.code, "GENESIS_VALIDATION_ERROR");
    }

    #[test]
    fn query_error_result_keeps_stable_and_legacy_codes_separate() {
        let error = QueryErrorV2::new("CAPABILITY_UNSUPPORTED", "bind", "safe_reason");
        let value = query_error_result_value(&error);
        assert_eq!(value["contract_version"], "genesis.api.errors.v1");
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["category"], "capability");
        assert_eq!(value["error"]["code"], "GENESIS_CAPABILITY_UNSUPPORTED");
        assert_eq!(value["code"], "CAPABILITY_UNSUPPORTED");
        assert_eq!(value["details"]["reason"], "safe_reason");
    }

    #[test]
    fn query_success_serializes_direct_plan_payload_without_enum_wrapper() {
        let value = query_success_result_value(QueryOutcomeV2::Plan(ExplainResultV2 {
            contract_version: "genesis.api.v2".into(),
            planner_version: "planner.v2".into(),
            plan_hash: "plan-hash".into(),
            request_id: "plan-request".into(),
            catalog: CatalogStampV2 {
                observed_frontier: 1,
                policy_revision: 0,
                schema_fingerprint: "schema-hash".into(),
            },
            plan: Vec::new(),
            root: "root".into(),
        }));

        assert_eq!(value["contract_version"], API_ERROR_CONTRACT_V1);
        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["request_id"], "plan-request");
        assert_eq!(value["data"]["planner_version"], "planner.v2");
        assert!(value["data"]["Plan"].is_null());
    }

    #[test]
    fn untyped_storage_errors_default_to_internal_without_message_classification() {
        let error =
            ApiErrorEnvelopeV1::from(crate::Error::from_reason("expected frontier conflict"));
        assert_eq!(error.category, ApiErrorCategory::Internal);
        assert_eq!(error.code, "GENESIS_INTERNAL_ERROR");
        assert_eq!(error.message, "expected frontier conflict");
    }
}
