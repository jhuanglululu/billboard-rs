//! Every entity's ABI plumbing, in one place.
//!
//! This is the only module in `entity` that touches pointers or `unsafe`; the
//! entity types above it are plain safe Rust that pass an `i32` id and typed
//! values down here. Keeping it shared is what makes "the transform ABI
//! applies to every entity id" real rather than copied five times.
//!
//! Two-call string reads (`*_len` then fill) are race-free because no blocking
//! point can sit between the two calls — nothing here parks.

use super::{BillboardMode, BlockState, DisplayContext, ItemStr, PosePart, StandFlags, TextFlags};
use crate::abi;
use crate::abi::marshal;
use crate::helpers::Color;
use crate::math::{Degrees, Position, Rotation, Scale, Ticks};

/// Narrow an interpolation duration to the ABI's `i64`; overflow kills the
/// animation rather than wrapping.
pub fn over_ticks(over: Ticks) -> i64 {
    i64::try_from(over.count()).expect("interpolation duration overflows i64")
}

/// Host-truth liveness for a weak reference.
pub fn alive(id: i32) -> bool {
    unsafe { abi::is_alive(id) != 0 }
}

pub fn despawn(id: i32) {
    unsafe { abi::despawn(id) }
}

// --- Shared transform: valid for every entity id. For entities without
// client-side interpolation (armor stands, items) the host tweens instead. ---

pub fn set_position(id: i32, p: &Position, over: Ticks) {
    unsafe { abi::set_position(id, p.x, p.y, p.z, over_ticks(over)) }
}

pub fn set_rotation(id: i32, r: &Rotation, over: Ticks) {
    unsafe { abi::set_rotation(id, r.x, r.y, r.z, r.w, over_ticks(over)) }
}

pub fn set_scale(id: i32, s: &Scale, over: Ticks) {
    unsafe { abi::set_scale(id, s.x, s.y, s.z, over_ticks(over)) }
}

pub fn get_position(id: i32) -> Position {
    let out = marshal::get_position(id);
    Position::new(out[0], out[1], out[2])
}

pub fn get_rotation(id: i32) -> Rotation {
    let out = marshal::get_rotation(id);
    Rotation {
        x: out[0],
        y: out[1],
        z: out[2],
        w: out[3],
    }
}

pub fn get_scale(id: i32) -> Scale {
    let out = marshal::get_scale(id);
    Scale::new(out[0], out[1], out[2])
}

// --- Strings: block state, item, text. Same two-call read protocol. ---

pub fn set_block(id: i32, block: &BlockState) {
    marshal::set_block(id, block.as_str());
}

pub fn get_block(id: i32) -> BlockState {
    BlockState::new(marshal::get_block(id))
}

pub fn set_item(id: i32, item: &ItemStr) {
    marshal::set_item(id, item.as_str());
}

pub fn get_item(id: i32) -> ItemStr {
    ItemStr::new(marshal::get_item(id))
}

pub fn set_text(id: i32, text: &str) {
    marshal::set_text(id, text);
}

pub fn get_text(id: i32) -> String {
    marshal::get_text(id)
}

// --- Display attributes. ---

pub fn set_billboard_mode(id: i32, mode: BillboardMode) {
    unsafe { abi::set_billboard_mode(id, mode.wire()) }
}

pub fn get_billboard_mode(id: i32) -> BillboardMode {
    BillboardMode::from_wire(unsafe { abi::get_billboard_mode(id) })
}

pub fn set_display_context(id: i32, ctx: DisplayContext) {
    unsafe { abi::set_display_context(id, ctx.wire()) }
}

pub fn get_display_context(id: i32) -> DisplayContext {
    DisplayContext::from_wire(unsafe { abi::get_display_context(id) })
}

// --- Text display attributes. ---

pub fn set_text_background(id: i32, color: Color) {
    unsafe { abi::set_text_background(id, color.to_argb_i64()) }
}

pub fn get_text_background(id: i32) -> Color {
    Color::from_argb_i64(unsafe { abi::get_text_background(id) })
}

pub fn set_text_opacity(id: i32, opacity: u8) {
    unsafe { abi::set_text_opacity(id, i64::from(opacity)) }
}

pub fn get_text_opacity(id: i32) -> u8 {
    let raw = unsafe { abi::get_text_opacity(id) };
    u8::try_from(raw)
        .unwrap_or_else(|_| panic!("host returned a text opacity outside 0..=255: {raw}"))
}

pub fn set_line_width(id: i32, width: u32) {
    unsafe { abi::set_line_width(id, i64::from(width)) }
}

pub fn get_line_width(id: i32) -> u32 {
    let raw = unsafe { abi::get_line_width(id) };
    u32::try_from(raw).unwrap_or_else(|_| panic!("host returned a negative line width: {raw}"))
}

pub fn set_text_flags(id: i32, flags: TextFlags) {
    unsafe { abi::set_text_flags(id, flags.bits()) }
}

pub fn get_text_flags(id: i32) -> TextFlags {
    TextFlags::from_bits(unsafe { abi::get_text_flags(id) })
}

// --- Armor stand attributes. Poses and yaw take `over_ticks` like any
// transform; the host tweens them with per-tick packets. ---

pub fn set_pose(id: i32, part: PosePart, angles: (Degrees, Degrees, Degrees), over: Ticks) {
    unsafe {
        abi::set_pose(
            id,
            part.wire(),
            angles.0.value(),
            angles.1.value(),
            angles.2.value(),
            over_ticks(over),
        )
    }
}

pub fn get_pose(id: i32, part: PosePart) -> (Degrees, Degrees, Degrees) {
    let out = marshal::get_pose(id, part.wire());
    (
        Degrees::new(out[0]),
        Degrees::new(out[1]),
        Degrees::new(out[2]),
    )
}

pub fn set_equipment(id: i32, slot: i32, item: &ItemStr) {
    marshal::set_equipment(id, slot, item.as_str());
}

pub fn set_stand_flags(id: i32, flags: StandFlags) {
    unsafe { abi::set_stand_flags(id, flags.bits()) }
}

pub fn get_stand_flags(id: i32) -> StandFlags {
    StandFlags::from_bits(unsafe { abi::get_stand_flags(id) })
}

pub fn set_yaw(id: i32, yaw: Degrees, over: Ticks) {
    unsafe { abi::set_yaw(id, yaw.value(), over_ticks(over)) }
}

pub fn get_yaw(id: i32) -> Degrees {
    Degrees::new(unsafe { abi::get_yaw(id) })
}

// --- Spawns. ---

pub fn spawn_block_display(block: &BlockState, p: &Position) -> i32 {
    marshal::spawn_block_display(block.as_str(), p.x, p.y, p.z)
}

pub fn spawn_item_display(item: &ItemStr, p: &Position) -> i32 {
    marshal::spawn_item_display(item.as_str(), p.x, p.y, p.z)
}

pub fn spawn_text_display(text: &str, p: &Position) -> i32 {
    marshal::spawn_text_display(text, p.x, p.y, p.z)
}

pub fn spawn_armor_stand(p: &Position) -> i32 {
    unsafe { abi::spawn_armor_stand(p.x, p.y, p.z) }
}

pub fn spawn_item(item: &ItemStr, p: &Position) -> i32 {
    marshal::spawn_item(item.as_str(), p.x, p.y, p.z)
}
