//! **Billboard v2 demo — "NOW SHOWING".**
//!
//! A panel assembles itself, a colour ramp sweeps across a strip of blocks
//! chosen by perceptual nearest-match, a logo orbits as one rigid group, a
//! sword flies through an eased keyframe timeline, an armor stand takes a bow,
//! and dust traces a circle in mid-air — with five tasks kept in step by a
//! barrier, a signal, a composite wait and a channel.
//!
//! This file is the SDK's worked example *and* the plugin's integration-test
//! fixture, so it is choreographed and commented rather than clever.
//!
//! # Timing — total scripted run: **603 ticks ≈ 30.2 s** (task 0's own sleeps)
//!
//! | Section | Ticks | Ends at |
//! |---|---|---|
//! | A. panel assembly | 50 | 50 |
//! | B. colour-ramp strip reveal (16 × 2) | 32 | 82 |
//! | C. barrier rendezvous (no sleeping) | 0 | 82 |
//! | D. conductor's cue (2 + 18) | 20 | 102 |
//! | E. headline typewriter (11 chars × 3) | 33 | 135 |
//! | F. ramp sweep (30 × 2) | 60 | 195 |
//! | G. logo group orbit (4 × 20) | 80 | 275 |
//! | H. logo group scale pulse (2 × 15) | 30 | 305 |
//! | I. sword timeline (keyframed, eased) | 60 | 365 |
//! | J. channel waypoints (4 × 15) | 60 | 425 |
//! | K. spotlight, then the bow | 90 | 515 |
//! | L. joins (performers already finished) | 0 | 515 |
//! | M. lamp checkpoint restore | 10 | 525 |
//! | N. farewell marquee (19 frames × 2) | 38 | 563 |
//! | O. burst and settle (20 + 20) | 40 | **603** |
//!
//! The performers run inside that window and are done before the joins: the
//! lamp pulse ends at ~156, the marquee at ~180, the dust orbit at ~204, the
//! waypoint runner at ~422, the bow at ~493.
//!
//! # Determinism
//!
//! `random_seed` is set, so `default_random()` draws from the host's
//! *deterministic* stream and nothing here ever calls the non-deterministic
//! one — the host-call trace is reproducible, which is what makes this usable
//! as a fixture. Per-task randomness comes from [`SplitRng`] splits taken
//! *before* spawning: a fork copies memory, so an unsplit generator would
//! replay the same sequence in every task.
//!
//! One note for whoever asserts on the trace: `random_nondet` still appears in
//! the module's **import** list, because `default_random()` picks its stream at
//! run time and both arms are compiled in. It is never *called* — the seeded
//! flag is set during init, before this function's first line. Assert on calls,
//! not on imports.

use billboard::prelude::*;

const PANEL_W: i64 = 5;
const PANEL_H: i64 = 3;
/// Blocks in the colour-ramp strip.
const STRIP_W: usize = 16;
/// Performers plus the conductor, all meeting at the barrier.
const CAST: u32 = 6;

/// Typed out one character at a time (11 characters).
const HEADLINE: &str = "NOW SHOWING";
/// Scrolled in the finale (19 characters ⇒ 19 marquee frames).
const FAREWELL: &str = "THANKS FOR WATCHING";

billboard::payload! {
    /// A waypoint handed to the runner task through a channel.
    ///
    /// `payload!` makes it `#[repr(C)]` and `Pod`, which is what lets it cross:
    /// every field is plain data with a known layout, so the bytes mean the same
    /// thing in the receiving task's *copy* of memory. A `String` or a `Vec` in
    /// here would be a compile error — rightly, since its heap would not exist
    /// over there. 24 + 8 + 16 = 48 bytes, no padding.
    struct Waypoint {
        /// Where to go next.
        target: Position,
        /// How long to take getting there.
        over: Ticks,
        /// A private random stream for the runner to sparkle with — a generator
        /// small and plain enough to post through a channel.
        sparkle: SplitRng,
    }
}

