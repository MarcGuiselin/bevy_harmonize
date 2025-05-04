#[link(wasm_import_module = "bevy_harmonize")]
extern "C" {
    #[allow(dead_code)]
    pub fn panic(ptr: u32, len: u32) -> !;
}
