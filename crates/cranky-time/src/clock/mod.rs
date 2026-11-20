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

/// A zone, and the `TZ` value that was in force when it was read.
#[derive(Debug)]
struct Remembered {
    tz: Option<String>,
    located: Located,
}

impl Clock {
    #[must_use]
    pub const fn new() -> Self {
        Self { last_read: None }
    }

    /// The local time now, in the time zone of this process.
    pub fn now(&mut self) -> LocalTime {
        self.at(SystemTime::now(), &Environment::system())
    }

    /// The local time at an instant, in the time zone of an environment.
    ///
    /// The zone is the one `TZ` names when it is set, and the one of the local
    /// time file otherwise. When that cannot be read the zone is, in order:
    /// the one an earlier call read from the same setting, the local time file
    /// (when `TZ` was the unusable one), UTC.
    pub fn at(&mut self, instant: SystemTime, environment: &Environment) -> LocalTime {
        let (wanted, source) = if environment.tz().is_some() {
            (environment.zone_from_tz(), Source::TzVariable)
        } else {
            (environment.zone_from_localtime(), Source::Localtime)
        };
        if let Some(located) = wanted {
            let time = LocalTime::at(instant, &located, source);
            self.last_read = Some(Remembered {
                tz: environment.tz().map(str::to_string),
                located,
            });
            return time;
        }
        if let Some(remembered) = &self.last_read
            && remembered.tz.as_deref() == environment.tz()
        {
            return LocalTime::at(instant, &remembered.located, Source::Previous);
        }
        let from_file = environment
            .tz()
            .and_then(|_| environment.zone_from_localtime());
        from_file.map_or_else(
            || LocalTime::at(instant, &Located::unnamed(Zone::utc()), Source::Utc),
            |located| LocalTime::at(instant, &located, Source::LocaltimeAfterInvalidTz),
        )
    }
}

#[cfg(test)]
mod tests;
