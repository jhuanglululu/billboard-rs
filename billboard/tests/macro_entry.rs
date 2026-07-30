//! What `#[billboard::main]` generates, checked by compiling it and calling
//! the exports it emits — the real generated exports, the same ones the host
//! looks up: the engine's `_engine_main`/`_engine_abi` and Billboard's own
//! `_billboard_abi`.

use billboard::prelude::*;
use billboard::random::{DefaultRng, default_random};

/// The seed here is `i64::MIN` on purpose: it is the one value whose *magnitude*
/// does not fit in an `i64`, so this doubles as a compile-pass regression test
/// for the seed parser's sign-then-range handling. (The demo covers an ordinary
/// positive seed; `billboard-macros`' own unit tests cover the rejections.)
#[billboard::main(random_seed = -9223372036854775808)]
fn animation() -> ExitCode {
    // Never reached on the host target: init's seeding hits the ABI stub
    // first, which is exactly what the test below asserts.
    ExitCode::End
}

#[test]
fn the_plugin_handshake_export_reports_version_three() {
    assert_eq!(_billboard_abi(), 3);
    // One source of truth: the macro exports the crate's constant.
    assert_eq!(billboard::ABI_VERSION, 3);
}

/// Beside Billboard's handshake sits the engine's, emitted by the same
/// attribute and identical in every guest whatever plugin it serves — that is
/// what lets the host check the engine ABI before it knows the plugin.
#[test]
fn the_engine_handshake_export_sits_beside_it() {
    assert_eq!(_engine_abi(), 1);
    assert_eq!(_engine_abi(), billboard::ENGINE_ABI_VERSION);
    // The two version spaces are independent, and this file proves they are
    // not accidentally the same number.
    assert_ne!(_engine_abi(), _billboard_abi());
}

/// The seeding hook the macro emits: it must reach the host's `seed_random`
/// import *and* flip `default_random()` over to the deterministic stream.
///
/// This calls the hook directly rather than through the generated
/// `_engine_main`, because that export is `extern "C"` — the stub's panic
/// inside it aborts instead of unwinding, so it can't be observed from a test.
/// That the *macro* emits this call, before the animation body, is what the
/// Phase-3 wasm integration test covers; that the attribute parses and expands
/// is covered by this file compiling at all.
#[test]
fn seeding_hits_the_host_import_and_switches_default_random() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(|| billboard::__rt::seed_random(20260726));
    std::panic::set_hook(previous);

    let payload = outcome.expect_err("seeding should have reached the host seed_random import");
    let message = match payload.downcast_ref::<&str>() {
        Some(s) => (*s).to_owned(),
        None => payload
            .downcast_ref::<String>()
            .cloned()
            .expect("panic payload should be a message"),
    };
    assert!(
        message.contains("seed_random"),
        "expected the seed_random import to be what stopped us, got {message:?}"
    );

    // The routing flag is set before the import call, so it is already
    // flipped — and it never goes back.
    assert!(
        matches!(default_random(), DefaultRng::Deterministic(_)),
        "after seeding, default_random must draw from the deterministic stream"
    );
}
