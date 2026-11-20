//! Where the system says its time zone is.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::posix_parse;
use crate::tzif;
use crate::zone::Zone;

const LOCALTIME: &str = "/etc/localtime";
/// The directories a zone database is installed in; NixOS uses the second.
const DATABASES: [&str; 2] = ["/usr/share/zoneinfo", "/etc/zoneinfo"];
/// The directory name that marks a path as leading into a zone database.
const DATABASE_MARKER: &str = "zoneinfo/";
/// No zone file comes near this size; it bounds what a stray `TZ` path can
/// make the clock read.
const MAX_FILE_SIZE: u64 = 1 << 20;

/// The settings a time zone is found from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Environment {
    tz: Option<String>,
    localtime: PathBuf,
    databases: Vec<PathBuf>,
}

/// A zone, and its name in the zone database when that is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    pub zone: Zone,
    pub name: Option<String>,
}

impl Environment {
    /// The settings of this process: the `TZ` variable, `/etc/localtime`, and
    /// the zone databases in `TZDIR`, `/usr/share/zoneinfo` and
    /// `/etc/zoneinfo`.
    #[must_use]
    pub fn system() -> Self {
        let variable = |name| std::env::var_os(name);
        let databases = variable("TZDIR")
            .map(PathBuf::from)
            .into_iter()
            .chain(DATABASES.map(PathBuf::from))
            .collect();
        Self {
            tz: variable("TZ").map(|value| value.to_string_lossy().into_owned()),
            localtime: PathBuf::from(LOCALTIME),
            databases,
        }
    }

    /// Settings that do not come from this process.
    #[must_use]
    pub fn new(tz: Option<&str>, localtime: impl Into<PathBuf>, databases: Vec<PathBuf>) -> Self {
        Self {
            tz: tz.map(str::to_string),
            localtime: localtime.into(),
            databases,
        }
    }

    pub(crate) fn tz(&self) -> Option<&str> {
        self.tz.as_deref()
    }

    /// The zone the `TZ` variable asks for. `None` when it is unset, or is
    /// neither a zone file, a name in a database, nor a rule.
    pub(crate) fn zone_from_tz(&self) -> Option<Located> {
        let value = self.tz.as_deref()?;
        let value = value.strip_prefix(':').unwrap_or(value);
        if value.is_empty() {
            return Some(Located::unnamed(Zone::utc()));
        }
        if value.starts_with('/') {
            return read_zone(Path::new(value)).map(Located::unnamed);
        }
        let in_database = is_database_name(value)
            .then(|| {
                self.databases
                    .iter()
                    .find_map(|database| read_zone(&database.join(value)))
            })
            .flatten();
        if let Some(zone) = in_database {
            return Some(Located {
                zone,
                name: Some(value.to_string()),
            });
        }
        posix_parse::parse(value.as_bytes()).map(|rule| Located::unnamed(Zone::from_rule(rule)))
    }

    /// The zone of the local time file. `None` when it cannot be read.
    pub(crate) fn zone_from_localtime(&self) -> Option<Located> {
        let zone = read_zone(&self.localtime)?;
        // The file is usually a link into a zone database, and the path it
        // points to then ends with the name of the zone.
        let name = std::fs::read_link(&self.localtime).ok().and_then(|target| {
            let (_, name) = target.to_str()?.split_once(DATABASE_MARKER)?;
            is_database_name(name).then(|| name.to_string())
        });
        Some(Located { zone, name })
    }
}

impl Located {
    pub(crate) const fn unnamed(zone: Zone) -> Self {
        Self { zone, name: None }
    }
}

/// Whether a name can only lead to a file inside a database.
fn is_database_name(name: &str) -> bool {
    name.split('/')
        .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn read_zone(path: &Path) -> Option<Zone> {
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(MAX_FILE_SIZE)
        .read_to_end(&mut bytes)
        .ok()?;
    tzif::parse(&bytes)
}

#[cfg(test)]
mod tests;
