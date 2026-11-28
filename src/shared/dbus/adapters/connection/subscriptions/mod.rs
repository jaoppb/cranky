use std::pin::Pin;

use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use zbus::Connection;

use crate::shared::dbus::domain::{
    BusObject, BusType, DBusSubscription, NameChangedStream, PropertyChangedStream, SignalStream,
};
use crate::shared::dbus::ports::DbusConnectionError;

use super::adapter::{Connected, ZbusConnectionAdapter};

mod decode;
mod rules;

impl ZbusConnectionAdapter<Connected> {
    pub(crate) async fn subscribe_properties_changed_impl(
        &self,
        object: &BusObject,
    ) -> Result<PropertyChangedStream, DbusConnectionError> {
        let conn = self.get_conn(object.bus())?.clone();
        let rule = rules::properties_changed(object)?;
        let stream = open_message_stream(rule, &conn).await?;
        Ok(forward_messages(stream, decode::properties_changed))
    }

    pub(crate) async fn subscribe_name_changes_impl(
        &self,
        bus: BusType,
    ) -> Result<NameChangedStream, DbusConnectionError> {
        let conn = self.get_conn(bus)?.clone();
        let rule = rules::name_owner_changed()?;
        let stream = open_message_stream(rule, &conn).await?;
        Ok(forward_messages(stream, decode::name_owner_changed))
    }

    pub(crate) async fn subscribe_signal_impl(
        &self,
        sub: DBusSubscription,
    ) -> Result<SignalStream, DbusConnectionError> {
        let conn = self.get_conn(sub.bus())?.clone();
        let rule = rules::signal(&sub)?;
        let stream = open_message_stream(rule, &conn).await?;
        Ok(forward_messages(stream, decode::signal))
    }
}

/// Opens the stream of the messages `conn` receives that match `rule`.
async fn open_message_stream(
    rule: zbus::MatchRule<'_>,
    conn: &Connection,
) -> Result<zbus::MessageStream, DbusConnectionError> {
    zbus::MessageStream::for_match_rule(rule, conn, None)
        .await
        .map_err(|error| DbusConnectionError::SubscriptionFailed(error.to_string()))
}

/// Forwards every message of `stream` that `decode` understands to the returned stream, from a
/// spawned task that skips unreadable messages and stops once the returned stream is dropped.
fn forward_messages<T, D>(
    stream: zbus::MessageStream,
    decode: D,
) -> Pin<Box<dyn Stream<Item = T> + Send + Sync>>
where
    T: Send + 'static,
    D: Fn(&zbus::Message) -> Option<T> + Send + 'static,
{
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(send_decoded(stream, decode, tx));
    Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
}

/// Sends `tx` every message of `stream` that `decode` understands, skipping unreadable ones,
/// until `stream` ends or the receiver of `tx` is dropped.
async fn send_decoded<T, D>(
    mut stream: zbus::MessageStream,
    decode: D,
    tx: mpsc::UnboundedSender<T>,
) where
    D: Fn(&zbus::Message) -> Option<T>,
{
    while let Some(msg_result) = stream.next().await {
        if let Ok(msg) = msg_result
            && let Some(item) = decode(&msg)
            && tx.send(item).is_err()
        {
            break;
        }
    }
}
