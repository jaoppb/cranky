//! The private `dbus-daemon`. It prints its address once it listens, so
//! waiting for that line is waiting for the bus.

use crate::support::{Context, Outcome, within};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

pub struct Bus {
    process: Child,
    address: String,
}

impl Bus {
    pub async fn start(dir: &Path) -> Outcome<Self> {
        let config = dir.join("bus.conf");
        std::fs::write(&config, config_for(dir)).context("write the dbus-daemon config")?;
        let mut process = Command::new("dbus-daemon")
            .arg("--nofork")
            .arg("--print-address")
            .arg(format!("--config-file={}", config.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("start dbus-daemon")?;
        let stdout = process
            .stdout
            .take()
            .context("dbus-daemon was started without a stdout pipe")?;
        let address = within("dbus-daemon to print its address", async {
            BufReader::new(stdout).lines().next_line().await
        })
        .await?
        .context("read the dbus-daemon address")?
        .context("dbus-daemon exited before printing its address")?;
        Ok(Self { process, address })
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub async fn stop(&mut self) {
        let _ = self.process.kill().await;
    }
}

/// A session bus that listens only inside `dir`, lets every peer own any
/// name and talk to any other, and knows no service to activate.
fn config_for(dir: &Path) -> String {
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
