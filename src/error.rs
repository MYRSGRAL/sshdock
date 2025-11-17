use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Config(String),
    #[error("command failed: {0}")]
    Command(String),
    #[error(transparent)]
    Dbus(#[from] zbus::Error),
    #[error(transparent)]
    Signal(#[from] io::Error),
}
