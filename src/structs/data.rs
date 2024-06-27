//! This module defines data used globally throughout Modcrab.

use crate::prelude::*;

/// Contains data used throughout Modcrab's runtime.
#[derive(Default)]
pub struct AppData {
    /// The user's configuration for Modcrab.
    pub config: AppConfig,

    /// This is the list of mods loaded by Modcrab.
    /// It is built by the returned tables in the modpack's 'TBD' folder.
    pub mods: IndexMap<String, ModSpec>,

	///  Non-error notices to show the user on command completion.
	/// These notices can be quickly printed through the `Self::print_notices(&mut self)` method.
	pub notices: Vec<Notice>,
}

impl AppData {
	/// Clears all stored notices while printing them to *STDOUT*.
	pub fn print_notices(&mut self) {
		for notice in self.notices.drain(..) {
			notice.print();
		}
	}
}
