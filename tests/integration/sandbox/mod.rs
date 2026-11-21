//! The private `dbus-daemon` and headless `sway` the cases run against, and
//! the re-execution of this binary inside them.

pub mod bus;
pub mod sway;

use crate::CHILD_CASE;
use crate::support::{Context, Outcome, ensure};
use bus::Bus;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use sway::Sway;
use tokio::process::Command;

/// Start of the name of every sandbox runtime directory.
const DIR_PREFIX: &str = "cranky-integration-";

/// Variables that would lead a process to the developer's own session.
const SESSION_VARIABLES: [&str; 5] = [
    "DISPLAY",
    "HYPRLAND_INSTANCE_SIGNATURE",
    "SWAYSOCK",
    "WAYLAND_DISPLAY",
    "WAYLAND_SOCKET",
];

/// The private bus and compositor, and the directory their sockets live in.
/// Dropping it kills both servers, then removes the directory: fields drop
/// in the order they are declared.
pub(crate) struct Sandbox {
    sway: Sway,
    bus: Bus,
    dir: RuntimeDir,
}

impl Sandbox {
    pub(crate) async fn start() -> Outcome<Self> {
        for program in ["dbus-daemon", "sway", "swaymsg", "grim"] {
            require(program)?;
        }
        let dir = RuntimeDir::create()?;
        let bus = Bus::start(dir.path()).await?;
        let sway = Sway::start(dir.path(), bus.address()).await?;
        Ok(Self { sway, bus, dir })
    }

    /// Kills both servers and waits for them, which dropping alone does not.
    pub(crate) async fn stop(mut self) {
        self.sway.stop().await;
        self.bus.stop().await;
    }

    /// Runs this binary again as the case `name`. The child keeps the
    /// parent's environment (so `cargo llvm-cov` still collects its profile)
    /// except for everything that names a session.
    pub(crate) async fn run_case(&self, name: &str) -> Outcome {
        let binary = std::env::current_exe().context("locate this test binary")?;
        let output = without_session(Command::new(binary))
            .env(CHILD_CASE, name)
            .env("XDG_RUNTIME_DIR", self.dir.path())
            .env("DBUS_SESSION_BUS_ADDRESS", self.bus.address())
            .env("DBUS_SYSTEM_BUS_ADDRESS", self.bus.address())
            .env("WAYLAND_DISPLAY", self.sway.wayland_display())
            .env("SWAYSOCK", self.sway.ipc_socket())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output()
            .await
            .context("re-execute this test binary inside the sandbox")?;
        ensure(output.status.success(), || {
            format!(
                "    {}\n    ({})",
                String::from_utf8_lossy(&output.stderr).trim_end(),
                output.status
            )
        })
    }
}

/// `command` without the variables that name the developer's session.
fn without_session(mut command: Command) -> Command {
    for variable in SESSION_VARIABLES {
        command.env_remove(variable);
    }
    command
}

/// Child side: refuses to run a case anywhere but inside a sandbox, whatever
/// the environment says, so a stray [`CHILD_CASE`] cannot point the adapters
/// at the developer's own buses or compositor.
pub(crate) fn ensure_inside() -> Outcome {
    let variable = |name: &str| std::env::var(name).unwrap_or_default();
    let dir = variable("XDG_RUNTIME_DIR");
    let session_bus = variable("DBUS_SESSION_BUS_ADDRESS");
    let private_dir = Path::new(&dir)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(DIR_PREFIX));
    let private_buses =
        session_bus.contains(&dir) && variable("DBUS_SYSTEM_BUS_ADDRESS") == session_bus;
    let display = variable("WAYLAND_DISPLAY");
    let private_display = !display.is_empty() && Path::new(&dir).join(display).exists();
    ensure(private_dir && private_buses && private_display, || {
        format!("{CHILD_CASE} is set outside a sandbox (XDG_RUNTIME_DIR={dir:?}); refusing to run")
    })
}

/// A missing program fails the run by name; nothing is ever skipped.
fn require(program: &str) -> Outcome {
    let found = std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()));
    ensure(found, || {
        format!("required program `{program}` was not found in PATH")
    })
}

/// A throwaway `XDG_RUNTIME_DIR`, removed on drop.
struct RuntimeDir(PathBuf);

impl RuntimeDir {
    fn create() -> Outcome<Self> {
        let path = std::env::temp_dir().join(format!("{DIR_PREFIX}{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .context("create the private runtime directory")?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for RuntimeDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
