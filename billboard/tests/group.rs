//! Known-answer tests for [`Group`]'s transform composition.
//!
//! Every expectation is worked out by hand from the composition rule
//!
//! ```text
//! world_pos   = group_pos + group_rot · (local_offset ⊙ group_scale)
//! world_rot   = group_rot · local_rot
//! world_scale = group_scale ⊙ local_scale
//! ```
//!
//! using quarter turns, where the rotated axes are exact: a +90° turn about +Y
//! sends +X to −Z and +Z to +X (right-hand rule).
//!
//! The composition is checked through `world_of` and through driving nested
//! groups, both of which are pure guest maths — no member entities, so no ABI.

use billboard::helpers::{Group, Local};
use billboard::math::{Degrees, Offset, Position, Rotation, Scale, Vector3d};

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn assert_pos(got: Position, x: f64, y: f64, z: f64) {
    assert!(
        approx(got.x, x) && approx(got.y, y) && approx(got.z, z),
        "expected position ({x}, {y}, {z}), got {got:?}"
    );
}

fn assert_rot(got: Rotation, want: Rotation) {
    assert!(
        approx(got.x, want.x)
            && approx(got.y, want.y)
            && approx(got.z, want.z)
            && approx(got.w, want.w),
        "expected rotation {want:?}, got {got:?}"
    );
}

fn quarter_turn_y() -> Rotation {
    Rotation::axis_angle(Vector3d::Y, Degrees::new(90.0))
}

#[test]
fn rotation_applied_to_vectors_matches_the_right_hand_rule() {
    // The primitive the composition rests on.
    let y90 = quarter_turn_y();
    let rotated = y90.rotate(Vector3d::X);
    assert!(approx(rotated.x, 0.0) && approx(rotated.y, 0.0) && approx(rotated.z, -1.0));
    let back = y90.rotate(Vector3d::Z);
    assert!(approx(back.x, 1.0) && approx(back.y, 0.0) && approx(back.z, 0.0));
    // A rotation about the axis it points along changes nothing.
    let along = y90.rotate(Vector3d::Y);
    assert!(approx(along.x, 0.0) && approx(along.y, 1.0) && approx(along.z, 0.0));

    // +90° about +X sends +Y to +Z.
    let x90 = Rotation::axis_angle(Vector3d::X, Degrees::new(90.0));
    let up = x90.rotate(Vector3d::Y);
    assert!(approx(up.x, 0.0) && approx(up.y, 0.0) && approx(up.z, 1.0));
}

#[test]
fn identity_group_leaves_locals_alone() {
    let group = Group::new(Position::new(3.0, 4.0, 5.0));
    let (position, rotation, scale) = group.world_of(&Local::at(Offset::new(1.0, 0.0, 0.0)));
    assert_pos(position, 4.0, 4.0, 5.0);
    assert_rot(rotation, Rotation::IDENTITY);
    assert_eq!(scale, Scale::splat(1.0));
}

#[test]
fn members_orbit_under_group_rotation() {
    let mut group = Group::new(Position::ZERO);
    group
        .set_transform(
            Position::new(10.0, 0.0, 0.0),
            quarter_turn_y(),
            Scale::splat(1.0),
        )
        .expect("no members to be dead");

    // local +X, turned 90° about +Y, becomes -Z — then offset by the origin.
    let (position, rotation, scale) = group.world_of(&Local::at(Offset::new(1.0, 0.0, 0.0)));
    assert_pos(position, 10.0, 0.0, -1.0);
    assert_rot(rotation, quarter_turn_y());
    assert_eq!(scale, Scale::splat(1.0));

    // local +Z becomes +X.
    let (position, _, _) = group.world_of(&Local::at(Offset::new(0.0, 0.0, 1.0)));
    assert_pos(position, 11.0, 0.0, 0.0);

    // A member sitting at the origin never moves, whatever the rotation.
    let (position, _, _) = group.world_of(&Local::IDENTITY);
    assert_pos(position, 10.0, 0.0, 0.0);
}

#[test]
fn group_scale_spreads_members_along_the_groups_own_axes() {
    let mut group = Group::new(Position::ZERO);
    group
        .set_transform(Position::ZERO, quarter_turn_y(), Scale::splat(2.0))
        .expect("no members");

    // The offset is scaled *before* it is rotated: (1,0,0) -> (2,0,0) -> (0,0,-2).
    let (position, _, scale) = group.world_of(&Local::at(Offset::new(1.0, 0.0, 0.0)));
    assert_pos(position, 0.0, 0.0, -2.0);
    // Member scale is the product of group and local scale.
    assert_eq!(scale, Scale::splat(2.0));

    let (_, _, scale) = group.world_of(&Local::new(
        Offset::ZERO,
        Rotation::IDENTITY,
        Scale::new(0.5, 3.0, 1.0),
    ));
    assert_eq!(scale, Scale::new(1.0, 6.0, 2.0));

    // Non-uniform group scale acts per axis, in group space.
    let mut oblong = Group::new(Position::ZERO);
    oblong
        .set_transform(
            Position::ZERO,
            Rotation::IDENTITY,
            Scale::new(3.0, 1.0, 1.0),
        )
        .expect("no members");
    let (position, _, _) = oblong.world_of(&Local::at(Offset::new(2.0, 1.0, 0.0)));
    assert_pos(position, 6.0, 1.0, 0.0);
}

