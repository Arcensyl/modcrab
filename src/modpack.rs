//! This module contains code used for initializing and building modpacks.
use std::{fs, mem};

use crate::{lua::eval_config, prelude::*, validation::{validate_mod_list, validate_modpack}};

/// Entrypoint for Modcrab's 'init' command.
/// This simply creates all missing directories, so it can also repair an existing instance.
pub fn init_modpack() -> AppResult<()> {
	fs::create_dir_all(".modcrab")?;
	fs::create_dir_all("prefix")?;

	fs::create_dir_all("config")?;

	fs::create_dir_all("mods")?;
	fs::create_dir_all("overwrite")?;
	fs::create_dir_all("downloads")?;
	
	Ok(())
}

/// Entrypoint for Modcrab's 'build' command.
///
/// # Summary of Build Process
/// 1. Evaluate all Lua files in this modpack's 'config' directory.
/// 2. Split the config's list of mods into two lists based on if they are root mods or not.
/// 3. Validate all mods, ensuring they are installed and structurally sound.
/// 4. Sort both mods lists to follow both dependencies and priority.
/// 5. Cache the built modpack to the '.modcrab/data.bin' file.
pub fn build_modpack() -> AppResult<()> {
	validate_modpack()?;

	let (mut data, specs) = eval_config()?;

	let (mut root_mods, mut mods): (IndexMap<String, ModSpec>, IndexMap<String, ModSpec>) = specs.into_iter()
		.filter(|s| s.is_enabled)
		.map(|s| (s.name.to_lowercase(), s))
		.partition(|(_, s)| s.is_root);

	validate_mod_list(&mut data, &mut root_mods)?;
	validate_mod_list(&mut data, &mut mods)?;

	sort_mod_list(&mut root_mods)?;
	sort_mod_list(&mut mods)?;

	data.root_mods = root_mods;
	data.mods = mods;
	
	data.print_notices();

	let sorted_output = data.root_mods.iter().map(|(_, v)| v)
		.chain(data.mods.iter().map(|(_, v)| v))
		.map(|v| format!("{}. {}", v.priority, v))
		.join("\n");

	println!("{}", sorted_output);
	data.save(".modcrab/data.bin")?;
	Ok(())
}

/// Sorts a list of mods by dependency and priority.
/// This algorithm is based on my friend ostech's proof-of-concept version in Go: <https://codeberg.org/ostech/modSort>.
fn sort_mod_list(mods: &mut IndexMap<String, ModSpec>) -> AppResult<()> {
	// Mods are sorted by priority ahead of the proper dependency-aware sort.
	let mut unsorted: IndexMap<String, Option<ModSpec>> = mods
		.tap_mut(|m| m.sort_by(|_, a, _, b| a.priority.cmp(&b.priority)))
		.drain(..)
		.map(|(k, v)| (k, Some(v)))
		.collect();
	
	let mut sorted: IndexMap<String, ModSpec> = IndexMap::with_capacity(unsorted.len());

	let mut index = 0;
	let mut is_ready;
	let mut temp_key;
	let mut temp_value;
	loop {
		if unsorted.len() == sorted.len() { break; }
		is_ready = false;

		// Mods can be loaded if they haven't already been and all their dependencies are met.
		if let Some(ref item) = unsorted[index] {
			is_ready = item.dependencies.is_empty() && item.after.is_empty()
				|| item.dependencies.iter()
				.chain(item.after.iter())
				.all(|d| sorted.contains_key(&d.to_lowercase()));
		}

		if is_ready {
			temp_key = unsorted.get_index(index).unwrap().0.clone();
			temp_value = mem::take(&mut unsorted[index]).unwrap();
			sorted.insert(temp_key, temp_value);

			// We jump to the start of the list to ensure mods with earlier priority load first.
			index = 0;
			continue;
		}

		index += 1;

		// If we go through an entire pass without loading anything, no more mods can be loaded.
		// This means that some mods have unmet or cyclic dependencies.
		if index == unsorted.len() {
			return Err(AppError::Modpack(ModpackError::UnsortableMods(
				unsorted.into_iter()
					.filter_map(|(_, v)| v)
					.collect()
			)));
			
		}
	}

	*mods = sorted;
	Ok(())
}
