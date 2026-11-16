//! A fake service on the private bus for the connection adapter to talk to.
//!
//! Written with plain Rust types so the `zbus` bump of #30 has nothing to
//! migrate here. The one exception is [`Variant`]: the connection adapter
//! only accepts bodies that are a single variant (#41), so the fake has to
//! produce one.

use crate::support::{Context, Outcome};
use std::collections::HashMap;
use zbus::object_server::{InterfaceRef, SignalEmitter};

pub const NAME: &str = "org.cranky.Probe";
pub const PATH: &str = "/org/cranky/Probe";
pub const INTERFACE: &str = "org.cranky.Probe";

type Variant = zbus::Variant<'static>;

pub struct Probe {
    label: String,
}

#[zbus::interface(name = "org.cranky.Probe")]
impl Probe {
    /// Replies with a variant, the only reply `call_method` understands.
    #[allow(clippy::unused_self)]
    fn wrapped(&self) -> Variant {
        Variant::from("wrapped reply")
    }

    /// Replies with a bare string, which `call_method` rejects (#41).
    #[allow(clippy::unused_self)]
    fn bare(&self) -> String {
        "bare reply".to_string()
    }

    #[zbus(property)]
    fn label(&self) -> String {
        self.label.clone()
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    const fn count(&self) -> i32 {
        7
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    const fn enabled(&self) -> bool {
        true
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    fn tags(&self) -> Vec<String> {
        vec!["one".to_string(), "two".to_string()]
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    fn limits(&self) -> HashMap<String, i32> {
        HashMap::from([("max".to_string(), 9)])
    }

    #[zbus(signal)]
    async fn pinged(emitter: &SignalEmitter<'_>, payload: Variant) -> zbus::Result<()>;
}

/// The fake's own connection to the private bus, owning [`NAME`].
pub struct ProbeService {
    connection: zbus::Connection,
}

impl ProbeService {
    pub async fn start() -> Outcome<Self> {
        let probe = Probe {
            label: "first".to_string(),
        };
        let connection = zbus::connection::Builder::session()
            .name(NAME)
            .serve_at(PATH, probe)
            .build()
            .await
            .context("probe: connect")?;
        Ok(Self { connection })
    }

    async fn interface(&self) -> Outcome<InterfaceRef<Probe>> {
        self.connection
            .object_server()
            .interface::<_, Probe>(PATH)
            .await
            .context("probe: interface")
    }

    /// Emits `Pinged` with a variant body holding `payload`.
    pub async fn ping(&self, payload: &'static str) -> Outcome {
        let interface = self.interface().await?;
        Probe::pinged(interface.signal_emitter(), Variant::from(payload))
            .await
            .context("probe: emit Pinged")
    }

    /// Changes `Label` and emits the matching `PropertiesChanged`.
    pub async fn relabel(&self, label: &str) -> Outcome {
        let interface = self.interface().await?;
        let mut probe = interface.get_mut().await;
        probe.label = label.to_string();
        probe
            .label_changed(interface.signal_emitter())
            .await
            .context("probe: emit PropertiesChanged")
    }
}
