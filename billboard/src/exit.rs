//! [`ExitCode`]: what `main` returns to tell the host what to do once the
//! animation ends (task 0 returning ends it — all live tasks are killed).

/// The disposition an animation hands back to the host when it finishes.
///
/// `#[billboard::main]` requires `fn main() -> ExitCode`; the returned value
/// crosses the ABI as an `i32` (`End = 0`, `Keep = 1`, `Repeat = 2`).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExitCode {
    /// Clear everything, including leaked entities. Runtime state and every
    /// entity the instance ever spawned are destroyed.
    End = 0,
    /// Keep leaked entities visible while eligible players are nearby, but
    /// release all runtime state (memory, tasks). When no one is nearby they
    /// clear; a player approaching restarts the animation fresh (the plugin's
    /// normal proximity lifecycle).
    Keep = 1,
    /// Clear everything, then start the animation again immediately — for
    /// looping shows.
    ///
    /// A repeat is a **fresh instance, not a jump back to the top of `main`**.
    /// The host despawns every entity the run ever spawned (leaked ones
    /// included), throws the interpreter away and builds a new one: new memory,
    /// so every global and every `static` is back at its initial value, no
    /// tasks, no channels, no entity ids carried over. Nothing you stashed
    /// outside `main`'s stack survives the loop, and nothing you leaked stays
    /// on screen through it — if you want a scene to persist between passes,
    /// don't repeat; rebuild it, or loop inside `main` instead.
    ///
    /// The **deterministic random stream restarts from the same seed**: the
    /// host derives it from (animation, placement id, owner), which a repeat
    /// does not change, so a `random_seed = N` animation replays an identical
    /// pass every time. The non-deterministic stream is the one that gives a
    /// repeat something new to say.
    Repeat = 2,
}

impl ExitCode {
    /// The ABI wire value for this code.
    #[doc(hidden)]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::ExitCode;

    #[test]
    fn wire_values() {
        // The `_engine_main` return contract the plugin reads: End=0, Keep=1,
        // Repeat=2. Values written out by hand, not derived from the enum.
        assert_eq!(ExitCode::End.as_i32(), 0);
        assert_eq!(ExitCode::Keep.as_i32(), 1);
        assert_eq!(ExitCode::Repeat.as_i32(), 2);
    }
}
