//! This module defines the specifications for games, mods, and more.
//! A specification provides instructions for what Modcrab should do.

use std::path::PathBuf;

#[derive(PartialEq, Eq, Hash)]
/// Describes a game that Modcrab is capable of handling.
pub struct GameSpec {
	/// The name of the game.
	name: String,

	/// Tells Modcrab how this game handles plugins.
	/// If this is not set, Modcrab will not use any plugin-specific features.
	plugin_config: Option<GamePluginSupportSpec>,
	
	/// Paths were the game's root can likely be found.
	/// Modcrab will scan for these paths in case the user's config does not point it to one.
	/// These paths should preferably be absolute as opposed to relative.
	common_root_paths: Vec<PathBuf>,

	/// Like *GameSpec::common_root_paths*, but for the game's mod installation directory.
	/// As an example, for the case of Skyrim, this could contain an absolute path to Skyrim's 'data' directory.
	common_mod_paths: Vec<PathBuf>,

	/// Like *GameSpec::common_root_paths*, but for a game's data directory.
	/// Not to be confused with Bethesda games' literal 'data' directory, but instead refers to were they keep saves and the load order.
	common_data_paths: Vec<PathBuf>,
}

#[derive(PartialEq, Eq, Hash)]
/// Used to define how a *GameSpec* handles plugins.
pub struct GamePluginSupportSpec {
	/// The format tells Modcrab how to inspect plugins.
	/// Without this, Modcrab won't be able to get detailed information on this game's plugins.
	/// This detailed info includes things like detecting if a plugin is ESL-flagged.
	format: Option<String>,

	/// This is how many plugins this game can run at once.
	/// If this is *None*, Modcrab assumes there is no limit.
	limit: Option<u32>,

	/// Same as *GamePluginSupportSpec::limit*, but for light plugins.
	/// If the plugin format does not support light plugins, this field does nothing.
	light_limit: Option<u32>,
}
