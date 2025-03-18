// cargo build -p my_cube_test --target wasm32-unknown-unknown --release -Z build-std=panic_abort,std
// wasm-bindgen target/wasm32-unknown-unknown/release/my_cube_test.wasm --out-dir ./test
// wasm-opt test/my_cube_test.wasm -Oz --strip-debug --strip-dwarf --strip-producers -o output.wasm

use api::ecs::system::{IntoSystem, System};

#[no_mangle]
pub unsafe extern "C" fn run() {
    let mut sys = IntoSystem::into_system(my_cube::update_frame_count::<54321>);
    sys.run(());
}
