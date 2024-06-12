//! This module defines data used globally throughout Modcrab.

use std::{collections::HashMap, path::PathBuf, sync::{Mutex, OnceLock}};

use crate::spec::GameSpec;

/// Contains data used throughout Modcrab's runtime.
/// This struct is mostly used through the 'APP_DATA' static.
pub struct AppData {
	config: AppConfig
}

/// This static is a globally-accessible instance of the *AppData* struct.
/// Because *AppData* needs values set by the end user, this is not available until **init.lua** is evaluated.
pub static APP_DATA: OnceLock<Mutex<AppData>> = OnceLock::new();

/// Configuration set by the end user.
/// This is mostly configured by **init.lua**.
pub struct AppConfig {
	/// A list of games supported by Modcrab.
	games: HashMap<String, GameSpec>,

	/// The game this modpack is for.
	target: TargetGame,
}

/// The game this modpack is targeting.
pub struct TargetGame {
	/// The associated specification for this game.
	spec: GameSpec,

	/// This game's root path (the one that holds its binary).
	/// If this is *None*, Modcrab will attempt to find this path using the specification.
	root_path: Option<PathBuf>,

	/// This game's path for holding mods. For example, this would be Skyrim's 'data' directory.
	/// Like the other paths, Modcrab will attempt to automatically find this if not specified.
	mod_path: Option<PathBuf>,

	///  This game's path for data, which is where it keeps saves and the load order.
	/// Like the other paths, Modcrab will attempt to automatically find this if not specified.
	data_path: Option<PathBuf>,
}
