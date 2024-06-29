//! This module contains code related to Lua interop.

use std::{ffi::OsStr, mem};

use walkdir::WalkDir;

use crate::prelude::*;

/// Evaluates this modpack's Lua config.
pub fn eval_config() -> AppResult<(AppData, Vec<ModSpec>)> {
	let lua = Lua::new();
	let mut specs = Vec::new();

	// Exposes Modcrab's config to Lua as a global table called 'modcrab'.
	lua.globals().set("modcrab", AppConfig::default())?;
	let sandbox = build_sandbox(&lua)?;
	
	let walker = WalkDir::new("config")
		.sort_by_file_name()
		.into_iter()
		.filter_map(|r| r.ok())
		.filter(|e| e.path().extension() == Some(OsStr::new("lua")));

	// Runs all Lua scripts in the modpack's 'config' directory.
	for script in walker {
		match lua.load(script.path()).set_environment(&sandbox).eval::<Option<Vec<ModSpec>>>()? {
			Some(mut list) => specs.append(&mut list),
			None => {},
		}
	}

	let config = lua.globals().get("modcrab")?;
	let mut data = AppData::with_config(config);

	// Transforms the config's raw target into the real one.
	let Some(target) = mem::take(&mut data.config.raw_target) else {
		return Err(AppError::Modpack(ModpackError::MissingTarget));
	};

	data.config.target = Some(target.to_real(&mut data)?);
	
	if specs.is_empty() {
		let warn = Notice::from_preset(NoticePreset::Warning, "Modpack")
			.add_field("Description", "Your config specifies no mods to manage.")
			.add_field("Suggestion", "Add some mod specifications to your config.");

		data.notices.push(warn);
	}
	
	Ok((data, specs))
}

/// Builds a sandbox environment to use with the user's Lua config.
/// This sandbox is a table that forwards the 'modcrab' global and safe parts of Lua's standard library.
fn build_sandbox<'lua> (lua: &'lua Lua) -> AppResult<LuaTable<'lua>> {
	let sandbox_env: LuaTable = lua.load(include_str!("sandbox.lua"))
		.set_name("SANDBOX")
		.eval()?;

	Ok(sandbox_env)
}

/// Retrieves a Lua value or table of values and then converts that into a *Vec<V>*.
/// If the key's corresponding value is nil, the returned list will be empty.
pub fn convert_table_item_to_vec<'lua, K: IntoLua<'lua> + Clone, V: FromLua<'lua>> (table: &'lua LuaTable, key: K) -> LuaResult<Vec<V>> {
	match table.get::<_, Option<V>>(key.clone()) {
		Ok(Some(value)) => return Ok(vec![value]),
		Ok(None) => return Ok(Vec::new()),
		_ => {},
	}

	table.get(key)
}
