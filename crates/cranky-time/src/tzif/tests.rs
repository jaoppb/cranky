use super::*;
use crate::test_support::zone_bytes;

/// The offset and abbreviation a checked-in zone has at an instant.
fn seen(name: &str, instant: i64) -> (i64, String) {
    let kind = parse(&zone_bytes(name)).unwrap().at(instant).kind;
    (kind.offset, kind.abbreviation)
}

fn expect(name: &str, cases: &[(i64, i64, &str)]) {
    for &(instant, offset, abbreviation) in cases {
        assert_eq!(
            seen(name, instant),
            (offset, abbreviation.to_string()),
            "{name} at {instant}"
        );
    }
}

#[test]
fn a_zone_without_daylight_time_keeps_its_last_offset() {
    expect(
        "America/Sao_Paulo",
        &[
            (-2_300_000_000, -11_188, "LMT"),
            (1_550_368_799, -7200, "-02"),
            (1_550_368_800, -10_800, "-03"),
            (1_795_000_000, -10_800, "-03"),
            (4_000_000_000, -10_800, "-03"),
        ],
    );
}

#[test]
fn transitions_in_the_table_apply_from_their_instant() {
    expect(
        "Europe/Lisbon",
        &[
            (-2_000_000_000, -2205, "LMT"),
            (1_774_745_999, 0, "WET"),
            (1_774_746_000, 3600, "WEST"),
            (1_792_889_999, 3600, "WEST"),
            (1_792_890_000, 0, "WET"),
        ],
    );
}

#[test]
fn the_rule_covers_the_years_after_the_table() {
    expect(
        "Europe/Lisbon",
        &[
            (2_531_955_599, 0, "WET"),
            (2_531_955_600, 3600, "WEST"),
            (2_550_704_399, 3600, "WEST"),
            (2_550_704_400, 0, "WET"),
        ],
    );
    expect(
        "Australia/Sydney",
        &[
            (1_782_000_000, 36_000, "AEST"),
            (1_795_000_000, 39_600, "AEDT"),
            (2_532_527_999, 39_600, "AEDT"),
            (2_532_528_000, 36_000, "AEST"),
            (2_548_252_799, 36_000, "AEST"),
            (2_548_252_800, 39_600, "AEDT"),
        ],
    );
}

#[test]
fn offsets_need_not_be_whole_hours() {
    expect(
        "Asia/Kolkata",
        &[
            (-3_000_000_000, 19_270, "MMT"),
            (1_795_000_000, 19_800, "IST"),
            (4_000_000_000, 19_800, "IST"),
        ],
    );
}

#[test]
fn a_leap_second_file_counts_its_leap_seconds() {
    let zone = parse(&zone_bytes("right/Europe/Lisbon")).unwrap();
    // The 27th leap second, at the end of 2016.
    let before = zone.at(1_483_228_825);
    let inserted = zone.at(1_483_228_826);
    let after = zone.at(1_483_228_827);
    assert_eq!(
        (before.unix_seconds, before.leap_second),
        (1_483_228_799, false)
    );
    assert_eq!(
        (inserted.unix_seconds, inserted.leap_second),
        (1_483_228_799, true)
    );
    assert_eq!(
        (after.unix_seconds, after.leap_second),
        (1_483_228_800, false)
    );
    // Transitions are stored on the same scale as the instants asked about.
    assert_eq!(zone.at(1_774_746_026).kind.abbreviation, "WET");
    assert_eq!(zone.at(1_774_746_027).kind.abbreviation, "WEST");
    assert_eq!(zone.at(1_795_000_000).unix_seconds, 1_794_999_973);
}

#[test]
fn every_truncation_of_a_file_is_refused() {
    for name in ["America/Sao_Paulo", "Europe/Lisbon", "right/Europe/Lisbon"] {
        let bytes = zone_bytes(name);
        assert!(parse(&bytes).is_some());
        for length in 0..bytes.len() {
            assert_eq!(parse(&bytes[..length]), None, "{name} cut at {length}");
        }
    }
}

fn count(value: usize) -> [u8; 4] {
    u32::try_from(value).unwrap().to_be_bytes()
}

/// A header and a block of data: transitions as (instant, kind), kinds as
/// (offset, index of the name), and leap seconds as (instant, total).
fn file_part(
    version: u8,
    wide: bool,
    transitions: &[(i64, u8)],
    kinds: &[(i32, u8)],
    names: &[u8],
    leaps: &[(i64, i32)],
) -> Vec<u8> {
    let instant = |at: i64| {
        if wide {
            at.to_be_bytes().to_vec()
        } else {
            i32::try_from(at).unwrap().to_be_bytes().to_vec()
        }
    };
    let mut bytes = b"TZif".to_vec();
    bytes.push(version);
    bytes.extend([0; 15]);
    for size in [
        0,
        0,
        leaps.len(),
        transitions.len(),
        kinds.len(),
        names.len(),
    ] {
        bytes.extend(count(size));
    }
    for &(at, _) in transitions {
        bytes.extend(instant(at));
    }
    bytes.extend(transitions.iter().map(|&(_, kind)| kind));
    for &(offset, name) in kinds {
        bytes.extend(offset.to_be_bytes());
        bytes.extend([0, name]);
    }
    bytes.extend(names);
    for &(at, total) in leaps {
        bytes.extend(instant(at));
        bytes.extend(total.to_be_bytes());
    }
    bytes
}

