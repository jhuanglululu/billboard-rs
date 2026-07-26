//! The pointer boundary: every `(ptr, len)` argument and every out-pointer the
//! guest ABI takes, wrapped so that **no module outside `abi` ever writes a
//! pointer expression**.
//!
//! Callers hand over `&str`, `&[u8]`, or take back a `String`/`[f64; N]`; the
//! raw addresses are formed and consumed here. The imports themselves live in
//! `abi::sys` (`wasm.rs` on wasm, `stubs.rs` on the host target); this layer sits
//! directly on top of them and nowhere else.
//!
//! Two-call reads (`*_len`, then fill a buffer) are race-free because nothing
//! here parks, so no other task can run between the calls. A **zero length
//! returns early** without a second call: an empty `Vec`'s `as_mut_ptr()` is a
//! dangling (though aligned) address, and there is no reason to hand the host a
//! pointer it must not dereference.

use super::sys;

// --- Diagnostics. ---

pub fn log(msg: &str) {
    unsafe { sys::log(msg.as_ptr(), msg.len()) }
}

/// Kill the animation with a message. Never returns.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn fail(msg: &str) -> ! {
    unsafe { sys::fail(msg.as_ptr(), msg.len()) }
}

// --- Two-call string reads. ---

/// How many bytes a two-call read needs, or `None` when there is nothing to
/// read and the second host call must be skipped entirely.
///
/// A negative length is the host contradicting the ABI, which is a kill.
fn read_len(len: i32, what: &str) -> Option<usize> {
    match len {
        0 => None,
        n if n > 0 => Some(n as usize),
        n => panic!("host returned a negative {what} length: {n}"),
    }
}

/// Run the two-call protocol: ask for the length, then let `fill` write exactly
/// that many bytes into a fresh buffer.
fn read_string(len: i32, what: &str, fill: impl FnOnce(*mut u8)) -> String {
    let Some(len) = read_len(len, what) else {
        return String::new();
    };
    let mut buf = vec![0u8; len];
    fill(buf.as_mut_ptr());
    String::from_utf8(buf).unwrap_or_else(|_| panic!("host returned a non-UTF-8 {what}"))
}

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

// --- Channels: payload bytes in and out. ---

pub fn channel_send(id: i32, bytes: &[u8]) {
    unsafe { sys::channel_send(id, bytes.as_ptr(), bytes.len()) }
}

pub fn channel_recv(id: i32, buf: &mut [u8]) {
    unsafe { sys::channel_recv(id, buf.as_mut_ptr()) }
}

pub fn channel_peek(id: i32, buf: &mut [u8]) {
    unsafe { sys::channel_peek(id, buf.as_mut_ptr()) }
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

// --- The allocator's one import. Only the global allocator calls this, and it
// is inherently pointer-shaped, so it stays raw — but it stays *here*. ---

#[cfg(target_arch = "wasm32")]
pub unsafe fn realloc(ptr: *mut u8, old_size: usize, align: usize, new_size: usize) -> *mut u8 {
    unsafe { sys::realloc(ptr, old_size, align, new_size) }
}

#[cfg(test)]
mod tests {
    use super::read_len;

    /// A zero-length read must report "nothing to read" so the caller skips the
    /// second host call: handing the host an empty `Vec`'s dangling pointer is
    /// pointless at best.
    #[test]
    fn a_zero_length_read_skips_the_second_call() {
        assert_eq!(read_len(0, "text"), None);
    }

    #[test]
    fn a_positive_length_asks_for_that_many_bytes() {
        assert_eq!(read_len(5, "text"), Some(5));
        assert_eq!(read_len(i32::MAX, "text"), Some(i32::MAX as usize));
    }

    #[test]
    #[should_panic(expected = "negative text length: -1")]
    fn a_negative_length_is_the_host_breaking_the_abi() {
        let _ = read_len(-1, "text");
    }
}
