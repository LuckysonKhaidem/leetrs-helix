//! Lightweight logging with a global "quiet" switch.
//!
//! The CLI prints progress to stdout via [`info`]/[`error`]. When the TUI is
//! active (alternate screen) those writes would corrupt the rendered frame, so
//! the TUI enables [`set_quiet`] to suppress them — the editor shows status and
//! results in its own panes instead.
use std::sync::atomic::{AtomicBool, Ordering};

static QUIET: AtomicBool = AtomicBool::new(false);

/// Enables or disables logging. When enabled, [`info`]/[`error`] are no-ops.
pub fn set_quiet(quiet: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
}

/// Returns whether logging is currently suppressed.
pub fn is_quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
}

/// Prints an informational message to stdout unless quiet.
pub fn info(msg: impl AsRef<str>) {
    if !is_quiet() {
        println!("{}", msg.as_ref());
    }
}

/// Prints an error message to stderr unless quiet.
pub fn error(msg: impl AsRef<str>) {
    if !is_quiet() {
        eprintln!("{}", msg.as_ref());
    }
}
