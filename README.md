# billboard-rs

The `billboard` Rust crate — the SDK for authoring
[Billboard](https://github.com/jhuanglululu/Billboard) animations: display
entities, sounds and particles, a compile-time-checked block/item registry,
`ExitCode`, and visual helpers (color/Oklab, easing, timelines, paths,
groups, grids). `#[billboard::main]` is the entry point; tasks, sync,
randomness and math come from the
[`wasmachine`](https://github.com/jhuanglululu/wasmachine-rs) guest core
(cargo git dependency), re-exported so animations depend on this crate
alone.

`demo/` is the worked example — build with
`cargo build --release --target wasm32-unknown-unknown` and drop the
`.wasm` into `plugins/Billboard/animations/`.

Personal-use library: versioned by git, no publishing pipeline.

## Where to look things up

Three places, in the order they are usually wanted:

1. **`cargo doc --open -p billboard`** — the reference, and the only complete
   one. Start at the crate root: it states the coordinate frame your
   `Position`s live in (origin-relative, world-aligned axes) and the per-tick
   instruction, memory and audience budgets the host gives an animation.
   Every entity module opens with its geometry — a `BlockDisplay`'s position is
   its low *corner*, a `TextDisplay`'s text is *centred* on its position —
   which is the difference between a grid that lines up and one that does not.
2. **`demo/src/lib.rs`** — the cookbook. One choreographed scene that uses
   essentially the whole surface (all five entity kinds, groups, timelines,
   paths, palettes, tasks, scopes, barriers, signals, channels, environ,
   player snapshots, seeded randomness),
   with a tick-by-tick table at the top and comments explaining *why* each
   piece is shaped the way it is. Copy from it rather than from memory.
3. **`billboard/tests/`** — behaviour, pinned. Known-answer tests written by
   hand: which host call each operation makes and with what arguments
   (`entity_wire.rs`, `effects.rs`), what `Group` composition produces
   (`group.rs`), what a `Grid`'s layout maths works out to (`grid.rs`), plus
   easings, colours, paths, timelines and the registry. When the docs and your
   intuition disagree, these say who is right.

The plugin's own design docs (`Billboard/context/designs/`) cover the host
side: the runtime, the error philosophy, the ABI.

## Building

```sh
cargo test                  # native: helpers, layout maths, wire format
cargo clippy --all-targets  # zero warnings expected
cargo build --release --target wasm32-unknown-unknown -p demo
```

The ABI is `wasm32-unknown-unknown`-only; on a native target the host calls
are stubs that panic, which is why the tests stay on the pure-guest side of
the SDK.

### Required link flag

The tasks of one instance share a single linear memory, and the host gives
each new task its own stack region inside it by writing the guest's shadow
stack pointer — so a module must export that global:

```toml
# .cargo/config.toml
[target.wasm32-unknown-unknown]
rustflags = ["-C", "link-arg=--export=__stack_pointer"]
```

This workspace carries it in `.cargo/config.toml` at the root (cargo reads
that file from the invocation directory upwards, and `demo` is built from
here); an animation crate of its own copies the file to its own root. Miss
it and the plugin refuses to construct the instance, with an error naming
the flag. Animations also build `panic = "abort"`, which the workspace
release profile here already sets.

### Make your animation crate testable

An animation is a `cdylib` because that is what the plugin loads — but a
`cdylib` alone cannot be linked by a test binary, so `cargo test` in an
animation crate finds nothing to run. Add `rlib` alongside it:

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

That buys the whole pure-logic half of an animation: the model, the
schedule, the layout arithmetic, anything that computes rather than spawns.
It is testable natively because the SDK's host stubs implement the math
kernel with Rust's own `f64` methods, so `sin`, `pow`, `exp` and friends give
the same answers off-target — only the entity and effect calls panic. Keep
the arithmetic in its own module, put `#[cfg(test)] mod tests` next to it,
and leave the entity driving to the `#[billboard::main]` side. `demo/` is set
up this way; an animation written against this SDK caught a real bug in its
own maths from such a test, before it was ever loaded into a server.

**It is not free, despite what you may have been told.** Asking for a second
crate type costs the cdylib its fat LTO, so the shipped module grows: `demo`
measures **143,582 bytes with the rlib against 98,452 without**, same source,
both at the workspace's `lto = true` / `opt-level = "s"`. (With `lto = false`
the two land within a kilobyte of each other, which is how you can tell LTO
is the cause and not the extra code.) Losing fat LTO also loses cross-crate
inlining, so a module built this way plausibly executes somewhat *more*
interpreted instructions per tick as well — that part is unmeasured, and
worth remembering only if an animation is already near the instruction
budget. Take the rlib by default; delete the line for a shipping build if an
animation is big or hot enough for either cost to matter.

## The prelude, item by item

`use billboard::prelude::*;` brings in everything below. The reference is
`cargo doc`; this is the map.

**Entry point and runtime** — `main` (the `#[billboard::main]` attribute),
`ExitCode` (how the run ends, and how the host cleans up), `log`,
`sleep`, `spawn` (start a task; it may take ownership of anything moved into
it, since tasks share one memory), `Task` (its handle: `join`/`kill`),
`scope` (tasks that *borrow* from the spawner, all joined before it returns),
and `environ` (the operator's read-only key/value settings for this run).

**Math** — `Position` (origin-relative point), `Offset` (a displacement
between them), `Vector3d` / `Vector3i`, `Velocity`, `Scale`, `Rotation`,
`Degrees` / `Radians`, `Ticks` (durations, 50 ms each).

**Entities** — `BlockDisplay`, `ItemDisplay`, `TextDisplay`, `ArmorStand`,
`Item` (the five kinds), each with a `…State` builder (`BlockDisplayState`,
`ItemDisplayState`, `TextDisplayState`, `ArmorStandState`, `ItemState`);
`Entity` (the shared trait), `BlockState` / `ItemStr` (what a display is
*showing*), `BillboardMode` and `DisplayContext` (client-side orientation and
item render context), `TextFlags`, `StandFlags`, `Pose` / `PosePart`
(armour-stand limbs), `EquipmentSlot`, `WeakRef` / `WeakMut` (non-owning
handles to hand another task) and `Dead` (what they return once the entity
is gone).

**Randomness** — `default_random` (the per-instance deterministic stream),
`SplitRng` (a `Pod` sub-stream you can send down a channel), `Rng` (the
trait both implement).

**Registry** — `blocks` / `items` (the compile-time-checked id constants),
`BlockId` / `ItemId`, `BlockStateBuilder` and the property enums it takes
(`Axis`, `Facing`, `Half`).

**Helpers** — `Color` / `Oklab` / `Gradient` (perceptual colour),
`BlockPalette` (nearest block to a colour), `Ease` (easing curves), `Path`
(lines, arcs, béziers), `Group` + `Local` (many entities moved as one),
`Grid` + `GridLayout` (a sheet of block displays, centred cells and all),
`Timeline` + `Animate` + `Tween` (keyframed states), and the `text` module
(`escape`, `styled`, `typewriter`, `marquee`).

**Effects** — `sound` and `particle` (the fire-and-forget builders),
`SoundCategory`, `Particle`.

**Players** — `players` / `players_with` (a snapshot of who is watching,
in the placement's own frame), `Query` + `Sort` (host-side filtering,
sorting and limiting), `Player` (`name`, `position`, `eye_position`, `yaw`,
`pitch`, `facing`, `looking_toward`, and `update` to refresh one in place).

**Tasks and sync** — `channel` + `Sender` / `Receiver` (typed, `Pod`
payloads), `Signal`, `Barrier`, `Waitable` (`.and()` / `.or()`), `Policy`.

**Payload derives** — `Pod`, `Zeroable`, the bound a channel payload must
satisfy. `billboard::payload!` applies them for you with the right crate
path.
