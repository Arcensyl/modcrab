use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::prelude::*;

use super::spec::generate_default_game_specs;

/// The game this modpack is targeting.
#[derive(Serialize, Deserialize)]
pub struct TargetGame {
    /// The associated specification for this game.
    pub spec: GameSpec,

    /// This game's root path (the one that holds its binary).
    /// If this is *None*, Modcrab will attempt to find this path using the specification.
    pub root_path: Option<PathBuf>,

    ///  This game's path for data, which is where it keeps saves and the load order.
    /// Like the other paths, Modcrab will attempt to automatically find this if not specified.
    pub data_path: Option<PathBuf>,
}

/// Configuration set by the end user.
/// This is mostly configured by **init.lua**.
#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    /// A list of games supported by Modcrab.
    pub games: HashMap<String, GameSpec>,

    /// The game this modpack is for.
    pub target: Option<TargetGame>,

	/// The path to the version of Proton to use.
	pub proton: Option<PathBuf>,
}


impl Default for AppConfig {
	fn default() -> Self {
		Self {
			games: generate_default_game_specs(),
			target: None,
			proton: None,
		}
	}
}
