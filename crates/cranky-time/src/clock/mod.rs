//! The clock: the current local time, whatever state the system zone is in.

use std::time::SystemTime;

use crate::environment::{Environment, Located};
use crate::local_time::{LocalTime, Source};
use crate::zone::Zone;

/// Tells the local time.
///
/// It reads the time zone again on every call. All it keeps is the last zone
/// it could read from each place, so that a zone file caught half-written or
/// missing for a moment does not send the clock to UTC.
///
/// One clock serves one environment: what it remembers of the local time file
/// is not told apart by the path of that file.
#[derive(Debug, Default)]
pub struct Clock {
    /// The last zone read from `TZ`, and the value `TZ` had.
    from_tz: Option<(String, Located)>,
    /// The last zone read from the local time file.
    from_file: Option<Located>,
}

impl Clock {
    /// A clock that remembers no zone yet: until it has read one, a zone it cannot read falls back
    /// to the local time file, and to UTC when neither can be read.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            from_tz: None,
            from_file: None,
        }
    }

    /// The local time now, in the time zone of this process.
    ///
    /// Reads a small file, so it can block for as long as the disk does.
    pub fn now(&mut self) -> LocalTime {
        self.at_instant(SystemTime::now(), &Environment::system())
    }

    /// The local time at an instant, in the time zone of an environment.
    ///
    /// The zone is the one `TZ` names when it is set, and the one of the local
    /// time file otherwise. When `TZ` cannot be read the zone is the one an
    /// earlier call read from the same value of `TZ`, and failing that the one
    /// of the local time file. When that file cannot be read either, the zone
    /// is the one an earlier call read from it, and failing that UTC.
    pub fn at_instant(&mut self, instant: SystemTime, environment: &Environment) -> LocalTime {
        let file_source = if let Some(timezone) = environment.timezone() {
            if let Some(located) = environment.zone_from_tz() {
                let time = LocalTime::at_instant(instant, &located, Source::TzVariable);
                self.from_tz = Some((timezone.to_string(), located));
                return time;
            }
            if let Some((_, located)) = self.from_tz.as_ref().filter(|(value, _)| value == timezone)
            {
                return LocalTime::at_instant(instant, located, Source::Previous);
            }
            Source::LocaltimeAfterInvalidTz
        } else {
            Source::Localtime
        };
        if let Some(located) = environment.zone_from_localtime() {
            let time = LocalTime::at_instant(instant, &located, file_source);
            self.from_file = Some(located);
            return time;
        }
        self.from_file.as_ref().map_or_else(
            || LocalTime::at_instant(instant, &Located::unnamed(Zone::utc()), Source::Utc),
            |located| LocalTime::at_instant(instant, located, Source::Previous),
        )
    }
}

#[cfg(test)]
mod tests;
