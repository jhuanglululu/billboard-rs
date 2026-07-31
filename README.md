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
   paths, palettes, tasks, barriers, signals, channels, seeded randomness),
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
cargo build --release --target wasm32-unknown-unknown -p billboard
```

The ABI is `wasm32-unknown-unknown`-only; on a native target the host calls
are stubs that panic, which is why the tests stay on the pure-guest side of
the SDK.
