//! This module defines the specifications for games, mods, and more.
//! A specification provides instructions for what Modcrab should do.

use std::{collections::HashMap, path::PathBuf};

use esplugin::GameId;
use serde::{Deserialize, Serialize};
use tap::Pipe;

/// Remote to allow Serde to (de)serialize Esplugin's *GameId*.
#[derive(Serialize, Deserialize)]
#[serde(remote = "GameId")]
enum GameIdRef {
    Oblivion,
    Skyrim,
    Fallout3,
    FalloutNV,
    Morrowind,
    Fallout4,
    SkyrimSE,
    Starfield,
}

/// Describes a game that Modcrab is capable of handling.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameSpec {
    /// The name of the game.
    pub name: String,

    /// Tells Modcrab how this game handles plugins.
    /// If this is not set, Modcrab will not use any plugin-specific features.
    pub plugin_config: Option<GamePluginSupportSpec>,

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

/// Used to define how a *GameSpec* handles plugins.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GamePluginSupportSpec {
    /// The format tells Modcrab how to inspect plugins.
    /// Without this, Modcrab won't be able to get detailed information on this game's plugins.
    /// This detailed info includes things like detecting if a plugin is ESL-flagged.
	#[serde(with = "GameIdRef")]
    pub format: GameId,

    /// This is how many plugins this game can run at once.
    /// If this is *None*, Modcrab assumes there is no limit.
    pub limit: Option<u32>,

    /// Same as *GamePluginSupportSpec::limit*, but for light plugins.
    /// If the plugin format does not support light plugins, this field does nothing.
    pub light_limit: Option<u32>,
}

/// Generates the specs for games that Modcrab offers OOTB support for.
pub fn generate_default_game_specs() -> HashMap<String, GameSpec> {
	let sse = GameSpec {
		name: "Skyrim Special Edition".to_owned(),
		
		plugin_config: GamePluginSupportSpec {
			format: GameId::SkyrimSE,
			limit: Some(255),
			light_limit: Some(4096),
		}.pipe(|s| Some(s)),
		
		common_root_paths: vec![
			"~/.steam/steam/steamapps/common/Skyrim Special Edition".into(),
		],
		
		mod_directory: "data".to_owned(),
		common_data_paths: Vec::new(),
	};

	let fo4 = GameSpec {
		name: "Fallout 4".to_owned(),
		
		plugin_config: GamePluginSupportSpec {
			format: GameId::Fallout4,
			limit: Some(255),
			light_limit: Some(4096),
		}.pipe(|s| Some(s)),
		
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
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModSpec {
    /// The name of this mod.
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
}
