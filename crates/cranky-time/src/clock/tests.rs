use super::*;
use crate::test_support::{NOVEMBER_2026, Scratch, environment, instant, zone_bytes, zone_path};

const SAO_PAULO: &str = "2026-11-18T08:06:40-03:00";
const KOLKATA: &str = "2026-11-18T16:36:40+05:30";
const UTC: &str = "2026-11-18T11:06:40+00:00";

/// The text and source of the local time in November 2026.
fn read(clock: &mut Clock, environment: &Environment) -> (String, Source) {
    let time = clock.at(instant(NOVEMBER_2026), environment);
    (time.to_string(), time.source())
}

#[test]
fn the_local_time_file_is_used_when_tz_is_unset() {
    let scratch = Scratch::new();
    scratch.link_zone("America/Sao_Paulo");
    let time = Clock::new().at(
        instant(NOVEMBER_2026),
        &environment(None, &scratch.localtime()),
    );
    assert_eq!(time.to_string(), SAO_PAULO);
    assert_eq!(time.source(), Source::Localtime);
    assert_eq!(time.zone_name(), Some("America/Sao_Paulo"));
    assert_eq!(time.abbreviation(), "-03");
}

#[test]
fn tz_wins_over_the_local_time_file() {
    let scratch = Scratch::new();
    scratch.link_zone("America/Sao_Paulo");
    let environment = environment(Some("Asia/Kolkata"), &scratch.localtime());
    let time = Clock::new().at(instant(NOVEMBER_2026), &environment);
    assert_eq!(time.to_string(), KOLKATA);
    assert_eq!(time.source(), Source::TzVariable);
    assert_eq!(time.zone_name(), Some("Asia/Kolkata"));
}

#[test]
fn a_retargeted_link_is_seen_on_the_next_call() {
    let scratch = Scratch::new();
    let environment = environment(None, &scratch.localtime());
    let mut clock = Clock::new();
    scratch.link_zone("America/Sao_Paulo");
    assert_eq!(
        read(&mut clock, &environment),
        (SAO_PAULO.to_string(), Source::Localtime)
    );
    scratch.link_zone("Asia/Kolkata");
    let time = clock.at(instant(NOVEMBER_2026), &environment);
    assert_eq!(time.to_string(), KOLKATA);
    assert_eq!(time.zone_name(), Some("Asia/Kolkata"));
}

#[test]
fn a_rewritten_file_is_seen_on_the_next_call() {
    let scratch = Scratch::new();
    let environment = environment(None, &scratch.localtime());
    let mut clock = Clock::new();
    scratch.copy_zone("America/Sao_Paulo");
    assert_eq!(read(&mut clock, &environment).0, SAO_PAULO);
    scratch.copy_zone("Asia/Kolkata");
    assert_eq!(
        read(&mut clock, &environment),
        (KOLKATA.to_string(), Source::Localtime)
    );
}

#[test]
fn a_half_written_file_keeps_the_previous_zone() {
    let scratch = Scratch::new();
    let environment = environment(None, &scratch.localtime());
    let mut clock = Clock::new();
    scratch.copy_zone("America/Sao_Paulo");
    assert_eq!(read(&mut clock, &environment).1, Source::Localtime);

    let whole = zone_bytes("Asia/Kolkata");
    std::fs::write(scratch.localtime(), &whole[..whole.len().div_euclid(2)]).unwrap();
    assert_eq!(
        read(&mut clock, &environment),
        (SAO_PAULO.to_string(), Source::Previous)
    );
    std::fs::write(scratch.localtime(), &whole).unwrap();
    assert_eq!(
        read(&mut clock, &environment),
        (KOLKATA.to_string(), Source::Localtime)
    );
}

#[test]
fn a_missing_file_keeps_the_previous_zone_and_its_name() {
    let scratch = Scratch::new();
    let environment = environment(None, &scratch.localtime());
    let mut clock = Clock::new();
    scratch.link_zone("America/Sao_Paulo");
    assert_eq!(read(&mut clock, &environment).1, Source::Localtime);

    std::fs::remove_file(scratch.localtime()).unwrap();
    let time = clock.at(instant(NOVEMBER_2026), &environment);
    assert_eq!(time.to_string(), SAO_PAULO);
    assert_eq!(time.source(), Source::Previous);
    assert_eq!(time.zone_name(), Some("America/Sao_Paulo"));
}

#[test]
fn a_file_that_was_never_readable_means_utc() {
    let scratch = Scratch::new();
    let time = Clock::new().at(
        instant(NOVEMBER_2026),
        &environment(None, &scratch.localtime()),
    );
    assert_eq!(time.to_string(), UTC);
    assert_eq!(time.source(), Source::Utc);
    assert_eq!(time.abbreviation(), "UTC");
}

#[test]
fn an_unusable_tz_falls_back_to_the_local_time_file() {
    let scratch = Scratch::new();
    scratch.link_zone("America/Sao_Paulo");
    let environment = environment(Some("Nope/Bogus"), &scratch.localtime());
    let time = Clock::new().at(instant(NOVEMBER_2026), &environment);
    assert_eq!(time.to_string(), SAO_PAULO);
    assert_eq!(time.source(), Source::LocaltimeAfterInvalidTz);
    assert_eq!(time.zone_name(), Some("America/Sao_Paulo"));
}

#[test]
fn an_unusable_tz_and_no_local_time_file_mean_utc() {
    let scratch = Scratch::new();
    let environment = environment(Some("Nope/Bogus"), &scratch.localtime());
    assert_eq!(
        read(&mut Clock::new(), &environment),
        (UTC.to_string(), Source::Utc)
    );
}

#[test]
fn a_zone_tz_named_is_kept_when_its_file_goes_away() {
    let scratch = Scratch::new();
    scratch.link_zone("America/Sao_Paulo");
    let zone = scratch.path("zone");
    std::fs::copy(zone_path("Asia/Kolkata"), &zone).unwrap();
    let environment = environment(zone.to_str(), &scratch.localtime());
    let mut clock = Clock::new();
    assert_eq!(
        read(&mut clock, &environment),
        (KOLKATA.to_string(), Source::TzVariable)
    );
    std::fs::remove_file(&zone).unwrap();
    assert_eq!(
        read(&mut clock, &environment),
        (KOLKATA.to_string(), Source::Previous)
    );
}

#[test]
fn a_remembered_zone_is_not_used_for_another_tz() {
    let scratch = Scratch::new();
    let mut clock = Clock::new();
    scratch.link_zone("America/Sao_Paulo");
    let unset = environment(None, &scratch.localtime());
    assert_eq!(read(&mut clock, &unset).1, Source::Localtime);

    std::fs::remove_file(scratch.localtime()).unwrap();
    let bogus = environment(Some("Nope/Bogus"), &scratch.localtime());
    assert_eq!(read(&mut clock, &bogus), (UTC.to_string(), Source::Utc));
    assert_eq!(read(&mut clock, &unset).1, Source::Previous);
}

#[test]
fn now_is_the_present_in_the_system_zone() {
    let mut clock = Clock::default();
    let time = clock.now();
    assert!(time.year() >= 2026);
    assert_eq!(time.to_string().len(), 25);
    assert!(time.subsec_millis() < 1000);
}
