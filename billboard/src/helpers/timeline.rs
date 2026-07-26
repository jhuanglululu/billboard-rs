//! [`Timeline`]: keyframed states with per-segment easing.
//!
//! Minecraft's client interpolation is strictly **linear**, so easing cannot be
//! handed to the client — it has to come from sub-stepped keyframes. A timeline
//! chops each segment into short chunks (two ticks by default), evaluates the
//! ease at each chunk boundary, interpolates the state there, and issues one
//! `animate` + `sleep` per chunk. Linear segments skip all that and go out as a
//! single `animate`, because the client already does linear perfectly.
//!
//! ```ignore
//! Timeline::new()
//!     .key(Ticks::new(0), resting.clone())
//!     .key(Ticks::new(20), lifted.clone()).ease(Ease::CubicOut)
//!     .key(Ticks::new(40), resting.clone()).ease(Ease::BounceOut)
//!     .play(&mut display);          // blocks for 40 ticks
//! ```
//!
//! # Cost
//!
//! [`play`](Timeline::play) starts by building the whole schedule —
//! [`steps`](Timeline::steps) allocates one [`Step`] per chunk and clones a
//! state into each — and all of that lands in the **single tick** where `play`
//! is called. A 60-tick eased timeline is ~30 steps, which is fine; a
//! multi-minute timeline at a one-tick sub-step is thousands of allocations at
//! once, against a per-tick instruction budget. Keep timelines to a scene, and
//! prefer a coarser [`sub_step`](Timeline::sub_step) over a finer one — the
//! client interpolates linearly between chunks anyway.

use crate::entity::{
    ArmorStand, ArmorStandState, BlockDisplay, BlockDisplayState, Item, ItemDisplay,
    ItemDisplayState, ItemState, Pose, TextDisplay, TextDisplayState,
};
use crate::helpers::Ease;
use crate::math::{Degrees, Position, Rotation, Scale, Ticks};

/// A state that can be interpolated — what a [`Timeline`] keys over.
///
/// Continuous fields blend; discrete ones (a block, an item, a text string, a
/// flag) cannot, so they switch at the *start* of a segment, matching what
/// `animate` already does with them.
pub trait Tween: Clone {
    /// Blend towards `other`: `t` is `0.0` at this state and `1.0` at `other`.
    ///
    /// **`t` is not clamped.** An overshooting ease (`Ease::BackOut`,
    /// `Ease::Elastic*`) deliberately returns values outside `0..=1`, and that
    /// overshoot is the whole effect — so continuous fields should extrapolate
    /// (the SDK's own impls do, rotations included, via
    /// [`Rotation::lerp_unclamped`](crate::math::Rotation::lerp_unclamped)).
    /// Fields that cannot meaningfully overshoot say so instead: colours clamp,
    /// and discrete fields switch as soon as `t` leaves `0.0`.
    fn tween(&self, other: &Self, t: f64) -> Self;
}

/// Something a [`Timeline`] can drive: an entity handle that accepts whole
/// states.
///
/// Implemented for the five owner handles. Weak references are deliberately
/// *not* implemented: their methods are fallible, and a timeline is a long
/// blocking sequence — it should not be the thing deciding what "the entity
/// died halfway through" means. Drive a weak reference with
/// [`Timeline::play_with`], where the decision is yours and visible:
///
/// ```ignore
/// timeline.play_with(|state, over| {
///     weak.animate(state, over).expect("panel gone mid-show");
/// });
/// ```
pub trait Animate {
    type State: Tween;

    /// Apply a state over `over` ticks (`0` = instantly). Non-blocking — the
    /// timeline does the sleeping.
    fn apply(&mut self, state: &Self::State, over: Ticks);
}

macro_rules! animate_owner {
    ($t:ty, $state:ty) => {
        impl Animate for $t {
            type State = $state;

            fn apply(&mut self, state: &$state, over: Ticks) {
                self.animate(state, over);
            }
        }
    };
}

animate_owner!(BlockDisplay, BlockDisplayState);
animate_owner!(ItemDisplay, ItemDisplayState);
animate_owner!(TextDisplay, TextDisplayState);
animate_owner!(ArmorStand, ArmorStandState);
animate_owner!(Item, ItemState);

/// One keyframe: a state, when it is reached, and how to get there.
#[derive(Clone, Debug, PartialEq)]
struct Key<S> {
    at: Ticks,
    state: S,
    /// How the segment *arriving at* this key is eased. Ignored on the first
    /// key, which has no segment before it.
    ease: Ease,
}

/// One chunk of a played timeline: a state to apply and how long to take.
#[derive(Clone, Debug, PartialEq)]
pub struct Step<S> {
    pub state: S,
    pub over: Ticks,
}

/// The default sub-step: two ticks.
///
/// Short enough that any ease reads as smooth (a 20-tick segment gets ten
/// samples), long enough that a busy show is not spending its whole fuel budget
/// on packets. Change it with [`Timeline::sub_step`].
pub const DEFAULT_SUB_STEP: Ticks = Ticks::new(2);

