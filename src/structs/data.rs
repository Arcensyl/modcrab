//! This module defines data used globally throughout Modcrab.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Contains data used throughout Modcrab's runtime.
#[derive(Default, Serialize, Deserialize)]
pub struct AppData {
    /// The user's configuration for Modcrab.
    pub config: AppConfig,

    /// This is a sorted list of mods within this modpack.
    /// This list is specifically the mods applied to the game's root.
    pub root_mods: IndexMap<String, ModSpec>,

    /// This is a sorted list of mods within this modpack.
    /// This list is the main one, containing all the mods that are applied to the actual mods folder.
    pub mods: IndexMap<String, ModSpec>,

	/// Non-error notices to show the user on command completion.
	/// These notices can be quickly printed through the `Self::print_notices(&mut self)` method.
	#[serde(skip, default)]
	pub notices: Vec<Notice>,
}

impl AppData {
	/// Builds a new *AppData* using the provided *AppConfig*.
	pub fn with_config(config: AppConfig) -> Self {
		Self {
			config,
			..Default::default()
		}
	}

	/// Clears all stored notices while printing them to *STDOUT*.
	pub fn print_notices(&mut self) {
		for notice in self.notices.drain(..) {
			notice.print();
		}
	}
}
