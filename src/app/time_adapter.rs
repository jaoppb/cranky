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

/// Sends the local time to the hub at the start of every second, and warns
/// each time the time zone starts coming from a fallback.
pub async fn run(hub: Arc<SignalHub>) {
    let mut clock = Clock::new();
    let mut now = clock.now();
    let mut source = None;
    loop {
        if source != Some(now.source()) {
            source = Some(now.source());
            if let Some(warning) = zone_warning(now.source()) {
                tracing::warn!("{warning}");
            }
        }
        let until_next_second = 1000_u64.saturating_sub(u64::from(now.subsec_millis()));
        tokio::time::sleep(Duration::from_millis(until_next_second)).await;
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
    fn the_time_is_sent_at_the_start_of_the_next_second() {
        crate::test_utils::block_on(async {
            let hub = Arc::new(SignalHub::new(Config::default()));
            let mut time_rx = hub.time_rx();
            let adapter = tokio::spawn(run(hub.clone()));

            tokio::time::timeout(Duration::from_secs(3), time_rx.changed())
                .await
                .expect("a tick within three seconds")
                .expect("the hub is alive");
            let sent = time_rx.borrow().clone();
            adapter.abort();

            assert!(sent.subsec_millis() < 500, "{} ms", sent.subsec_millis());
            assert_eq!(sent.to_string().len(), 25);
        });
    }
}
