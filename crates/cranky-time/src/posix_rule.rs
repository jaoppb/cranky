//! A POSIX time zone rule, such as `WET0WEST,M3.5.0/1,M10.5.0`.
//!
//! It is the last line of a zone file, where it covers every instant after the
//! last transition, and one of the forms the `TZ` variable takes.

use crate::civil::{self, SECONDS_PER_DAY};
use crate::zone::Kind;

/// The day of a year on which a change happens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Day {
    /// `Mm.w.d`: a weekday (0 for Sunday) of a week of a month; week 5 is the
    /// last one.
    Weekday { month: i64, week: i64, weekday: i64 },
    /// `Jn`: day 1 to 365, where February 29 is never counted.
    Julian(i64),
    /// `n`: day 0 to 365, where February 29 is counted in leap years.
    ZeroBased(i64),
}

/// A change between standard and daylight time: a day, and the seconds after
/// its local midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    pub day: Day,
    pub time: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Daylight {
    pub kind: Kind,
    pub start: Change,
    pub end: Change,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    standard: Kind,
    daylight: Option<Daylight>,
}

impl Rule {
    pub const fn new(standard: Kind, daylight: Option<Daylight>) -> Self {
        Self { standard, daylight }
    }

    pub const fn standard(&self) -> &Kind {
        &self.standard
    }

    /// The kind of local time in force at an instant.
    pub fn kind_at(&self, unix_seconds: i64) -> &Kind {
        let Some(daylight) = &self.daylight else {
            return &self.standard;
        };
        let local = unix_seconds.saturating_add(self.standard.offset);
        let year = civil::date_from_days(local.div_euclid(SECONDS_PER_DAY)).year;
        // Each change is written in the local time in force just before it.
        let start = daylight.start.instant(year, self.standard.offset);
        let end = daylight.end.instant(year, daylight.kind.offset);
        let in_daylight = if start < end {
            (start..end).contains(&unix_seconds)
        } else {
            // South of the equator daylight time spans the new year.
            !(end..start).contains(&unix_seconds)
        };
        if in_daylight {
            &daylight.kind
        } else {
            &self.standard
        }
    }
}

impl Change {
    fn instant(self, year: i64, offset_before: i64) -> i64 {
        self.day
            .in_year(year)
            .saturating_mul(SECONDS_PER_DAY)
            .saturating_add(self.time)
            .saturating_sub(offset_before)
    }
}

impl Day {
    /// The day number of this day in a year.
    fn in_year(self, year: i64) -> i64 {
        let new_year = civil::days_from_date(year, 1, 1);
        match self {
            Self::ZeroBased(day) => new_year.saturating_add(day),
            Self::Julian(day) => {
                let after_february = civil::is_leap_year(year) && day >= 60;
                new_year
                    .saturating_add(day)
                    .saturating_sub(1)
                    .saturating_add(i64::from(after_february))
            }
            Self::Weekday {
                month,
                week,
                weekday,
            } => {
                let first = civil::days_from_date(year, month, 1);
                let to_weekday = weekday.saturating_sub(civil::weekday(first)).rem_euclid(7);
                let day = to_weekday.saturating_add(week.saturating_sub(1).saturating_mul(7));
                let day = if day >= civil::days_in_month(year, month) {
                    day.saturating_sub(7)
                } else {
                    day
                };
                first.saturating_add(day)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::posix_parse::parse;

    fn offset_at(rule: &str, unix_seconds: i64) -> (i64, String) {
        let rule = parse(rule.as_bytes()).unwrap();
        let kind = rule.kind_at(unix_seconds);
        (kind.offset, kind.abbreviation.clone())
    }

    #[test]
    fn a_rule_without_daylight_time_is_one_fixed_offset() {
        assert_eq!(offset_at("<-03>3", 0), (-10_800, "-03".to_string()));
        assert_eq!(
            offset_at("IST-5:30", 4_000_000_000),
            (19_800, "IST".to_string())
        );
    }

    #[test]
    fn northern_daylight_time_starts_and_ends_at_the_written_local_time() {
        // Lisbon 2050: 01:00 WET on March 27 and 02:00 WEST on October 30,
        // both 01:00 UTC.
        let rule = "WET0WEST,M3.5.0/1,M10.5.0";
        assert_eq!(offset_at(rule, 2_531_955_599), (0, "WET".to_string()));
        assert_eq!(offset_at(rule, 2_531_955_600), (3600, "WEST".to_string()));
        assert_eq!(offset_at(rule, 2_550_704_399), (3600, "WEST".to_string()));
        assert_eq!(offset_at(rule, 2_550_704_400), (0, "WET".to_string()));
    }

    #[test]
    fn southern_daylight_time_spans_the_new_year() {
        // Sydney 2050: ends 03:00 AEDT on April 3, starts 02:00 AEST on October 2.
        let rule = "AEST-10AEDT,M10.1.0,M4.1.0/3";
        assert_eq!(offset_at(rule, 2_524_608_000), (39_600, "AEDT".to_string()));
        assert_eq!(offset_at(rule, 2_532_527_999), (39_600, "AEDT".to_string()));
        assert_eq!(offset_at(rule, 2_532_528_000), (36_000, "AEST".to_string()));
        assert_eq!(offset_at(rule, 2_548_252_799), (36_000, "AEST".to_string()));
        assert_eq!(offset_at(rule, 2_548_252_800), (39_600, "AEDT".to_string()));
    }

    #[test]
    fn julian_days_skip_february_29_and_zero_based_days_count_it() {
        let march_first_2024 = civil::days_from_date(2024, 3, 1);
        assert_eq!(Day::Julian(60).in_year(2024), march_first_2024);
        assert_eq!(
            Day::Julian(59).in_year(2024),
            march_first_2024.saturating_sub(2)
        );
        assert_eq!(Day::ZeroBased(60).in_year(2024), march_first_2024);
        assert_eq!(
            Day::ZeroBased(59).in_year(2026),
            civil::days_from_date(2026, 3, 1)
        );
        assert_eq!(
            Day::Julian(60).in_year(2026),
            civil::days_from_date(2026, 3, 1)
        );
    }

    #[test]
    fn week_five_is_the_last_such_weekday_of_the_month() {
        let last_sunday = Day::Weekday {
            month: 10,
            week: 5,
            weekday: 0,
        };
        assert_eq!(
            last_sunday.in_year(2026),
            civil::days_from_date(2026, 10, 25)
        );
        let first_sunday = Day::Weekday {
            month: 11,
            week: 1,
            weekday: 0,
        };
        assert_eq!(
            first_sunday.in_year(2026),
            civil::days_from_date(2026, 11, 1)
        );
    }
}
