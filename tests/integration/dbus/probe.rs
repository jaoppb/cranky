//! A fake service on the private bus for the connection adapter to talk to.
//!
//! Written with plain Rust types so the `zbus` bump of #30 has nothing to
//! migrate here. The one exception is [`Variant`]: the connection adapter
//! only accepts bodies that are a single variant (#41), so the fake has to
//! produce one.

use crate::support::{Context, Outcome};
use std::collections::HashMap;
use zbus::object_server::{InterfaceRef, SignalEmitter};

pub(crate) const NAME: &str = "org.cranky.Probe";
pub(crate) const PATH: &str = "/org/cranky/Probe";
pub(crate) const INTERFACE: &str = "org.cranky.Probe";

type Variant = zbus::Variant<'static>;

/// What the `Enabled` property reports. The probe is always on: the case
/// that reads it only checks that a boolean arrives as one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Switch {
    /// Reported as `true`.
    Enabled,
}

pub(crate) struct Probe {
    label: String,
    wrapped: &'static str,
    bare: &'static str,
    count: i32,
    enabled: Switch,
    tags: Vec<String>,
    limits: HashMap<String, i32>,
}

#[zbus::interface(name = "org.cranky.Probe")]
impl Probe {
    /// Replies with a variant, the only reply `call_method` understands.
    fn wrapped(&self) -> Variant {
        Variant::from(self.wrapped)
    }

    /// Replies with a bare string, which `call_method` rejects (#41).
    fn bare(&self) -> String {
        self.bare.to_string()
    }

    #[zbus(property)]
    fn label(&self) -> String {
        self.label.clone()
    }

    #[zbus(property)]
    const fn count(&self) -> i32 {
        self.count
    }

    #[zbus(property)]
    const fn enabled(&self) -> bool {
        matches!(self.enabled, Switch::Enabled)
    }

    #[zbus(property)]
    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    #[zbus(property)]
    fn limits(&self) -> HashMap<String, i32> {
        self.limits.clone()
    }

    #[zbus(signal)]
    async fn pinged(emitter: &SignalEmitter<'_>, payload: Variant) -> zbus::Result<()>;
}

/// The fake's own connection to the private bus, owning [`NAME`].
pub(crate) struct ProbeService {
    connection: zbus::Connection,
}

impl ProbeService {
    pub(crate) async fn start() -> Outcome<Self> {
        let probe = Probe {
            label: "first".to_string(),
            wrapped: "wrapped reply",
            bare: "bare reply",
            count: 7,
            enabled: Switch::Enabled,
            tags: vec!["one".to_string(), "two".to_string()],
            limits: HashMap::from([("max".to_string(), 9)]),
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
    pub(crate) async fn ping(&self, payload: &'static str) -> Outcome {
        let interface = self.interface().await?;
        Probe::pinged(interface.signal_emitter(), Variant::from(payload))
            .await
            .context("probe: emit Pinged")
    }

    /// Emits `stray` from the probe's connection, with a variant body naming
    /// it, whether or not the probe declares a signal at that address.
    pub(crate) async fn emit_stray(&self, stray: Stray) -> Outcome {
        let (path, interface, member) = stray.address();
        self.connection
            .emit_signal(
                None::<&str>,
                path,
                interface,
                member,
                &Variant::from(format!("{stray:?}")),
            )
            .await
            .context(&format!("probe: emit the {stray:?} stray"))
    }

    /// Changes `Label` and emits the matching `PropertiesChanged`.
    pub(crate) async fn relabel(&self, label: &str) -> Outcome {
        let interface = self.interface().await?;
        let mut probe = interface.get_mut().await;
        probe.label = label.to_string();
        probe
            .label_changed(interface.signal_emitter())
            .await
            .context("probe: emit PropertiesChanged")
    }
}

/// A signal that differs from `Pinged` in exactly one part of its address,
/// for cases that check a subscription does not receive it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Stray {
    /// Another member of the probe's interface, at the probe's path.
    Member,
    /// `Pinged` of another interface, at the probe's path.
    Interface,
    /// `Pinged` of the probe's interface, at another path.
    Path,
}

impl Stray {
    /// The path, interface and member the stray signal is sent with.
    const fn address(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Member => (PATH, INTERFACE, "Ponged"),
            Self::Interface => (PATH, "org.cranky.Elsewhere", "Pinged"),
            Self::Path => ("/org/cranky/Elsewhere", INTERFACE, "Pinged"),
        }
    }
}
