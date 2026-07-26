//! Known-answer tests for [`Timeline`]'s sub-step schedule and the state
//! blending it relies on.
//!
//! `steps()` is the exact sequence `play()` executes, and it is pure — so the
//! whole schedule is checkable without a host. Every chunk count and eased value
//! below is hand-computed:
//!
//! - chunks = ⌈duration ÷ sub_step⌉, lengths `base = duration ÷ chunks` with the
//!   remainder handed to the leading chunks, so they sum to the duration exactly;
//! - `CubicInOut(t) = 4t³` below the halfway point and `1 − (2−2t)³ ÷ 2` above it.

use billboard::helpers::{DEFAULT_SUB_STEP, Ease, Timeline};
use billboard::math::{Position, Ticks};

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn from() -> Position {
    Position::ZERO
}

fn to() -> Position {
    Position::new(10.0, 0.0, 0.0)
}

/// The x coordinate of each step, and its duration in ticks.
fn schedule(timeline: &Timeline<Position>) -> Vec<(f64, u64)> {
    timeline
        .steps()
        .into_iter()
        .map(|step| (step.state.x, step.over.count()))
        .collect()
}

#[test]
fn the_default_sub_step_is_two_ticks() {
    assert_eq!(DEFAULT_SUB_STEP, Ticks::new(2));
}

#[test]
fn an_empty_timeline_does_nothing() {
    let timeline: Timeline<Position> = Timeline::new();
    assert!(timeline.steps().is_empty());
    assert_eq!(timeline.duration(), Ticks::new(0));
}

#[test]
fn a_single_key_is_one_instant_snap() {
    let timeline = Timeline::new().key(Ticks::new(0), to());
    assert_eq!(schedule(&timeline), vec![(10.0, 0)]);
    assert_eq!(timeline.duration(), Ticks::new(0));
}

#[test]
fn a_linear_segment_is_one_packet_because_the_client_lerps_linearly() {
    let timeline = Timeline::new()
        .key(Ticks::new(0), from())
        .key(Ticks::new(30), to());
    // Opening snap, then the whole 30 ticks in one animate.
    assert_eq!(schedule(&timeline), vec![(0.0, 0), (10.0, 30)]);
    assert_eq!(timeline.duration(), Ticks::new(30));
}

#[test]
fn an_eased_segment_is_chopped_into_sub_steps() {
    let timeline = Timeline::new()
        .key(Ticks::new(0), from())
        .key(Ticks::new(10), to())
        .ease(Ease::CubicInOut);

    // 10 ticks ÷ 2 = 5 chunks of 2 ticks. Progress at each boundary is
    // 0.2, 0.4, 0.6, 0.8, 1.0, and CubicInOut of those is
    // 4·0.008 = 0.032, 4·0.064 = 0.256, 1 − 0.8³/2 = 0.744,
    // 1 − 0.4³/2 = 0.968, 1 — so x is ten times each.
    let got = schedule(&timeline);
    let want = [
        (0.0, 0),
        (0.32, 2),
        (2.56, 2),
        (7.44, 2),
        (9.68, 2),
        (10.0, 2),
    ];
    assert_eq!(got.len(), want.len(), "step count: {got:?}");
    for (i, ((gx, gt), (wx, wt))) in got.iter().zip(want.iter()).enumerate() {
        assert!(approx(*gx, *wx), "step {i}: x was {gx}, expected {wx}");
        assert_eq!(gt, wt, "step {i}: duration");
    }
    // The chunks add up to the segment exactly.
    assert_eq!(got.iter().skip(1).map(|(_, t)| t).sum::<u64>(), 10);
}

#[test]
fn a_remainder_is_spread_over_the_leading_chunks() {
    // 7 ticks at a 2-tick sub-step: ⌈7/2⌉ = 4 chunks, base 1, remainder 3, so
    // the first three chunks get an extra tick: 2, 2, 2, 1 — summing to 7.
    let timeline = Timeline::new()
        .key(Ticks::new(0), from())
        .key(Ticks::new(7), to())
        .ease(Ease::QuadIn);
    let durations: Vec<u64> = schedule(&timeline)
        .into_iter()
        .skip(1)
        .map(|(_, t)| t)
        .collect();
    assert_eq!(durations, vec![2, 2, 2, 1]);
    assert_eq!(durations.iter().sum::<u64>(), 7);

    // The eased values follow the *elapsed* time, so progress is 2/7, 4/7,
    // 6/7, 1 and QuadIn squares it.
    let xs: Vec<f64> = schedule(&timeline)
        .into_iter()
        .skip(1)
        .map(|(x, _)| x)
        .collect();
    let want: Vec<f64> = [2.0 / 7.0, 4.0 / 7.0, 6.0 / 7.0, 1.0]
        .iter()
        .map(|t| 10.0 * t * t)
        .collect();
    for (i, (got, want)) in xs.iter().zip(want.iter()).enumerate() {
        assert!(approx(*got, *want), "chunk {i}: {got} vs {want}");
    }
}

