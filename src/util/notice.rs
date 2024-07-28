//! This module provides the *Notice* struct, which is used for pretty-printing warnings, errors, or other messages to users.

use crate::{prelude::*, structs::error::{GameError, ModpackError}, util::text::TextStyle};
use std::{fmt::Display, io};

use super::{misc::display_slice, text::TextColor};

/// Notices allow you to easily pretty-print warning, errors, and other various information.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Notice {
    color: TextColor,
    prefix: String,
    header: String,
    fields: Vec<(String, String)>,
}

/// Presets to use while making a notice, allowing you to quickly recreate common forms of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoticePreset {
	/// A red notice with the prefix "ERROR".
    Error,

	/// A yellow notice with the prefix "WARN".
    Warning,

	/// A green notice with the prefix "DONE".
	Success,

	/// A cyan notice with the prefix "STATS".
	Statistics,
}

impl Notice {
	/// Builds a new notice from raw components.
    pub fn new(color: TextColor, prefix: &str, header: &str) -> Self {
        Self {
            color,
            prefix: prefix.to_owned(),
            header: header.to_owned(),
            fields: Vec::new(),
        }
    }

	/// Builds a new notice from a preset and a header.
    pub fn from_preset(preset: NoticePreset, header: &str) -> Self {
        match preset {
            NoticePreset::Error => Notice::new(TextColor::Red, "ERROR", header),
            NoticePreset::Warning => Notice::new(TextColor::Yellow, "WARN", header),
			NoticePreset::Success => Notice::new(TextColor::Green, "DONE", header),
			NoticePreset::Statistics => Notice::new(TextColor::Cyan, "STATS", header),
        }
    }

	/// Adds a new field to this notice, which will be printed after any other fields.
	/// A field will be presented in the form of `"{label}: {content}"`.
	pub fn add_field(mut self, label: &str, content: &str) -> Self {
		self.fields.push((label.to_owned(), content.to_owned()));
		self
	}

	/// Convenience method to allow printing a notice at the end of a dot-call chain. 
	pub fn print(self) {
		println!("{self}");
	}
}

impl Display for Notice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let title = format!("[{} - {}]", self.prefix, self.header).stylize(Some(TextStyle::Bold), Some(self.color), None);
        write!(f, "{}\n", title)?;

		let mut formatted_label;
		for (label, content) in self.fields.iter() {
			formatted_label = format!("  {label}: ").stylize(Some(TextStyle::Bold), Some(self.color), None);
			write!(f, "{}{}\n", formatted_label, content)?;
		}

		Ok(())
    }
}

// Beyond this point is just conversions between errors and notices.

impl From<AppError> for Notice {
    fn from(value: AppError) -> Self {
        match value {
            AppError::IO(error) => error.into(),
            AppError::Lua(error) => error.into(),
			AppError::Modpack(error) => error.into(),
			AppError::Game(error) => error.into(),
			AppError::Custom(notice) => notice,
			
            AppError::Bincode(error) => Notice::from_preset(NoticePreset::Error, "(De)serialization")
				.add_field("Description", "Failed to convert a Rust type to a string or vice-versa.")
				.add_field("Details", &error.to_string()),
			
            AppError::Unknown(error) => Notice::from_preset(NoticePreset::Error, "Unknown")
				.add_field("Message", "An unknown error has occurred!")
				.add_field("Details", &error.to_string()),
        }
    }
}

impl From<io::Error> for Notice {
    fn from(value: io::Error) -> Self {
		let notice = Notice::from_preset(NoticePreset::Error, "IO");
		
        match value.kind() {
            io::ErrorKind::NotFound => notice
				.add_field("Description", "Modcrab tried to access a file that doesn't exist.")
				.add_field("Suggestion", "Run 'modcrab repair' to attempt to regenerate any missing files."),
			
            io::ErrorKind::PermissionDenied => notice
				.add_field("Description", "Modcrab tried to access a file, but it didn't have the right permissions.")
				.add_field("Suggestion", "Ensure you have full permissions for all files in this modpack."),
			
            io::ErrorKind::AlreadyExists => notice
				.add_field("Description", "Modcrab tried to create a new file, but that file already exists.")
				.add_field("Note", "This error is likely a bug. Please open an issue using the link below.")
				.add_field("Link", "https://github.com/Arcensyl/modcrab/issues"),
			
            other => notice
				.add_field("Description", "An unknown error has occurred!")
				.add_field("Details", &other.to_string()),
        }
    }
}

