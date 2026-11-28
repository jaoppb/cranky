//! Turns signal kinds into streams a module actor can await.

use std::collections::HashSet;

use futures_util::StreamExt;
use futures_util::stream::{BoxStream, SelectAll};
use tokio_stream::wrappers::WatchStream;

use super::hub::SignalHub;
use super::kind::SignalKind;

impl SignalHub {
    pub(crate) fn signal_to_stream(&self, kind: SignalKind) -> BoxStream<'static, SignalKind> {
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

    pub(crate) fn subscribe_streams(
        &self,
        subs: &[SignalKind],
    ) -> SelectAll<BoxStream<'static, SignalKind>> {
        let mut seen = HashSet::new();
        subs.iter()
            .filter(|kind| seen.insert(**kind))
            .map(|kind| self.signal_to_stream(*kind))
            .collect()
    }
}
