//! The guest ABI boundary. This module (and only this module) is where raw
//! pointers and `extern` functions exist; everything above it is safe Rust.
//!
//! Contract: docs/designs/rust-sdk-api.md in the Billboard plugin repo.
//! Only wasm core types cross; strings pass as (ptr, len) in the calling
//! task's memory. Entity/task ids are i32 host handles; all math crosses
//! as f64/i64. Getters write into out-pointers in the calling task's
//! memory; the `get_block_len`/`get_block` pair has no blocking point
//! between its two calls, so it is race-free.
//!
//! The split:
//! - [`wasm`] — the real host imports (`unsafe extern`), only on wasm.
//! - [`stubs`] — host-target stand-ins so the SDK's pure logic is testable
//!   with plain `cargo test`; anything that would cross the boundary panics.
//!
//! Both expose the same set of names; the rest of the crate calls them as
//! `crate::abi::*` regardless of target.

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
mod stubs;
#[cfg(not(target_arch = "wasm32"))]
pub use stubs::*;
