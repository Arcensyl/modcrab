// This is the root of ModcrabFS, a case-insensitive, overlay filesystem.
// This file reuses parts of PassthroughFS's 'main.rs' file, but is mostly different.

#[macro_use]
extern crate log;

mod filesystem;
pub use filesystem::ModcrabFS;

mod libc_extras;
mod libc_wrappers;
mod persistence;
mod shadow;
mod tree;

#[cfg(test)]
mod tests;
