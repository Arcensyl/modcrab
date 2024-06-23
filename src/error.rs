use std::io;

use crate::prelude::*;

/// Convenience wrapper around *Result<T, AppError>*.
pub type AppResult<T> = Result<T, AppError>;

/// Error returned by several functions in Modcrab.
#[derive(Error, Debug)]
pub enum AppError {
    /// Error returned by failing IO operations.
    /// Most of these will occur during filesystem interactions.
    #[error(transparent)]
    IO(#[from] io::Error),

    /// Error returned while running Lua code.
    /// This is mostly seen while executing a modpack's Lua config.
    #[error(transparent)]
    Lua(#[from] LuaError),

    /// Error returned when failing to (de)serialize type using Serde and Bincode.
    #[error(transparent)]
    Bincode(#[from] bincode::Error),

    /// Error converted from any error that does not have a matching *AppError* variant.
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}
