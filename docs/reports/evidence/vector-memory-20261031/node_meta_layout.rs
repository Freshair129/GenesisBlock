#[no_mangle]
pub extern "C" fn node_metadata_size() -> usize {
    std::mem::size_of::<genesis_block_native::NodeMetadata>()
}
