//! [`Group`]: move a pile of entities as one rigid assembly.
//!
//! A group is pure guest bookkeeping — there is no host-side group object. It
//! holds weak references plus each member's *local* transform, and when you
//! move, turn or scale the group it recomputes every member's world state and
//! pushes it out. Members orbit under rotation, exactly as if they were welded
//! to a frame.

use crate::entity::{BlockDisplay, Dead, ItemDisplay, TextDisplay, WeakMut};
use crate::math::{Offset, Position, Rotation, Scale, Ticks};

/// Where a member sits *relative to its group's origin*, before the group's own
/// transform is applied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Local {
    pub offset: Offset,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Local {
    /// At the group's origin, unrotated, unscaled.
    pub const IDENTITY: Local = Local {
        offset: Offset::ZERO,
        rotation: Rotation::IDENTITY,
        scale: Scale::splat(1.0),
    };

    /// Offset from the group's origin, otherwise identity — the common case.
    pub fn at(offset: impl AsRef<Offset>) -> Local {
        Local {
            offset: *offset.as_ref(),
            ..Local::IDENTITY
        }
    }

    /// Offset plus a local orientation of its own.
    pub fn new(
        offset: impl AsRef<Offset>,
        rotation: impl AsRef<Rotation>,
        scale: impl AsRef<Scale>,
    ) -> Local {
        Local {
            offset: *offset.as_ref(),
            rotation: *rotation.as_ref(),
            scale: *scale.as_ref(),
        }
    }
}

impl Default for Local {
    fn default() -> Local {
        Local::IDENTITY
    }
}

/// A world transform: where something actually is.
#[derive(Clone, Copy, Debug, PartialEq)]
struct World {
    position: Position,
    rotation: Rotation,
    scale: Scale,
}

impl World {
    /// Compose a local transform onto this one:
    ///
    /// ```text
    /// position = position + rotation · (local.offset ⊙ scale)
    /// rotation = rotation · local.rotation
    /// scale    = scale ⊙ local.scale
    /// ```
    ///
    /// The scale multiplies the offset *before* the rotation turns it, so
    /// scaling a group spreads its members apart along the group's own axes
    /// rather than the world's.
    fn compose(self, local: &Local) -> World {
        World {
            position: self.position + self.rotation.rotate_offset(mul(local.offset, self.scale)),
            rotation: self.rotation * local.rotation,
            scale: self.scale * local.scale,
        }
    }
}

/// Componentwise offset × scale.
fn mul(offset: Offset, scale: Scale) -> Offset {
    Offset::new(offset.x * scale.x, offset.y * scale.y, offset.z * scale.z)
}

/// Anything a group can drive: the three display kinds, through a
/// [`WeakMut`].
///
/// Armor stands and items are deliberately absent — they have no rotation or
/// scale, so "orbit under the group's rotation, quaternion-composed" has no
/// honest meaning for them. Drive those alongside a group instead of inside it.
pub trait GroupMember: Send + Sync {
    /// Push a world transform, instantly (`over == 0`) or interpolated.
    fn apply_world(
        &mut self,
        position: &Position,
        rotation: &Rotation,
        scale: &Scale,
        over: Ticks,
    ) -> Result<(), Dead>;

    /// Whether the entity is still alive.
    fn member_alive(&self) -> bool;
}

macro_rules! group_member {
    ($t:ty) => {
        impl GroupMember for WeakMut<$t> {
            fn apply_world(
                &mut self,
                position: &Position,
                rotation: &Rotation,
                scale: &Scale,
                over: Ticks,
            ) -> Result<(), Dead> {
                self.move_to(position, over)?;
                self.rotate_to(rotation, over)?;
                self.scale_to(scale, over)?;
                Ok(())
            }

            fn member_alive(&self) -> bool {
                self.is_alive()
            }
        }
    };
}

group_member!(BlockDisplay);
group_member!(ItemDisplay);
group_member!(TextDisplay);

/// Which members were already gone when a transform was applied.
///
/// Each path locates a member from the group you called: `[2]` is the third
/// member, `[2, 0]` is the first member of the nested group that is the third
/// member.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeadMembers {
    paths: Vec<Vec<usize>>,
}

impl DeadMembers {
    /// How many members were dead.
    pub fn count(&self) -> usize {
        self.paths.len()
    }

