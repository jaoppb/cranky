//! The clock: the current local time, whatever state the system zone is in.

use std::time::SystemTime;

use crate::environment::{Environment, Located};
use crate::local_time::{LocalTime, Source};
use crate::zone::Zone;

/// Tells the local time.
///
/// It reads the time zone again on every call. All it keeps is the last zone
/// it could read, so that a zone file caught half-written or missing for a
/// moment does not send the clock to UTC.
#[derive(Debug, Default)]
pub struct Clock {
    last_read: Option<Remembered>,
}

/// A zone that was read, and the settings it was read under.
#[derive(Debug)]
struct Remembered {
    tz: Option<String>,
    /// It came from the source the settings ask for, not from a fallback.
    wanted: bool,
    located: Located,
}

impl Clock {
    #[must_use]
    pub const fn new() -> Self {
        Self { last_read: None }
    }

    /// The local time now, in the time zone of this process.
    ///
    /// Reads a small file, so it can block for as long as the disk does.
    pub fn now(&mut self) -> LocalTime {
        self.at(SystemTime::now(), &Environment::system())
    }

    /// The local time at an instant, in the time zone of an environment.
    ///
    /// The zone is the one `TZ` names when it is set, and the one of the local
    /// time file otherwise. When that cannot be read the zone is, in order:
    /// the one an earlier call read there, the local time file (when `TZ` was
    /// the unusable one), the one an earlier call read from that file, UTC.
    pub fn at(&mut self, instant: SystemTime, environment: &Environment) -> LocalTime {
        let (wanted, source) = if environment.tz().is_some() {
            (environment.zone_from_tz(), Source::TzVariable)
        } else {
            (environment.zone_from_localtime(), Source::Localtime)
        };
        if let Some(located) = wanted {
            return self.read(instant, environment, located, source);
        }
        if let Some(time) = self.remembered(instant, environment, true) {
            return time;
        }
        let from_file = environment
            .tz()
            .and_then(|_| environment.zone_from_localtime());
        if let Some(located) = from_file {
            return self.read(
                instant,
                environment,
                located,
                Source::LocaltimeAfterInvalidTz,
            );
        }
        self.remembered(instant, environment, false)
            .unwrap_or_else(|| LocalTime::at(instant, &Located::unnamed(Zone::utc()), Source::Utc))
    }

    /// The local time in a zone that was just read, which is remembered.
    fn read(
        &mut self,
        instant: SystemTime,
        environment: &Environment,
        located: Located,
        source: Source,
    ) -> LocalTime {
        let time = LocalTime::at(instant, &located, source);
        self.last_read = Some(Remembered {
            tz: environment.tz().map(str::to_string),
            wanted: source != Source::LocaltimeAfterInvalidTz,
            located,
        });
        time
    }

    /// The local time in the remembered zone, when it was read under the same
    /// `TZ`, and from the wanted source if `only_wanted`.
    fn remembered(
        &self,
        instant: SystemTime,
        environment: &Environment,
        only_wanted: bool,
    ) -> Option<LocalTime> {
        self.last_read
            .as_ref()
            .filter(|remembered| remembered.tz.as_deref() == environment.tz())
            .filter(|remembered| remembered.wanted || !only_wanted)
            .map(|remembered| LocalTime::at(instant, &remembered.located, Source::Previous))
    }
}

#[cfg(test)]
mod tests;
