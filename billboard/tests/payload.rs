//! Channel-payload contract tests: the SDK types that claim to be `Pod` must
//! actually survive a byte round trip, and the sync handles must be `Sync`
//! (capturable by `spawn`) with the receiver deliberately not clonable.
//!
//! The host stubs panic for anything that crosses the ABI, so these tests
//! check the *type-level* contract — which is exactly what the ABI cannot
//! check for us.

use billboard::math::{Degrees, Offset, Position, Radians, Rotation, Scale, Ticks, Vector3i};
use billboard::prelude::{Color, Pod, Zeroable};
use billboard::sync::{Barrier, Composite, Receiver, Sender, Signal};

fn requires_sync<T: Sync>() {}
fn requires_send<T: Send>() {}
fn requires_clone<T: Clone>() {}
fn requires_pod<T: Pod>() {}

#[test]
fn sync_handles_can_cross_into_a_spawned_task() {
    // `spawn`'s closure is `Sync + 'static`: every sync handle must satisfy
    // it, whatever the payload type is.
    requires_sync::<Signal>();
    requires_sync::<Barrier>();
    requires_sync::<Composite>();
    requires_sync::<Sender<Position>>();
    requires_sync::<Receiver<Position>>();
    requires_send::<Receiver<Position>>();

    // Senders clone (one per producer); the receiver deliberately does not,
    // which is what keeps the channel single-consumer.
    requires_clone::<Signal>();
    requires_clone::<Barrier>();
    requires_clone::<Sender<Position>>();
}

#[test]
fn math_types_are_channel_payloads() {
    requires_pod::<Position>();
    requires_pod::<Offset>();
    requires_pod::<Scale>();
    requires_pod::<Vector3i>();
    requires_pod::<Rotation>();
    requires_pod::<Ticks>();
    requires_pod::<Degrees>();
    requires_pod::<Radians>();
    requires_pod::<Color>();
}

#[test]
fn math_types_round_trip_through_bytes() {
    let p = Position::new(1.5, -2.25, 3.0);
    let bytes = bytemuck::bytes_of(&p);
    assert_eq!(bytes.len(), 24); // three f64, no padding
    assert_eq!(*bytemuck::from_bytes::<Position>(bytes), p);

    let v = Vector3i::new(-1, 2, i64::MAX);
    assert_eq!(*bytemuck::from_bytes::<Vector3i>(bytemuck::bytes_of(&v)), v);

    let r = Rotation::axis_angle(billboard::math::Vector3d::Y, Degrees::new(90.0));
    let bytes = bytemuck::bytes_of(&r);
    assert_eq!(bytes.len(), 32); // four f64
    assert_eq!(*bytemuck::from_bytes::<Rotation>(bytes), r);

    let t = Ticks::new(40);
    assert_eq!(bytemuck::bytes_of(&t).len(), 8);
    assert_eq!(*bytemuck::from_bytes::<Ticks>(bytemuck::bytes_of(&t)), t);

    let c = Color::rgba(1, 2, 3, 4);
    assert_eq!(bytemuck::bytes_of(&c), &[1, 2, 3, 4]);
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
