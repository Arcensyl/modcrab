//! This module defines data used globally throughout Modcrab.

use crate::prelude::*;

/// Contains data used throughout Modcrab's runtime.
#[derive(Default)]
pub struct AppData {
    /// The user's configuration for Modcrab.
    pub config: AppConfig,

    /// This is the list of mods loaded by Modcrab.
    /// It is built by the returned tables in the modpack's 'TBD' folder.
    pub mod_list: IndexMap<String, ModSpec>,
}
