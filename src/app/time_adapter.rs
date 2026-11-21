//! Feeds the time signal: the local time, once per second, on the second.

use std::sync::Arc;
use std::time::Duration;

use cranky_time::{Clock, Source};

use crate::shared::events::signals::SignalHub;

/// What to tell the user about a time zone that did not come from where it
/// should.
const fn zone_warning(source: Source) -> Option<&'static str> {
    match source {
        Source::TzVariable | Source::Localtime => None,
        Source::LocaltimeAfterInvalidTz => {
            Some("TZ is not a zone file, a zone name or a rule; using /etc/localtime")
        }
        Source::Previous => Some("the time zone can no longer be read; keeping the last one"),
        Source::Utc => Some("no time zone could be read; showing UTC"),
    }
}

/// The warning a reading calls for: the one of its source, the first time the
/// zone comes from it and not again while it stays there.
fn warning_on_change(previous: &mut Option<Source>, source: Source) -> Option<&'static str> {
    if *previous == Some(source) {
        return None;
    }
    *previous = Some(source);
    zone_warning(source)
}

/// How long to sleep to wake at the start of the next second.
fn until_next_second(subsec_millis: u32) -> Duration {
    Duration::from_millis(1000_u64.saturating_sub(u64::from(subsec_millis)))
}

/// Sends the local time to the hub at the start of every second, and warns
/// each time the time zone starts coming from a fallback.
pub(super) async fn run(hub: Arc<SignalHub>) {
    let mut clock = Clock::new();
    let mut now = clock.now();
    let mut source = None;
    loop {
        if let Some(warning) = warning_on_change(&mut source, now.source()) {
            tracing::warn!(source = ?now.source(), "{warning}");
        }
        tokio::time::sleep(until_next_second(now.subsec_millis())).await;
        now = clock.now();
        let _ = hub.time_tx().send(now.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::Config;

    #[test]
    fn only_a_fallback_zone_is_warned_about() {
        assert_eq!(zone_warning(Source::TzVariable), None);
        assert_eq!(zone_warning(Source::Localtime), None);
        for source in [
            Source::LocaltimeAfterInvalidTz,
            Source::Previous,
            Source::Utc,
        ] {
            assert!(zone_warning(source).is_some(), "{source:?}");
        }
    }

    #[test]
    fn a_fallback_is_warned_about_once_each_time_it_starts() {
        let mut previous = None;
        let warned: Vec<bool> = [
            Source::Utc,
            Source::Utc,
            Source::Localtime,
            Source::Localtime,
            Source::Previous,
            Source::Utc,
            Source::Utc,
        ]
        .into_iter()
        .map(|source| warning_on_change(&mut previous, source).is_some())
        .collect();
        assert_eq!(warned, [true, false, false, false, true, true, false]);
    }

    #[test]
    fn the_sleep_ends_at_the_start_of_the_next_second() {
        assert_eq!(until_next_second(0), Duration::from_millis(1000));
        assert_eq!(until_next_second(250), Duration::from_millis(750));
        assert_eq!(until_next_second(999), Duration::from_millis(1));
    }

    #[test]
    fn the_time_reaches_the_hub() {
        crate::test_utils::block_on(async {
            let hub = Arc::new(SignalHub::new(Config::default()));
            let mut time_rx = hub.time_rx();
            let adapter = tokio::spawn(run(hub.clone()));

            tokio::time::timeout(Duration::from_secs(3), time_rx.changed())
                .await
                .expect("a tick within three seconds")
                .expect("the hub is alive");
            adapter.abort();
        });
    }
}
