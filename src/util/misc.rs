//! This module is the general place for utilities that don't need their own module.

use std::{fmt::Display, fs, io::{self, Read, Write}, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Allows a struct to easily be saved and loaded using a file.
/// This is powered via Serde and Bincode.
pub trait SaveLoad {
    /// Attempts to save (serialize) this struct to the file provided.
    fn save(self, path: impl AsRef<Path>) -> AppResult<()>;

    /// Attempts to load (deserialize) this struct from the file provided.
    fn load(path: impl AsRef<Path>) -> AppResult<Self>
    where
        Self: Sized;
}

/// Trait to extend the *SaveLoad* trait with the ability to fallback to a struct's default value.
pub trait LoadOrDefault {
    /// Attempts to load this struct, and it will fallback to its default value on failure.
    /// Specifically, this method will fallback when the provided file doesn't exist or its its content is invalid.
    fn load_or_default(path: impl AsRef<Path>) -> AppResult<Self>
    where
        Self: Sized;
}

impl<T> SaveLoad for T
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn save(self, path: impl AsRef<Path>) -> AppResult<()> {
        let bin = bincode::serialize(&self)?;
        fs::write(&path, bin)?;
        Ok(())
    }

    fn load(path: impl AsRef<Path>) -> AppResult<Self>
    where
        Self: Sized,
    {
        let bin = fs::read(path)?;

        let item: T = bincode::deserialize(&bin[..])?;
        Ok(item)
    }
}

impl<T> LoadOrDefault for T
where
    T: SaveLoad + Default,
{
    fn load_or_default(path: impl AsRef<Path>) -> AppResult<Self>
    where
        Self: Sized,
    {
        match T::load(path) {
            Err(AppError::IO(e)) if e.kind() == io::ErrorKind::NotFound => Ok(T::default()),
            Err(AppError::Bincode(_)) => Ok(T::default()),
            other => other,
        }
    }
}

/// Helper function to generate a pretty string based on a slice's contents.
pub fn display_slice<T: Display> (slice: &[T]) -> String {
	let mut output = String::new();

	for item in slice.iter() {
		output.push_str(&format!("{item}, "))
	}

	output.trim_end_matches(", ").to_owned()
}

/// Prints the provided message, and then waits for the user to press the enter key.
/// Based on this function from Rust's forum: <https://users.rust-lang.org/t/rusts-equivalent-of-cs-system-pause/4494/4>
pub fn wait_for_enter_key(msg: impl AsRef<str>) -> AppResult<()> {
	let mut stdin = io::stdin();
	let mut stdout = io::stdout();

	// Writes and manually flushes STDOUT so the cursor is at the end.
	stdout.write(msg.as_ref().as_bytes())?;
	stdout.flush()?;

	// Reads a single byte from STDIN, and then discards it.
	let _ = stdin.read(&mut [0u8])?;

	Ok(())
}

/// Builds a new *String* with a string slice transformed by a map between patterns and replacement strings.
pub fn apply_string_sub_map(text: impl AsRef<str>, map: &[(impl AsRef<str>, impl AsRef<str>)]) -> String {
	let text = text.as_ref();
	let mut out = String::from(text);

	for (from, to) in map.iter() {
		out = out.replace(from.as_ref(), to.as_ref());
	}

	out
}

/// Replaces a path's prefix of '~' with the user's home directory.
/// If a path does not start with '~', this function will return a unchanged copy of that path instead.
pub fn replace_path_home_prefix(path: impl AsRef<Path>) -> AppResult<PathBuf> {
	let path = path.as_ref();

	if !path.starts_with("~") { return Ok(path.to_owned()); }

	let Some(home) = dirs::home_dir() else {
		let error = Notice::from_preset(NoticePreset::Error, "Other")
			.add_field("Description", "Failed to retrieve the user's home directory.");

		return Err(AppError::Custom(error));
	};

	Ok(home.join(path.strip_prefix("~").unwrap()))
}