    /// Where they were, as member index paths from the group you called.
    pub fn paths(&self) -> &[Vec<usize>] {
        &self.paths
    }
}

impl core::fmt::Display for DeadMembers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} group member(s) already despawned at ", self.count())?;
        for (i, path) in self.paths.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{path:?}")?;
        }
        Ok(())
    }
}

impl std::error::Error for DeadMembers {}

enum Entry {
    Member {
        target: Box<dyn GroupMember>,
        local: Local,
    },
    Group {
        group: Group,
        local: Local,
    },
}

/// A rigid assembly of display entities, movable as one.
///
/// ```ignore
/// let mut sign = Group::new(Position::new(0.0, 5.0, 0.0));
/// for (i, panel) in panels.iter().enumerate() {
///     sign.add(panel.weak_mut(), Local::at(Offset::new(i as f64, 0.0, 0.0)));
/// }
/// // The whole sign swings 90° about its own origin over one second; every
/// // panel orbits with it.
/// sign.animate(
///     sign.position(),
///     Rotation::axis_angle(Vector3d::Y, Degrees::new(90.0)),
///     sign.scale(),
///     Ticks::new(20),
/// ).expect("panels alive");
/// ```
///
/// Groups **nest**: add a group as a member with its own local transform, and
/// its members' locals compose through. A nested group's own transform is
/// driven by its parent.
///
/// A `Group` is guest-side data — the one place the SDK does hold state, because
/// there is no host object to ask. That state is the group's *own* transform and
/// its members' locals; member world states still live only on the host.
///
/// # Cost
///
/// Every transform costs **six host calls per member** (a liveness check and a
/// setter each for position, rotation and scale), all in the tick you call it —
/// nested groups included, recursively. The composition maths itself is a
/// handful of multiplications and one quaternion product per member, which is
/// nothing. A fifty-member group is three hundred host calls in one tick: still
/// fine, but it is the number to have in mind before driving a group every tick
/// rather than handing it a duration and letting the host interpolate.
pub struct Group {
    position: Position,
    rotation: Rotation,
    scale: Scale,
    entries: Vec<Entry>,
}

impl Group {
    /// An empty group with its origin at `position`, unrotated, unscaled.
    pub fn new(position: impl AsRef<Position>) -> Group {
        Group {
            position: *position.as_ref(),
            rotation: Rotation::IDENTITY,
            scale: Scale::splat(1.0),
            entries: Vec::new(),
        }
    }

    /// Add a display, at `local` relative to the group's origin.
    ///
    /// Adding does not move the entity; call
    /// [`apply`](Group::apply) (or any transform method) to place it.
    pub fn add(&mut self, target: impl GroupMember + 'static, local: Local) -> &mut Group {
        self.entries.push(Entry::Member {
            target: Box::new(target),
            local,
        });
        self
    }

    /// Add a nested group, at `local` relative to this group's origin.
    pub fn add_group(&mut self, group: Group, local: Local) -> &mut Group {
        self.entries.push(Entry::Group { group, local });
        self
    }

    /// How many direct members (nested groups count as one).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The group's own origin.
    pub fn position(&self) -> Position {
        self.position
    }

    /// The group's own orientation.
    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    /// The group's own scale.
    pub fn scale(&self) -> Scale {
        self.scale
    }

    /// Re-push the current transform to every member — after `add`, or to undo
    /// something that drove a member directly.
    pub fn apply(&mut self) -> Result<(), DeadMembers> {
        let world = self.world();
        self.drive(world, Ticks::new(0))
    }

    /// Set the group's transform and place every member instantly.
    ///
    /// Live members are driven even if others are gone — the move has already
    /// happened for them, and reporting is more honest than rolling back. A
    /// dead member is not silently ignored: it comes back in [`DeadMembers`],
    /// which you can `.expect(…)` (killing the animation, per the error
    /// philosophy) or handle by pruning.
    pub fn set_transform(
        &mut self,
        position: impl AsRef<Position>,
        rotation: impl AsRef<Rotation>,
        scale: impl AsRef<Scale>,
    ) -> Result<(), DeadMembers> {
        self.animate(position, rotation, scale, Ticks::new(0))
    }

