use std::{fs, io::BufReader, path::Path};

use serde::{Serialize, Deserialize};

use crate::prelude::*;

/// Allows a struct to easily be saved and loaded using a file.
/// This is powered via Serde and Bincode.
pub trait SaveLoad {
	/// Attempts to save (serialize) this struct to the file provided.
    fn save(self, path: impl AsRef<Path>) -> Result<(), AppError>;

	/// Attempts to load (deserialize) this struct from the file provided.
    fn load(path: impl AsRef<Path>) -> Result<Self, AppError>
    where Self: Sized;
}

impl<T> SaveLoad for T
where T: Serialize + for<'de> Deserialize<'de> {
    fn save(self, path: impl AsRef<Path>) -> Result<(), AppError> {
        let bin = bincode::serialize(&self)?;
		fs::write(&path, &bin)?;
		Ok(())
    }

    fn load(path: impl AsRef<Path>) -> Result<Self, AppError>
    where Self: Sized {
        let bin = fs::read(path)?;
		
		let item: T = bincode::deserialize(&bin[..])?;
		Ok(item)
    }
}
