# Zone file fixtures

`zoneinfo/` holds unmodified files of the IANA Time Zone Database, release
2026f, as compiled by `zic` for the Arch Linux `tzdata` package. The database is
in the public domain.

The tests read them so that their results do not depend on the zone database
of the machine they run on. The expected values in the tests were computed
once with GNU `date` against the same files.

One test differs from `date` on purpose. In a `right/` zone, after the last
transition of the table, glibc dates the rule of the footer on the scale that
counts leap seconds, and so changes 27 seconds early. This crate dates it on
the scale without them. `a_rule_is_dated_without_the_leap_seconds` pins the
difference.