    /// Set the group's transform and interpolate every member into place over
    /// `over` ticks. Non-blocking, like every other `*_to`/`animate`.
    pub fn animate(
        &mut self,
        position: impl AsRef<Position>,
        rotation: impl AsRef<Rotation>,
        scale: impl AsRef<Scale>,
        over: Ticks,
    ) -> Result<(), DeadMembers> {
        let world = World {
            position: *position.as_ref(),
            rotation: *rotation.as_ref(),
            scale: *scale.as_ref(),
        };
        self.drive(world, over)
    }

    /// Move the group without changing its orientation or scale.
    pub fn move_to(
        &mut self,
        position: impl AsRef<Position>,
        over: Ticks,
    ) -> Result<(), DeadMembers> {
        self.animate(position, self.rotation, self.scale, over)
    }

    /// Turn the group about its own origin.
    pub fn rotate_to(
        &mut self,
        rotation: impl AsRef<Rotation>,
        over: Ticks,
    ) -> Result<(), DeadMembers> {
        self.animate(self.position, rotation, self.scale, over)
    }

    /// Scale the group about its own origin.
    pub fn scale_to(&mut self, scale: impl AsRef<Scale>, over: Ticks) -> Result<(), DeadMembers> {
        self.animate(self.position, self.rotation, scale, over)
    }

    /// Drop every member whose entity is gone, nested groups included, and
    /// return how many were removed. The obvious way to handle
    /// [`DeadMembers`] when a group outliving some of its parts is expected.
    pub fn prune_dead(&mut self) -> usize {
        let mut removed = 0;
        self.entries.retain_mut(|entry| match entry {
            Entry::Member { target, .. } => {
                let alive = target.member_alive();
                if !alive {
                    removed += 1;
                }
                alive
            }
            Entry::Group { group, .. } => {
                removed += group.prune_dead();
                true
            }
        });
        removed
    }

    /// The world transform each member composes against.
    fn world(&self) -> World {
        World {
            position: self.position,
            rotation: self.rotation,
            scale: self.scale,
        }
    }

    /// Store `world` as this group's transform and push the composed result to
    /// every member, recursing into nested groups.
    fn drive(&mut self, world: World, over: Ticks) -> Result<(), DeadMembers> {
        let mut dead = DeadMembers::default();
        self.drive_inner(world, over, &mut Vec::new(), &mut dead);
        if dead.paths.is_empty() {
            Ok(())
        } else {
            Err(dead)
        }
    }

    fn drive_inner(
        &mut self,
        world: World,
        over: Ticks,
        path: &mut Vec<usize>,
        dead: &mut DeadMembers,
    ) {
        self.position = world.position;
        self.rotation = world.rotation;
        self.scale = world.scale;

        for (index, entry) in self.entries.iter_mut().enumerate() {
            match entry {
                Entry::Member { target, local } => {
                    let member = world.compose(local);
                    if target
                        .apply_world(&member.position, &member.rotation, &member.scale, over)
                        .is_err()
                    {
                        let mut full = path.clone();
                        full.push(index);
                        dead.paths.push(full);
                    }
                }
                Entry::Group { group, local } => {
                    let child = world.compose(local);
                    path.push(index);
                    group.drive_inner(child, over, path, dead);
                    path.pop();
                }
            }
        }
    }

    /// The nested group at `index`, if that member is a group.
    ///
    /// Its transform is whatever its parent last drove it to, so this is how you
    /// read a subassembly's current placement.
    pub fn nested(&self, index: usize) -> Option<&Group> {
        match self.entries.get(index) {
            Some(Entry::Group { group, .. }) => Some(group),
            _ => None,
        }
    }

    /// The nested group at `index`, mutably — to add members to a subassembly
    /// after building it.
    pub fn nested_mut(&mut self, index: usize) -> Option<&mut Group> {
        match self.entries.get_mut(index) {
            Some(Entry::Group { group, .. }) => Some(group),
            _ => None,
        }
    }

    /// The world transform a member at `local` would get right now — the pure
    /// composition, without touching the host. Handy for placing something
    /// alongside a group, and what the SDK's own tests check.
    pub fn world_of(&self, local: &Local) -> (Position, Rotation, Scale) {
        let w = self.world().compose(local);
        (w.position, w.rotation, w.scale)
    }
}
