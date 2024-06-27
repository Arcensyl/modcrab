//! This module defines the specifications for games, mods, and more.
//! A specification provides instructions for what Modcrab should do.

use std::{collections::HashMap, fmt::Display, path::PathBuf};

use crate::prelude::*;
use serde::{Deserialize, Serialize};

/// Describes a game that Modcrab is capable of handling.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameSpec {
    /// The name of the game.
    pub name: String,

    /// Paths were the game's root can likely be found.
    /// Modcrab will scan for these paths in case the user's config does not point it to one.
    /// These paths should preferably be absolute as opposed to relative.
    pub common_root_paths: Vec<PathBuf>,

	/// The name of this game's mod directory.
	/// This is a directory directly under the game's root.
    pub mod_directory: String,

    /// Like *GameSpec::common_root_paths*, but for a game's data directory.
    /// Not to be confused with Bethesda games' literal 'data' directory, but instead refers to were they keep saves and the load order.
    pub common_data_paths: Vec<PathBuf>,
}

/// Generates the specs for games that Modcrab offers OOTB support for.
pub fn generate_default_game_specs() -> HashMap<String, GameSpec> {
	let sse = GameSpec {
		name: "Skyrim Special Edition".to_owned(),
		
		common_root_paths: vec![
			"~/.steam/steam/steamapps/common/Skyrim Special Edition".into(),
		],
		
		mod_directory: "data".to_owned(),
		common_data_paths: Vec::new(),
	};

	let fo4 = GameSpec {
		name: "Fallout 4".to_owned(),
		
		common_root_paths: vec![
			"~/.steam/steam/steamapps/common/Fallout 4".into(),
		],
		
		mod_directory: "data".to_owned(),
		common_data_paths: Vec::new(),
	};

	let mut games = HashMap::with_capacity(2);
	games.insert(sse.name.to_lowercase(), sse);
	games.insert(fo4.name.to_lowercase(), fo4);

	games
} 

// TODO Add ModSpec fields for NexusMods-related settings and plugin management.

/// Describes how Modcrab should manage and handle a specific mod.
/// This is mostly instructions on how a mod should be sorted and acquired when not found.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModSpec {
    /// The name of this mod.
	/// In Lua, this is the first item of the table.
    pub name: String,

    /// Determines if this mod should be loaded.
    pub enabled: bool,

    /// A list of the names of mods this one depends on.
    /// This mod will always be loaded before this one.
    pub dependencies: Vec<String>,

    /// like the *ModSpec::dependencies* field, this is a list of mods that are loaded before this one.
    /// This is kept seperate to aid in modpack organization.
    pub after: Vec<String>,

    /// A numerical priority to hint where a mod should be sorted.
    /// A lower priority is placed earlier in the mod load order.
    /// If this mod has any that it loads after, a lower priority will place it closer to the latest preceeding mod.
    pub priority: u32,

	/// Determines if Modcrab will check this mod's structure for validity.
	/// For example, a Skyrim mod would have an invalid structure if it had a 'data' folder in its root.
	/// This field is exposed to Lua as 'check'.
	pub should_check: bool,
}

impl Default for ModSpec {
    fn default() -> Self {
        Self {
			name: "DEFAULT".to_owned(), // This default field shouldn't be used.
			enabled: true,
			dependencies: Vec::new(),
			after: Vec::new(),
			priority: 50,
			should_check: true,
		}
    }
}

impl Display for ModSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
