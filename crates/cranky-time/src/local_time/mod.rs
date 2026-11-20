//! A local date and time, and the text scripts receive for it.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::civil::{self, SECONDS_PER_DAY};
use crate::environment::Located;
use crate::zone::Zone;

/// 0001-01-01T00:00:00 and 9999-12-31T23:59:59, in seconds from 1970. Local
/// times are kept between them so that a year always has four digits.
const FIRST_SECOND: i64 = -62_135_596_800;
const LAST_SECOND: i64 = 253_402_300_799;

/// Where the time zone of a [`LocalTime`] came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The `TZ` variable.
    TzVariable,
    /// The local time file, with `TZ` unset.
    Localtime,
    /// The local time file, because `TZ` is set to something unusable.
    LocaltimeAfterInvalidTz,
    /// The zone of an earlier call, because the source it came from can no
    /// longer be read. That source is `TZ`, or the local time file, with `TZ`
    /// unset or unusable.
    Previous,
    /// Nothing could be read: the time is in UTC.
    Utc,
}

/// A date and time in a time zone.
///
/// Its text form is RFC 3339 in whole seconds with a numeric offset, always 25
/// bytes long: `2026-11-18T08:06:40-03:00`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalTime {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    subsec_millis: u32,
    offset_seconds: i64,
    abbreviation: String,
    zone_name: Option<String>,
    source: Source,
}

impl LocalTime {
    /// An instant, given in seconds from 1970, in UTC.
    #[must_use]
    pub fn utc(unix_seconds: i64) -> Self {
        Self::in_zone(unix_seconds, 0, &Located::unnamed(Zone::utc()), Source::Utc)
    }

    pub(crate) fn at(now: SystemTime, located: &Located, source: Source) -> Self {
        // A clock set before 1970 is read as 1970.
        let since_epoch = now.duration_since(UNIX_EPOCH).unwrap_or_default();
        let seconds = i64::try_from(since_epoch.as_secs()).unwrap_or(i64::MAX);
        Self::in_zone(seconds, since_epoch.subsec_millis(), located, source)
    }

    fn in_zone(instant: i64, subsec_millis: u32, located: &Located, source: Source) -> Self {
        let moment = located.zone.at(instant.clamp(FIRST_SECOND, LAST_SECOND));
        let local = moment
            .unix_seconds
            .saturating_add(moment.kind.offset)
            .clamp(FIRST_SECOND, LAST_SECOND);
        let date = civil::date_from_days(local.div_euclid(SECONDS_PER_DAY));
        let second_of_day = local.rem_euclid(SECONDS_PER_DAY);
        Self {
            year: date.year,
            month: date.month,
            day: date.day,
            hour: second_of_day.div_euclid(3600),
            minute: second_of_day.rem_euclid(3600).div_euclid(60),
            second: second_of_day
                .rem_euclid(60)
                .saturating_add(i64::from(moment.leap_second)),
            subsec_millis,
            offset_seconds: moment.kind.offset,
            abbreviation: moment.kind.abbreviation,
            zone_name: located.name.clone(),
            source,
        }
    }

    #[must_use]
    pub const fn year(&self) -> i64 {
        self.year
    }

    /// 1 to 12.
    #[must_use]
    pub const fn month(&self) -> i64 {
        self.month
    }

    /// 1 to 31.
    #[must_use]
    pub const fn day(&self) -> i64 {
        self.day
    }

    #[must_use]
    pub const fn hour(&self) -> i64 {
        self.hour
    }

    #[must_use]
    pub const fn minute(&self) -> i64 {
        self.minute
    }

    /// 0 to 59, or 60 during an inserted leap second.
    #[must_use]
    pub const fn second(&self) -> i64 {
        self.second
    }

    /// The milliseconds past the whole second.
    #[must_use]
    pub const fn subsec_millis(&self) -> u32 {
        self.subsec_millis
    }

    /// Seconds east of UTC.
    #[must_use]
    pub const fn offset_seconds(&self) -> i64 {
        self.offset_seconds
    }

    /// The short name of the local time in force, such as `WEST` or `-03`.
    #[must_use]
    pub fn abbreviation(&self) -> &str {
        &self.abbreviation
    }

    /// The name of the zone in the zone database, such as `Europe/Lisbon`.
    /// Known when `TZ` holds a name, or the local time file is a link into a
    /// database.
    #[must_use]
    pub fn zone_name(&self) -> Option<&str> {
        self.zone_name.as_deref()
    }

    #[must_use]
    pub const fn source(&self) -> Source {
        self.source
    }
}

impl fmt::Display for LocalTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.offset_seconds < 0 { '-' } else { '+' };
        let offset = self.offset_seconds.saturating_abs();
        write!(
            formatter,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{sign}{:02}:{:02}",
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
            offset.div_euclid(3600),
            offset.rem_euclid(3600).div_euclid(60),
        )
    }
}

#[cfg(test)]
mod tests;
