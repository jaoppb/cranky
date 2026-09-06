use std::marker::PhantomData;

use tracing::{debug, error};
use zbus::Connection;

use crate::shared::dbus::domain::BusType;
use crate::shared::dbus::ports::DbusConnectionError;

pub struct Connected;
pub struct Disconnected;

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
    /// Returns `DbusConnectionError` if connection to buses fails.
    pub async fn connect(
        mut self,
    ) -> Result<ZbusConnectionAdapter<Connected>, DbusConnectionError> {
        debug!("Connecting to DBus Session Bus...");
        match Connection::session().await {
            Ok(conn) => self.session_conn = Some(conn),
            Err(e) => error!("Failed to connect to Session Bus: {e}"),
        }

        debug!("Connecting to DBus System Bus...");
        match Connection::system().await {
            Ok(conn) => self.system_conn = Some(conn),
            Err(e) => error!("Failed to connect to System Bus: {e}"),
        }

        Ok(ZbusConnectionAdapter {
            session_conn: self.session_conn,
            system_conn: self.system_conn,
            _state: PhantomData,
        })
    }
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
