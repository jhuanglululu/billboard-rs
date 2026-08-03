//! Host-target stubs so the SDK's entity/state logic is unit-testable with
//! plain `cargo test`. Anything that would actually cross the boundary
//! panics. Compiled only for non-wasm targets. The engine-owned imports have
//! their own stubs in `wasmachine`.

#![allow(clippy::missing_safety_doc)]

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

// --- ABI v4: player snapshots. Asking who is watching needs a server to ask,
// so these panic like every other crossing; the pure half — building the query
// struct, parsing the blob, the `facing`/`looking_toward` derivations — is
// reached without them and is what the tests cover. ---
pub unsafe fn players_len(_: *const u8) -> i32 {
    unimplemented!("billboard ABI: players_len is wasm-only")
}
pub unsafe fn players_read(_: *const u8, _: *mut u8) {
    unimplemented!("billboard ABI: players_read is wasm-only")
}
pub unsafe fn player_update(_: *const u8, _: usize, _: *mut f64) -> i32 {
    unimplemented!("billboard ABI: player_update is wasm-only")
}
