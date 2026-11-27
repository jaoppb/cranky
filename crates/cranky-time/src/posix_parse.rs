//! Reads the text of a POSIX time zone rule.

use crate::posix_rule::{Change, Day, Daylight, Rule};
use crate::zone::Kind;

/// The hours an offset from UTC may reach.
const MAX_OFFSET_HOURS: i64 = 24;
/// The hours the time of a change may reach, either side of midnight.
const MAX_CHANGE_HOURS: i64 = 167;
const SECONDS_PER_HOUR: i64 = 3600;
/// A change happens at 02:00 unless the rule says otherwise.
const DEFAULT_CHANGE_TIME: i64 = 7200;
/// The changes of a rule that names daylight time without dating it: the
/// second Sunday of March and the first Sunday of November, the current rules
/// of the United States.
const DEFAULT_START: Day = Day::Weekday {
    month: 3,
    week: 2,
    weekday: 0,
};
const DEFAULT_END: Day = Day::Weekday {
    month: 11,
    week: 1,
    weekday: 0,
};

/// `None` when the text is not a whole, valid rule.
pub(crate) fn parse(text: &[u8]) -> Option<Rule> {
    let mut cursor = Cursor { rest: text };
    let standard_name = cursor.name()?;
    // A rule counts its offsets west of UTC.
    let standard_offset = cursor.clock(MAX_OFFSET_HOURS)?.saturating_neg();
    let standard = Kind::new(standard_offset, standard_name)?;
    if cursor.rest.is_empty() {
        return Some(Rule::new(standard, None));
    }

    let daylight = cursor.daylight(standard_offset)?;
    cursor
        .rest
        .is_empty()
        .then(|| Rule::new(standard, Some(daylight)))
}

struct Cursor<'a> {
    rest: &'a [u8],
}

impl<'a> Cursor<'a> {
    const fn peek(&self) -> Option<u8> {
        self.rest.first().copied()
    }

    /// Steps over `byte` when it is next.
    const fn eat(&mut self, byte: u8) -> bool {
        match self.rest.split_first() {
            Some((&first, tail)) if first == byte => {
                self.rest = tail;
                true
            }
            _ => false,
        }
    }

