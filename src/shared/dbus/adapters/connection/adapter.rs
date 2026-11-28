use std::marker::PhantomData;

use tracing::{debug, error};
use zbus::Connection;

use crate::shared::dbus::domain::BusType;
use crate::shared::dbus::ports::DbusConnectionError;

/// Typestate marker: the adapter has attempted both bus connections and can serve requests.
#[derive(Debug)]
pub struct Connected;
/// Typestate marker: the adapter holds no connection yet; only `connect` is available.
#[derive(Debug)]
pub struct Disconnected;

/// zbus-backed implementation of `DbusConnectionPort`, holding one optional connection per bus.
///
/// A bus that fails to connect stays `None`, and requests on it fail with `NotInitialized`.
#[derive(Debug)]
pub struct ZbusConnectionAdapter<State = Disconnected> {
    pub(crate) session_conn: Option<Connection>,
    pub(crate) system_conn: Option<Connection>,
    pub(crate) _state: PhantomData<State>,
}

impl Default for ZbusConnectionAdapter<Disconnected> {
    fn default() -> Self {
        Self::new()
    }
}

impl ZbusConnectionAdapter<Disconnected> {
    /// Builds an adapter with no open connection; call `connect` to reach the buses.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            session_conn: None,
            system_conn: None,
            _state: PhantomData,
        }
    }

    /// Connects to `DBus` session and system buses.
    ///
    /// # Errors
    ///
    /// Never returns an error today: a bus that cannot be reached is logged and left unset, and
    /// calls on it fail later with `DbusConnectionError::NotInitialized`.
    pub async fn connect(self) -> Result<ZbusConnectionAdapter<Connected>, DbusConnectionError> {
        let session_conn = open_bus(BusType::Session).await.or(self.session_conn);
        let system_conn = open_bus(BusType::System).await.or(self.system_conn);

        Ok(ZbusConnectionAdapter {
            session_conn,
            system_conn,
            _state: PhantomData,
        })
    }
}

/// Connects to `bus`, logging the attempt, and logging and returning `None` when it fails.
async fn open_bus(bus: BusType) -> Option<Connection> {
    let name = match bus {
        BusType::Session => "Session",
        BusType::System => "System",
    };
    log_connecting(name);
    let connection = match bus {
        BusType::Session => Connection::session().await,
        BusType::System => Connection::system().await,
    };
    connection
        .inspect_err(|error| error!("Failed to connect to {name} Bus: {error}"))
        .ok()
}

/// Logs the attempt to connect to the bus called `name`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_connecting(name: &str) {
    debug!("Connecting to DBus {name} Bus...");
}

impl ZbusConnectionAdapter<Connected> {
    pub(crate) fn get_conn(&self, bus: BusType) -> Result<&Connection, DbusConnectionError> {
        match bus {
            BusType::Session => self.session_conn.as_ref(),
            BusType::System => self.system_conn.as_ref(),
        }
        .ok_or(DbusConnectionError::NotInitialized(bus))
    }
}
