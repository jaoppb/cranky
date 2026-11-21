//! The headless sway. Its config ends with an `exec` that prints the socket
//! names, so waiting for that line is waiting for the compositor.

use super::without_session;
use crate::support::{Context, Failure, Outcome, within_for};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};

/// Starts the line sway's `exec` prints once its sockets exist.
const READY: &str = "cranky-ready";

/// Deadline for sway to come up. Longer than a case's wait: on a fresh CI
/// runner sway once did not report within those five seconds, while the same
/// tests passed on two other runners. The cause was not measured.
const STARTUP: Duration = Duration::from_secs(30);

pub(crate) struct Sway {
    process: Child,
    wayland_display: String,
    ipc_socket: String,
}

impl Sway {
    pub(crate) async fn start(dir: &Path, bus_address: &str) -> Outcome<Self> {
        let config = dir.join("sway.conf");
        let log = std::fs::File::create(dir.join("sway.log")).context("create the sway log")?;
        std::fs::write(
            &config,
            format!("xwayland disable\nexec echo \"{READY} $WAYLAND_DISPLAY $SWAYSOCK\"\n"),
        )
        .context("write the sway config")?;
        let mut process = without_session(Command::new("sway"))
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
        let stdout = process
            .stdout
            .take()
            .context("sway was started without a stdout pipe")?;
        let ready = within_for(STARTUP, "sway to report its sockets", ready_line(stdout))
            .await
            .and_then(|line| line)
            .map_err(|failure| with_log(dir, &failure))?;
        let mut sockets = ready.split_whitespace().skip(1);
        let wayland_display = sockets
            .next()
            .context("sway did not report its WAYLAND_DISPLAY")?
            .to_string();
        let ipc_socket = sockets
            .next()
            .context("sway did not report its SWAYSOCK")?
            .to_string();
        std::os::unix::net::UnixStream::connect(dir.join(&wayland_display))
            .context("connect to the sway Wayland socket")?;
        Ok(Self {
            process,
            wayland_display,
            ipc_socket,
        })
    }

    pub(crate) fn wayland_display(&self) -> &str {
        &self.wayland_display
    }

    pub(crate) fn ipc_socket(&self) -> &str {
        &self.ipc_socket
    }

    pub(crate) async fn stop(&mut self) {
        let _ = self.process.kill().await;
    }
}

async fn ready_line(stdout: ChildStdout) -> Outcome<String> {
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

/// `failure` followed by what sway wrote to its log.
fn with_log(dir: &Path, failure: &Failure) -> Failure {
    let log = std::fs::read_to_string(dir.join("sway.log")).unwrap_or_default();
    Failure::new(format!("{failure}\nsway log:\n{log}"))
}
