// Utilities for persisting changes to a VirtualFileTree.
// This file is completely new, and was written for use with ModcrabFS.

use std::{io, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::tree::VirtualFileTree;

/// Repeatable instructions for manipulating a *VirtualFileTree*.
/// This is mostly used persistent deletions and file movement.
#[derive(Serialize, Deserialize)]
pub enum VirtualFileTransformation {
	/// A request to delete a file from a tree.
	/// This does not actually delete the file; it simply hides it from the tree.
	Deletion {
		/// The path to remove from the tree.
		target: PathBuf
	},

	/// A request to move a file within a tree.
	Relocation {
		/// The file's path before the transformation.
		from: PathBuf,

		/// The file's path after the transformation.
		to: PathBuf,
	},
}

impl VirtualFileTransformation {
	/// Attempts to apply this transformation to the provided tree.
	pub fn apply(&self, tree: &mut VirtualFileTree) -> io::Result<()> {
		match self {
			VirtualFileTransformation::Deletion { target } => { tree.remove_file(&target)?; },
			VirtualFileTransformation::Relocation { from, to } => tree.move_file(&from, &to)?,
		}

		Ok(())
	}

	/// Checks if this transformation can be applied to the provided tree.
	/// This method assumes the transformation has not already been applied.
	pub fn is_valid(&self, tree: &VirtualFileTree) -> bool {
		match self {
			VirtualFileTransformation::Deletion { target } => tree.contains(&target),
			VirtualFileTransformation::Relocation { from, to } => tree.contains(&from) && !tree.contains(&to),
		}
	}
}
