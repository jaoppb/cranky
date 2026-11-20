//! The local time Cranky shows, computed in safe Rust with no dependencies.
//!
//! A [`Clock`] finds the system time zone the way the C library does: the `TZ`
//! variable when it is set, `/etc/localtime` otherwise. It reads the zone again
//! on every call, so a zone change is seen without a restart. It never fails:
//! every [`LocalTime`] names the [`Source`] its zone came from, down to UTC
//! when nothing else can be read.

mod civil;
mod clock;
mod environment;
mod local_time;
mod posix_parse;
mod posix_rule;
mod tzif;
mod zone;

pub use clock::Clock;
pub use environment::Environment;
pub use local_time::{LocalTime, Source};

#[cfg(test)]
mod test_support;
