//! This module contains code used for initializing and building modpacks.
use std::fs;

use crate::prelude::*;

/// Function for Modcrab's 'init' command.
/// This simply creates all missing directories, so it can also repair an existing instance.
pub fn init_modpack() -> AppResult<()> {
	fs::create_dir_all(".modcrab")?;

	fs::create_dir_all("config/early")?;
	fs::create_dir_all("config/main")?;

	fs::create_dir_all("mods")?;
	fs::create_dir_all("overwrite")?;
	fs::create_dir_all("downloads")?;
	
	Ok(())
}
