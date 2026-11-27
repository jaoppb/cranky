//! Calendar arithmetic on day numbers, counted from 1970-01-01.
//!
//! The proleptic Gregorian calendar, after Howard Hinnant's `days_from_civil`
//! and `civil_from_days`. Every operation saturates; callers keep their input
//! within the years 1 to 9999, far from the limits.

pub(crate) const SECONDS_PER_DAY: i64 = 86_400;

const DAYS_PER_ERA: i64 = 146_097;
/// Days from 0000-03-01 to 1970-01-01.
const EPOCH_SHIFT: i64 = 719_468;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Date {
    pub year: i64,
    pub month: i64,
    pub day: i64,
}

/// The day number of a calendar date.
pub(crate) const fn days_from_date(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 {
        year.saturating_sub(1)
    } else {
        year
    };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_from_march = if month > 2 {
        month.saturating_sub(3)
    } else {
        month.saturating_add(9)
    };
    let day_of_year = days_before_month(month_from_march)
        .saturating_add(day)
        .saturating_sub(1);
    let day_of_era = days_before_year(year_of_era).saturating_add(day_of_year);
    era.saturating_mul(DAYS_PER_ERA)
        .saturating_add(day_of_era)
        .saturating_sub(EPOCH_SHIFT)
}

/// The calendar date of a day number.
pub(crate) const fn date_from_days(days: i64) -> Date {
    let shifted = days.saturating_add(EPOCH_SHIFT);
    let era = shifted.div_euclid(DAYS_PER_ERA);
    let day_of_era = shifted.rem_euclid(DAYS_PER_ERA);
    let year_of_era = year_of_era_of_day(day_of_era);
    let day_of_year = day_of_era.saturating_sub(days_before_year(year_of_era));
    let month_from_march = day_of_year
        .saturating_mul(5)
        .saturating_add(2)
        .div_euclid(153);
    let day = day_of_year
        .saturating_sub(days_before_month(month_from_march))
        .saturating_add(1);
    let month = month_from_january(month_from_march);
    let year = year_of_era.saturating_add(era.saturating_mul(400));
    Date {
        year: if month <= 2 {
            year.saturating_add(1)
        } else {
            year
        },
        month,
        day,
    }
}

/// The days of an era, counted from 0000-03-01, before the March of one of
/// its years.
const fn days_before_year(year_of_era: i64) -> i64 {
    year_of_era
        .saturating_mul(365)
        .saturating_add(year_of_era.div_euclid(4))
        .saturating_sub(year_of_era.div_euclid(100))
}

/// The days of a year that starts in March before the first of a month,
/// counted 0 for March.
const fn days_before_month(month_from_march: i64) -> i64 {
    month_from_march
        .saturating_mul(153)
        .saturating_add(2)
        .div_euclid(5)
}

/// The year of an era, counted from March, that holds a day of the era.
const fn year_of_era_of_day(day_of_era: i64) -> i64 {
    day_of_era
        .saturating_sub(day_of_era.div_euclid(1460))
        .saturating_add(day_of_era.div_euclid(36_524))
        .saturating_sub(day_of_era.div_euclid(146_096))
        .div_euclid(365)
}

/// The month numbered 1 for January from the month numbered 0 for March.
const fn month_from_january(month_from_march: i64) -> i64 {
    if month_from_march < 10 {
        month_from_march.saturating_add(3)
    } else {
        month_from_march.saturating_sub(9)
    }
}

/// The day of the week of a day number, 0 for Sunday.
pub(crate) const fn weekday(days: i64) -> i64 {
    // 1970-01-01 was a Thursday.
    days.saturating_add(4).rem_euclid(7)
}

pub(crate) const fn is_leap_year(year: i64) -> bool {
    (year.rem_euclid(4) == 0 && year.rem_euclid(100) != 0) || year.rem_euclid(400) == 0
}

/// The number of days in a month of a year.
pub(crate) const fn days_in_month(year: i64, month: i64) -> i64 {
    let (next_year, next_month) = if month >= 12 {
        (year.saturating_add(1), 1)
    } else {
        (year, month.saturating_add(1))
    };
    days_from_date(next_year, next_month, 1).saturating_sub(days_from_date(year, month, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates_have_their_day_numbers() {
        for (year, month, day, days) in [
            (1970, 1, 1, 0),
            (1969, 12, 31, -1),
            (2000, 2, 29, 11_016),
            (2026, 11, 18, 20_775),
            (1, 1, 1, -719_162),
            (9999, 12, 31, 2_932_896),
        ] {
            assert_eq!(days_from_date(year, month, day), days);
            assert_eq!(date_from_days(days), Date { year, month, day });
        }
    }

    #[test]
    fn every_day_of_four_centuries_round_trips_in_order() {
        let mut previous = date_from_days(-1);
        for days in 0..DAYS_PER_ERA {
            let date = date_from_days(days);
            assert_eq!(days_from_date(date.year, date.month, date.day), days);
            let rolled =
                date.day == 1 && (date.month == previous.month.rem_euclid(12).saturating_add(1));
            assert!(date.day == previous.day.saturating_add(1) || rolled);
            previous = date;
        }
    }

    #[test]
    fn weekdays_count_from_sunday() {
        assert_eq!(weekday(0), 4, "1970-01-01 was a Thursday");
        assert_eq!(weekday(days_from_date(2026, 11, 18)), 3, "a Wednesday");
        assert_eq!(weekday(days_from_date(1969, 12, 28)), 0, "a Sunday");
    }

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        assert!(is_leap_year(2024));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2026));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 12), 31);
    }
}
