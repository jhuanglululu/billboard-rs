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

// --- Player snapshots: a query struct out, a blob back. ---

/// The query `players_len`/`players_read` read out of guest memory: 40 packed
/// bytes, built here and pointed at by both calls of the pair.
///
/// `#[repr(C)]` is the whole point — this struct *is* the wire format, so its
/// field order and the absence of padding are pinned by the ABI, not by
/// convenience. Four `f64`s (8-aligned, so the trailing pair of `i32`s fills the
/// last eight bytes exactly) come to 40 bytes with no holes.
///
/// `range` negative means unlimited, `limit` zero or less means unlimited, and
/// `sort` is 0 for distance-ascending or 1 for name-ascending. The origin is in
/// placement-local coordinates, like every other position that crosses.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawQuery {
    pub origin_x: f64,
    pub origin_y: f64,
    pub origin_z: f64,
    pub range: f64,
    pub limit: i32,
    pub sort: i32,
}

// The 40 bytes the host reads. A field reordered or widened breaks the wire, so
// say so at compile time rather than in a comment.
const _: () = assert!(core::mem::size_of::<RawQuery>() == 40);
const _: () = assert!(core::mem::align_of::<RawQuery>() == 8);

/// Run the query and hand back the raw snapshot blob for `players` to parse.
///
/// Both calls get the *same* query pointer, and nothing parks between them, so
/// the length and the bytes describe one consistent list.
pub fn players(query: &RawQuery) -> Vec<u8> {
    let ptr = (query as *const RawQuery).cast::<u8>();
    let len = unsafe { sys::players_len(ptr) };
    read_blob(len, "player snapshot", |buf| unsafe {
        sys::players_read(ptr, buf)
    })
}

/// Refresh one player by name: `Some` with `x, y, z, eye_height, yaw, pitch` if
/// they are still a viewer, `None` if they are not (the host leaves `out`
/// untouched, so nothing here reads uninitialised data — the buffer is zeroed
/// and simply discarded).
pub fn player_update(name: &str) -> Option<[f64; 6]> {
    let mut out = [0.0f64; 6];
    let found = unsafe { sys::player_update(name.as_ptr(), name.len(), out.as_mut_ptr()) };
    (found != 0).then_some(out)
}

/// [`read_string`]'s protocol for a binary payload: ask for the length, skip the
/// second call when there is nothing to read, and treat a negative length as the
/// host contradicting the ABI.
fn read_blob(len: i32, what: &str, fill: impl FnOnce(*mut u8)) -> Vec<u8> {
    match len {
        0 => Vec::new(),
        n if n > 0 => {
            let mut buf = vec![0u8; n as usize];
            fill(buf.as_mut_ptr());
            buf
        }
        n => panic!("host returned a negative {what} length: {n}"),
    }
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
