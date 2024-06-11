//! Modcrab is a programmable mod manager for Linux.
//! It focuses on Bethesda games, but it can also handle many other games.

mod prelude;

use simple_logger::SimpleLogger;
use log::LevelFilter;

use crate::prelude::*;

fn main() {
	SimpleLogger::new()
		.with_colors(true)
		.with_level(LevelFilter::Info)
		.init()
		.expect("Failed to start logger!");

	info!("Hello from Modcrab!");
}
