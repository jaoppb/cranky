//! The private `dbus-daemon` and headless `sway` the cases run against, and
//! the re-execution of this binary inside them.
//!
//! No wait here is a sleep: `dbus-daemon` prints its address when it is
//! ready, and sway runs an `exec` that prints its socket names.

use crate::CHILD_CASE;
use crate::support::{Context, Failure, Outcome, ensure, within};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

/// Start of the name of every sandbox runtime directory.
const DIR_PREFIX: &str = "cranky-integration-";

/// Printed by sway's own `exec` once its sockets exist.
const READY: &str = "cranky-ready";

/// Variables that would lead a process to the developer's own session.
const SESSION_VARIABLES: [&str; 5] = [
    "DISPLAY",
    "HYPRLAND_INSTANCE_SIGNATURE",
    "SWAYSOCK",
    "WAYLAND_DISPLAY",
    "WAYLAND_SOCKET",
];

/// The private bus and compositor, and the directory their sockets live in.
pub struct Sandbox {
    dir: RuntimeDir,
    bus: Bus,
    sway: Sway,
}

impl Sandbox {
    pub async fn start() -> Outcome<Self> {
        for program in ["dbus-daemon", "sway", "swaymsg", "grim"] {
            require(program)?;
        }
        let dir = RuntimeDir::create()?;
        let bus = start_bus(dir.path()).await?;
        let sway = start_sway(dir.path(), &bus.address).await?;
        Ok(Self { dir, bus, sway })
    }

    /// Both servers are also killed on drop; stopping reaps them too.
    pub async fn stop(mut self) {
        let _ = self.sway.process.kill().await;
        let _ = self.bus.process.kill().await;
    }

    /// Runs this binary again as the case `name`. The child keeps the
    /// parent's environment (so `cargo llvm-cov` still collects its profile)
    /// except for everything that names a session.
    pub async fn run_case(&self, name: &str) -> Outcome {
        let binary = std::env::current_exe().context("locate this test binary")?;
        let mut command = Command::new(binary);
        for variable in SESSION_VARIABLES {
            command.env_remove(variable);
        }
        let output = command
            .env(CHILD_CASE, name)
            .env("XDG_RUNTIME_DIR", self.dir.path())
            .env("DBUS_SESSION_BUS_ADDRESS", &self.bus.address)
            .env("DBUS_SYSTEM_BUS_ADDRESS", &self.bus.address)
            .env("WAYLAND_DISPLAY", &self.sway.wayland_display)
            .env("SWAYSOCK", &self.sway.ipc_socket)
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

/// Child side: refuses to run a case anywhere but inside a sandbox, whatever
/// the environment says, so a stray [`CHILD_CASE`] cannot point the adapters
/// at the developer's own bus or compositor.
pub fn ensure_inside() -> Outcome {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
    let bus = std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_default();
    let private = Path::new(&dir)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(DIR_PREFIX));
    ensure(private && bus.contains(&dir), || {
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

struct Bus {
    process: Child,
    address: String,
}

struct Sway {
    process: Child,
    wayland_display: String,
    ipc_socket: String,
}

async fn start_bus(dir: &Path) -> Outcome<Bus> {
    let config = dir.join("bus.conf");
    std::fs::write(&config, bus_config(dir)).context("write the dbus-daemon config")?;
    let mut process = Command::new("dbus-daemon")
        .arg("--nofork")
        .arg("--print-address")
        .arg(format!("--config-file={}", config.display()))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("start dbus-daemon")?;
    let stdout = process.stdout.take().context("dbus-daemon stdout")?;
    let address = within("dbus-daemon to print its address", async {
        BufReader::new(stdout).lines().next_line().await
    })
    .await?
    .context("read the dbus-daemon address")?
    .context("dbus-daemon exited before printing its address")?;
    Ok(Bus { process, address })
}

fn bus_config(dir: &Path) -> String {
    format!(
        r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:dir={}</listen>
  <policy context="default">
    <allow send_destination="*"/>
    <allow receive_sender="*"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#,
        dir.display()
    )
}

async fn start_sway(dir: &Path, bus_address: &str) -> Outcome<Sway> {
    let config = dir.join("sway.conf");
    let log = std::fs::File::create(dir.join("sway.log")).context("create the sway log")?;
    std::fs::write(
        &config,
        format!("xwayland disable\nexec echo \"{READY} $WAYLAND_DISPLAY $SWAYSOCK\"\n"),
    )
    .context("write the sway config")?;
    let mut command = Command::new("sway");
    for variable in SESSION_VARIABLES {
        command.env_remove(variable);
    }
    let mut process = command
        .arg("--config")
        .arg(&config)
        .env("XDG_RUNTIME_DIR", dir)
        .env("DBUS_SESSION_BUS_ADDRESS", bus_address)
        .env("WLR_BACKENDS", "headless")
        .env("WLR_RENDERER", "pixman")
        .env("WLR_LIBINPUT_NO_DEVICES", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(log)
        .kill_on_drop(true)
        .spawn()
        .context("start sway")?;
    let stdout = process.stdout.take().context("sway stdout")?;
    let ready = within("sway to report its sockets", ready_line(stdout))
        .await?
        .map_err(|failure| with_sway_log(dir, &failure))?;
    let mut sockets = ready.split_whitespace().skip(1);
    let wayland_display = sockets.next().context("sway WAYLAND_DISPLAY")?.to_string();
    let ipc_socket = sockets.next().context("sway SWAYSOCK")?.to_string();
    std::os::unix::net::UnixStream::connect(dir.join(&wayland_display))
        .context("connect to the sway Wayland socket")?;
    Ok(Sway {
        process,
        wayland_display,
        ipc_socket,
    })
}

async fn ready_line(stdout: tokio::process::ChildStdout) -> Outcome<String> {
    let mut lines = BufReader::new(stdout).lines();
    loop {
        let line = lines
            .next_line()
            .await
            .context("read sway stdout")?
            .context("sway exited before its sockets existed")?;
        if line.starts_with(READY) {
            return Ok(line);
        }
    }
}

fn with_sway_log(dir: &Path, failure: &Failure) -> Failure {
    let log = std::fs::read_to_string(dir.join("sway.log")).unwrap_or_default();
    Failure::new(format!("{failure}\nsway log:\n{log}"))
}
