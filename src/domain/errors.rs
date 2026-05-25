use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Failed to parse configuration: {reason}")]
    ConfigParseError { reason: String },
    
    #[error("Module '{module_name}' not found in registry")]
    ModuleNotFound { module_name: String },
    
    #[error("Internal domain error: {message}")]
    Internal { message: String },

    #[error("Script error in module '{module_name}': {message}")]
    ScriptError { module_name: String, message: String },

    #[error("Color error: {0}")]
    Color(#[from] crate::domain::color::ColorError),
}

#[derive(Error, Debug)]
pub enum PortError {
    #[error("Display server connection failed: {reason}")]
    DisplayConnectionFailed { reason: String },
    
    #[error("Surface error for target {target_id}: {reason}")]
    SurfaceError { target_id: u32, reason: String },
    
    #[error("Window manager IPC error: {reason}")]
    WindowManagerIpcError { reason: String },
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("DBus error: {reason}")]
    DBusError { reason: String },

    #[error("Port initialization failed: {0}")]
    InitFailed(String),

    #[error("Internal port error: {0}")]
    Internal(String),
}