/// A keyframed animation over one entity's state.
#[derive(Clone, Debug, PartialEq)]
pub struct Timeline<S> {
    keys: Vec<Key<S>>,
    sub_step: Ticks,
}

impl<S: Tween> Default for Timeline<S> {
    fn default() -> Timeline<S> {
        Timeline::new()
    }
}

impl<S: Tween> Timeline<S> {
    pub fn new() -> Timeline<S> {
        Timeline {
            keys: Vec::new(),
            sub_step: DEFAULT_SUB_STEP,
        }
    }

    /// Add a keyframe at `at`, measured from the start of the timeline.
    ///
    /// Keys must be added in non-decreasing time order; anything else is a bug
    /// and kills the animation. Two keys at the same time make a hard cut.
    pub fn key(mut self, at: Ticks, state: S) -> Timeline<S> {
        if let Some(last) = self.keys.last() {
            assert!(
                at >= last.at,
                "Timeline keys must be in time order: {:?} came after {:?}",
                at,
                last.at
            );
        }
        self.keys.push(Key {
            at,
            state,
            ease: Ease::Linear,
        });
        self
    }

    /// Ease the segment arriving at the key you just added.
    ///
    /// Calling it before any key, or on the first key (which has no segment
    /// before it), is a bug and kills the animation.
    pub fn ease(mut self, ease: Ease) -> Timeline<S> {
        let count = self.keys.len();
        assert!(
            count >= 2,
            "Timeline::ease applies to the segment before the last key, so it \
             needs at least two keys (got {count})"
        );
        self.keys[count - 1].ease = ease;
        self
    }

    /// Sub-step length for eased segments. Must be at least one tick.
    pub fn sub_step(mut self, sub_step: Ticks) -> Timeline<S> {
        assert!(
            sub_step.count() >= 1,
            "Timeline sub-step must be at least one tick"
        );
        self.sub_step = sub_step;
        self
    }

    /// How long a full [`play`](Timeline::play) takes.
    pub fn duration(&self) -> Ticks {
        match (self.keys.first(), self.keys.last()) {
            (Some(first), Some(last)) => last.at - first.at,
            _ => Ticks::new(0),
        }
    }

    /// The exact schedule [`play`](Timeline::play) will execute: one step per
    /// chunk, in order. The first step is the opening snap (`over == 0`).
    ///
    /// Pure — it touches no entity and no host, which is what makes a
    /// timeline's timing testable.
    pub fn steps(&self) -> Vec<Step<S>> {
        let mut steps = Vec::new();
        let Some(first) = self.keys.first() else {
            return steps;
        };
        // Open by snapping to the first key, so a timeline always starts from a
        // known state rather than wherever the entity happened to be.
        steps.push(Step {
            state: first.state.clone(),
            over: Ticks::new(0),
        });

        for pair in self.keys.windows(2) {
            let (from, to) = (&pair[0], &pair[1]);
            let total = (to.at - from.at).count();
            if total == 0 {
                // Coincident keys: a hard cut.
                steps.push(Step {
                    state: to.state.clone(),
                    over: Ticks::new(0),
                });
                continue;
            }
            if to.ease == Ease::Linear {
                // The client interpolates linearly on its own — one packet.
                steps.push(Step {
                    state: to.state.clone(),
                    over: Ticks::new(total),
                });
                continue;
            }
            // Split into as few chunks of `sub_step` as cover the segment, with
            // the remainder spread over the leading chunks so every chunk is
            // within one tick of the others and they sum to exactly `total`.
            let chunks = total.div_ceil(self.sub_step.count()).max(1);
            let base = total / chunks;
            let extra = total % chunks;
            let mut elapsed = 0u64;
            for chunk in 0..chunks {
                let len = base + u64::from(chunk < extra);
                elapsed += len;
                let progress = elapsed as f64 / total as f64;
                steps.push(Step {
                    state: from.state.tween(&to.state, to.ease.apply(progress)),
                    over: Ticks::new(len),
                });
            }
        }
        steps
    }

    /// Play the timeline on `target`, **blocking** until it finishes.
    pub fn play(&self, target: &mut impl Animate<State = S>) {
        self.play_with(|state, over| target.apply(state, over));
    }

    /// Play the timeline, applying each step through `apply` — the escape hatch
    /// for weak references, groups, or several entities at once.
    ///
    /// Blocks: it sleeps each step's duration after applying it.
    pub fn play_with(&self, mut apply: impl FnMut(&S, Ticks)) {
        for step in self.steps() {
            apply(&step.state, step.over);
            if step.over.count() > 0 {
                crate::sleep(step.over);
            }
        }
    }
}

// --- Tween impls for the entity states. Continuous fields blend; discrete
// fields switch at the start of the segment (t > 0), which is what `animate`
// does with them anyway. ---

