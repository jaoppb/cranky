//! Integration tests for the D-Bus and Wayland adapters (#35).
//!
//! The adapters find their servers only through the process environment, so
//! this binary never runs a case in the environment it was started in. The
//! parent starts a private `dbus-daemon` and a headless `sway`, then runs this
//! same binary again, once per case, with the environment pointing at them;
//! a child refuses to run anywhere else. The developer's session bus and
//! compositor are never contacted.

mod args;
mod cases;
mod dbus;
mod sandbox;
mod support;
mod systray;
mod wayland;

use std::process::ExitCode;

/// Set by the parent, to the name of the case to run, on each child it
/// starts inside the sandbox.
const CHILD_CASE: &str = "CRANKY_INTEGRATION_CASE";

fn main() -> ExitCode {
    let outcome = std::env::var(CHILD_CASE).map_or_else(
        |_| {
            args::Selection::parse(std::env::args().skip(1))
                .and_then(|selection| cases::run_selected(&selection))
        },
        |name| cases::run_one(&name),
    );
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("integration: {failure}");
            ExitCode::FAILURE
        }
    }
}
