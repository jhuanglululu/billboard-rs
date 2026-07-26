//! The guest ABI boundary. This module — and only this module — is where raw
//! pointers and `extern` functions exist; everything above it is safe Rust.
//!
//! Contract: docs/designs/rust-sdk-api.md in the Billboard plugin repo.
//! Only wasm core types cross; strings pass as (ptr, len) in the calling
//! task's memory. Entity/task ids are i32 host handles; all math crosses
//! as f64/i64. Getters write into out-pointers in the calling task's
//! memory; a `get_*_len`/`get_*` pair has no blocking point between its two
//! calls, so it is race-free.
//!
//! The split:
//! - `sys` — the imports themselves: `wasm.rs` declares the real
//!   `unsafe extern` block on wasm; `stubs.rs` stands in on the host target so
//!   the SDK's pure logic is testable with plain `cargo test`, and anything that
//!   would actually cross the boundary panics.
//! - [`marshal`] — safe wrappers for every import that takes a pointer, so that
//!   no module outside `abi` ever forms one. Callers pass `&str`/`&[u8]` and get
//!   back `String`/`[f64; N]`.
//!
//! Imports that pass only scalars (`sleep`, `set_position`, `signal_notify`, …)
//! are re-exported directly and called as `abi::sleep(…)` in an `unsafe` block:
//! those calls carry no addresses. Anything with a pointer in its signature is
//! reached through [`marshal`] instead.
//!
//! The one pointer outside this module is the `#[global_allocator]` in
//! [`__rt`](crate::__rt) — `GlobalAlloc`'s own trait methods are defined in terms
//! of `*mut u8`, so it has no choice. It forwards straight to
//! [`marshal::realloc`].

#[cfg(target_arch = "wasm32")]
#[path = "wasm.rs"]
mod sys;
#[cfg(not(target_arch = "wasm32"))]
#[path = "stubs.rs"]
mod sys;

pub mod marshal;

// The scalar-only imports, callable directly. The pointer-taking ones live here
// too — `marshal` is built on them — but nothing outside this module calls those.
pub use sys::*;
