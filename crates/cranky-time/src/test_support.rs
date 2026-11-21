//! Shared by the tests: the zone files checked into `fixtures/`, and scratch
//! directories to play the system's part.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::environment::{Environment, Located};
use crate::tzif;

/// 2026-11-18T11:06:40Z.
pub(crate) const NOVEMBER_2026: u64 = 1_795_000_000;

/// The zone database checked into the crate.
pub(crate) fn database() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/zoneinfo")
}

pub(crate) fn zone_path(name: &str) -> PathBuf {
    database().join(name)
}

pub(crate) fn zone_bytes(name: &str) -> Vec<u8> {
    std::fs::read(zone_path(name)).unwrap()
}

pub(crate) fn located(name: &str) -> Located {
    Located {
        zone: tzif::parse(&zone_bytes(name)).unwrap(),
        name: Some(name.to_string()),
    }
}

pub(crate) fn instant(unix_seconds: u64) -> SystemTime {
    UNIX_EPOCH
        .checked_add(Duration::from_secs(unix_seconds))
        .unwrap()
}

/// An environment whose local time file is `localtime` and whose only zone
/// database is the checked-in one.
pub(crate) fn environment(timezone: Option<&str>, localtime: &Path) -> Environment {
    Environment::new(timezone, localtime, vec![database()])
}

/// A directory of its own for one test, removed when dropped.
pub(crate) struct Scratch {
    path: PathBuf,
}

impl Scratch {
    pub(crate) fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "cranky-time-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(name);
        std::fs::create_dir(&path).unwrap();
        Self { path }
    }

    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    /// The path of the local time file of this directory.
    pub(crate) fn localtime(&self) -> PathBuf {
        self.path("localtime")
    }

    /// Makes the local time file a copy of a checked-in zone file.
    pub(crate) fn copy_zone(&self, name: &str) {
        std::fs::write(self.localtime(), zone_bytes(name)).unwrap();
    }

    /// Points the local time file at a checked-in zone file, replacing it in
    /// one step as `timedatectl` does.
    pub(crate) fn link_zone(&self, name: &str) {
        let fresh = self.path("localtime.new");
        std::os::unix::fs::symlink(zone_path(name), &fresh).unwrap();
        std::fs::rename(fresh, self.localtime()).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
