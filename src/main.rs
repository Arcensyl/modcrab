//! Modcrab is a programmable mod manager for Linux.
//! It focuses on Bethesda games, but it can also handle many other games.

mod data;
mod error;
mod prelude;
mod spec;
mod util;

use log::LevelFilter;
use simple_logger::SimpleLogger;

use crate::prelude::*;

fn main() {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(LevelFilter::Info)
        .init()
        .expect("Failed to start logger!");

    info!("Hello from Modcrab!");
}
