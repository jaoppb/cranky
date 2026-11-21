use crate::app::builtins::BuiltinError;
use crate::app::state::AppError;
use crate::shared::config::adapters::ConfigAdapterError;
use crate::shared::dbus::ports::DbusConnectionError;
use crate::shared::env::ports::EnvironmentError;
use crate::shared::wayland::ports::DisplayServerError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BootError {
    #[error(transparent)]
    Runtime(#[from] std::io::Error),
    #[error(transparent)]
    Environment(#[from] EnvironmentError),
    #[error(transparent)]
    Config(#[from] ConfigAdapterError),
    #[error(transparent)]
    DisplayServer(#[from] DisplayServerError),
    #[error("DBus connection required: {0}")]
    DbusConnection(#[from] DbusConnectionError),
    #[error(transparent)]
    Builtins(#[from] BuiltinError),
    #[error(transparent)]
    App(#[from] AppError),
}
