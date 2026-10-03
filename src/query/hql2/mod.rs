//! Explicit v2 frontend and execution boundary.
//! Storage-backed scans are enabled only for revision-bound sources covered by P8.
pub mod ast;
pub(crate) mod bind;
pub(crate) mod catalog;
pub mod error;
pub(crate) mod exec;
pub(crate) mod key_codec;
pub(crate) mod legacy;
pub(crate) mod lower;
pub(crate) mod plan;
pub(crate) mod request;
pub mod result;
pub(crate) mod source;
mod syntax;
pub mod value;
pub(crate) mod wire;

pub use error::QueryErrorV2;
pub use result::{ExplainResultV2, QueryOutcomeV2, QueryResultV2};
pub use syntax::parse_hql2;
pub(crate) use syntax::{required_heap_bytes, required_stack_bytes};
