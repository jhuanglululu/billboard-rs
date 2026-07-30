# billboard-rs

The `billboard` Rust crate — the SDK for authoring
[Billboard](https://github.com/jhuanglululu/Billboard) animations: display
entities, sounds and particles, a compile-time-checked block/item registry,
`ExitCode`, and visual helpers (color/Oklab, easing, timelines, paths,
groups). `#[billboard::main]` is the entry point; tasks, sync, randomness
and math come from the
[`wasmachine`](https://github.com/jhuanglululu/wasmachine-rs) guest core
(cargo git dependency), re-exported so animations depend on this crate
alone.

`demo/` is the worked example — build with
`cargo build --release --target wasm32-unknown-unknown` and drop the
`.wasm` into `plugins/Billboard/animations/`.

Personal-use library: versioned by git, no publishing pipeline.