impl From<LuaError> for Notice {
    fn from(value: LuaError) -> Self {
		let notice = Notice::from_preset(NoticePreset::Error, "Lua");
		
        match value {
            LuaError::SyntaxError { message, .. } => notice
				.add_field("Description", "Your config contains a syntax error.")
				.add_field("Details", &message),
			
            LuaError::RuntimeError(msg) => notice
				.add_field("Description", "Your config caused a Lua runtime error.")
				.add_field("Details", &msg),
			
            LuaError::MemoryError(msg) => notice
				.add_field("Description", "Lua ran out of memory while executing your config.")
				.add_field("Details", &msg),
			
            LuaError::ToLuaConversionError { from, to, message } => notice
				.add_field("Description", &format!("Failed to convert a {from} into a Lua {to}."))
				.pipe(|n| match message { Some(msg) => n.add_field("Details", &msg), None => n, })
				.add_field("Note", "This is a bug. Please open an issue using the link below.")
				.add_field("Link", "https://github.com/Arcensyl/modcrab/issues"),
			
            LuaError::FromLuaConversionError { from, to, message } => notice
				.add_field("Description", &format!("Failed to convert a Lua {from} into a {to}."))
				.pipe(|n| match message { Some(msg) => n.add_field("Details", &msg), None => n, }),
			
            LuaError::WithContext { context, cause } => notice
				.add_field("Description", "Encountered an error with extra context while executing your config.")
				.add_field("Source", &cause.to_string())
				.add_field("Context", &context),

			other => notice
				.add_field("Description", "An unknown error occurred while executing your config.")
				.add_field("Details", &other.to_string()),
        }
    }
}

impl From<ModpackError> for Notice {
	fn from(value: ModpackError) -> Self {
		let notice = Notice::from_preset(NoticePreset::Error, "Modpack");

		match value {
			ModpackError::InvalidModpack => notice
				.add_field("Description", "The current directory is not a valid modpack.")
				.add_field("Details", "This is because the current directory doesn't contain a '.modcrab' directory.")
				.add_field("Suggestion", "If it is supposed to be a modpack, try running 'modcrab init' to regenerate missing files."),
			
			ModpackError::MissingTarget => notice
				.add_field("Description", "This modpack does not specify a target game.")
				.add_field("Suggestion", "Set 'modcrab.target' in your config."),
			
			ModpackError::LocalModNotFound(spec) => notice
				.add_field("Description", &format!("The mod {} is local but isn't installed.", spec.name))
				.add_field("Suggestion #1", &format!("If this mod should be local, manually add {} to your modpack's 'mods' folder.", spec.name))
				.add_field("Suggestion #2", &format!("If this mod should be from the Nexus, specify {}'s 'slug' field in your config.", spec.name)),
			
			ModpackError::MissingDependency { cause, dep } => notice
				.add_field("Description", &format!("The mod {} depends on {dep}, which is not in your config.", cause.name))
				.add_field("Suggestion", &format!("Add {dep}'s specification to your config.")),
			
			ModpackError::UnsortableMods(specs) => notice
				.add_field("Description", "The following mods cannot be sorted, likely due to a dependency cycle.")
				.add_field("Mods", &display_slice(&specs))
				.add_field("Suggestion", "Search through the broken mod list, while looking for any illogical dependencies."),
		}
    }
}

impl From<GameError> for Notice {
    fn from(value: GameError) -> Self {
        let notice = Notice::from_preset(NoticePreset::Error, "Game");

		match value {
			GameError::MissingSpec(target) => notice
				.add_field("Description", &format!("This modpack's target game is {}, but that game's specification doesn't exist.", target.spec_key))
				.add_field("Suggestion #1", "Change the target game's name to correspond with a known game specification.")
				.add_field("Suggestion #2", &format!("Write your own specification for {} so Modcrab knows how to manage it.", target.spec_key)),
			
			GameError::MissingProton => notice
				.add_field("Description", "Your config does not specify a Proton binary to use, but the game or a tool is for Windows.")
				.add_field("Suggestion", "Set 'modcrab.proton', to a Proton binary's path, in your config."),
			
			GameError::InvalidProton => notice
				.add_field("Description", "The config's 'modcrab.proton' field does not point to a valid file.")
				.add_field("Suggestion", "Ensure the path in 'modcrab.proton' is valid."),
			
			GameError::ScanUnavailable(label) => notice
				.add_field("Description", &format!("This config does not explicitly set its target's {label} path, but the game's specification not support automatically determining that path."))
				.add_field("Suggestion", &format!("Set 'modcrab.target.{label}' in your config.")),
			
			GameError::ScanFailed(label) => notice
				.add_field("Description", &format!("Failed to automatically determine the target game's {label} path."))
				.add_field("Suggestion", &format!("Tell Modcrab where to find this by explicitly setting the 'modcrab.target.{label}' field.")),

			GameError::InvalidPath { label, path } => notice
				.add_field("Description", &format!("The game's {label} path, '{}', does not point to a valid location.", path.display()))
				.add_field("Suggestion", &format!("Ensure the path in 'modcrab.target.{label}' is valid.")),
		}
	}
}
