//! Catalog data is issued under Storage's commit lock, never deserialized.
use super::result::CatalogStampV2;

pub(crate) const LEXICAL_PROFILE_ID: &str = "unicode-whitespace-bm25-v1";
pub(crate) const CONTEXT_TOKENIZER_ID: &str = "unicode-scalar-v1";

pub(crate) fn is_registered_analyzer(id: &str) -> bool {
    id == LEXICAL_PROFILE_ID
}

pub(crate) fn is_registered_tokenizer(id: &str) -> bool {
    id == CONTEXT_TOKENIZER_ID
}

pub(crate) fn text_profile_fingerprints() -> [(&'static str, String); 2] {
    use sha2::{Digest, Sha256};

    [
        (
            "analyzer:unicode-whitespace-bm25-v1",
            hex::encode(Sha256::digest(
                b"genesis.hql2.analyzer.v1:unicode-whitespace:preserve-bytes:no-normalize:no-casefold:no-segmentation:bm25-v1",
            )),
        ),
        (
            "tokenizer:unicode-scalar-v1",
            hex::encode(Sha256::digest(
                b"genesis.hql2.tokenizer.v1:unicode-scalars-v1",
            )),
        ),
    ]
}

pub(crate) struct AuthorizedCatalogV2<'a> {
    pub(crate) stamp: CatalogStampV2,
    tables: std::collections::BTreeSet<String>,
    collections: std::collections::BTreeMap<String, CollectionSpaceV2>,
    // A query-local guard lifetime, not a storage reference or data-open capability.
    _guard_marker: std::marker::PhantomData<&'a ()>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CollectionSpaceV2 {
    pub(crate) name: String,
    pub(crate) space_id: String,
    pub(crate) dimension: u16,
    pub(crate) metric: String,
}

impl<'a> AuthorizedCatalogV2<'a> {
    #[allow(dead_code)] // Retained for standalone scalar-kernel test harnesses.
    pub(crate) fn with_tables<T: ?Sized>(
        stamp: CatalogStampV2,
        guard_marker: &'a T,
        tables: std::collections::BTreeSet<String>,
    ) -> Self {
        Self::with_catalog(
            stamp,
            guard_marker,
            tables,
            std::collections::BTreeMap::new(),
        )
    }

    pub(crate) fn with_catalog<T: ?Sized>(
        stamp: CatalogStampV2,
        _guard_marker: &'a T,
        tables: std::collections::BTreeSet<String>,
        collections: std::collections::BTreeMap<String, CollectionSpaceV2>,
    ) -> Self {
        Self {
            stamp,
            tables,
            collections,
            _guard_marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn has_table(&self, table: &str) -> bool {
        self.tables.contains(table)
    }

    pub(crate) fn collection(&self, name: &str) -> Option<&CollectionSpaceV2> {
        self.collections.get(name)
    }
}