#[test]
fn local_rotation_composes_after_the_groups() {
    let mut group = Group::new(Position::ZERO);
    group
        .set_transform(Position::ZERO, quarter_turn_y(), Scale::splat(1.0))
        .expect("no members");

    // Two quarter turns about the same axis make a half turn: (0, 1, 0, 0).
    let (_, rotation, _) = group.world_of(&Local::new(
        Offset::ZERO,
        quarter_turn_y(),
        Scale::splat(1.0),
    ));
    assert_rot(
        rotation,
        Rotation {
            x: 0.0,
            y: 1.0,
            z: 0.0,
            w: 0.0,
        },
    );
}

#[test]
fn nested_groups_compose_through() {
    // Parent turned a quarter turn about +Y, child hanging one block along the
    // parent's local +X.
    let mut parent = Group::new(Position::ZERO);
    let child = Group::new(Position::ZERO);
    parent.add_group(child, Local::at(Offset::new(1.0, 0.0, 0.0)));
    parent
        .set_transform(Position::ZERO, quarter_turn_y(), Scale::splat(1.0))
        .expect("no members");

    // The child's own transform is now whatever the parent drove it to:
    // (1,0,0) turned about +Y is (0,0,-1), and it inherits the rotation.
    let child = parent.nested(0).expect("member 0 is a group");
    assert_pos(child.position(), 0.0, 0.0, -1.0);
    assert_rot(child.rotation(), quarter_turn_y());
    assert_eq!(child.scale(), Scale::splat(1.0));

    // A member one block along the child's local +X ends up at (0,0,-2): both
    // turns point the same way, so the offsets stack along -Z.
    let (position, _, _) = child.world_of(&Local::at(Offset::new(1.0, 0.0, 0.0)));
    assert_pos(position, 0.0, 0.0, -2.0);

    // Scale compounds through the nesting: parent ×2, child local ×3 => ×6.
    let mut parent = Group::new(Position::ZERO);
    parent.add_group(
        Group::new(Position::ZERO),
        Local::new(
            Offset::new(1.0, 0.0, 0.0),
            Rotation::IDENTITY,
            Scale::splat(3.0),
        ),
    );
    parent
        .set_transform(Position::ZERO, Rotation::IDENTITY, Scale::splat(2.0))
        .expect("no members");
    let child = parent.nested(0).expect("member 0 is a group");
    // The child's offset was scaled by the parent's 2: (1,0,0) -> (2,0,0).
    assert_pos(child.position(), 2.0, 0.0, 0.0);
    assert_eq!(child.scale(), Scale::splat(6.0));
    // And a member of the child sits 6 blocks out along the scaled offset.
    let (position, _, _) = child.world_of(&Local::at(Offset::new(1.0, 0.0, 0.0)));
    assert_pos(position, 8.0, 0.0, 0.0);
}

#[test]
fn move_rotate_scale_keep_the_other_two_components() {
    let mut group = Group::new(Position::ZERO);
    group
        .set_transform(
            Position::new(1.0, 2.0, 3.0),
            quarter_turn_y(),
            Scale::splat(2.0),
        )
        .expect("no members");

    group
        .move_to(
            Position::new(4.0, 5.0, 6.0),
            billboard::math::Ticks::new(10),
        )
        .expect("no members");
    assert_pos(group.position(), 4.0, 5.0, 6.0);
    assert_rot(group.rotation(), quarter_turn_y());
    assert_eq!(group.scale(), Scale::splat(2.0));

    group
        .scale_to(Scale::splat(0.5), billboard::math::Ticks::new(0))
        .expect("no members");
    assert_eq!(group.scale(), Scale::splat(0.5));
    assert_pos(group.position(), 4.0, 5.0, 6.0);

    group
        .rotate_to(Rotation::IDENTITY, billboard::math::Ticks::new(0))
        .expect("no members");
    assert_rot(group.rotation(), Rotation::IDENTITY);
    assert_eq!(group.scale(), Scale::splat(0.5));
}

#[test]
fn membership_bookkeeping() {
    let mut group = Group::new(Position::ZERO);
    assert!(group.is_empty());
    assert_eq!(group.len(), 0);
    assert!(group.nested(0).is_none());

    group.add_group(Group::new(Position::ZERO), Local::IDENTITY);
    assert_eq!(group.len(), 1);
    assert!(!group.is_empty());
    assert!(group.nested(0).is_some());

    // Nested groups are never pruned (they are guest data, not entities), and
    // an empty group has nothing dead in it.
    assert_eq!(group.prune_dead(), 0);
    assert_eq!(group.len(), 1);

    // Subassemblies can be filled in after nesting.
    group
        .nested_mut(0)
        .expect("nested group")
        .add_group(Group::new(Position::ZERO), Local::IDENTITY);
    assert_eq!(group.nested(0).expect("nested").len(), 1);
}

#[test]
fn quaternion_interpolation_takes_the_short_way() {
    // Halfway between no rotation and a half turn about +Y is a quarter turn:
    // nlerp((0,0,0,1), (0,1,0,0), 0.5) = (0, .5, 0, .5) normalized.
    let half_turn = Rotation {
        x: 0.0,
        y: 1.0,
        z: 0.0,
        w: 0.0,
    };
    assert_rot(Rotation::IDENTITY.lerp(half_turn, 0.5), quarter_turn_y());

    // Endpoints are exact.
    assert_rot(Rotation::IDENTITY.lerp(half_turn, 0.0), Rotation::IDENTITY);
    assert_rot(Rotation::IDENTITY.lerp(half_turn, 1.0), half_turn);

    // A quaternion and its negation are the *same* orientation. Blending
    // towards the negated form must stay put rather than travelling the long
    // way round (which would also pass through a zero quaternion at t = 0.5).
    let negated_identity = Rotation {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: -1.0,
    };
    assert_rot(
        Rotation::IDENTITY.lerp(negated_identity, 0.5),
        Rotation::IDENTITY,
    );
}
