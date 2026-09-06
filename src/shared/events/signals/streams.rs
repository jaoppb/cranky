use std::collections::HashSet;

use futures_util::stream::{BoxStream, SelectAll};
use futures_util::StreamExt;
use tokio_stream::wrappers::WatchStream;

use super::hub::SignalHub;
use super::kind::SignalKind;

impl SignalHub {
    #[must_use]
    pub fn signal_to_stream(&self, kind: &SignalKind) -> BoxStream<'static, SignalKind> {
        match kind {
            SignalKind::Time => WatchStream::new(self.time_rx())
                .map(|_| SignalKind::Time)
                .boxed(),
            SignalKind::Hyprland => WatchStream::new(self.hyprland_rx())
                .map(|_| SignalKind::Hyprland)
                .boxed(),
            SignalKind::Systray => WatchStream::new(self.systray_rx())
                .map(|_| SignalKind::Systray)
                .boxed(),
            SignalKind::Metrics => WatchStream::new(self.metrics_rx())
                .map(|_| SignalKind::Metrics)
                .boxed(),
            SignalKind::Mpris => WatchStream::new(self.mpris_rx())
                .map(|_| SignalKind::Mpris)
                .boxed(),
            SignalKind::DBus => WatchStream::new(self.dbus_rx())
                .map(|_| SignalKind::DBus)
                .boxed(),
        }
    }

    pub fn subscribe_streams(
        &self,
        subs: &[SignalKind],
    ) -> SelectAll<BoxStream<'static, SignalKind>> {
        let mut events_stream = SelectAll::new();
        let mut seen = HashSet::new();

        for kind in subs {
            if seen.insert(kind) {
                events_stream.push(self.signal_to_stream(kind));
            }
        }

        events_stream
    }
}