#[billboard::main(random_seed = 20_260_726)]
fn main() -> ExitCode {
    log("demo v2: places, everyone");

    // One master stream, split per performer *before* any spawn. RANDOM_SEED
    // is the `random_seed` literal above, re-emitted by the entry macro.
    let mut master = SplitRng::new(RANDOM_SEED as u64);
    let mut lamp_rng = master.split();
    let mut orbit_rng = master.split();
    let mut runner_rng = master.split();

    // The host's deterministic stream, for scenery decisions.
    let mut scenery = default_random();

    let origin = Position::new(0.0, 4.0, 0.0);

    // ---------------------------------------------------------------- A ----
    // The panel glides up into place, one column slightly after another. Block
    // ids come from the generated registry, so a typo is a compile error; the
    // fiddly ones are built with typed state properties.
    let mut panel: Vec<BlockDisplay> = Vec::new();
    for x in 0..PANEL_W {
        for y in 0..PANEL_H {
            let target = Position::new((x - PANEL_W / 2) as f64, y as f64 + 3.0, 0.0);
            let mut block =
                BlockDisplay::spawn(blocks::GRAY_CONCRETE, target - Offset::new(0.0, 4.0, 0.0));
            block.move_to(target, Ticks::new(20 + 5 * x as u64));
            panel.push(block);
        }
    }

    // Trim: stairs along the top edge (facing + half), a lit furnace as the
    // projector, a repeater whose rare property goes in as a string, and a
    // pillar with an explicit axis.
    //
    // Note these are bound to `_name`, not `_`: `let _ = BlockDisplay::spawn(…)`
    // would drop the handle immediately and despawn the entity on the spot.
    let mut trim: Vec<BlockDisplay> = Vec::new();
    for x in 0..PANEL_W {
        let facing = if x < PANEL_W / 2 {
            Facing::West
        } else {
            Facing::East
        };
        trim.push(BlockDisplay::spawn(
            blocks::OAK_STAIRS.state().facing(facing).half(Half::Top),
            Position::new((x - PANEL_W / 2) as f64, 6.0, 0.0),
        ));
    }
    let _projector = BlockDisplay::spawn(
        blocks::FURNACE.state().facing(Facing::North).lit(true),
        Position::new(-3.0, 3.0, 0.5),
    );
    let _timing = BlockDisplay::spawn(
        blocks::REPEATER
            .state()
            .facing(Facing::West)
            .with("delay", "3"),
        Position::new(3.0, 3.0, 0.5),
    );
    let _mast = BlockDisplay::spawn(
        blocks::OAK_LOG.state().axis(Axis::Y),
        Position::new(0.0, 7.5, 0.0),
    );

    sleep(Ticks::new(50));

    // ---------------------------------------------------------------- B ----
    // The colour→block showcase: sample a gradient in Oklab, then ask the
    // concrete palette which block is perceptually closest to each sample.
    let ramp = Gradient::new([
        (0.0, Color::hex("#2c2e8f")),
        (0.45, Color::hex("#e06100")),
        (1.0, Color::hex("#f1af15")),
    ]);
    let palette = BlockPalette::CONCRETE;
    let mut strip: Vec<BlockDisplay> = Vec::new();
    for i in 0..STRIP_W {
        let t = i as f64 / (STRIP_W - 1) as f64;
        let block = palette.nearest(ramp.sample(t));
        let at = Position::new(i as f64 - 7.5, 2.0, 0.6);
        let mut tile = BlockDisplay::spawn(block, at);
        // Each tile lands as a thin sliver.
        tile.set_scale(Scale::new(1.0, 0.4, 0.4));
        // A puff of the tile's own texture as it appears.
        particle(Particle::block(block), at)
            .count(4)
            .offset(Offset::splat(0.2))
            .speed(0.01)
            .emit();
        strip.push(tile);
        sleep(Ticks::new(2));
    }

    // The cast. Owner handles are `!Sync` and cannot be captured by a task, so
    // the performers get weak references — the compiler enforces it.
    let mut lamp = BlockDisplay::spawn(blocks::SEA_LANTERN, Position::new(0.0, 4.0, 0.45));
    // Checkpoint the lamp fresh from the host, to restore it at the end.
    let resting = lamp.state();

    let mut headline = TextDisplay::spawn("", Position::new(0.0, 7.0, 0.0));
    headline.set_background(Color::rgba(8, 8, 12, 160));
    headline.set_billboard_mode(BillboardMode::Center);
    // `set_shadow` flips one bit of a shared mask, so it reads the current mask
    // from the host first — the SDK caches nothing. `set_flags` would set all
    // three in one round trip.
    headline.set_shadow(true);
    headline.set_line_width(240);
    // A checkpoint of the whole text state, restored at the very end.
    let headline_resting = headline.state();

    let mut sword = ItemDisplay::spawn(items::DIAMOND_SWORD, Position::new(-4.0, 4.0, 1.0));
    sword.set_context(DisplayContext::ThirdPersonRightHand);
    sword.set_scale(Scale::splat(1.5));

    let mut usher = ArmorStand::spawn(Position::new(4.0, 2.0, 1.5));
    usher.set_flags(StandFlags {
        small: false,
        arms: true,
        no_baseplate: true,
        invisible: false,
    });
    usher.set_equipment(EquipmentSlot::Helmet, items::PLAYER_HEAD);
    usher.set_equipment(EquipmentSlot::MainHand, items::NETHERITE_SWORD);
    usher.set_equipment(EquipmentSlot::OffHand, items::TORCH);

    // Packet-only, so it can never be picked up or merged away.
    let mut token = Item::spawn(items::EMERALD, Position::new(0.0, 2.6, 1.2));

    let runner = BlockDisplay::spawn(blocks::REDSTONE_BLOCK, Position::new(-6.5, 6.5, 0.8));

    // ---------------------------------------------------------------- C ----
    // Choreography objects. All host-side and addressed by a plain integer, so
    // they are `Sync + Copy` and survive the fork into every task for free.
    let ready = Barrier::new(CAST);
    let cue = Signal::new();
    let spotlight = Signal::new();
    let curtain = Signal::new();
    // A chained boolean tree: the usher bows once the show has started *and*
    // either the spotlight or an early curtain call has come. Composites latch
    // per waiter, so the arms may fire in any order — provided the waiter is
    // already parked, which section D's opening sleep is there to guarantee.
    let bow_when = curtain.or(&spotlight).and(&cue);

    // Bounded MPSC: main is the only producer, the runner the only consumer.
    // The receiver is not `Clone`, so moving it into the runner is what makes
    // "single consumer" a fact rather than a promise.
    let (waypoints, incoming) = channel::<Waypoint>(4);

    // Performer 1 — the lamp pulses, driven through a weak reference.
    let pulse_task = spawn({
        let mut pulse = lamp.weak_mut();
        move || {
            ready.wait();
            cue.wait();
            let big: &Scale = &Scale::splat(1.6);
            let small: &Scale = &Scale::splat(1.0);
            for _ in 0..6 {
                pulse.scale_to(big, Ticks::new(5)).expect("lamp alive");
                // Jitter from this task's own stream, so the pulses are not
                // metronomic — and reproducible, because the split happened
                // before the spawn. Each iteration is exactly 12 ticks.
                let hold: u64 = lamp_rng.range(4..8);
                sleep(Ticks::new(hold));
                pulse.scale_to(small, Ticks::new(5)).expect("lamp alive");
                sleep(Ticks::new(12 - hold));
            }
        }
    });

    // Performer 2 — a scrolling sign, leaked so it outlives this scope and is
    // driven entirely from the task.
    let marquee_task = spawn({
        let mut sign = TextDisplay::spawn("", Position::new(0.0, 1.2, 0.6)).leak();
        move || {
            ready.wait();
            cue.wait();
            // The helper slices by character, never by byte, so the "·" comes
            // through whole. Styling stays outside the sliced text.
            text::marquee_weak(
                &mut sign,
                "** NOW SHOWING · BILLBOARD v2 **",
                12,
                Ticks::new(3),
                1,
            )
            .expect("sign alive");
        }
    });

    // Performer 3 — dust traces a circle in the air, coloured along the same
    // ramp, with the occasional spark thrown along the tangent.
    let orbit_task = spawn({
        let trail = ramp.clone();
        let centre = origin + Offset::new(0.0, 1.0, 2.0);
        move || {
            ready.wait();
            cue.wait();
            let circle = Path::circle(centre, 4.0, Vector3d::Y);
            for step in 0..60u32 {
                let t = step as f64 / 60.0;
                let at = circle.sample(t);
                particle(
                    Particle::Dust {
                        color: trail.sample(t),
                        size: 1.2,
                    },
                    at,
                )
                .count(3)
                .offset(Offset::splat(0.05))
                .speed(0.01)
                .emit();
                // Shed forwards along the direction of travel.
                if orbit_rng.chance(0.35) {
                    let ahead = at + circle.tangent(t) * 0.02;
                    particle(Particle::named("minecraft:end_rod"), ahead)
                        .count(1)
                        .speed(0.0)
                        .emit();
                }
                sleep(Ticks::new(2));
            }
        }
    });

    // Performer 4 — the usher waits on the composite, then waves and turns.
    let bow_task = spawn({
        let mut stand = usher.weak_mut();
        move || {
            ready.wait();
            bow_when.wait();
            let up = (Degrees::new(-120.0), Degrees::new(0.0), Degrees::new(-20.0));
            let down = (Degrees::new(-15.0), Degrees::new(0.0), Degrees::new(10.0));
            for _ in 0..3 {
                stand
                    .animate_pose_part(PosePart::RightArm, up, Ticks::new(8))
                    .expect("usher alive");
                sleep(Ticks::new(8));
                stand
                    .animate_pose_part(PosePart::RightArm, down, Ticks::new(8))
                    .expect("usher alive");
                sleep(Ticks::new(8));
            }
            // A slow turn to face the crowd, tweened host-side: armor stands
            // have no client interpolation at all.
            stand
                .turn_to(Degrees::new(180.0), Ticks::new(20))
                .expect("usher alive");
            sleep(Ticks::new(20));
        }
    });

    // Performer 5 — the runner takes its next target off the channel, parking
    // whenever the queue is empty.
    let runner_task = spawn({
        let mut mover = runner.weak_mut();
        move || {
            ready.wait();
            for _ in 0..4 {
                let mut waypoint = incoming.recv();
                mover
                    .move_to(waypoint.target, waypoint.over)
                    .expect("runner alive");
                // Peek ahead without consuming: if the next waypoint is already
                // queued, keep the trail bright instead of letting it thin out.
                let more_coming = incoming.try_peek().is_some();
                // The stream that arrived *inside the payload* decides the
                // trail colour.
                let bright = waypoint.sparkle.next_f64();
                particle(
                    Particle::Dust {
                        color: Color::WHITE.lerp(Color::hex("#f1af15"), bright),
                        size: 0.8,
                    },
                    waypoint.target,
                )
                .count(if more_coming { 8 } else { 4 })
                .offset(Offset::splat(0.25))
                .speed(0.02)
                .emit();
                sleep(waypoint.over);
            }
        }
    });

    // Everyone is spawned; the conductor is the last to arrive, which releases
    // the whole cast together, in spawn order.
    ready.wait();

    // ---------------------------------------------------------------- D ----
    // Two ticks for the cast to park on their cues before the cue is given: a
    // signal is an event, not a counter, so a notify with nobody parked is
    // simply lost. This is the one piece of timing an animation has to think
    // about for itself.
    sleep(Ticks::new(2));
    sound("minecraft:block.note_block.pling", origin)
        .volume(1.5)
        .pitch(1.2)
        .category(SoundCategory::Record)
        .play();
    cue.notify_all();
    log("demo v2: cue given");
    sleep(Ticks::new(18));

    // ---------------------------------------------------------------- E ----
    // The headline types itself out, with the styling wrapped around every
    // frame so the MiniMessage tags are never sliced.
    text::typewriter_styled(
        &mut headline,
        "<bold><gradient:#f1af15:#e06100>",
        "</gradient></bold>",
        HEADLINE,
        Ticks::new(3),
    );

    // ---------------------------------------------------------------- F ----
    // The ramp sweeps: every tile re-picks its nearest block as the gradient
    // slides underneath it.
    //
    // Sixteen colour lookups inside one tick is the busiest moment in this
    // animation, and it is worth knowing why it fits: both the gradient's stops
    // and the palette's entries carry their Oklab already, so a lookup costs one
    // conversion out to sRGB and one back, not sixty-five. If you are doing this
    // per pixel of something big, stay in Oklab the whole way with
    // `palette.nearest_to_oklab(ramp.sample_oklab(t))` and pay neither.
    for step in 0..30u32 {
        let phase = step as f64 / 30.0;
        for (i, tile) in strip.iter_mut().enumerate() {
            let t = (i as f64 / STRIP_W as f64 + phase).fract();
            tile.set_block(palette.nearest(ramp.sample(t)));
        }
        sleep(Ticks::new(2));
    }
    sound("minecraft:block.note_block.chime", origin)
        .volume(0.8)
        .pitch(1.6)
        .category(SoundCategory::Block)
        .play();

    // ---------------------------------------------------------------- G ----
    // The logo: five displays welded into a group and moved as one object.
    // Members orbit under the group's rotation because their local offsets are
    // rotated with it.
    let logo_colours = [
        Color::hex("#b02e26"),
        Color::hex("#f1af15"),
        Color::hex("#5ea918"),
        Color::hex("#2489c7"),
    ];
    let mut logo = Group::new(origin + Offset::new(0.0, 2.0, 2.5));
    // The owners live here; the group only holds weak references.
    let mut logo_parts: Vec<BlockDisplay> = Vec::new();
    let hub = BlockDisplay::spawn(blocks::QUARTZ_BLOCK, logo.position());
    logo.add(hub.weak_mut(), Local::IDENTITY);
    logo_parts.push(hub);
    for (i, colour) in logo_colours.iter().enumerate() {
        // One arm per quadrant, placed with a zero-width arc — the tidy way to
        // ask "where is the point at this angle on this circle?".
        let angle = Degrees::new(90.0 * i as f64);
        let at = Path::arc(logo.position(), 1.2, Vector3d::Y, angle, angle).sample(0.0);
        let arm = BlockDisplay::spawn(palette.nearest(*colour), at);
        logo.add(
            arm.weak_mut(),
            Local::new(
                at - logo.position(),
                Rotation::axis_angle(Vector3d::Y, angle),
                Scale::splat(0.6),
            ),
        );
        logo_parts.push(arm);
    }
    // Snap everyone into their computed places, then turn the whole assembly a
    // full circle in four quarter turns. A dead member would come back in the
    // `Err` with its index — here they are all owned right above, so `expect`
    // is honest.
    logo.apply().expect("logo intact");
    for quarter in 1..=4u32 {
        logo.rotate_to(
            Rotation::axis_angle(Vector3d::Y, Degrees::new(90.0 * quarter as f64)),
            Ticks::new(20),
        )
        .expect("logo intact");
        sleep(Ticks::new(20));
    }

    // ---------------------------------------------------------------- H ----
    // Scaling the group spreads its members along the group's own axes.
    logo.scale_to(Scale::splat(1.8), Ticks::new(15))
        .expect("logo intact");
    sleep(Ticks::new(15));
    logo.scale_to(Scale::splat(1.0), Ticks::new(15))
        .expect("logo intact");
    sleep(Ticks::new(15));

    // ---------------------------------------------------------------- I ----
    // The sword flies through three keyframes with real easing. Minecraft only
    // interpolates linearly, so the timeline sub-steps each eased segment (two
    // ticks per chunk by default) and blocks for exactly its 60-tick duration.
    let base = sword.state();
    let lifted = ItemDisplayState {
        position: Position::new(0.0, 8.5, 1.5),
        rotation: Rotation::axis_angle(Vector3d::Z, Degrees::new(180.0)),
        scale: Scale::splat(2.2),
        ..base.clone()
    };
    let landed = ItemDisplayState {
        position: Position::new(4.0, 3.0, 1.0),
        rotation: Rotation::axis_angle(Vector3d::Y, Degrees::new(90.0)),
        scale: Scale::splat(1.2),
        context: DisplayContext::Ground,
        ..base.clone()
    };
    Timeline::new()
        .key(Ticks::new(0), base.clone())
        .key(Ticks::new(25), lifted)
        .ease(Ease::CubicOut)
        .key(Ticks::new(60), landed)
        .ease(Ease::BounceOut)
        .play(&mut sword);

    // ---------------------------------------------------------------- J ----
    // Four waypoints posted to the runner, each carrying a position, a
    // duration, and its own random stream.
    let lane = 6.5;
    for step in 0..4u32 {
        let x = if step % 2 == 0 { lane } else { -lane };
        waypoints.send(Waypoint {
            target: Position::new(x, 6.5, 0.8),
            over: Ticks::new(12),
            sparkle: runner_rng.split(),
        });
        // The token bobs along with the traffic.
        token.move_to(
            Position::new(0.0, if step % 2 == 0 { 3.2 } else { 2.6 }, 1.2),
            Ticks::new(12),
        );
        sleep(Ticks::new(15));
    }

    // ---------------------------------------------------------------- K ----
    // The spotlight completes the usher's composite: the `and` arm (the cue)
    // fired long ago and was latched, so this is what releases the bow.
    particle(Particle::named("minecraft:end_rod"), usher.position())
        .count(12)
        .offset(Offset::splat(0.4))
        .speed(0.05)
        .emit();
    spotlight.notify_all();
    log("demo v2: spotlight on the usher");
    sleep(Ticks::new(90));

    // ---------------------------------------------------------------- L ----
    // Everyone has finished their number by now; join in curtain-call order.
    pulse_task.join();
    marquee_task.join();
    orbit_task.join();
    bow_task.join();
    runner_task.join();

    // ---------------------------------------------------------------- M ----
    // The lamp drifted mid-pulse; the checkpoint puts it back exactly.
    let drifted = lamp.scale();
    log(&format!(
        "demo v2: lamp scale before restore = {:.2}",
        drifted.x
    ));
    lamp.set(&resting);
    // Host-truth reads on the armor stand: its getters report the *target* of a
    // host-side tween, exactly as a display's report the target of a client one.
    let facing = usher.yaw();
    let waving_arm = usher.pose_part(PosePart::RightArm);
    log(&format!(
        "demo v2: usher yaw {:.0}deg, right arm x {:.0}deg",
        facing.value(),
        waving_arm.0.value()
    ));
    sleep(Ticks::new(10));

    // ---------------------------------------------------------------- N ----
    // A farewell scroll, and one strip tile flipped to a lit lamp for
    // punctuation.
    if let Some(tile) = strip.first_mut() {
        tile.set_block(blocks::REDSTONE_LAMP.state().lit(true));
    }
    headline.set_text("<italic><gray>that's all, folks");
    let mut sign = TextDisplay::spawn("", Position::new(0.0, 1.2, 0.6));
    sign.set_billboard_mode(BillboardMode::Center);
    sign.set_flags(TextFlags {
        shadow: true,
        see_through: true,
        default_background: false,
    });
    text::marquee(&mut sign, FAREWELL, 10, Ticks::new(2), 1);

    // ---------------------------------------------------------------- O ----
    // A two-colour burst that fades as it rises, a note picked from the host's
    // deterministic stream, and the logo folds away.
    let burst_at = origin + Offset::new(0.0, 1.0, 1.0);
    // An operator can retint the finale via `/billboard env`; unset, this is
    // the same navy the demo has always closed on.
    let burst_to = billboard::env::get("burst_color").unwrap_or("#2c2e8f");
    particle(
        Particle::DustTransition {
            from: Color::hex("#f1af15"),
            to: Color::hex(burst_to),
            size: 1.6,
        },
        burst_at,
    )
    .count(80)
    .offset(Offset::new(3.0, 1.5, 0.5))
    .speed(0.08)
    .emit();
    let note = *scenery.choose(&[
        "minecraft:block.note_block.bell",
        "minecraft:block.note_block.bass",
    ]);
    sound(note, burst_at)
        .volume(2.0)
        .pitch(0.6 + scenery.next_f64() * 0.2)
        .category(SoundCategory::Record)
        .play();
    sleep(Ticks::new(20));

    logo.scale_to(Scale::splat(0.1), Ticks::new(20))
        .expect("logo intact");
    // The headline goes back to exactly the state checkpointed in section B —
    // text, background, opacity, width and flags in one apply.
    headline.set(&headline_resting);
    // One shared rotation, applied to the whole panel by reference.
    let lean: &Rotation = &Rotation::axis_angle(Vector3d::X, Degrees::new(-20.0));
    for part in panel.iter_mut().chain(trim.iter_mut()) {
        part.rotate_to(lean, Ticks::new(20));
    }
    sleep(Ticks::new(20));

    log("demo v2: goodnight");
    // Returning ends the animation. `End` clears everything the instance ever
    // spawned — the panel, the strip, the whole cast, and the sign that was
    // leaked into the marquee task.
    ExitCode::End
}

