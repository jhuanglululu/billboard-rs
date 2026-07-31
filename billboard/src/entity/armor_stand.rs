//! [`ArmorStand`]: a posable armor stand — six limbs, six equipment slots,
//! and a yaw, all tweened by the host.
//!
//! # Geometry
//!
//! An armor stand stands **on** its position: the position is the feet, and the
//! model rises from there, like any vanilla mob. Yaw turns it about that
//! vertical axis. It has no rotation or scale of its own — a compile error
//! rather than a runtime kill if you reach for `rotate_to`/`scale_to`.
//!
//! Getters report the **target** even mid-tween: the host stores what you asked
//! for and the per-tick tween packets are bookkeeping the guest never sees.
//!
//! # Accessor methods
//!
//! [`position`](ArmorStand::position) / [`teleport_to`](ArmorStand::teleport_to) /
//! [`move_to`](ArmorStand::move_to),
//! [`yaw`](ArmorStand::yaw) / [`set_yaw`](ArmorStand::set_yaw) /
//! [`turn_to`](ArmorStand::turn_to),
//! [`pose`](ArmorStand::pose) / [`set_pose`](ArmorStand::set_pose) /
//! [`animate_pose`](ArmorStand::animate_pose) and the per-part
//! [`pose_part`](ArmorStand::pose_part) / [`set_pose_part`](ArmorStand::set_pose_part) /
//! [`animate_pose_part`](ArmorStand::animate_pose_part),
//! [`set_equipment`](ArmorStand::set_equipment),
//! [`flags`](ArmorStand::flags) / [`set_flags`](ArmorStand::set_flags) plus the
//! four single-bit setters,
//! [`state`](ArmorStand::state) / [`set`](ArmorStand::set) /
//! [`animate`](ArmorStand::animate),
//! [`weak`](ArmorStand::weak) / [`weak_mut`](ArmorStand::weak_mut) /
//! [`despawn`](ArmorStand::despawn) / [`leak`](ArmorStand::leak). The weak
//! references carry the same set, each returning `Result<_, Dead>`.

use super::{Dead, EquipmentSlot, ItemStr, PosePart, StandFlags, WeakMut, WeakRef, raw};
use crate::math::{Degrees, Position, Ticks};

/// The six limb angles of an armor stand, each an euler `(x, y, z)` triple in
/// degrees — **euler end to end**, no quaternions anywhere near a pose, so
/// there is never a conversion to disagree about.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub head: (Degrees, Degrees, Degrees),
    pub body: (Degrees, Degrees, Degrees),
    pub left_arm: (Degrees, Degrees, Degrees),
    pub right_arm: (Degrees, Degrees, Degrees),
    pub left_leg: (Degrees, Degrees, Degrees),
    pub right_leg: (Degrees, Degrees, Degrees),
}

impl Pose {
    /// Every limb at zero — the default T-pose-ish stance vanilla starts with.
    pub const ZERO: Pose = Pose {
        head: ZERO_ANGLES,
        body: ZERO_ANGLES,
        left_arm: ZERO_ANGLES,
        right_arm: ZERO_ANGLES,
        left_leg: ZERO_ANGLES,
        right_leg: ZERO_ANGLES,
    };

    /// The angles for one part.
    pub const fn part(&self, part: PosePart) -> (Degrees, Degrees, Degrees) {
        match part {
            PosePart::Head => self.head,
            PosePart::Body => self.body,
            PosePart::LeftArm => self.left_arm,
            PosePart::RightArm => self.right_arm,
            PosePart::LeftLeg => self.left_leg,
            PosePart::RightLeg => self.right_leg,
        }
    }

    /// Replace the angles for one part.
    pub fn set_part(&mut self, part: PosePart, angles: (Degrees, Degrees, Degrees)) {
        let slot = match part {
            PosePart::Head => &mut self.head,
            PosePart::Body => &mut self.body,
            PosePart::LeftArm => &mut self.left_arm,
            PosePart::RightArm => &mut self.right_arm,
            PosePart::LeftLeg => &mut self.left_leg,
            PosePart::RightLeg => &mut self.right_leg,
        };
        *slot = angles;
    }
}

const ZERO_ANGLES: (Degrees, Degrees, Degrees) =
    (Degrees::new(0.0), Degrees::new(0.0), Degrees::new(0.0));

/// An [`ArmorStand`]'s checkpoint state.
///
/// **Equipment is not part of it.** The ABI has `set_equipment` and no getter —
/// equipment is write-only, because the host sends it as packets and never
/// reads it back — so a state can't honestly claim to capture it. Set equipment
/// explicitly with [`ArmorStand::set_equipment`]; a `state()`/`set()` round trip
/// leaves it untouched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArmorStandState {
    pub pose: Pose,
    pub flags: StandFlags,
    pub yaw: Degrees,
    pub position: Position,
}

