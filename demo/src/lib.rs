//! Demo animation: a small billboard panel assembles itself, a marquee
//! runner sweeps across it while the center lamp pulses in a spawned task,
//! then everything settles and the show ends.
//!
//! Exercises: spawn, move/scale interpolation, host-queried getters
//! (`state`), checkpoint save/restore, block flipbook, task spawn/join,
//! weak references across tasks, and shared `&`-borrowed math params reused
//! across many entities.

use billboard::prelude::*;

const PANEL_W: i64 = 5;
const PANEL_H: i64 = 3;

#[billboard::main]
fn main() -> ExitCode {
    log("demo: assembling panel");

    // The panel glides up into place, one column slightly after another.
    let mut panel: Vec<BlockDisplay> = Vec::new();

    for x in 0..PANEL_W {
        for y in 0..PANEL_H {
            let target = Position::new((x - PANEL_W / 2) as f64, y as f64 + 3.0, 0.0);
            let mut block = BlockDisplay::spawn(
                "minecraft:gray_concrete",
                target - Offset::new(0.0, 4.0, 0.0),
            );
            block.move_to(target, Ticks::new(20 + 5 * x as u64));
            panel.push(block);
        }
    }

    sleep(Ticks::from_secs(2.5));

    // Center lamp, pulsed from a spawned task. The owner handle is !Sync and
    // can't be captured by the closure — the WeakMut is what crosses over.
    let mut lamp = BlockDisplay::spawn("minecraft:sea_lantern", Position::new(0.0, 4.0, 0.4));
    // Checkpoint the lamp fresh from the host, to restore it later.
    let resting = lamp.state();
    let pulse_task = spawn({
        let mut pulse = lamp.weak_mut();
        move || {
            log("demo: pulse task running");
            // One Scale each for the two poses, held by reference and reused
            // across every pulse — `scale_to` takes `impl AsRef<Scale>`, so a
            // shared `&Scale` applies without a per-call copy.
            let big: &Scale = &Scale::splat(1.5);
            let small: &Scale = &Scale::splat(1.0);
            for _ in 0..6 {
                pulse
                    .scale_to(big, Ticks::new(5))
                    .expect("lamp gone mid-pulse");
                sleep(Ticks::new(6));
                pulse
                    .scale_to(small, Ticks::new(5))
                    .expect("lamp gone mid-pulse");
                sleep(Ticks::new(6));
            }
        }
    });

    // Meanwhile: a marquee runner sweeps left-right across the panel top,
    // flipping color at each turn. The two turn-points are borrowed and
    // reused every pass.
    let edge = (PANEL_W / 2) as f64;
    let left = Position::new(-edge, 6.5, 0.4);
    let right = Position::new(edge, 6.5, 0.4);
    let mut runner = BlockDisplay::spawn("minecraft:red_concrete", left);
    for pass in 0..4u32 {
        let to = if pass % 2 == 0 { &right } else { &left };
        runner.move_to(to, Ticks::new(15));
        sleep(Ticks::new(15));
        runner.set_block(if pass % 2 == 0 {
            "minecraft:yellow_concrete"
        } else {
            "minecraft:red_concrete"
        });
    }
    runner.despawn();

    pulse_task.join();
    // Read the lamp's live scale straight from the host before restoring.
    let drifted = lamp.scale();
    log(&format!("demo: lamp scale before restore = {}", drifted.x));
    lamp.set(&resting); // restore the lamp exactly as spawned
    log("demo: finale");

    // Finale: the whole panel leans back, row by row. One Rotation, shared by
    // reference across every block — AsRef means no per-entity clone.
    let lean: &Rotation = &Rotation::axis_angle(Vector3d::X, Degrees::new(-15.0));
    for (i, block) in panel.iter_mut().enumerate() {
        block.rotate_to(lean, Ticks::new(10));
        if i % 3 == 0 {
            sleep(Ticks::new(2));
        }
    }
    sleep(Ticks::from_secs(1.0));

    // Returning ends the animation; `End` tells the host to clear everything
    // (every remaining entity — panel, lamp — plus any leaked entities).
    ExitCode::End
}
