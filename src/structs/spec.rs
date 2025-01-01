//! This module defines the specifications for games, mods, and more.
//! A specification provides instructions for what Modcrab should do.

use std::{collections::HashMap, fmt::Display, path::PathBuf};

use crate::{lua::convert_table_item_to_vec, prelude::*, util::misc::replace_path_home_prefix};
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

impl GameSpec {
	/// Scan for this game's root path using a list of common locations. 
	pub fn scan_for_root(&self) -> AppResult<PathBuf> {
		if self.common_root_paths.is_empty() {
			return Err(AppError::Game(GameError::ScanUnavailable("root".to_string())))
		}
		
		let mut real;
		for path in self.common_root_paths.iter() {
			real = replace_path_home_prefix(path)?;

			if real.exists() {
				return Ok(real);
			}
		}

		Err(AppError::Game(GameError::ScanFailed("root".to_owned()))) 
	}
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

	let ut99 = GameSpec {
		name: "Unreal Tournament(1999)".to_owned(),

		common_root_paths: Vec::new(),

		mod_directory: "".to_owned(),
		common_data_paths: Vec::new(),
	};

	let mut games = HashMap::with_capacity(3);
	games.insert(sse.name.to_lowercase(), sse);
	games.insert(fo4.name.to_lowercase(), fo4);
	games.insert(ut99.name.to_lowercase(), ut99);

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
    pub is_enabled: bool,

	/// Determines if this mod should be overlayed over the game's root directory.
	pub is_root: bool,

	/// This mod's ID on NexusMods.
	/// This used to automatically install this mod when it is missing.
	pub id: Option<String>,
	
    /// A list of the names of mods this one depends on.
    /// This mod will always be loaded before this one.
    pub dependencies: Vec<String>,

    /// like the *ModSpec::dependencies* field, this is a list of mods that are loaded before this one.
    /// This is kept seperate to aid in modpack organization.
    pub after: Vec<String>,

    /// A numerical priority to hint where a mod should be sorted.
    /// A lower priority is placed earlier in the mod load order.
    /// If this mod has any that it loads after, a lower priority will place it closer to the latest preceding mod.
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
			is_enabled: true,
			is_root: false,
			id: None,
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

impl<'lua> FromLua<'lua> for ModSpec {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if value.is_string() {
			return Ok(Self {
				name: String::from_lua(value, lua)?,
				..Default::default()
			})
		}

		let Some(table) = value.as_table() else {
			return Err(LuaError::FromLuaConversionError {
                from: "table",
                to: "ModSpec",
                message: Some("A mod's specification can only be a string or table.".to_owned()),
            });
		};

		let ModSpec {
			name: _,
			is_enabled: def_is_enabled,
			is_root: def_is_root,
			id: _,
			dependencies: _,
			after: _,
			priority: def_priority,
			should_check: def_should_check
		} = ModSpec::default();

		let Some(name) = table.get::<_, Option<String>>(1)? else {
			return Err(LuaError::FromLuaConversionError {
				from: "table",
				to: "ModSpec",
				message: Some("The first item in a mod's specification should be a string containing its name.".to_owned()),
			});
		};

		let is_enabled = match table.get::<_, Option<bool>>("enabled")? {
			Some(bool) => bool,
			None => def_is_enabled,
		};

		let is_root = match table.get::<_, Option<bool>>("root")? {
			Some(bool) => bool,
			None => def_is_root,
		};

		let id = table.get::<_, Option<String>>("id")?;

		let dependencies = convert_table_item_to_vec(&table, "deps")?;

		let after = convert_table_item_to_vec(&table, "after")?;
		
		let priority = match table.get::<_, Option<u32>>("priority")? {
			Some(priority) => priority,
			None => def_priority,
		};

		let should_check = match table.get::<_, Option<bool>>("check")? {
			Some(bool) => bool,
			None => def_should_check,
		};

		let spec = Self {
			name,
			is_enabled,
			is_root,
			id,
			dependencies,
			after,
			priority,
			should_check,
		};

		Ok(spec)
    }
}
