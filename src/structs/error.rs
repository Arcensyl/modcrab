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

	/// Error returned by failing modpack-related operations.
	#[error(transparent)]
	Modpack(ModpackError),

	/// Custom error that simply wraps a *Notice*.
	#[error("{0}")]
	Custom(Notice),

    /// Error converted from any error that does not have a matching *AppError* variant.
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

/// An error returned by failed operation involving a modpack.
/// This typically occurs during the modpack build process.
#[derive(Error, Debug)]
pub enum ModpackError {
	/// Indicates the current directory is not a modpack.
	#[error("The current directory is missing a '.modcrab' folder.")]
	InvalidModpack,

	/// This modpack has no Lua configuration files.
	#[error("Your modpack has zero configuration files.")]
	EmptyConfig,

	/// This modpack's config never sets the 'target' field.
	#[error("Your config does not specify a target game.")]
	MissingTarget,

	/// A local mod is not installed.
	/// By local, this means a mod that does not have a Nexus slug specified.
	#[error("The local mod {} is not installed.", .0.name)]
	LocalModNotFound(ModSpec),

	/// A mod requres another mod that is not present in this modpack.
	#[error("The mod {} depends on {dep}, but {dep} is never declared.", .cause.name)]
	MissingDependency {
		/// The mod asking for the missing dependency.
		cause: ModSpec,

		/// The missing dependency.
		dep: String,
	},

	/// One or more mods cannot be sorted, which is likely due to the mod having a non-sensical specification.
	/// As missing dependencies are already handled, this usually means there is a cyclic dependency somewhere.
	#[error("These mods cannot be sorted: {0:?}")]
	UnsortableMods(Vec<ModSpec>),
}