    fn take_while(&mut self, keep: impl Fn(u8) -> bool) -> &'a [u8] {
        let count = self.rest.iter().take_while(|&&byte| keep(byte)).count();
        let (head, tail) = self
            .rest
            .split_at_checked(count)
            .unwrap_or((self.rest, &[]));
        self.rest = tail;
        head
    }

    /// An unsigned number of at most four digits.
    fn number(&mut self) -> Option<i64> {
        let digits = self.take_while(|byte| byte.is_ascii_digit());
        if digits.is_empty() || digits.len() > 4 {
            return None;
        }
        digits.iter().try_fold(0_i64, |number, digit| {
            let digit = i64::from(digit.checked_sub(b'0')?);
            number.checked_mul(10)?.checked_add(digit)
        })
    }

    /// A number within `range`.
    fn number_in(&mut self, range: std::ops::RangeInclusive<i64>) -> Option<i64> {
        self.number().filter(|number| range.contains(number))
    }

    /// An abbreviation: three or more letters, or anything between `<` and `>`.
    fn name(&mut self) -> Option<&'a [u8]> {
        let name = if self.eat(b'<') {
            let quoted = self.take_while(|byte| byte != b'>');
            if !self.eat(b'>') {
                return None;
            }
            quoted
        } else {
            self.take_while(|byte| byte.is_ascii_alphabetic())
        };
        (name.len() >= 3).then_some(name)
    }

    /// `[+-]hh[:mm[:ss]]`, in seconds.
    fn clock(&mut self, max_hours: i64) -> Option<i64> {
        let negative = self.eat(b'-');
        if !negative {
            self.eat(b'+');
        }
        let mut seconds = self
            .number_in(0..=max_hours)?
            .saturating_mul(SECONDS_PER_HOUR);
        for unit in [60, 1] {
            if !self.eat(b':') {
                break;
            }
            seconds = seconds.saturating_add(self.number_in(0..=59)?.saturating_mul(unit));
        }
        Some(if negative {
            seconds.saturating_neg()
        } else {
            seconds
        })
    }

    /// `name[offset][,start,end]`: daylight time after standard time at
    /// `standard_offset`, one hour ahead of it and changing on the default
    /// days unless the text says otherwise.
    fn daylight(&mut self, standard_offset: i64) -> Option<Daylight> {
        let name = self.name()?;
        let offset = if matches!(self.peek(), None | Some(b',')) {
            standard_offset.saturating_add(SECONDS_PER_HOUR)
        } else {
            self.clock(MAX_OFFSET_HOURS)?.saturating_neg()
        };
        let (start, end) = self.changes()?;
        Some(Daylight {
            kind: Kind::new(offset, name)?,
            start,
            end,
        })
    }

    /// `,start,end`, or the default changes at the end of the text.
    fn changes(&mut self) -> Option<(Change, Change)> {
        if self.rest.is_empty() {
            let at_two = |day| Change {
                day,
                time: DEFAULT_CHANGE_TIME,
            };
            return Some((at_two(DEFAULT_START), at_two(DEFAULT_END)));
        }
        Some((self.change()?, self.change()?))
    }

    /// `,day[/time]`.
    fn change(&mut self) -> Option<Change> {
        if !self.eat(b',') {
            return None;
        }
        let day = if self.eat(b'M') {
            let month = self.number_in(1..=12)?;
            let week = self.eat(b'.').then(|| self.number_in(1..=5)).flatten()?;
            let weekday = self.eat(b'.').then(|| self.number_in(0..=6)).flatten()?;
            Day::Weekday {
                month,
                week,
                weekday,
            }
        } else if self.eat(b'J') {
            Day::Julian(self.number_in(1..=365)?)
        } else {
            Day::ZeroBased(self.number_in(0..=365)?)
        };
        let time = if self.eat(b'/') {
            self.clock(MAX_CHANGE_HOURS)?
        } else {
            DEFAULT_CHANGE_TIME
        };
        Some(Change { day, time })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(offset: i64, abbreviation: &str) -> Kind {
        Kind {
            offset,
            abbreviation: abbreviation.to_string(),
        }
    }

    fn weekday(month: i64, week: i64, weekday: i64) -> Day {
        Day::Weekday {
            month,
            week,
            weekday,
        }
    }

    #[test]
    fn offsets_count_west_of_utc() {
        assert_eq!(
            parse(b"<-03>3"),
            Some(Rule::new(kind(-10_800, "-03"), None))
        );
        assert_eq!(
            parse(b"IST-5:30"),
            Some(Rule::new(kind(19_800, "IST"), None))
        );
        assert_eq!(
            parse(b"LMT+0:36:45"),
            Some(Rule::new(kind(-2205, "LMT"), None))
        );
        assert_eq!(parse(b"UTC0"), Some(Rule::new(kind(0, "UTC"), None)));
    }

    #[test]
    fn daylight_time_is_one_hour_ahead_unless_given() {
        let lisbon = Daylight {
            kind: kind(3600, "WEST"),
            start: Change {
                day: weekday(3, 5, 0),
                time: 3600,
            },
            end: Change {
                day: weekday(10, 5, 0),
                time: 7200,
            },
        };
        assert_eq!(
            parse(b"WET0WEST,M3.5.0/1,M10.5.0"),
            Some(Rule::new(kind(0, "WET"), Some(lisbon)))
        );
        let lord_howe = Daylight {
            kind: kind(39_600, "+11"),
            start: Change {
                day: weekday(10, 1, 0),
                time: 7200,
            },
            end: Change {
                day: weekday(4, 1, 0),
                time: 7200,
            },
        };
        assert_eq!(
            parse(b"<+1030>-10:30<+11>-11,M10.1.0,M4.1.0"),
            Some(Rule::new(kind(37_800, "+1030"), Some(lord_howe)))
        );
    }

    #[test]
    fn undated_daylight_time_follows_the_default_changes() {
        let daylight = Daylight {
            kind: kind(-14_400, "EDT"),
            start: Change {
                day: weekday(3, 2, 0),
                time: 7200,
            },
            end: Change {
                day: weekday(11, 1, 0),
                time: 7200,
            },
        };
        assert_eq!(
            parse(b"EST5EDT"),
            Some(Rule::new(kind(-18_000, "EST"), Some(daylight)))
        );
    }

    #[test]
    fn days_may_be_numbered_and_times_may_be_negative_or_past_a_day() {
        let daylight = Daylight {
            kind: kind(3600, "BBB"),
            start: Change {
                day: Day::Julian(60),
                time: -3600,
            },
            end: Change {
                day: Day::ZeroBased(300),
                time: 93_784,
            },
        };
        assert_eq!(
            parse(b"AAA0BBB,J60/-1,300/26:03:04"),
            Some(Rule::new(kind(0, "AAA"), Some(daylight)))
        );
    }

    #[test]
    fn anything_else_is_not_a_rule() {
        for text in [
            "",
            "Nope/Bogus",
            "America/Sao_Paulo",
            "AB3",
            "<-03",
            "<>3",
            "UTC",
            "UTC25",
            "UTC1:60",
            "UTC12345",
            "WET0WEST,M3.5.0",
            "WET0WEST,M13.5.0,M10.5.0",
            "WET0WEST,M3.6.0,M10.5.0",
            "WET0WEST,M3.5.7,M10.5.0",
            "WET0WEST,M3.5,M10.5.0",
            "WET0WEST,M3,M10.5.0",
            "WET0WEST,J0,J365",
            "WET0WEST,0,366",
            "WET0WEST,M3.5.0/168,M10.5.0",
            "WET0WEST,M3.5.0,M10.5.0,",
            "WET0WEST M3.5.0",
            "<-03>3 ",
            "<a b>3",
            "AAA0<a b>,M3.2.0,M11.1.0",
            "<\u{1b}[31mX>3",
            "<abcdefghijklmnopqrstuvwxyz0123456>3",
        ] {
            assert_eq!(parse(text.as_bytes()), None, "{text:?}");
        }
    }

    #[test]
    fn text_that_is_not_utf8_is_not_a_rule() {
        assert_eq!(parse(b"<\xff\xfe\xfd>3"), None);
    }
}
