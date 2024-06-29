//! This module provides functions to check and ensure the validity of modpack instances.

use std::{collections::HashSet, ffi::OsString, fs, path::PathBuf};

use crate::prelude::*;


/// Ensures the current directory is a valid modpack, returning an *Err* if it isn't.
pub fn validate_modpack() -> AppResult<()> {
	let paths_to_check = [
		".modcrab",
		"prefix",
		"config",
		"mods",
		"overwrite",
		"downloads",
	].map(|s| PathBuf::from(s));

	if paths_to_check.into_iter()
		.any(|p| !p.exists()) {
			return Err(AppError::Modpack(ModpackError::InvalidModpack))
		}

	Ok(())
}

/// Ensures the validity of a specific mod.
/// This will return an *Err* if the mod is not installed.
/// If a mutable reference to an *AppData* is provided, other structural problems will be caught as warnings.
pub fn validate_mod(spec: &ModSpec, data: Option<&mut AppData>) -> AppResult<()> {
	let path = PathBuf::from("mods").join(&spec.name);

	if !path.exists() {
		return Err(AppError::Modpack(ModpackError::LocalModNotFound(spec.clone())));
	}

	// Exit early if we don't need to check the mod's structure or can't show warnings.
	if !spec.should_check || data.is_none() { return Ok(()); }

	let data = data.unwrap();

	// This function also exits early if no target game is set.
	let Some(ref target) = data.config.target else {
		return Err(AppError::Modpack(ModpackError::MissingTarget))
	};
	
	let mut count = 0;
	let is_invalid = fs::read_dir(&path)?
		.filter_map(|r| r.ok())
		.inspect(|_| count += 1) // This is definitely not idiomatic, but it works.
		.any(|e| e.path().is_dir() && e.file_name().to_ascii_lowercase() == target.spec.name.to_lowercase().conv::<OsString>());

	// Warns if a mod contains a directory with the same name as the target game's mod directory.
	// This usually means a mod has been packaged in a way that will not work properly with the VFS.
	if is_invalid && count > 0 {
		let warning = Notice::from_preset(NoticePreset::Warning, "Mod")
			.add_field("Description", &format!("The mod {} may be invalid, as it contains a '{}' folder in its root.", spec.name, target.spec.mod_directory))
			.add_field("Suggestion #1", "Manually correct this mod's file structure.")
			.add_field("Suggestion #2", "If this is intentional, you can hide this warning by setting 'check' to false for this mod.");

		data.notices.push(warning);
	}

	// Warns if a mod seemingly contains no files.
	else if count == 0 {
		let warning = Notice::from_preset(NoticePreset::Warning, "Mod")
			.add_field("Description", &format!("The mod {} appears to be empty.", spec.name))
			.add_field("Note", &format!("This warning will also occur if Modcrab does not have permissions to see the contents of 'mods/{}'.", spec.name))
			.add_field("Suggestion #1", &format!("If this mod is from the Nexus, you can redownload it by deleting 'mods/{}' and rebuilding your modpack.", spec.name))
			.add_field("Suggestion #2", "If this is intentional, you can hide this warning by setting 'check' to false for this mod.");

		data.notices.push(warning);
	}

	Ok(())
}

/// Tests an entire list of mods for validity.
/// This function will eventually download missing mods using the NexusMods API.
pub fn validate_mod_list(data: &mut AppData, mods: &mut IndexMap<String, ModSpec>) -> AppResult<()> {
	// This is a stupid hack, but its 7 AM and I just want my code to compile.
	let mod_keys: HashSet<_> = mods.keys()
		.map(|k| k.clone())
		.collect();
	
	for spec in mods.values_mut() {
		// As no AppData instance is provided, this can only fail when the checked mod is not installed.
		if let Err(error) = validate_mod(spec, None) {
			match spec.id {
				Some(_) => todo!(), // Future entrypoint for Nexus API
				None => return Err(error),
			}
		}

		// Throws an error for any missing dependencies.
		for dep in spec.dependencies.iter() {
			if !mod_keys.contains(&dep.to_lowercase()) {
				return Err(AppError::Modpack(ModpackError::MissingDependency {
					cause: spec.clone(),
					dep: dep.to_owned(),
				}));
			}
		}

		// Invalid entries in the 'after' list are simply removed and ignored.
		spec.after = spec.after.drain(..)
			.filter(|ps| mod_keys.contains(&ps.to_lowercase()))
			.collect();

		// We can run a full structural check now.
		validate_mod(spec, Some(data))?;
	}

	Ok(())
}
