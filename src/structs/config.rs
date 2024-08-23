//! This module contains the structs usually accessed by a modpack's Lua config.

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{prelude::*, util::misc::replace_path_home_prefix};

use super::spec::generate_default_game_specs;

/// The game this modpack is targeting.
/// Exposed to Lua as 'modcrab.target'.
#[derive(Clone, Serialize, Deserialize)]
pub struct TargetGame {
    /// The associated specification for this game.
    pub spec: GameSpec,

    /// This game's root path (the one that holds its binary).
    /// If this wasn't explicitly specified, it was found automatically by Modcrab.
    pub root_path: PathBuf,

    ///  This game's path for data, which is where it keeps saves and the load order.
    /// Like the other paths, Modcrab will attempt to automatically find this if not specified.
    pub data_path: Option<PathBuf>, // TODO: Update this.
}

/// A raw version of *TargetGame* designed to be generated from Lua.
/// See *TargetGame*'s docs for information on most fields.
#[derive(Default, Debug, Clone)]
pub struct RawTargetGame {
	/// A key used for retrieving this game's spec.
	pub spec_key: String,

	pub root_path: Option<String>,
	pub data_path: Option<String>,
}

impl RawTargetGame {
	/// Uses this struct and an *AppData* reference to build a real *TargetGame*.
	pub fn to_real(self, data: &AppData) -> AppResult<TargetGame> {
		let Some(spec) = data.config.games.get(&self.spec_key.to_lowercase()) else {
			return Err(AppError::Game(GameError::MissingSpec(self)));
		};

		let root_path = match self.root_path {
			Some(ref path) => replace_path_home_prefix(path)?,
			None => spec.scan_for_root()?,
		};

		let real = TargetGame {
			spec: spec.clone(),
			root_path,
			data_path: self.data_path.map(|s| PathBuf::from(s)), // TODO: Update this.
		};

		Ok(real)
	}
}

impl<'lua> FromLua<'lua> for RawTargetGame {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
		if value.is_string() {
			return Ok(Self {
				spec_key: String::from_lua(value, lua)?,
				..Default::default()
			})
		}

		let Some(table) = value.as_table() else {
			return Err(LuaError::FromLuaConversionError {
                from: "table",
                to: "TargetGame",
                message: Some("The target game can only be a string or table.".to_owned()),
            });
		};

		let Some(spec_key) = table.get::<_, Option<String>>(1)? else {
			return Err(LuaError::FromLuaConversionError {
				from: "table",
				to: "ModSpec",
				message: Some("The first item in a mod's specification should be a string containing its name.".to_owned()),
			});
		};

		let target = Self {
			spec_key,
			root_path: table.get("root")?,
			data_path: table.get("data")?,
		};

		Ok(target)
    }
}

/// Configuration set by the end user.
/// This struct is exposed to Lua via the 'modcrab' table.
#[derive(Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// A list of games supported by Modcrab.
    pub games: HashMap<String, GameSpec>,

	/// Temporary value to hold the raw version of the target game.
	#[serde(skip, default)]
	pub raw_target: Option<RawTargetGame>,
	
    /// The game this modpack is for.
    pub target: Option<TargetGame>,
}


impl Default for AppConfig {
	fn default() -> Self {
		Self {
			games: generate_default_game_specs(),
			raw_target: None,
			target: None,
		}
	}
}

impl LuaUserData for AppConfig {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
		// TODO: Expose 'games' field to Lua.

		fields.add_field_method_set("target", |_, this, value| {
			this.raw_target = value;
			Ok(())
		});
	}
}

impl<'lua> FromLua<'lua> for AppConfig {
	fn from_lua(value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
		match value {
			LuaValue::UserData(data) => Ok(data.borrow::<Self>()?.clone()),
			_ => unreachable!(),
		}
	}
}
