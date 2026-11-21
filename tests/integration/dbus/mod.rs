//! The connection adapter (`src/shared/dbus/adapters/connection/`) against
//! the private bus, which stands in for both the session and the system bus.

mod probe;

use crate::cases::{Case, case};
use crate::support::{Context, Outcome, ensure, ensure_eq, within};
use cranky::shared::dbus::adapters::connection::{Connected, ZbusConnectionAdapter};
use cranky::shared::dbus::domain::{
    BusType, DBusSubscription, DBusValue, Destination, Interface, Member, NameChangedStream,
    NameOwnerChanged, Path, PropertyName,
};
use cranky::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};
use probe::ProbeService;
use std::collections::HashMap;
use tokio_stream::StreamExt;

pub fn cases() -> Vec<Case> {
    vec![
        case!(dbus::connects_on_both_bus_addresses),
        case!(dbus::lists_an_owned_name),
        case!(dbus::calls_a_method_returning_a_variant),
        case!(dbus::rejects_a_reply_that_is_not_a_variant),
        case!(dbus::reads_all_properties_with_their_types),
        case!(dbus::streams_property_changes),
        case!(dbus::streams_name_owner_changes),
        case!(dbus::streams_a_signal_with_a_variant_body),
    ]
}

async fn adapter() -> Outcome<ZbusConnectionAdapter<Connected>> {
    ZbusConnectionAdapter::new()
        .connect()
        .await
        .context("connect the adapter")
}

fn probe_destination() -> Destination {
    Destination::new(probe::NAME)
}

fn probe_path() -> Path {
    Path::new(probe::PATH)
}

fn probe_interface() -> Interface {
    Interface::new(probe::INTERFACE)
}

/// Both addresses lead to the same private daemon, so this proves that both
/// connections are made, not that they are told apart.
async fn connects_on_both_bus_addresses() -> Outcome {
    let adapter = adapter().await?;
    for bus in [BusType::Session, BusType::System] {
        let names = adapter
            .list_names(bus)
            .await
            .context("list names right after connecting")?;
        ensure(
            names
                .iter()
                .any(|bus_name| bus_name.as_str() == "org.freedesktop.DBus"),
            || format!("{bus:?} bus did not list the bus driver"),
        )?;
    }
    Ok(())
}

async fn lists_an_owned_name() -> Outcome {
    let adapter = adapter().await?;
    let _probe = ProbeService::start().await?;
    let names = adapter
        .list_names(BusType::Session)
        .await
        .context("list names")?;
    ensure(names.contains(&probe_destination()), || {
        format!("{} missing from {names:?}", probe::NAME)
    })
}

async fn call(
    adapter: &ZbusConnectionAdapter<Connected>,
    method: &str,
) -> Result<DBusValue, DbusConnectionError> {
    adapter
        .call_method(
            BusType::Session,
            &probe_destination(),
            &probe_path(),
            &probe_interface(),
            &Member::new(method),
        )
        .await
}

async fn calls_a_method_returning_a_variant() -> Outcome {
    let adapter = adapter().await?;
    let _probe = ProbeService::start().await?;
    let reply = call(&adapter, "Wrapped").await.context("call Wrapped")?;
    ensure_eq(
        "Wrapped reply",
        &reply,
        &DBusValue::String("wrapped reply".to_string()),
    )
}

/// Pins today's behaviour, which is a bug (#41): a reply that is not a single
/// variant is an error. The `zbus` bump must not change this unnoticed.
async fn rejects_a_reply_that_is_not_a_variant() -> Outcome {
    let adapter = adapter().await?;
    let _probe = ProbeService::start().await?;
    // The method exists and answers: the failure below is about the reply.
    let direct: String = zbus::Connection::session()
        .await
        .context("direct connection")?
        .call_method(
            Some(probe::NAME),
            probe::PATH,
            Some(probe::INTERFACE),
            "Bare",
            &(),
        )
        .await
        .context("call Bare directly")?
        .body()
        .deserialize()
        .context("read the Bare reply")?;
    ensure_eq("direct Bare reply", &direct.as_str(), &"bare reply")?;

    let reply = call(&adapter, "Bare").await;
    ensure(
        matches!(reply, Err(DbusConnectionError::MethodCallFailed(_))),
        || format!("expected MethodCallFailed for a bare string reply, got {reply:?}"),
    )
}

