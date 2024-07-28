//! This module contains the errors used all over this codebase.

use std::{io, path::PathBuf};

use crate::prelude::*;

use super::config::RawTargetGame;

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

	#[error(transparent)]
	Game(GameError),

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

	/// This modpack's config never sets the 'target' field.
	#[error("Your config does not specify a target game.")]
	MissingTarget,

	/// A local mod is not installed.
	/// By local, this means a mod that does not have a Nexus ID specified.
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

/// An error related to issues involving a *GameSpec* or *TargetGame*.
#[derive(Error, Debug)]
pub enum GameError {
	/// This modpack's target refers to a game specification that doesn't exist.
	#[error("This modpack's target refers to the specification for {}, but that doesn't exist.", .0.spec_key)]
	MissingSpec(RawTargetGame),

	/// This modpack does not specify a Proton binary to use, but the target game or one of the tools is for Windows.
	#[error("Your config does not specify a path to Proton, but the game or a tool requires it.")]
	MissingProton,

	/// The specified path for Proton points to a non-existent file.
	#[error("The provided Proton path does not exist.")]
	InvalidProton,
	
	/// This modpack's target does not specify a path that doesn't support automatic detection.
	/// This is caused by the target's selected specification not listing any default paths to search for.
	/// This error wraps a label referring to what kind of path was being automatically determined.
	#[error("Automatically determining this game's {0} path is unsupported.")]
	ScanUnavailable(String),
	
	/// Indicates Modcrab could not automatically determine one of the game's paths.
	/// This error wraps a label referring to what kind of path was being automatically determined.
	#[error("Failed to automatically determine the game's {0} path.")]
	ScanFailed(String),

	/// A modpack's target explicitly sets one of the game's paths, but the path they provided doesn't exist.
	#[error("This modpack's target sets the game's {label} path to '{path}', but that path does not exist.")]
	InvalidPath {
		/// The kind of path this is.
		label: String,

		/// The path that does not exist.
		path: PathBuf,
	}
}