/// The token test that earns this crate its `rlib`.
///
/// Nothing here touches an entity — the host stubs would panic — so it is the
/// half of an animation that *is* testable natively: the constants and the
/// pure helpers that decide how long the scene runs. The comments above claim
/// specific frame counts; this is what stops them being lies after an edit.
///
/// A real animation puts its model and its schedule in modules of their own and
/// tests them the same way.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_effects_run_for_the_lengths_the_comments_claim() {
        // "NOW SHOWING" is 11 characters, so the typewriter plays 11 frames.
        assert_eq!(HEADLINE.chars().count(), 11);
        assert_eq!(text::typewriter_frames(HEADLINE).len(), 11);

        // "THANKS FOR WATCHING" is 19 characters and the marquee emits one
        // frame per character of the source, whatever the window.
        assert_eq!(FAREWELL.chars().count(), 19);
        assert_eq!(text::marquee_frames(FAREWELL, 12).len(), 19);
    }

    #[test]
    fn the_panel_and_strip_are_the_sizes_the_entity_budget_assumes() {
        // 5 x 3 = 15 panel blocks, plus a 16-tile strip: the numbers the
        // crate-root budget note quotes as "about 50 entities at once".
        assert_eq!(PANEL_W * PANEL_H, 15);
        assert_eq!(STRIP_W, 16);
        assert_eq!(CAST, 6);
    }
}
