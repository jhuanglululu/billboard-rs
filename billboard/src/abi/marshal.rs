//! The pointer boundary for the plugin half: every `(ptr, len)` argument and
//! every out-pointer the entity/effect imports take, wrapped so that **no module
//! outside `abi` ever writes a pointer expression**.
//!
//! Callers hand over `&str`, or take back a `String`/`[f64; N]`; the raw
//! addresses are formed and consumed here. The imports themselves live in
//! `abi::sys` (`wasm.rs` on wasm, `stubs.rs` on the host target); this layer sits
//! directly on top of them and nowhere else.
//!
//! Two-call reads (`*_len`, then fill a buffer) are race-free because nothing
//! here parks, so no other task can run between the calls. The protocol itself —
//! including the **zero length returns early** rule, since an empty `Vec`'s
//! `as_mut_ptr()` is a dangling (though aligned) address the host must not
//! dereference — is [`wasmachine`'s](wasmachine::abi::marshal::read_string), so
//! both halves of the wire ABI read strings the same way.

use wasmachine::abi::marshal::read_string;

use super::sys;

// --- Two-call string reads. ---

pub fn get_block(entity: i32) -> String {
    let len = unsafe { sys::get_block_len(entity) };
    read_string(len, "block state", |buf| unsafe {
        sys::get_block(entity, buf)
    })
}

pub fn get_item(entity: i32) -> String {
    let len = unsafe { sys::get_item_len(entity) };
    read_string(len, "item", |buf| unsafe { sys::get_item(entity, buf) })
}

pub fn get_text(entity: i32) -> String {
    let len = unsafe { sys::get_text_len(entity) };
    read_string(len, "text", |buf| unsafe { sys::get_text(entity, buf) })
}

// --- Out-pointer reads. ---

pub fn get_position(entity: i32) -> [f64; 3] {
    let mut out = [0.0f64; 3];
    unsafe { sys::get_position(entity, out.as_mut_ptr()) }
    out
}

pub fn get_rotation(entity: i32) -> [f64; 4] {
    let mut out = [0.0f64; 4];
    unsafe { sys::get_rotation(entity, out.as_mut_ptr()) }
    out
}

pub fn get_scale(entity: i32) -> [f64; 3] {
    let mut out = [0.0f64; 3];
    unsafe { sys::get_scale(entity, out.as_mut_ptr()) }
    out
}

pub fn get_pose(entity: i32, part: i32) -> [f64; 3] {
    let mut out = [0.0f64; 3];
    unsafe { sys::get_pose(entity, part, out.as_mut_ptr()) }
    out
}

// --- String writes. ---

pub fn set_block(entity: i32, state: &str) {
    unsafe { sys::set_block(entity, state.as_ptr(), state.len()) }
}

pub fn set_item(entity: i32, item: &str) {
    unsafe { sys::set_item(entity, item.as_ptr(), item.len()) }
}

pub fn set_text(entity: i32, text: &str) {
    unsafe { sys::set_text(entity, text.as_ptr(), text.len()) }
}

pub fn set_equipment(entity: i32, slot: i32, item: &str) {
    unsafe { sys::set_equipment(entity, slot, item.as_ptr(), item.len()) }
}

// --- Spawns that carry a string. ---

pub fn spawn_block_display(state: &str, x: f64, y: f64, z: f64) -> i32 {
    unsafe { sys::spawn_block_display(state.as_ptr(), state.len(), x, y, z) }
}

pub fn spawn_item_display(item: &str, x: f64, y: f64, z: f64) -> i32 {
    unsafe { sys::spawn_item_display(item.as_ptr(), item.len(), x, y, z) }
}

pub fn spawn_text_display(text: &str, x: f64, y: f64, z: f64) -> i32 {
    unsafe { sys::spawn_text_display(text.as_ptr(), text.len(), x, y, z) }
}

pub fn spawn_item(item: &str, x: f64, y: f64, z: f64) -> i32 {
    unsafe { sys::spawn_item(item.as_ptr(), item.len(), x, y, z) }
}

// --- Sound and particles. ---

pub fn play_sound(id: &str, x: f64, y: f64, z: f64, category: i32, volume: f64, pitch: f64) {
    unsafe { sys::play_sound(id.as_ptr(), id.len(), x, y, z, category, volume, pitch) }
}

/// The position/count/offset/speed arguments every particle emission shares.
#[derive(Clone, Copy, Debug)]
pub struct ParticleArgs {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub count: i32,
    pub ox: f64,
    pub oy: f64,
    pub oz: f64,
    pub speed: f64,
}

pub fn emit_particle(name: &str, a: ParticleArgs) {
    unsafe {
        sys::emit_particle(
            name.as_ptr(),
            name.len(),
            a.x,
            a.y,
            a.z,
            a.count,
            a.ox,
            a.oy,
            a.oz,
            a.speed,
        )
    }
}

pub fn emit_particle_dust(rgb: (f64, f64, f64), size: f64, a: ParticleArgs) {
    unsafe {
        sys::emit_particle_dust(
            rgb.0, rgb.1, rgb.2, size, a.x, a.y, a.z, a.count, a.ox, a.oy, a.oz, a.speed,
        )
    }
}

pub fn emit_particle_dust_transition(
    from: (f64, f64, f64),
    to: (f64, f64, f64),
    size: f64,
    a: ParticleArgs,
) {
    unsafe {
        sys::emit_particle_dust_transition(
            from.0, from.1, from.2, to.0, to.1, to.2, size, a.x, a.y, a.z, a.count, a.ox, a.oy,
            a.oz, a.speed,
        )
    }
}

pub fn emit_particle_block(state: &str, a: ParticleArgs) {
    unsafe {
        sys::emit_particle_block(
            state.as_ptr(),
            state.len(),
            a.x,
            a.y,
            a.z,
            a.count,
            a.ox,
            a.oy,
            a.oz,
            a.speed,
        )
    }
}

pub fn emit_particle_item(item: &str, a: ParticleArgs) {
    unsafe {
        sys::emit_particle_item(
            item.as_ptr(),
            item.len(),
            a.x,
            a.y,
            a.z,
            a.count,
            a.ox,
            a.oy,
            a.oz,
            a.speed,
        )
    }
}
