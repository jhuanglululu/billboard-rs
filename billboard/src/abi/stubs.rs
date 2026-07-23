//! Host-target stubs so the SDK's math/state logic is unit-testable with
//! plain `cargo test`. Anything that would actually cross the boundary
//! panics. Compiled only for non-wasm targets.

// realloc/fail are referenced only from wasm-gated code (allocator, panic
// hook), so they're dead on the host target by design.
#![allow(dead_code, clippy::missing_safety_doc)]

pub unsafe fn realloc(_: *mut u8, _: usize, _: usize, _: usize) -> *mut u8 {
    unreachable!("billboard ABI called outside wasm")
}
pub unsafe fn fork() -> i32 {
    unimplemented!("billboard ABI: fork is wasm-only")
}
pub unsafe fn join(_: i32) {
    unimplemented!("billboard ABI: join is wasm-only")
}
pub unsafe fn kill(_: i32) {
    unimplemented!("billboard ABI: kill is wasm-only")
}
pub unsafe fn exit() -> ! {
    unimplemented!("billboard ABI: exit is wasm-only")
}
pub unsafe fn sleep(_: i64) {
    unimplemented!("billboard ABI: sleep is wasm-only")
}
pub unsafe fn spawn_block_display(_: *const u8, _: usize, _: f64, _: f64, _: f64) -> i32 {
    unimplemented!("billboard ABI: spawn is wasm-only")
}
pub unsafe fn set_position(_: i32, _: f64, _: f64, _: f64, _: i64) {
    unimplemented!("billboard ABI: set_position is wasm-only")
}
pub unsafe fn set_rotation(_: i32, _: f64, _: f64, _: f64, _: f64, _: i64) {
    unimplemented!("billboard ABI: set_rotation is wasm-only")
}
pub unsafe fn set_scale(_: i32, _: f64, _: f64, _: f64, _: i64) {
    unimplemented!("billboard ABI: set_scale is wasm-only")
}
pub unsafe fn set_block(_: i32, _: *const u8, _: usize) {
    unimplemented!("billboard ABI: set_block is wasm-only")
}
pub unsafe fn get_position(_: i32, _: *mut f64) {
    unimplemented!("billboard ABI: get_position is wasm-only")
}
pub unsafe fn get_rotation(_: i32, _: *mut f64) {
    unimplemented!("billboard ABI: get_rotation is wasm-only")
}
pub unsafe fn get_scale(_: i32, _: *mut f64) {
    unimplemented!("billboard ABI: get_scale is wasm-only")
}
pub unsafe fn get_block_len(_: i32) -> i32 {
    unimplemented!("billboard ABI: get_block_len is wasm-only")
}
pub unsafe fn get_block(_: i32, _: *mut u8) {
    unimplemented!("billboard ABI: get_block is wasm-only")
}
pub unsafe fn despawn(_: i32) {
    unimplemented!("billboard ABI: despawn is wasm-only")
}
pub unsafe fn is_alive(_: i32) -> i32 {
    unimplemented!("billboard ABI: is_alive is wasm-only")
}
pub unsafe fn log(_: *const u8, _: usize) {
    unimplemented!("billboard ABI: log is wasm-only")
}
pub unsafe fn fail(_: *const u8, _: usize) -> ! {
    panic!("billboard ABI: fail called outside wasm")
}
