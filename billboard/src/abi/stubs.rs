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

// --- ABI v2: sync primitives. ---
pub unsafe fn signal_new() -> i32 {
    unimplemented!("billboard ABI: signal_new is wasm-only")
}
pub unsafe fn signal_notify(_: i32, _: i32) {
    unimplemented!("billboard ABI: signal_notify is wasm-only")
}
pub unsafe fn barrier_new(_: i32) -> i32 {
    unimplemented!("billboard ABI: barrier_new is wasm-only")
}
pub unsafe fn wait_all(_: i32, _: i32) -> i32 {
    unimplemented!("billboard ABI: wait_all is wasm-only")
}
pub unsafe fn wait_any(_: i32, _: i32) -> i32 {
    unimplemented!("billboard ABI: wait_any is wasm-only")
}
pub unsafe fn wait(_: i32) {
    unimplemented!("billboard ABI: wait is wasm-only")
}
pub unsafe fn channel_new(_: i32) -> i32 {
    unimplemented!("billboard ABI: channel_new is wasm-only")
}
pub unsafe fn channel_send(_: i32, _: *const u8, _: usize) {
    unimplemented!("billboard ABI: channel_send is wasm-only")
}
pub unsafe fn channel_recv_len(_: i32) -> i32 {
    unimplemented!("billboard ABI: channel_recv_len is wasm-only")
}
pub unsafe fn channel_recv(_: i32, _: *mut u8) {
    unimplemented!("billboard ABI: channel_recv is wasm-only")
}
pub unsafe fn channel_peek_len(_: i32) -> i32 {
    unimplemented!("billboard ABI: channel_peek_len is wasm-only")
}
pub unsafe fn channel_peek(_: i32, _: *mut u8) {
    unimplemented!("billboard ABI: channel_peek is wasm-only")
}
pub unsafe fn channel_try_len(_: i32) -> i32 {
    unimplemented!("billboard ABI: channel_try_len is wasm-only")
}
pub unsafe fn channel_clear(_: i32) {
    unimplemented!("billboard ABI: channel_clear is wasm-only")
}

// --- ABI v2: randomness. `SplitRng` is pure guest Rust and needs none of
// these, so the pure random logic stays testable on the host. ---
pub unsafe fn random_nondet() -> i64 {
    unimplemented!("billboard ABI: random_nondet is wasm-only")
}
pub unsafe fn random_det() -> i64 {
    unimplemented!("billboard ABI: random_det is wasm-only")
}
pub unsafe fn seed_random(_: i64) {
    unimplemented!("billboard ABI: seed_random is wasm-only")
}

