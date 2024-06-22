// ShadowedDirectory :: An interface between a VFS and the directory its mounted over.
// This file is entirely new, as it was written for use by ModcrabFS.

use std::{os::{fd::{FromRawFd, OwnedFd, RawFd}, unix::ffi::OsStringExt}, path::{Path, PathBuf}, time::{Duration, SystemTime}};

use fuse_mt::FileAttr;
use nix::{fcntl::{open, openat, readlinkat, AtFlags, OFlag}, sys::{stat::{self, fchmodat, fstatat, utimensat, FchmodatFlags, Mode, UtimensatFlags}, time::TimeSpec}, unistd::{close, fchownat, ftruncate, Gid, Uid}};
use tap::prelude::*;

use crate::{filesystem::{mode_to_filetype, TTL}, libc_extras::io_to_libc_error};

/// An interface to access a directory after it has been shadowed by a mounted filesystem.
/// It does this by managing its own file descriptor and leveraging the 'at' family of syscalls.
pub struct ShadowedDirectory {
	/// The path that has been shadowed.
	path: PathBuf,

	/// A file descriptor that references the shadowed directory.
	/// This is automatically closed when this struct is dropped.
	handle: RawFd,
}

/// Type alias for a *Result* that uses C error codes.
type LowResult<T> = Result<T, libc::c_int>;

/// Helper function to convert Nix's *FileStat* struct to fuse_mt's *FileAttr*.
/// This is an exact copy of *filesystem::stat_to_fuse()*, barring the signature.
fn nix_to_fuse_stat(stat: stat::FileStat) -> FileAttr {
    // st_mode encodes both the kind and the permissions
    let kind = mode_to_filetype(stat.st_mode);
    let perm = (stat.st_mode & 0o7777) as u16;

    let time = |secs: i64, nanos: i64|
        SystemTime::UNIX_EPOCH + Duration::new(secs as u64, nanos as u32);

    // libc::nlink_t is wildly different sizes on different platforms:
    // linux amd64: u64
    // linux x86:   u32
    // macOS amd64: u16
    #[allow(clippy::cast_lossless)]
    let nlink = stat.st_nlink as u32;

    FileAttr {
        size: stat.st_size as u64,
        blocks: stat.st_blocks as u64,
        atime: time(stat.st_atime, stat.st_atime_nsec),
        mtime: time(stat.st_mtime, stat.st_mtime_nsec),
        ctime: time(stat.st_ctime, stat.st_ctime_nsec),
        crtime: SystemTime::UNIX_EPOCH,
        kind,
        perm,
        nlink,
        uid: stat.st_uid,
        gid: stat.st_gid,
        rdev: stat.st_rdev as u32,
        flags: 0,
    }
}

impl ShadowedDirectory {
	/// Opens a new shadowed directory.
	/// This method should be ran before the directory is shadowed.
	/// This is because this method can only access the top-most filesystem.
	pub fn new(path: impl AsRef<Path>) -> LowResult<Self> {
		let path = path.as_ref().canonicalize().map_err(io_to_libc_error)?;
		let handle = open(
			&path,
			OFlag::O_DIRECTORY | OFlag::O_RDONLY,
			Mode::empty(),
		).map_err(|e| e as i32)?;

		let dir = Self { path, handle };
		Ok(dir)
	}

	/// Returns the path belonging to the shadowed directory.
	pub fn path(&self) -> &Path {
		&self.path
	}

	/// Helper method to convert an absolute path to one relative to this directory.
	/// Fails if the provided path is not within this directory.
	fn relate(&self, path: impl AsRef<Path>) -> LowResult<PathBuf> {
		let path = path.as_ref();
		match path.strip_prefix(&self.path) {
			Ok(rel) => Ok(rel.to_path_buf()),
			_ => Err(libc::EINVAL),
		}
	}

	/// Retrieves a shadowed file's attributes and returns them in a FUSE-ready format.
	/// The provided path should be absolute and point to a file under this directory.
	pub fn stat(&self, path: impl AsRef<Path>) -> LowResult<(Duration, FileAttr)> {
		let path = path.pipe(|p| self.relate(p))?;
		let stat = fstatat(Some(self.handle), &path, AtFlags::empty())
			.map_err(|e| e as i32)?
			.pipe(nix_to_fuse_stat);

		Ok((TTL, stat))
	}

	/// Opens a shadowed file and returns all the information needed by FUSE.
	/// This returned data is a tuple containing the opened file's handle and the flags it was opened with.
	pub fn open(&self, path: impl AsRef<Path>, flags: u32) -> LowResult<(u64, u32)> {
		let path = path.pipe(|p| self.relate(p))?;
		
		let fh = openat(
			Some(self.handle),
			&path,
			OFlag::from_bits_retain(flags as i32),
			Mode::S_IRWXU | Mode::S_IROTH | Mode::S_IXOTH,
		).map_err(|e| e as i32)?;

		Ok((fh as u64, flags))
	}

	/// Reads a shadowed symbolic link, and returns its target as a list of bytes.
	pub fn readlink(&self, path: impl AsRef<Path>) -> LowResult<Vec<u8>> {
		let path = path.pipe(|p| self.relate(p))?;

		readlinkat(Some(self.handle), &path)
			.map(|s| s.into_vec())
			.map_err(|e| e as i32)
	}

	/// Changes the permissions of a shadowed file.
	/// This method does not follow symbolic links.
	pub fn chmod(&self, path: impl AsRef<Path>, mode: u32) -> LowResult<()> {
		let path = path.pipe(|p| self.relate(p))?;

		fchmodat(
			Some(self.handle),
			&path,
			Mode::from_bits_retain(mode),
			FchmodatFlags::NoFollowSymlink,
		).map_err(|e| e as i32)
	}

	/// Changes the owner of a shadowed file.
	/// This method does not follow symbolic links.
	pub fn chown(&self, path: impl AsRef<Path>, uid: Option<u32>, gid: Option<u32>) -> LowResult<()> {
		let path = path.pipe(|p| self.relate(p))?;

		fchownat(
			Some(self.handle),
			&path,
			uid.map(|id| Uid::from_raw(id)),
			gid.map(|id| Gid::from_raw(id)),
			AtFlags::AT_SYMLINK_NOFOLLOW,
		).map_err(|e| e as i32)
	}

	/// Changes a shadowed file's timestamps.
	pub fn utimens(&self, path: impl AsRef<Path>, atime: libc::timespec, mtime: libc::timespec) -> LowResult<()> {
		let path = path.pipe(|p| self.relate(p))?;

		utimensat(
			Some(self.handle),
			&path,
			&TimeSpec::from_timespec(atime),
			&TimeSpec::from_timespec(mtime),
			UtimensatFlags::NoFollowSymlink,
		).map_err(|e| e as i32)
	}

	/// Truncates a shadowed file.
	pub fn truncate(&self, path: impl AsRef<Path>, size: u64) -> LowResult<()> {
		let path = path.pipe(|p| self.relate(p))?;

		let raw_fh = openat(
			Some(self.handle),
			&path,
			OFlag::O_WRONLY,
			Mode::S_IRWXU | Mode::S_IROTH | Mode::S_IXOTH,
		).map_err(|e| e as i32)?;

		// OwnedFd should close the opened file on its own.
		unsafe {
			ftruncate(OwnedFd::from_raw_fd(raw_fh), size as i64).map_err(|e| e as i32)
		}
	}
}

impl Drop for ShadowedDirectory {
    fn drop(&mut self) {
		close(self.handle)
			.expect("Failed to close shadowed directory's file handle!");
    }
}
