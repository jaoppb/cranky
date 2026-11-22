use super::*;
use crate::test_support::{instant, located};

fn text(zone: &str, unix_seconds: u64) -> String {
    LocalTime::at_instant(instant(unix_seconds), &located(zone), Source::Localtime).to_string()
}

#[test]
fn the_text_is_rfc_3339_in_whole_seconds_with_a_numeric_offset() {
    for (zone, unix_seconds, expected) in [
        (
            "America/Sao_Paulo",
            1_795_000_000,
            "2026-11-18T08:06:40-03:00",
        ),
        (
            "America/Sao_Paulo",
            1_550_368_799,
            "2019-02-16T23:59:59-02:00",
        ),
        (
            "America/Sao_Paulo",
            1_550_368_800,
            "2019-02-16T23:00:00-03:00",
        ),
        (
            "America/Sao_Paulo",
            4_000_000_000,
            "2096-10-02T04:06:40-03:00",
        ),
        ("Europe/Lisbon", 1_795_000_000, "2026-11-18T11:06:40+00:00"),
        ("Europe/Lisbon", 1_774_745_999, "2026-03-29T00:59:59+00:00"),
        ("Europe/Lisbon", 1_774_746_000, "2026-03-29T02:00:00+01:00"),
        ("Europe/Lisbon", 1_792_889_999, "2026-10-25T01:59:59+01:00"),
        ("Europe/Lisbon", 1_792_890_000, "2026-10-25T01:00:00+00:00"),
        ("Europe/Lisbon", 2_531_955_600, "2050-03-27T02:00:00+01:00"),
        ("Europe/Lisbon", 2_550_704_400, "2050-10-30T01:00:00+00:00"),
        ("Asia/Kolkata", 1_795_000_000, "2026-11-18T16:36:40+05:30"),
        (
            "Australia/Sydney",
            1_782_000_000,
            "2026-06-21T10:00:00+10:00",
        ),
        (
            "Australia/Sydney",
            1_795_000_000,
            "2026-11-18T22:06:40+11:00",
        ),
        (
            "Australia/Sydney",
            2_532_528_000,
            "2050-04-03T02:00:00+10:00",
        ),
        (
            "Australia/Sydney",
            2_548_252_800,
            "2050-10-02T03:00:00+11:00",
        ),
    ] {
        let text = text(zone, unix_seconds);
        assert_eq!(text, expected, "{zone} at {unix_seconds}");
        assert_eq!(text.len(), 25);
    }
}

#[test]
fn an_inserted_leap_second_is_second_sixty() {
    for (unix_seconds, expected) in [
        (1_483_228_825, "2016-12-31T23:59:59+00:00"),
        (1_483_228_826, "2016-12-31T23:59:60+00:00"),
        (1_483_228_827, "2017-01-01T00:00:00+00:00"),
        (78_796_799, "1972-07-01T00:59:59+01:00"),
        (78_796_800, "1972-07-01T00:59:60+01:00"),
        (78_796_801, "1972-07-01T01:00:00+01:00"),
        (1_795_000_000, "2026-11-18T11:06:13+00:00"),
        (1_774_746_026, "2026-03-29T00:59:59+00:00"),
        (1_774_746_027, "2026-03-29T02:00:00+01:00"),
        (2_531_955_627, "2050-03-27T02:00:00+01:00"),
    ] {
        assert_eq!(
            text("right/Europe/Lisbon", unix_seconds),
            expected,
            "at {unix_seconds}"
        );
    }
}

#[test]
fn a_rule_is_dated_without_the_leap_seconds() {
    // Past the table, Lisbon changes at 01:00 UTC on 2050-03-27, which a
    // clock that counts 27 leap seconds reaches at 2_531_955_627. GNU `date`
    // changes at 2_531_955_600 instead: glibc dates the rule on the raw
    // scale. These two values are therefore not what `date` prints.
    for (unix_seconds, expected) in [
        (2_531_955_600, "2050-03-27T00:59:33+00:00"),
        (2_531_955_626, "2050-03-27T00:59:59+00:00"),
        (2_531_955_627, "2050-03-27T02:00:00+01:00"),
    ] {
        assert_eq!(
            text("right/Europe/Lisbon", unix_seconds),
            expected,
            "at {unix_seconds}"
        );
    }
}

#[test]
fn an_offset_with_seconds_is_shown_to_the_minute() {
    // Local mean time, before standard time: 3 h 06 min 28 s west.
    let before_1970 = LocalTime::in_zone(
        -2_300_000_000,
        0,
        &located("America/Sao_Paulo"),
        Source::Localtime,
    );
    assert_eq!(before_1970.to_string(), "1897-02-11T12:00:12-03:06");
    assert_eq!(before_1970.offset_seconds(), -11_188);
    assert_eq!(before_1970.abbreviation(), "LMT");
}

#[test]
fn utc_is_written_with_a_zero_offset_and_never_as_z() {
    let time = LocalTime::utc(1_795_000_000);
    assert_eq!(time.to_string(), "2026-11-18T11:06:40+00:00");
    assert_eq!(time.abbreviation(), "UTC");
    assert_eq!(time.zone_name(), None);
    assert_eq!(time.source(), Source::Utc);
    assert_eq!(time.subsec_millis(), 0);
    assert_eq!(LocalTime::utc(0).to_string(), "1970-01-01T00:00:00+00:00");
    assert_eq!(LocalTime::utc(-1).to_string(), "1969-12-31T23:59:59+00:00");
}

#[test]
fn the_parts_of_a_local_time_can_be_read() {
    let now = instant(1_795_000_000)
        .checked_add(std::time::Duration::from_millis(250))
        .unwrap();
    let time = LocalTime::at_instant(now, &located("Asia/Kolkata"), Source::TzVariable);
    assert_eq!((time.year(), time.month(), time.day()), (2026, 11, 18));
    assert_eq!((time.hour(), time.minute(), time.second()), (16, 36, 40));
    assert_eq!(time.subsec_millis(), 250);
    assert_eq!(time.offset_seconds(), 19_800);
    assert_eq!(time.abbreviation(), "IST");
    assert_eq!(time.zone_name(), Some("Asia/Kolkata"));
    assert_eq!(time.source(), Source::TzVariable);
}

#[test]
fn the_year_always_has_four_digits() {
    assert_eq!(
        LocalTime::utc(i64::MIN).to_string(),
        "0001-01-01T00:00:00+00:00"
    );
    assert_eq!(
        LocalTime::utc(i64::MAX).to_string(),
        "9999-12-31T23:59:59+00:00"
    );
    // The offset cannot push the local time past either end.
    let last = LocalTime::in_zone(i64::MAX, 0, &located("Asia/Kolkata"), Source::Localtime);
    assert_eq!(last.to_string(), "9999-12-31T23:59:59+05:30");
    let first = LocalTime::in_zone(
        i64::MIN,
        0,
        &located("America/Sao_Paulo"),
        Source::Localtime,
    );
    assert_eq!(first.to_string(), "0001-01-01T00:00:00-03:06");
}

#[test]
fn a_clock_set_before_1970_reads_as_1970() {
    let before = std::time::UNIX_EPOCH
        .checked_sub(std::time::Duration::from_secs(10))
        .unwrap();
    let time = LocalTime::at_instant(before, &located("Europe/Lisbon"), Source::Localtime);
    assert_eq!(time.to_string(), "1970-01-01T01:00:00+01:00");
}
