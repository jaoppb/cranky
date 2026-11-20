//! A time zone: what the local time is at any instant.

use crate::posix_rule::Rule;

/// One kind of local time a zone observes, such as its standard time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kind {
    /// Seconds east of UTC.
    pub offset: i64,
    pub abbreviation: String,
}

/// The instant from which a zone observes one of its kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    pub at: i64,
    /// An index into the kinds of the zone.
    pub kind: usize,
}

/// A leap second: from `at`, the clock counts `total` seconds more than UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leap {
    pub at: i64,
    pub total: i64,
}

/// What a zone says about one instant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moment {
    pub kind: Kind,
    /// The instant with the leap seconds taken out.
    pub unix_seconds: i64,
    /// The instant is an inserted leap second, shown as second 60.
    pub leap_second: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zone {
    /// Never empty; the first one applies before the first transition.
    kinds: Vec<Kind>,
    /// In ascending order.
    transitions: Vec<Transition>,
    leaps: Vec<Leap>,
    /// Applies after the last transition.
    rule: Option<Rule>,
}

impl Zone {
    /// `None` when there is no kind, or a transition names a missing one.
    pub fn new(
        kinds: Vec<Kind>,
        transitions: Vec<Transition>,
        leaps: Vec<Leap>,
        rule: Option<Rule>,
    ) -> Option<Self> {
        let valid = !kinds.is_empty() && transitions.iter().all(|t| t.kind < kinds.len());
        valid.then_some(Self {
            kinds,
            transitions,
            leaps,
            rule,
        })
    }

    pub fn utc() -> Self {
        Self {
            kinds: vec![Kind {
                offset: 0,
                abbreviation: "UTC".to_string(),
            }],
            transitions: Vec::new(),
            leaps: Vec::new(),
            rule: None,
        }
    }

    /// A zone that follows one rule at every instant.
    pub fn from_rule(rule: Rule) -> Self {
        Self {
            kinds: vec![rule.standard().clone()],
            transitions: Vec::new(),
            leaps: Vec::new(),
            rule: Some(rule),
        }
    }

    pub fn at(&self, instant: i64) -> Moment {
        let (correction, leap_second) = self.leap_correction(instant);
        let unix_seconds = instant.saturating_sub(correction);
        let passed = self.transitions.partition_point(|t| t.at <= instant);
        // The rule is read on the scale without leap seconds, where its dates
        // mean what they say. The C library reads it on the raw scale, and so
        // changes that many seconds early in a leap second zone.
        let kind = match (&self.rule, passed.checked_sub(1)) {
            (Some(rule), _) if passed == self.transitions.len() => Some(rule.kind_at(unix_seconds)),
            (_, Some(last)) => self
                .transitions
                .get(last)
                .and_then(|transition| self.kinds.get(transition.kind)),
            (_, None) => self.kinds.first(),
        };
        Moment {
            kind: kind.cloned().unwrap_or_else(|| Kind {
                offset: 0,
                abbreviation: "UTC".to_string(),
            }),
            unix_seconds,
            leap_second,
        }
    }

    /// The leap seconds counted up to an instant, and whether the instant is
    /// itself an inserted one.
    fn leap_correction(&self, instant: i64) -> (i64, bool) {
        let passed = self.leaps.partition_point(|leap| leap.at <= instant);
        let Some(index) = passed.checked_sub(1) else {
            return (0, false);
        };
        let Some(leap) = self.leaps.get(index) else {
            return (0, false);
        };
        let before = index
            .checked_sub(1)
            .and_then(|previous| self.leaps.get(previous))
            .map_or(0, |previous| previous.total);
        (leap.total, leap.at == instant && leap.total > before)
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

    fn two_kinds() -> Zone {
        Zone::new(
            vec![kind(-100, "OLD"), kind(3600, "NEW")],
            vec![
                Transition { at: 1000, kind: 1 },
                Transition { at: 2000, kind: 0 },
            ],
            Vec::new(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn the_first_kind_applies_before_the_first_transition() {
        assert_eq!(two_kinds().at(999).kind, kind(-100, "OLD"));
    }

    #[test]
    fn a_transition_applies_from_its_own_instant() {
        let zone = two_kinds();
        assert_eq!(zone.at(1000).kind, kind(3600, "NEW"));
        assert_eq!(zone.at(1999).kind, kind(3600, "NEW"));
        assert_eq!(zone.at(2000).kind, kind(-100, "OLD"));
    }

    #[test]
    fn without_a_rule_the_last_transition_lasts_forever() {
        assert_eq!(two_kinds().at(i64::MAX).kind, kind(-100, "OLD"));
    }

    #[test]
    fn a_zone_needs_a_kind_and_valid_transitions() {
        assert_eq!(Zone::new(Vec::new(), Vec::new(), Vec::new(), None), None);
        let dangling = vec![Transition { at: 0, kind: 1 }];
        assert_eq!(
            Zone::new(vec![kind(0, "X")], dangling, Vec::new(), None),
            None
        );
    }

    #[test]
    fn utc_has_no_offset_at_any_instant() {
        let moment = Zone::utc().at(1_795_000_000);
        assert_eq!(moment.kind, kind(0, "UTC"));
        assert_eq!(moment.unix_seconds, 1_795_000_000);
        assert!(!moment.leap_second);
    }

    #[test]
    fn leap_seconds_are_taken_out_and_the_inserted_one_is_flagged() {
        let leaps = vec![Leap { at: 100, total: 1 }, Leap { at: 200, total: 2 }];
        let zone = Zone::new(vec![kind(0, "X")], Vec::new(), leaps, None).unwrap();
        let seen = |instant| {
            let moment = zone.at(instant);
            (moment.unix_seconds, moment.leap_second)
        };
        assert_eq!(seen(99), (99, false));
        assert_eq!(seen(100), (99, true), "the inserted second repeats 99");
        assert_eq!(seen(101), (100, false));
        assert_eq!(seen(200), (198, true));
        assert_eq!(seen(201), (199, false));
    }

    #[test]
    fn a_repeated_leap_total_inserts_nothing() {
        // The last record of a file may only mark when the table expires.
        let leaps = vec![Leap { at: 100, total: 1 }, Leap { at: 200, total: 1 }];
        let zone = Zone::new(vec![kind(0, "X")], Vec::new(), leaps, None).unwrap();
        assert_eq!(zone.at(200).unix_seconds, 199);
        assert!(!zone.at(200).leap_second);
    }
}