fn raw_apply(id: i32, s: &ArmorStandState, over: Ticks) {
    raw::set_position(id, &s.position, over);
    for part in PosePart::ALL {
        raw::set_pose(id, part, s.pose.part(part), over);
    }
    raw::set_yaw(id, s.yaw, over);
    raw::set_stand_flags(id, s.flags);
}

fn raw_state(id: i32) -> ArmorStandState {
    let mut pose = Pose::ZERO;
    for part in PosePart::ALL {
        pose.set_part(part, raw::get_pose(id, part));
    }
    ArmorStandState {
        pose,
        flags: raw::get_stand_flags(id),
        yaw: raw::get_yaw(id),
        position: raw::get_position(id),
    }
}

entity_handle! {
    /// The absolute owner of an armor stand.
    ///
    /// Armor stands have **no client-side interpolation**, so every `over`
    /// duration here (pose, position, yaw) is tweened by the *host*, which
    /// sends per-tick packets. The API surface is the same as a display's —
    /// non-blocking, no guest fuel burned — and getters report the target
    /// value, exactly like displays.
    ///
    /// ```ignore
    /// let mut guard = ArmorStand::spawn(pos);
    /// guard.set_flags(StandFlags { arms: true, no_baseplate: true, ..Default::default() });
    /// guard.set_equipment(EquipmentSlot::Helmet, items::DIAMOND_HELMET);
    /// let raised = (Degrees::new(-90.0), Degrees::new(0.0), Degrees::new(0.0));
    /// guard.animate_pose_part(PosePart::RightArm, raised, Ticks::new(20));
    /// ```
    ArmorStand => ArmorStandState
}

state_api!(owner ArmorStand, ArmorStandState);
state_api!(weak ArmorStand, ArmorStandState);
position_api!(owner ArmorStand);
position_api!(weak ArmorStand);

impl ArmorStand {
    /// Spawn an armor stand at `position`, in the default pose, facing yaw 0.
    pub fn spawn(position: impl AsRef<Position>) -> ArmorStand {
        ArmorStand::from_id(raw::spawn_armor_stand(position.as_ref()))
    }

    /// All six limb angles, freshly queried from the host (six calls).
    pub fn pose(&self) -> Pose {
        raw_state(self.id).pose
    }

    /// One limb's angles, freshly queried from the host.
    pub fn pose_part(&self, part: PosePart) -> (Degrees, Degrees, Degrees) {
        raw::get_pose(self.id, part)
    }

    /// Set one limb instantly.
    pub fn set_pose_part(&mut self, part: PosePart, angles: (Degrees, Degrees, Degrees)) {
        raw::set_pose(self.id, part, angles, Ticks::new(0));
    }

    /// Tween one limb over `over` ticks, host-side (non-blocking).
    pub fn animate_pose_part(
        &mut self,
        part: PosePart,
        angles: (Degrees, Degrees, Degrees),
        over: Ticks,
    ) {
        raw::set_pose(self.id, part, angles, over);
    }

    /// Set all six limbs instantly.
    pub fn set_pose(&mut self, pose: &Pose) {
        for part in PosePart::ALL {
            raw::set_pose(self.id, part, pose.part(part), Ticks::new(0));
        }
    }

    /// Tween all six limbs over `over` ticks, host-side (non-blocking).
    pub fn animate_pose(&mut self, pose: &Pose, over: Ticks) {
        for part in PosePart::ALL {
            raw::set_pose(self.id, part, pose.part(part), over);
        }
    }

    /// Put an item in one of the six slots.
    ///
    /// Write-only at the ABI: there is no getter, so equipment is the one
    /// attribute a `state()` can't report. An empty item (`""`) clears the
    /// slot.
    pub fn set_equipment(&mut self, slot: EquipmentSlot, item: impl Into<ItemStr>) {
        raw::set_equipment(self.id, slot.wire(), &item.into());
    }

    /// The four boolean options, freshly queried from the host.
    pub fn flags(&self) -> StandFlags {
        raw::get_stand_flags(self.id)
    }

    /// Set all four boolean options at once.
    pub fn set_flags(&mut self, flags: StandFlags) {
        raw::set_stand_flags(self.id, flags);
    }

    /// The small (baby-sized) armor stand.
    ///
    /// The flags share one ABI bitmask and the SDK caches nothing, so this
    /// reads the current mask, changes one bit and writes it back — use
    /// [`set_flags`](ArmorStand::set_flags) to change several at once.
    pub fn set_small(&mut self, small: bool) {
        let mut flags = self.flags();
        flags.small = small;
        self.set_flags(flags);
    }