// --- ABI v2: new entity kinds. ---
pub unsafe fn spawn_item_display(_: *const u8, _: usize, _: f64, _: f64, _: f64) -> i32 {
    unimplemented!("billboard ABI: spawn_item_display is wasm-only")
}
pub unsafe fn spawn_text_display(_: *const u8, _: usize, _: f64, _: f64, _: f64) -> i32 {
    unimplemented!("billboard ABI: spawn_text_display is wasm-only")
}
pub unsafe fn spawn_armor_stand(_: f64, _: f64, _: f64) -> i32 {
    unimplemented!("billboard ABI: spawn_armor_stand is wasm-only")
}
pub unsafe fn spawn_item(_: *const u8, _: usize, _: f64, _: f64, _: f64) -> i32 {
    unimplemented!("billboard ABI: spawn_item is wasm-only")
}
pub unsafe fn set_item(_: i32, _: *const u8, _: usize) {
    unimplemented!("billboard ABI: set_item is wasm-only")
}
pub unsafe fn get_item_len(_: i32) -> i32 {
    unimplemented!("billboard ABI: get_item_len is wasm-only")
}
pub unsafe fn get_item(_: i32, _: *mut u8) {
    unimplemented!("billboard ABI: get_item is wasm-only")
}
pub unsafe fn set_display_context(_: i32, _: i32) {
    unimplemented!("billboard ABI: set_display_context is wasm-only")
}
pub unsafe fn get_display_context(_: i32) -> i32 {
    unimplemented!("billboard ABI: get_display_context is wasm-only")
}
pub unsafe fn set_billboard_mode(_: i32, _: i32) {
    unimplemented!("billboard ABI: set_billboard_mode is wasm-only")
}
pub unsafe fn get_billboard_mode(_: i32) -> i32 {
    unimplemented!("billboard ABI: get_billboard_mode is wasm-only")
}
pub unsafe fn set_text(_: i32, _: *const u8, _: usize) {
    unimplemented!("billboard ABI: set_text is wasm-only")
}
pub unsafe fn get_text_len(_: i32) -> i32 {
    unimplemented!("billboard ABI: get_text_len is wasm-only")
}
pub unsafe fn get_text(_: i32, _: *mut u8) {
    unimplemented!("billboard ABI: get_text is wasm-only")
}
pub unsafe fn set_text_background(_: i32, _: i64) {
    unimplemented!("billboard ABI: set_text_background is wasm-only")
}
pub unsafe fn get_text_background(_: i32) -> i64 {
    unimplemented!("billboard ABI: get_text_background is wasm-only")
}
pub unsafe fn set_text_opacity(_: i32, _: i64) {
    unimplemented!("billboard ABI: set_text_opacity is wasm-only")
}
pub unsafe fn get_text_opacity(_: i32) -> i64 {
    unimplemented!("billboard ABI: get_text_opacity is wasm-only")
}
pub unsafe fn set_line_width(_: i32, _: i64) {
    unimplemented!("billboard ABI: set_line_width is wasm-only")
}
pub unsafe fn get_line_width(_: i32) -> i64 {
    unimplemented!("billboard ABI: get_line_width is wasm-only")
}
pub unsafe fn set_text_flags(_: i32, _: i32) {
    unimplemented!("billboard ABI: set_text_flags is wasm-only")
}
pub unsafe fn get_text_flags(_: i32) -> i32 {
    unimplemented!("billboard ABI: get_text_flags is wasm-only")
}
pub unsafe fn set_pose(_: i32, _: i32, _: f64, _: f64, _: f64, _: i64) {
    unimplemented!("billboard ABI: set_pose is wasm-only")
}
pub unsafe fn get_pose(_: i32, _: i32, _: *mut f64) {
    unimplemented!("billboard ABI: get_pose is wasm-only")
}
pub unsafe fn set_equipment(_: i32, _: i32, _: *const u8, _: usize) {
    unimplemented!("billboard ABI: set_equipment is wasm-only")
}
pub unsafe fn set_stand_flags(_: i32, _: i32) {
    unimplemented!("billboard ABI: set_stand_flags is wasm-only")
}
pub unsafe fn get_stand_flags(_: i32) -> i32 {
    unimplemented!("billboard ABI: get_stand_flags is wasm-only")
}
pub unsafe fn set_yaw(_: i32, _: f64, _: i64) {
    unimplemented!("billboard ABI: set_yaw is wasm-only")
}
pub unsafe fn get_yaw(_: i32) -> f64 {
    unimplemented!("billboard ABI: get_yaw is wasm-only")
}

// --- ABI v2: sound & particles. ---
#[allow(clippy::too_many_arguments)]
pub unsafe fn play_sound(_: *const u8, _: usize, _: f64, _: f64, _: f64, _: i32, _: f64, _: f64) {
    unimplemented!("billboard ABI: play_sound is wasm-only")
}
#[allow(clippy::too_many_arguments)]
pub unsafe fn emit_particle(
    _: *const u8,
    _: usize,
    _: f64,
    _: f64,
    _: f64,
    _: i32,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
) {
    unimplemented!("billboard ABI: emit_particle is wasm-only")
}
#[allow(clippy::too_many_arguments)]
pub unsafe fn emit_particle_dust(
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: i32,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
) {
    unimplemented!("billboard ABI: emit_particle_dust is wasm-only")
}
#[allow(clippy::too_many_arguments)]
pub unsafe fn emit_particle_dust_transition(
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
    _: i32,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
) {
    unimplemented!("billboard ABI: emit_particle_dust_transition is wasm-only")
}
#[allow(clippy::too_many_arguments)]
pub unsafe fn emit_particle_block(
    _: *const u8,
    _: usize,
    _: f64,
    _: f64,
    _: f64,
    _: i32,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
) {
    unimplemented!("billboard ABI: emit_particle_block is wasm-only")
}
#[allow(clippy::too_many_arguments)]
pub unsafe fn emit_particle_item(
    _: *const u8,
    _: usize,
    _: f64,
    _: f64,
    _: f64,
    _: i32,
    _: f64,
    _: f64,
    _: f64,
    _: f64,
) {
    unimplemented!("billboard ABI: emit_particle_item is wasm-only")
}