async fn reads_all_properties_with_their_types() -> Outcome {
    let adapter = adapter().await?;
    let _probe = ProbeService::start().await?;
    let properties = adapter
        .get_all_properties(
            BusType::Session,
            &probe_destination(),
            &probe_path(),
            &probe_interface(),
        )
        .await
        .context("get all properties")?;
    let expected = [
        ("Label", DBusValue::String("first".to_string())),
        ("Count", DBusValue::Int(7)),
        ("Enabled", DBusValue::Bool(true)),
        (
            "Tags",
            DBusValue::Array(vec![
                DBusValue::String("one".to_string()),
                DBusValue::String("two".to_string()),
            ]),
        ),
        (
            "Limits",
            DBusValue::Dict(HashMap::from([("max".to_string(), DBusValue::Int(9))])),
        ),
    ];
    for (name, value) in expected {
        let actual = properties
            .get(&PropertyName::new(name))
            .context(&format!("the probe has no {name} property"))?;
        ensure_eq(name, actual, &value)?;
    }
    Ok(())
}

async fn streams_property_changes() -> Outcome {
    let adapter = adapter().await?;
    let probe = ProbeService::start().await?;
    let mut updates = adapter
        .subscribe_properties_changed(BusType::Session, &probe_destination(), &probe_path())
        .await
        .context("subscribe to property changes")?;
    probe.relabel("second").await?;
    let (interface, changed) = within("a PropertiesChanged signal", updates.next())
        .await?
        .context("the property stream ended")?;
    ensure_eq("changed interface", &interface, &probe_interface())?;
    let label = changed
        .get(&PropertyName::new("Label"))
        .context("the change does not carry Label")?;
    ensure_eq("Label", label, &DBusValue::String("second".to_string()))
}

/// The next change of the probe's name; other names come and go around it.
async fn next_probe_change(
    what: &str,
    changes: &mut NameChangedStream,
) -> Outcome<NameOwnerChanged> {
    within(what, async {
        loop {
            match changes.next().await {
                Some(change) if change.name() == &probe_destination() => break Some(change),
                Some(_) => {}
                None => break None,
            }
        }
    })
    .await?
    .context("the name stream ended")
}

async fn streams_name_owner_changes() -> Outcome {
    let adapter = adapter().await?;
    let mut changes = adapter
        .subscribe_name_changes(BusType::Session)
        .await
        .context("subscribe to name changes")?;
    let probe = ProbeService::start().await?;
    let appeared = next_probe_change("the probe name to appear", &mut changes).await?;
    ensure(appeared.is_new(), || {
        format!("expected the name to be new, got {appeared:?}")
    })?;

    drop(probe);
    let vanished = next_probe_change("the probe name to vanish", &mut changes).await?;
    ensure(vanished.is_gone(), || {
        format!("expected the name to be gone, got {vanished:?}")
    })
}

/// A signal is only delivered when its body is a single variant (#41), so
/// that is what the probe emits.
async fn streams_a_signal_with_a_variant_body() -> Outcome {
    let adapter = adapter().await?;
    let probe = ProbeService::start().await?;
    let mut signals = adapter
        .subscribe_signal(DBusSubscription::new(
            BusType::Session,
            Some(probe_destination()),
            Some(probe_path()),
            Some(probe_interface()),
            Some(Member::new("Pinged")),
        ))
        .await
        .context("subscribe to Pinged")?;
    probe.ping("hello").await?;
    let (path, member, body) = within("the Pinged signal", signals.next())
        .await?
        .context("the signal stream ended")?;
    ensure_eq("signal path", &path, &probe_path())?;
    ensure_eq("signal member", &member, &Member::new("Pinged"))?;
    ensure_eq(
        "signal body",
        &body,
        &DBusValue::String("hello".to_string()),
    )
}
