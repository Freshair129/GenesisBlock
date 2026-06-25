// napi_build::setup() only matters for the cdylib (Node addon) link. When the
// napi-bindings feature is off (REST server / Linux integration-test build),
// skip it so the build pulls in no napi tooling at all.
#[cfg(feature = "napi-bindings")]
extern crate napi_build;

fn main() {
    #[cfg(feature = "napi-bindings")]
    napi_build::setup();
}
