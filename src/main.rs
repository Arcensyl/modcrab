//! Modcrab is a programmable mod manager for Linux.
//! It focuses on Bethesda games, but it can also handle many other games.

mod prelude;
mod structs;
mod util;

use std::{env, io, path::PathBuf};

// use log::LevelFilter;
// use simple_logger::SimpleLogger;

use clap::{Parser, Subcommand};

use crate::prelude::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Run Modcrab as if it was in the provided directory.
    #[arg(short = 'R', long)]
    remote: Option<PathBuf>,

    /// The command to execute.
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Clone, PartialEq, Eq)]
enum Command {
    /// Create a new modpack in the current directory.
    Init,

    /// Builds your modpack's config and acquires any missing mods.
    Build,

    /// Runs this modpack's target game or one of its tools.
    Run {
        /// If this is set, Modcrab will run this tool instead of the target game.
        tool: Option<String>,
    },
}

/// Entrypoint for Modcrab.
fn main() {
    // SimpleLogger::new()
    //     .with_colors(true)
    //     .with_level(LevelFilter::Info)
    //     .init()
    //     .expect("Failed to start logger!");

    // info!("Hello from Modcrab!");

    let args = Cli::parse();

	let mut old_cwd = None;
	if let Some(ref remote) = args.remote {
		old_cwd = match env::current_dir() {
			Ok(cwd) => Some(cwd),
			Err(_) => {
				Notice::from_preset(NoticePreset::Error, "Remote")
					.add_field("Description", "Failed to get the current working directory.")
					.add_field("Suggestion", "Ensure the working directory exists and is accessible to your user.")
					.print();

				return;
			}
		};

		if env::set_current_dir(remote).is_err() {
			Notice::from_preset(NoticePreset::Error, "Remote")
				.add_field("Description", "Failed to change working directory for the remote flag.")
				.add_field("Suggestion", "Ensure the remote directory exists and is accessible to your user.")
				.print();
			
			return;
		}
	}

	if let Err(error) = run_command(args) { error.conv::<Notice>().print(); }

	if let Some(owd) = old_cwd {
		if let Err(error) = env::set_current_dir(owd) { error.conv::<Notice>().print(); }
	}
}

/// Runs the command specified by the passed CLI arguements.
fn run_command(args: Cli) -> AppResult<()> {
    match args.cmd {
        Command::Init => todo!(),
        Command::Build => todo!(),
        Command::Run { tool } => todo!(),
    }

    Ok(())
}
