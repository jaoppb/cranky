# Zone file fixtures

`zoneinfo/` holds unmodified files of the IANA Time Zone Database, release
2026f, as compiled by `zic` for the Arch Linux `tzdata` package. The database is
in the public domain.

The tests read them so that their results do not depend on the zone database
of the machine they run on. The expected values in the tests were computed
once with GNU `date` against the same files.

A few tests differ from `date` on purpose, where glibc does not follow the
rule as written:

- In a `right/` zone, after the last transition of the table, glibc dates the
  rule of the footer on the scale that counts leap seconds, and so changes 27
  seconds early. This crate dates it on the scale without them.
  `a_rule_is_dated_without_the_leap_seconds` pins the difference.
- glibc looks for the changes of a rule only in the year of the instant,
  counted in UTC. This crate also looks in the years around it, which matters
  within a day of a new year and when a change time reaches into another year: `a_change_time_may_reach_into_the_next_or_previous_year`,
  `both_changes_of_a_year_may_fall_in_the_next_one` and
  `daylight_time_that_ends_as_it_starts_is_permanent`.

Two more tests have no counterpart in `date`, because they pin limits this
crate sets for itself: `the_year_always_has_four_digits` and
`a_clock_set_before_1970_reads_as_1970`.
