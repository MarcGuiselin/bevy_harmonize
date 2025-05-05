#[link(wasm_import_module = "bevy_harmonize")]
extern "C" {
    #[allow(dead_code)]
    pub fn panic(ptr: u32, len: u32) -> !;

    pub fn spawn_empty() -> u32;

    pub fn flag_component_changed(component_id: usize);

}
