//! Modcrab is a programmable mod manager for Linux.
//! It focuses on Bethesda games, but it can also handle many other games.

mod prelude;
mod modpack;
mod validation;
mod lua;
mod structs;
mod util;

use std::{env, path::PathBuf};

use clap::{Parser, Subcommand};
use modpack::{build_modpack, init_modpack, mount_modpack, run_modpack};

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

	/// Mounts a modpack over the target game.
	Mount,

	/// Mounts a modpack before running a specified command.
	Run {
		/// The command to run.
		#[clap(required = true, trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
		cmd: Vec<String>,
	},
}

/// Entrypoint for Modcrab.
fn main() {
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
        Command::Init => init_modpack()?,
        Command::Build => build_modpack()?,
		Command::Mount => mount_modpack(None)?,
		Command::Run { cmd } => run_modpack(cmd)?,
    }

    Ok(())
}
