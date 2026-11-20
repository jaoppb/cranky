use std::path::PathBuf;

use super::*;
use crate::test_support::{Scratch, database, environment, zone_path};

/// The offset in November 2026 of the zone `TZ` selects, and its name.
fn from_tz(tz: &str) -> Option<(i64, Option<String>)> {
    let located = environment(Some(tz), &PathBuf::from("/nonexistent")).zone_from_tz()?;
    Some((located.zone.at(1_795_000_000).kind.offset, located.name))
}

#[test]
fn tz_may_name_a_zone_of_a_database() {
    let kolkata = Some((19_800, Some("Asia/Kolkata".to_string())));
    assert_eq!(from_tz("Asia/Kolkata"), kolkata);
    assert_eq!(from_tz(":Asia/Kolkata"), kolkata);
    assert_eq!(
        from_tz("right/Europe/Lisbon"),
        Some((0, Some("right/Europe/Lisbon".to_string())))
    );
}

#[test]
fn tz_may_be_the_path_of_a_zone_file() {
    let path = zone_path("Australia/Sydney");
    let path = path.to_str().unwrap();
    assert_eq!(from_tz(path), Some((39_600, None)));
    assert_eq!(from_tz(&format!(":{path}")), Some((39_600, None)));
    assert_eq!(from_tz("/nonexistent/zone"), None);
}

#[test]
fn tz_may_be_a_rule() {
    assert_eq!(from_tz("<-03>3"), Some((-10_800, None)));
    assert_eq!(from_tz("WET0WEST,M3.5.0/1,M10.5.0"), Some((0, None)));
    assert_eq!(
        from_tz("AEST-10AEDT,M10.1.0,M4.1.0/3"),
        Some((39_600, None))
    );
}

#[test]
fn an_empty_tz_is_utc() {
    assert_eq!(from_tz(""), Some((0, None)));
    assert_eq!(from_tz(":"), Some((0, None)));
}

#[test]
fn anything_else_in_tz_selects_nothing() {
    for tz in ["Nope/Bogus", "Asia", "Asia/", "/", "x"] {
        assert_eq!(from_tz(tz), None, "{tz:?}");
    }
    assert_eq!(
        environment(None, &PathBuf::from("/nonexistent")).zone_from_tz(),
        None
    );
}

#[test]
fn a_name_cannot_leave_the_database() {
    let scratch = Scratch::new();
    std::fs::create_dir(scratch.path("zoneinfo")).unwrap();
    std::fs::copy(zone_path("Asia/Kolkata"), scratch.path("outside")).unwrap();
    let environment =
        |tz| Environment::new(Some(tz), "/nonexistent", vec![scratch.path("zoneinfo")]);
    assert!(scratch.path("zoneinfo/../outside").exists());
    assert_eq!(environment("../outside").zone_from_tz(), None);
    assert_eq!(environment("./../outside").zone_from_tz(), None);
}

#[test]
fn databases_are_searched_in_order() {
    let scratch = Scratch::new();
    std::fs::copy(zone_path("Asia/Kolkata"), scratch.path("Zone")).unwrap();
    let offset = |databases| {
        let located = Environment::new(Some("Europe/Lisbon"), "/nonexistent", databases)
            .zone_from_tz()
            .unwrap();
        located.zone.at(1_795_000_000).kind.offset
    };
    assert_eq!(offset(vec![scratch.path("missing"), database()]), 0);

    let other = scratch.path("Europe");
    std::fs::create_dir(&other).unwrap();
    std::fs::copy(zone_path("Asia/Kolkata"), other.join("Lisbon")).unwrap();
    assert_eq!(offset(vec![scratch.path(""), database()]), 19_800);
}

#[test]
fn the_local_time_file_names_its_zone_when_it_links_into_a_database() {
    let scratch = Scratch::new();
    scratch.link_zone("America/Sao_Paulo");
    let located = environment(None, &scratch.localtime())
        .zone_from_localtime()
        .unwrap();
    assert_eq!(located.name.as_deref(), Some("America/Sao_Paulo"));
    assert_eq!(located.zone.at(1_795_000_000).kind.offset, -10_800);
}

#[test]
fn a_copied_local_time_file_has_no_name() {
    let scratch = Scratch::new();
    scratch.copy_zone("America/Sao_Paulo");
    let located = environment(None, &scratch.localtime())
        .zone_from_localtime()
        .unwrap();
    assert_eq!(located.name, None);
    assert_eq!(located.zone.at(1_795_000_000).kind.offset, -10_800);
}

#[test]
fn a_link_outside_a_database_has_no_name() {
    let scratch = Scratch::new();
    std::fs::copy(zone_path("Asia/Kolkata"), scratch.path("elsewhere")).unwrap();
    std::os::unix::fs::symlink(scratch.path("elsewhere"), scratch.localtime()).unwrap();
    let located = environment(None, &scratch.localtime())
        .zone_from_localtime()
        .unwrap();
    assert_eq!(located.name, None);
    assert_eq!(located.zone.at(1_795_000_000).kind.offset, 19_800);
}

#[test]
fn an_unreadable_local_time_file_selects_nothing() {
    let scratch = Scratch::new();
    let environment = environment(None, &scratch.localtime());
    assert_eq!(environment.zone_from_localtime(), None);
    std::fs::write(scratch.localtime(), b"not a zone file").unwrap();
    assert_eq!(environment.zone_from_localtime(), None);
    std::os::unix::fs::symlink(
        "/nonexistent/zoneinfo/Asia/Kolkata",
        scratch.path("dangling"),
    )
    .unwrap();
    let dangling = Environment::new(None, scratch.path("dangling"), Vec::new());
    assert_eq!(dangling.zone_from_localtime(), None);
}

#[test]
fn a_file_without_end_is_given_up_on() {
    assert_eq!(from_tz("/dev/zero"), None);
}

#[test]
fn the_system_environment_reads_the_usual_places() {
    let system = Environment::system();
    assert_eq!(system.localtime, PathBuf::from("/etc/localtime"));
    assert!(system.databases.ends_with(&[
        PathBuf::from("/usr/share/zoneinfo"),
        PathBuf::from("/etc/zoneinfo"),
    ]));
    assert_eq!(
        system.databases.len() == 3,
        std::env::var_os("TZDIR").is_some()
    );
    assert_eq!(system.tz().is_some(), std::env::var_os("TZ").is_some());
}