#[test]
fn a_custom_sub_step_changes_the_chunk_count() {
    let timeline = Timeline::new()
        .key(Ticks::new(0), from())
        .key(Ticks::new(10), to())
        .ease(Ease::QuadIn)
        .sub_step(Ticks::new(5));
    // Two chunks of five.
    let durations: Vec<u64> = schedule(&timeline)
        .into_iter()
        .skip(1)
        .map(|(_, t)| t)
        .collect();
    assert_eq!(durations, vec![5, 5]);

    // A sub-step longer than the segment collapses to a single chunk.
    let coarse = Timeline::new()
        .key(Ticks::new(0), from())
        .key(Ticks::new(3), to())
        .ease(Ease::QuadIn)
        .sub_step(Ticks::new(20));
    assert_eq!(schedule(&coarse), vec![(0.0, 0), (10.0, 3)]);
}

#[test]
fn coincident_keys_are_a_hard_cut() {
    let timeline = Timeline::new()
        .key(Ticks::new(0), from())
        .key(Ticks::new(0), to())
        .ease(Ease::CubicInOut);
    assert_eq!(schedule(&timeline), vec![(0.0, 0), (10.0, 0)]);
}

#[test]
fn several_segments_run_back_to_back() {
    let timeline = Timeline::new()
        .key(Ticks::new(0), from())
        .key(Ticks::new(20), to())
        .key(Ticks::new(24), from())
        .ease(Ease::QuadOut);
    // First segment linear (one packet of 20), second eased into two 2-tick
    // chunks. QuadOut(0.5) = 0.75 and QuadOut(1) = 1, and the second segment
    // runs from x=10 back to x=0, so x = 10 - 10·eased.
    let got = schedule(&timeline);
    assert_eq!(got.len(), 4);
    assert_eq!(got[0], (0.0, 0));
    assert_eq!(got[1], (10.0, 20));
    assert!(approx(got[2].0, 2.5) && got[2].1 == 2, "{:?}", got[2]);
    assert!(approx(got[3].0, 0.0) && got[3].1 == 2, "{:?}", got[3]);
    assert_eq!(timeline.duration(), Ticks::new(24));
}

#[test]
fn timing_keys_must_move_forward() {
    let result = std::panic::catch_unwind(|| {
        Timeline::new()
            .key(Ticks::new(20), from())
            .key(Ticks::new(10), to())
    });
    assert!(result.is_err(), "out-of-order keys must kill");
}

#[test]
#[should_panic(expected = "at least two keys")]
fn easing_needs_a_segment_to_ease() {
    let _ = Timeline::new()
        .key(Ticks::new(0), from())
        .ease(Ease::QuadIn);
}

#[test]
#[should_panic(expected = "at least one tick")]
fn a_zero_sub_step_kills() {
    let _: Timeline<Position> = Timeline::new().sub_step(Ticks::new(0));
}

// --- State blending. Continuous fields interpolate; discrete ones switch as
// soon as the segment starts. ---

#[test]
fn entity_states_blend_continuously_and_snap_discretely() {
    use billboard::entity::{BillboardMode, BlockDisplayState, BlockState};
    use billboard::helpers::Tween;
    use billboard::math::{Rotation, Scale};

    let a = BlockDisplayState {
        block: BlockState::new("minecraft:red_concrete"),
        position: Position::new(0.0, 0.0, 0.0),
        rotation: Rotation::IDENTITY,
        scale: Scale::splat(1.0),
        billboard_mode: BillboardMode::Fixed,
    };
    let b = BlockDisplayState {
        block: BlockState::new("minecraft:blue_concrete"),
        position: Position::new(4.0, 8.0, -2.0),
        rotation: Rotation::IDENTITY,
        scale: Scale::splat(3.0),
        billboard_mode: BillboardMode::Center,
    };

    // At t = 0 nothing has changed at all.
    let start = a.tween(&b, 0.0);
    assert_eq!(start, a);

    // A quarter of the way: positions and scales are a quarter blended, but the
    // block and the billboard mode have already switched — they cannot
    // interpolate, and waiting until the end would be a surprise mid-segment.
    let quarter = a.tween(&b, 0.25);
    assert_eq!(quarter.position, Position::new(1.0, 2.0, -0.5));
    assert_eq!(quarter.scale, Scale::splat(1.5));
    assert_eq!(quarter.block, b.block);
    assert_eq!(quarter.billboard_mode, BillboardMode::Center);

    // At t = 1 it is exactly the destination.
    assert_eq!(a.tween(&b, 1.0), b);
}

