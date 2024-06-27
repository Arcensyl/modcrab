use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::prelude::*;

use super::spec::generate_default_game_specs;

/// The game this modpack is targeting.
/// Exposed to Lua as 'modcrab.target'.
#[derive(Serialize, Deserialize)]
pub struct TargetGame {
    /// The associated specification for this game.
    pub spec: GameSpec,

	/// The command to run the game.
	/// If this game is Windows-native, this command will ran under Proton.
	/// This will fallback to game's default command if not set.
	pub command: Option<String>,

    /// This game's root path (the one that holds its binary).
    /// If this is *None*, Modcrab will attempt to find this path using the specification.
    pub root_path: Option<PathBuf>,

    ///  This game's path for data, which is where it keeps saves and the load order.
    /// Like the other paths, Modcrab will attempt to automatically find this if not specified.
    pub data_path: Option<PathBuf>,
}

/// A raw version of *TargetGame* designed to be generated from Lua.
/// See *TargetGame*'s docs for information on most fields.
#[derive(Default)]
pub struct RawTargetGame {
	/// A key used for retrieving this game's spec.
	pub spec_key: String,

	pub command: Option<String>,
	pub root_path: Option<String>,
	pub data_path: Option<String>,
}

impl RawTargetGame {
	/// Uses this struct and an *AppData* reference to build a real *TargetGame*.
	pub fn to_real(self, data: &AppData) -> AppResult<TargetGame> {
		let Some(spec) = data.config.games.get(&self.spec_key.to_lowercase()) else {
			// TODO: Consider making this a unique ModpackError.
			let error = Notice::from_preset(NoticePreset::Error, "Modpack")
				.add_field("Description", &format!("This modpack's target game is {}, but that game's specification doesn't exist.", self.spec_key))
				.add_field("Suggestion #1", "Change the target game's name to correspond with a known game specification.")
				.add_field("Suggestion #2", &format!("Write your own specification for {} so Modcrab knows how to manage it.", self.spec_key));
			
			return Err(AppError::Custom(error));
		};

		let real = TargetGame {
			spec: spec.clone(),
			command: self.command,
			root_path: self.root_path.map(|s| PathBuf::from(s)),
			data_path: self.data_path.map(|s| PathBuf::from(s)),
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
			command: table.get("cmd")?,
			root_path: table.get("root")?,
			data_path: table.get("data")?,
		};

		Ok(target)
    }
}

/// Configuration set by the end user.
/// This is mostly configured by **init.lua**.
#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    /// A list of games supported by Modcrab.
    pub games: HashMap<String, GameSpec>,

	/// Temporary value to hold the raw version of the target game.
	#[serde(skip, default)]
	pub raw_target: Option<RawTargetGame>,
	
    /// The game this modpack is for.
    pub target: Option<TargetGame>,

	/// The path to the version of Proton to use.
	pub proton: Option<PathBuf>,
}


impl Default for AppConfig {
	fn default() -> Self {
		Self {
			games: generate_default_game_specs(),
			raw_target: None,
			target: None,
			proton: None,
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

		fields.add_field_method_get("proton", |_, this| {
			let value = match this.proton.clone().map(|p| p.to_str().map(|s| s.to_owned())) {
				Some(Some(path)) => Some(path),
				_ => None,
			};

			Ok(value)
		});

		fields.add_field_method_set("proton", |_, this, value: String| {
			this.proton = Some(PathBuf::from(value));
			Ok(())
		})
	}
}