    /// Show arms.
    pub fn set_arms(&mut self, arms: bool) {
        let mut flags = self.flags();
        flags.arms = arms;
        self.set_flags(flags);
    }

    /// Hide the base plate.
    pub fn set_no_baseplate(&mut self, no_baseplate: bool) {
        let mut flags = self.flags();
        flags.no_baseplate = no_baseplate;
        self.set_flags(flags);
    }

    /// Hide the stand itself, leaving its equipment floating.
    pub fn set_invisible(&mut self, invisible: bool) {
        let mut flags = self.flags();
        flags.invisible = invisible;
        self.set_flags(flags);
    }

    /// Facing direction, freshly queried from the host.
    pub fn yaw(&self) -> Degrees {
        raw::get_yaw(self.id)
    }

    /// Turn instantly.
    pub fn set_yaw(&mut self, yaw: impl Into<Degrees>) {
        raw::set_yaw(self.id, yaw.into(), Ticks::new(0));
    }

    /// Turn over `over` ticks, host-tweened (non-blocking).
    pub fn turn_to(&mut self, yaw: impl Into<Degrees>, over: Ticks) {
        raw::set_yaw(self.id, yaw.into(), over);
    }
}

impl WeakRef<ArmorStand> {
    pub fn pose(&self) -> Result<Pose, Dead> {
        self.check()?;
        Ok(raw_state(self.id()).pose)
    }

    pub fn pose_part(&self, part: PosePart) -> Result<(Degrees, Degrees, Degrees), Dead> {
        self.check()?;
        Ok(raw::get_pose(self.id(), part))
    }

    pub fn flags(&self) -> Result<StandFlags, Dead> {
        self.check()?;
        Ok(raw::get_stand_flags(self.id()))
    }

    pub fn yaw(&self) -> Result<Degrees, Dead> {
        self.check()?;
        Ok(raw::get_yaw(self.id()))
    }
}

impl WeakMut<ArmorStand> {
    pub fn pose(&self) -> Result<Pose, Dead> {
        self.check()?;
        Ok(raw_state(self.id()).pose)
    }

    pub fn pose_part(&self, part: PosePart) -> Result<(Degrees, Degrees, Degrees), Dead> {
        self.check()?;
        Ok(raw::get_pose(self.id(), part))
    }

    pub fn set_pose_part(
        &mut self,
        part: PosePart,
        angles: (Degrees, Degrees, Degrees),
    ) -> Result<(), Dead> {
        self.check()?;
        raw::set_pose(self.id(), part, angles, Ticks::new(0));
        Ok(())
    }

    pub fn animate_pose_part(
        &mut self,
        part: PosePart,
        angles: (Degrees, Degrees, Degrees),
        over: Ticks,
    ) -> Result<(), Dead> {
        self.check()?;
        raw::set_pose(self.id(), part, angles, over);
        Ok(())
    }

    pub fn set_pose(&mut self, pose: &Pose) -> Result<(), Dead> {
        self.check()?;
        for part in PosePart::ALL {
            raw::set_pose(self.id(), part, pose.part(part), Ticks::new(0));
        }
        Ok(())
    }

    pub fn animate_pose(&mut self, pose: &Pose, over: Ticks) -> Result<(), Dead> {
        self.check()?;
        for part in PosePart::ALL {
            raw::set_pose(self.id(), part, pose.part(part), over);
        }
        Ok(())
    }

    pub fn set_equipment(
        &mut self,
        slot: EquipmentSlot,
        item: impl Into<ItemStr>,
    ) -> Result<(), Dead> {
        self.check()?;
        raw::set_equipment(self.id(), slot.wire(), &item.into());
        Ok(())
    }

    pub fn flags(&self) -> Result<StandFlags, Dead> {
        self.check()?;
        Ok(raw::get_stand_flags(self.id()))
    }

    pub fn set_flags(&mut self, flags: StandFlags) -> Result<(), Dead> {
        self.check()?;
        raw::set_stand_flags(self.id(), flags);
        Ok(())
    }

    pub fn yaw(&self) -> Result<Degrees, Dead> {
        self.check()?;
        Ok(raw::get_yaw(self.id()))
    }

    pub fn set_yaw(&mut self, yaw: impl Into<Degrees>) -> Result<(), Dead> {
        self.check()?;
        raw::set_yaw(self.id(), yaw.into(), Ticks::new(0));
        Ok(())
    }

    pub fn turn_to(&mut self, yaw: impl Into<Degrees>, over: Ticks) -> Result<(), Dead> {
        self.check()?;
        raw::set_yaw(self.id(), yaw.into(), over);
        Ok(())
    }
}
