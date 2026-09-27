//! Catalog data is issued under Storage's commit lock, never deserialized.
use super::result::CatalogStampV2;

pub(crate) struct AuthorizedCatalogV2<'a> {
    pub(crate) stamp: CatalogStampV2,
    // A query-local guard lifetime, not a storage reference or data-open capability.
    _guard_marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> AuthorizedCatalogV2<'a> {
    pub(crate) fn new<T: ?Sized>(stamp: CatalogStampV2, _guard_marker: &'a T) -> Self {
        Self {
            stamp,
            _guard_marker: std::marker::PhantomData,
        }
    }
}
