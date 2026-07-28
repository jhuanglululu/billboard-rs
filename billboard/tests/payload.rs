//! Channel-payload contract tests for the *Billboard* side of a payload: the
//! SDK's own `Pod` types, and the [`billboard::payload!`] macro that gives an
//! animation the right derives with the right crate path.
//!
//! The core types (math, the sync handles) carry the same contract and are
//! tested in `wasmachine`; what has to be re-checked here is that they still
//! satisfy it *through this crate's re-exports*, next to Billboard's own types.

use billboard::math::{Position, Ticks};
use billboard::prelude::{Color, Pod, Zeroable};
use billboard::sync::Sender;

fn requires_sync<T: Sync>() {}
fn requires_pod<T: Pod>() {}

#[test]
fn billboard_types_are_channel_payloads() {
    requires_pod::<Color>();
    // Re-exported core types keep the bound they had upstream.
    requires_pod::<Position>();
    requires_pod::<Ticks>();
}

#[test]
fn a_color_round_trips_through_bytes() {
    let c = Color::rgba(1, 2, 3, 4);
    assert_eq!(bytemuck::bytes_of(&c), &[1, 2, 3, 4]);
    assert_eq!(*bytemuck::from_bytes::<Color>(bytemuck::bytes_of(&c)), c);
}

/// The shape an animation author writes: a `#[repr(C)]` struct of SDK types,
/// deriving the prelude's `Pod`/`Zeroable`.
///
/// The `#[bytemuck(crate = …)]` line is load-bearing and easy to forget, which
/// is exactly why [`billboard::payload!`] exists — see the test below. Without
/// it, the derive emits `::bytemuck::…` paths that only resolve inside this
/// crate's own test targets, and every real animation fails to compile.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[bytemuck(crate = "::billboard::bytemuck")]
struct Cue {
    at: Position,
    over: Ticks,
    tint: Color,
    _pad: [u8; 4],
}

billboard::payload! {
    /// What an animation should actually write. `payload!` supplies
    /// `#[repr(C)]`, the derives, and the crate path they need, so a payload
    /// struct needs no bytemuck knowledge and no bytemuck dependency.
    struct Waypoint {
        /// Doc comments and field attributes pass through.
        target: Position,
        over: Ticks,
    }
}

#[test]
fn the_payload_macro_produces_a_usable_channel_payload() {
    requires_pod::<Waypoint>();
    requires_sync::<Sender<Waypoint>>();

    let w = Waypoint {
        target: Position::new(1.0, 2.0, 3.0),
        over: Ticks::new(12),
    };
    // repr(C), so the layout is exactly the fields in order: 24 + 8.
    let bytes = bytemuck::bytes_of(&w);
    assert_eq!(bytes.len(), 32);
    assert_eq!(*bytemuck::from_bytes::<Waypoint>(bytes), w);
    // Derived Copy/Debug/PartialEq come along too.
    let copy = w;
    assert_eq!(copy, w);
    // And Zeroable, for the receive buffer.
    assert_eq!(Waypoint::zeroed().over, Ticks::new(0));
}

#[test]
fn a_user_payload_struct_round_trips() {
    let cue = Cue {
        at: Position::new(0.0, 4.0, -1.5),
        over: Ticks::new(20),
        tint: Color::hex("#ff6b35"),
        _pad: [0; 4],
    };
    let bytes = bytemuck::bytes_of(&cue);
    assert_eq!(bytes.len(), 40); // 24 + 8 + 4 + 4, no padding
    assert_eq!(*bytemuck::from_bytes::<Cue>(bytes), cue);

    // Zeroable, so a receive buffer can start from zeros.
    let zeroed = Cue::zeroed();
    assert_eq!(zeroed.at, Position::ZERO);
    assert_eq!(zeroed.over, Ticks::new(0));
}
