//! What every case shares: the failure type and the deadline on each wait.

use std::fmt::{self, Display};
use std::future::Future;
use std::time::Duration;

/// Deadline of a single wait. No case sleeps to synchronise: it waits on
/// something observable, and gives up after this long.
pub const WAIT: Duration = Duration::from_secs(5);

/// Why a case, or the sandbox around the cases, failed.
#[derive(Debug)]
pub struct Failure(String);

impl Failure {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub type Outcome<T = ()> = Result<T, Failure>;

/// Turns any error into a [`Failure`] that says what was being attempted.
pub trait Context<T> {
    fn context(self, what: &str) -> Outcome<T>;
}

impl<T, E: Display> Context<T> for Result<T, E> {
    fn context(self, what: &str) -> Outcome<T> {
        self.map_err(|error| Failure(format!("{what}: {error}")))
    }
}

impl<T> Context<T> for Option<T> {
    fn context(self, what: &str) -> Outcome<T> {
        self.ok_or_else(|| Failure(format!("{what}: missing")))
    }
}

/// Fails the case unless `condition` holds.
pub fn ensure(condition: bool, message: impl FnOnce() -> String) -> Outcome {
    if condition {
        Ok(())
    } else {
        Err(Failure(message()))
    }
}

/// Fails the case unless `actual` equals `expected`.
pub fn ensure_eq<T: PartialEq + fmt::Debug>(what: &str, actual: &T, expected: &T) -> Outcome {
    ensure(actual == expected, || {
        format!("{what}: expected {expected:?}, got {actual:?}")
    })
}

/// Awaits `future`, failing once [`WAIT`] has passed.
pub async fn within<T>(what: &str, future: impl Future<Output = T>) -> Outcome<T> {
    within_for(WAIT, what, future).await
}

/// Awaits `future`, failing once `deadline` has passed.
pub async fn within_for<T>(
    deadline: Duration,
    what: &str,
    future: impl Future<Output = T>,
) -> Outcome<T> {
    tokio::time::timeout(deadline, future)
        .await
        .map_err(|_| Failure(format!("timed out after {deadline:?} waiting for {what}")))
}
