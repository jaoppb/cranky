# Zone file fixtures

`zoneinfo/` holds unmodified files of the IANA Time Zone Database, release
2026f, as compiled by `zic` for the Arch Linux `tzdata` package. The database is
in the public domain.

The tests read them so that their results do not depend on the zone database
of the machine they run on. The expected values in the tests were computed
once with GNU `date` against the same files.