const KINDS: &[(i32, u8)] = &[(-3600, 0), (7200, 4)];
const NAMES: &[u8] = b"OLD\0NEW\0";

fn version_one(transitions: &[(i64, u8)], kinds: &[(i32, u8)], names: &[u8]) -> Vec<u8> {
    file_part(0, false, transitions, kinds, names, &[])
}

fn version_two(footer: &[u8]) -> Vec<u8> {
    let mut bytes = file_part(b'2', false, &[], &[(0, 0)], b"X\0", &[]);
    bytes.extend(file_part(
        b'2',
        true,
        &[(1000, 1)],
        KINDS,
        NAMES,
        &[(2000, 1)],
    ));
    bytes.extend(footer);
    bytes
}

#[test]
fn a_version_one_file_is_read_from_its_only_block() {
    let zone = parse(&version_one(&[(1000, 1), (2000, 0)], KINDS, NAMES)).unwrap();
    let seen = |instant| {
        let kind = zone.at(instant).kind;
        (kind.offset, kind.abbreviation)
    };
    assert_eq!(seen(999), (-3600, "OLD".to_string()));
    assert_eq!(seen(1000), (7200, "NEW".to_string()));
    assert_eq!(seen(2000), (-3600, "OLD".to_string()));
    assert_eq!(seen(i64::MAX), (-3600, "OLD".to_string()));
}

#[test]
fn a_later_version_is_read_from_its_second_block() {
    let zone = parse(&version_two(b"\n\n")).unwrap();
    assert_eq!(zone.at(999).kind.abbreviation, "OLD");
    assert_eq!(zone.at(1000).kind.abbreviation, "NEW");
    assert_eq!(zone.at(i64::MAX).kind.abbreviation, "NEW");
    assert_eq!(zone.at(2000).unix_seconds, 1999);
    assert!(zone.at(2000).leap_second);
}

#[test]
fn the_footer_rule_applies_after_the_last_transition() {
    let zone = parse(&version_two(b"\n<+05>-5\n")).unwrap();
    assert_eq!(zone.at(999).kind.abbreviation, "OLD");
    assert_eq!(zone.at(1000).kind.offset, 18_000);
}

#[test]
fn an_unreadable_footer_rule_is_left_out() {
    let zone = parse(&version_two(b"\nnot a rule\n")).unwrap();
    assert_eq!(zone.at(i64::MAX).kind.abbreviation, "NEW");
}

#[test]
fn a_later_version_needs_both_newlines_of_its_footer() {
    assert_eq!(parse(&version_two(b"")), None);
    assert_eq!(parse(&version_two(b"<+05>-5\n")), None);
    assert_eq!(parse(&version_two(b"\n<+05>-5")), None);
}

#[test]
fn malformed_files_are_refused() {
    let good = version_one(&[(1000, 1)], KINDS, NAMES);
    assert!(parse(&good).is_some());

    let mut wrong_magic = good.clone();
    wrong_magic[0] = b'X';
    assert_eq!(parse(&wrong_magic), None);
    assert_eq!(parse(b"TZif"), None);
    assert_eq!(parse(b""), None);

    let no_kinds = version_one(&[], &[], NAMES);
    assert_eq!(parse(&no_kinds), None);
    let missing_kind = version_one(&[(1000, 2)], KINDS, NAMES);
    assert_eq!(parse(&missing_kind), None);
    let name_out_of_range = version_one(&[], &[(0, 9)], NAMES);
    assert_eq!(parse(&name_out_of_range), None);
    let name_without_end = version_one(&[], &[(0, 0)], b"OLD");
    assert_eq!(parse(&name_without_end), None);
    let name_not_utf8 = version_one(&[], &[(0, 0)], b"\xff\xfe\0");
    assert_eq!(parse(&name_not_utf8), None);
    let offset_too_far = version_one(&[], &[(93_600, 0)], NAMES);
    assert_eq!(parse(&offset_too_far), None);

    // A count larger than the file must not be believed.
    let mut huge_count = good;
    huge_count[32..36].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(parse(&huge_count), None);

    let mut second_header_wrong = version_two(b"\n\n");
    let second = second_header_wrong
        .windows(4)
        .rposition(|window| window == b"TZif")
        .unwrap();
    assert_ne!(second, 0);
    second_header_wrong[second] = b'X';
    assert_eq!(parse(&second_header_wrong), None);
}
