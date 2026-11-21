use crate::app::builtins::BuiltinError;
use crate::app::state::AppError;
use crate::shared::config::adapters::ConfigAdapterError;
use crate::shared::dbus::ports::DbusConnectionError;
use crate::shared::env::ports::EnvironmentError;
use crate::shared::wayland::ports::DisplayServerError;
use thiserror::Error;

/// A boot-fatal failure; `main` returns it, so its `Debug` text is what a failed start prints.
#[derive(Error, Debug)]
pub enum BootError {
    /// The Tokio runtime could not be built.
    #[error(transparent)]
    Runtime(std::io::Error),
    /// A required environment variable is missing.
    #[error(transparent)]
    Environment(#[from] EnvironmentError),
    /// The configuration could not be loaded or watched.
    #[error(transparent)]
    Config(#[from] ConfigAdapterError),
    /// The Wayland connection could not be set up.
    #[error(transparent)]
    DisplayServer(#[from] DisplayServerError),
    /// The `DBus` connection failed; not produced today, because `ZbusConnectionAdapter::connect`
    /// logs an unreachable bus and carries on.
    #[error("DBus connection required: {0}")]
    DbusConnection(DbusConnectionError),
    /// The module script directories could not be watched.
    #[error(transparent)]
    Builtins(#[from] BuiltinError),
    /// The application failed to load its modules or its loop stopped on an error.
    #[error(transparent)]
    App(#[from] AppError),
}
