//! This module defines data used globally throughout Modcrab.

use std::sync::{Mutex, OnceLock};

/// Contains data used throughout Modcrab's runtime.
/// This struct is mostly used through the 'APP_DATA' static.
pub struct AppData {
	config: AppConfig
}

/// This static is a globally-accessible instance of the *AppData* struct.
/// Because *AppData* needs values set by the end user, this is not available until **init.lua** is evaluated.
pub static APP_DATA: OnceLock<Mutex<AppData>> = OnceLock::new();

/// Configuration set by the end user.
/// This is mostly configured by **init.lua**.
pub struct AppConfig {
	// TODO fill this out
}
