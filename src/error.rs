use std::io;

use crate::prelude::*;

/// Error returned by several functions in Modcrab.
#[derive(Error, Debug)]
pub enum AppError {
	/// Error returned by failing IO operations.
	/// Most of these will occur during filesystem interactions.
	#[error(transparent)]
	IOError(#[from] io::Error),

	/// Error returned while running Lua code.
	/// This is mostly seen while executing a modpack's Lua config.
	#[error(transparent)]
	LuaError(#[from] LuaError),

	/// Error returned when failing to (de)serialize type using Serde and Bincode.
	#[error(transparent)]
	BinError(#[from] bincode::Error),

	/// Error returned when attempting to access a poisoned *Mutex*.
	/// This isn't converted from the actual *PoisonError* due to that error's inclusion of a *MutexGuard*.
	#[error("attempted to access a poisoned mutex")]
	MutexError,

	/// Error converted from any error that does not have a matching *AppError* variant.
	#[error(transparent)]
	Unknown(#[from] anyhow::Error),
}