/// Pick the discrete value for a blend: the destination as soon as the segment
/// has started at all. `t < 0` (the wind-up of `BackIn`) still counts as
/// started — the segment is under way, the block just cannot be halfway.
fn snap<T: Clone>(from: &T, to: &T, t: f64) -> T {
    if t != 0.0 { to.clone() } else { from.clone() }
}

fn tween_position(a: Position, b: Position, t: f64) -> Position {
    a + (b - a) * t
}

fn tween_scale(a: Scale, b: Scale, t: f64) -> Scale {
    Scale::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
    )
}

fn tween_degrees(a: Degrees, b: Degrees, t: f64) -> Degrees {
    Degrees::new(a.value() + (b.value() - a.value()) * t)
}

fn tween_angles(
    a: (Degrees, Degrees, Degrees),
    b: (Degrees, Degrees, Degrees),
    t: f64,
) -> (Degrees, Degrees, Degrees) {
    (
        tween_degrees(a.0, b.0, t),
        tween_degrees(a.1, b.1, t),
        tween_degrees(a.2, b.2, t),
    )
}

fn tween_u8(a: u8, b: u8, t: f64) -> u8 {
    let v = a as f64 + (b as f64 - a as f64) * t;
    v.round().clamp(0.0, 255.0) as u8
}

fn tween_u32(a: u32, b: u32, t: f64) -> u32 {
    let v = a as f64 + (b as f64 - a as f64) * t;
    v.round().max(0.0) as u32
}

impl Tween for BlockDisplayState {
    fn tween(&self, other: &BlockDisplayState, t: f64) -> BlockDisplayState {
        BlockDisplayState {
            block: snap(&self.block, &other.block, t),
            position: tween_position(self.position, other.position, t),
            rotation: self.rotation.lerp_unclamped(other.rotation, t),
            scale: tween_scale(self.scale, other.scale, t),
            billboard_mode: snap(&self.billboard_mode, &other.billboard_mode, t),
        }
    }
}

impl Tween for ItemDisplayState {
    fn tween(&self, other: &ItemDisplayState, t: f64) -> ItemDisplayState {
        ItemDisplayState {
            item: snap(&self.item, &other.item, t),
            context: snap(&self.context, &other.context, t),
            position: tween_position(self.position, other.position, t),
            rotation: self.rotation.lerp_unclamped(other.rotation, t),
            scale: tween_scale(self.scale, other.scale, t),
            billboard_mode: snap(&self.billboard_mode, &other.billboard_mode, t),
        }
    }
}

impl Tween for TextDisplayState {
    fn tween(&self, other: &TextDisplayState, t: f64) -> TextDisplayState {
        TextDisplayState {
            text: snap(&self.text, &other.text, t),
            // Backgrounds blend perceptually, like every other colour blend.
            // `Color::lerp` clamps on purpose: an "overshot" colour has no
            // meaning (the channels would just saturate) so the spring shows up
            // in the geometry, not the palette.
            background: self.background.lerp(other.background, t),
            opacity: tween_u8(self.opacity, other.opacity, t),
            line_width: tween_u32(self.line_width, other.line_width, t),
            flags: snap(&self.flags, &other.flags, t),
            position: tween_position(self.position, other.position, t),
            rotation: self.rotation.lerp_unclamped(other.rotation, t),
            scale: tween_scale(self.scale, other.scale, t),
            billboard_mode: snap(&self.billboard_mode, &other.billboard_mode, t),
        }
    }
}

impl Tween for ArmorStandState {
    fn tween(&self, other: &ArmorStandState, t: f64) -> ArmorStandState {
        ArmorStandState {
            pose: Pose {
                head: tween_angles(self.pose.head, other.pose.head, t),
                body: tween_angles(self.pose.body, other.pose.body, t),
                left_arm: tween_angles(self.pose.left_arm, other.pose.left_arm, t),
                right_arm: tween_angles(self.pose.right_arm, other.pose.right_arm, t),
                left_leg: tween_angles(self.pose.left_leg, other.pose.left_leg, t),
                right_leg: tween_angles(self.pose.right_leg, other.pose.right_leg, t),
            },
            flags: snap(&self.flags, &other.flags, t),
            yaw: tween_degrees(self.yaw, other.yaw, t),
            position: tween_position(self.position, other.position, t),
        }
    }
}

impl Tween for ItemState {
    fn tween(&self, other: &ItemState, t: f64) -> ItemState {
        ItemState {
            item: snap(&self.item, &other.item, t),
            position: tween_position(self.position, other.position, t),
        }
    }
}

impl Tween for Rotation {
    fn tween(&self, other: &Rotation, t: f64) -> Rotation {
        self.lerp_unclamped(*other, t)
    }
}

impl Tween for Position {
    fn tween(&self, other: &Position, t: f64) -> Position {
        tween_position(*self, *other, t)
    }
}

impl Tween for Scale {
    fn tween(&self, other: &Scale, t: f64) -> Scale {
        tween_scale(*self, *other, t)
    }
}
