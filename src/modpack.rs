//! This module contains code used for initializing and building modpacks.
use std::{ffi::OsString, fs, mem, path::PathBuf, process::Command};

use modcrabfs::ModcrabFS;

use crate::{lua::eval_config, prelude::*, util::misc::wait_for_enter_key, validation::{validate_config, validate_mod, validate_mod_list, validate_modpack}};

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

	validate_config(&mut data.config)?;
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

/// Entrypoint for Modcrab's 'mount' command.
/// If a command is provided to this function, then it'll run that command after the filesystem is mounted.
pub fn mount_modpack(cmd: Option<Command>) -> AppResult<()> {
	validate_modpack()?;
	let mut data = AppData::load(".modcrab/data.bin")?;

	// By the time this code runs, the presence of a target should already be known.
	let target = match data.config.target {
		Some(ref target) => target,
		None => unreachable!(),
	};
	
	let root_path = target.root_path.clone();
	let attach_point = target.spec.mod_directory
		.clone()
		.conv::<OsString>();
	
	let mut root_overlay: Vec<PathBuf> = Vec::with_capacity(data.root_mods.len());
	let mut mod_overlay: Vec<PathBuf> = Vec::with_capacity(data.mods.len());

	// The game's root directory is the base of the overlay filesystem.
	root_overlay.push(root_path.clone());

	// Overlays every root mod on to the target's root directory.
	let root_mods_list = mem::take(&mut data.root_mods);
	for (_, root_mod) in root_mods_list {
		validate_mod(&root_mod, Some(&mut data))?;
		root_overlay.push(PathBuf::from("mods").join(&root_mod.name));
	}

	// Overlays all normal mods onto the attachment point under the target's root directory.
	// As an example, this would put Skyrim mods under '{root_path}/data'.
	let mods_list = mem::take(&mut data.mods);
	for (_, game_mod) in mods_list {
		validate_mod(&game_mod, Some(&mut data))?;
		mod_overlay.push(PathBuf::from("mods").join(&game_mod.name));
	}

	// This modpack's overwrite directory is always on top.
	root_overlay.push("overwrite".into());

	// This mounts the actual overlay filesystem; spawning a new thread to manage it.
	// This filesystem will stay mounted until its handle goes out of scope.
	let _fs_handle = ModcrabFS::new(root_path.clone(), ".modcrab/cache.bin", root_overlay)?
		.attach(&attach_point, mod_overlay)?
		.spawn_mount()?;

	// If we are given a command, we execute it and wait for it to finish.
	// If not, we simply wait for the user to press enter.
	match cmd {
		Some(mut cmd) => cmd.status()?.pipe(|_| ()),
		None => wait_for_enter_key("Modpack mounted! Press enter to unmount...")?,
	}

	Ok(())
}

/// Entrypoint for Modcrab's 'run' command.
/// This just a wrapper around *mount_modpack()* that prepares a command for it.
pub fn run_modpack(cmd: Vec<String>) -> AppResult<()> {
	// Shadows the command with an actual executable one.
	let cmd = Command::new(&cmd[0])
		.tap_mut(|c| { c.args(&cmd[1..]); });
	
	mount_modpack(Some(cmd))
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
