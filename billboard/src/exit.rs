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