#[test]
fn text_state_blends_its_numbers_and_its_background_colour() {
    use billboard::entity::{BillboardMode, TextDisplayState, TextFlags};
    use billboard::helpers::{Color, Tween};
    use billboard::math::{Rotation, Scale};

    let base = TextDisplayState {
        text: "one".to_owned(),
        background: Color::BLACK,
        opacity: 0,
        line_width: 100,
        flags: TextFlags::default(),
        position: Position::ZERO,
        rotation: Rotation::IDENTITY,
        scale: Scale::splat(1.0),
        billboard_mode: BillboardMode::Fixed,
    };
    let target = TextDisplayState {
        text: "two".to_owned(),
        background: Color::WHITE,
        opacity: 200,
        line_width: 300,
        flags: TextFlags {
            shadow: true,
            ..Default::default()
        },
        ..base.clone()
    };

    let mid = base.tween(&target, 0.5);
    // Numbers interpolate linearly and round: 0 -> 200 gives 100, 100 -> 300
    // gives 200.
    assert_eq!(mid.opacity, 100);
    assert_eq!(mid.line_width, 200);
    // The background blends in Oklab, like every colour blend in the SDK: the
    // midpoint of black and white is 99, not 128.
    assert_eq!(mid.background, Color::rgb(99, 99, 99));
    // Text and flags switch instantly.
    assert_eq!(mid.text, "two");
    assert!(mid.flags.shadow);
}

#[test]
fn armor_stand_poses_blend_per_limb() {
    use billboard::entity::{ArmorStandState, Pose, StandFlags};
    use billboard::helpers::Tween;
    use billboard::math::Degrees;

    let zero = ArmorStandState {
        pose: Pose::ZERO,
        flags: StandFlags::default(),
        yaw: Degrees::new(0.0),
        position: Position::ZERO,
    };
    let mut raised = zero;
    raised.pose.right_arm = (Degrees::new(-90.0), Degrees::new(0.0), Degrees::new(30.0));
    raised.yaw = Degrees::new(180.0);

    let mid = zero.tween(&raised, 0.5);
    assert!(approx(mid.pose.right_arm.0.value(), -45.0));
    assert!(approx(mid.pose.right_arm.2.value(), 15.0));
    assert!(approx(mid.yaw.value(), 90.0));
    // Untouched limbs stay where they were.
    assert!(approx(mid.pose.left_arm.0.value(), 0.0));
}

/// An overshooting ease (`BackOut`, `Elastic*`) hands the tween a `t` outside
/// `0..=1` on purpose — that overshoot *is* the effect. Position and scale
/// extrapolate; rotation must too, or a springy move arrives with its
/// orientation already parked at the target while everything else is still
/// swinging past it.
#[test]
fn rotations_extrapolate_under_an_overshooting_ease() {
    use billboard::helpers::Tween;
    use billboard::math::Rotation;

    // BackOut(0.5) = 1 + 2.70158·(-0.5)³ + 1.70158·(-0.5)² = 1.0876975.
    let t = Ease::BackOut.apply(0.5);
    assert!(approx(t, 1.0876975), "BackOut midpoint was {t}");

    let half_turn = Rotation {
        x: 0.0,
        y: 1.0,
        z: 0.0,
        w: 0.0,
    };
    let overshot = Rotation::IDENTITY.tween(&half_turn, t);

    // nlerp at t = 1.0876975: (0, 1.0876975, 0, -0.0876975), norm
    // sqrt(1.0876975² + 0.0876975²) = 1.0912271546348633, giving
    // (0, 0.996765426318552, 0, -0.080365943632831) — a 189.2° turn, past the
    // 180° target, which is exactly the overshoot.
    assert!(
        approx(overshot.y, 0.996765426318552),
        "y was {}",
        overshot.y
    );
    assert!(
        approx(overshot.w, -0.080365943632831),
        "w was {} (a clamped lerp would give exactly 0.0)",
        overshot.w
    );
    assert!(overshot.w < 0.0, "the rotation did not overshoot at all");
}
