// ModcrabFS :: A case-insensitive, overlay filesystem for the Modcrab mod manager.

// This is derived from the implementation of PassthroughFS, the example filesystem for the 'fuse_mt' crate.
// PassthroughFS's original copyright :: Copyright (c) 2016-2022 by William R. Fraser

// This file has been heavily modified from its original form as 'passthrough.rs'.
// Most functions have been rewritten.

use std::ffi::{CString, OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::mem;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{Duration, SystemTime};

use crate::libc_extras::{io_to_libc_error, libc};
use crate::libc_wrappers;
use crate::persistence::VirtualFileTransformation;
use crate::shadow::ShadowedDirectory;
use crate::tree::VirtualFileTree;

use fuse_mt::*;

/// A case-insensitive, overlay filesystem.
pub struct ModcrabFS {
    /// The highest directory in the overlay.
    surface: OsString,

    /// A *VirtualFileTree* (VFT) that handles the logic behind the overlay.
    /// The tree itself is behind a *RwLock*.
    tree: RwLock<VirtualFileTree>,

    /// The location where *VirtualFileTransformations* are stored on disk.
    /// This allows this filesystem to remember and reapply changes to the directory tree.
    cache: PathBuf,

    shadowed: ShadowedDirectory,
}

pub fn mode_to_filetype(mode: libc::mode_t) -> FileType {
    match mode & libc::S_IFMT {
        libc::S_IFDIR => FileType::Directory,
        libc::S_IFREG => FileType::RegularFile,
        libc::S_IFLNK => FileType::Symlink,
        libc::S_IFBLK => FileType::BlockDevice,
        libc::S_IFCHR => FileType::CharDevice,
        libc::S_IFIFO => FileType::NamedPipe,
        libc::S_IFSOCK => FileType::Socket,
        _ => {
            panic!("unknown file type");
        }
    }
}

pub fn stat_to_fuse(stat: libc::stat64) -> FileAttr {
    // st_mode encodes both the kind and the permissions
    let kind = mode_to_filetype(stat.st_mode);
    let perm = (stat.st_mode & 0o7777) as u16;

    let time =
        |secs: i64, nanos: i64| SystemTime::UNIX_EPOCH + Duration::new(secs as u64, nanos as u32);

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

#[cfg(target_os = "macos")]
fn statfs_to_fuse(statfs: libc::statfs) -> Statfs {
    Statfs {
        blocks: statfs.f_blocks,
        bfree: statfs.f_bfree,
        bavail: statfs.f_bavail,
        files: statfs.f_files,
        ffree: statfs.f_ffree,
        bsize: statfs.f_bsize as u32,
        namelen: 0, // TODO
        frsize: 0,  // TODO
    }
}

#[cfg(target_os = "linux")]
fn statfs_to_fuse(statfs: libc::statfs) -> Statfs {
    Statfs {
        blocks: statfs.f_blocks,
        bfree: statfs.f_bfree,
        bavail: statfs.f_bavail,
        files: statfs.f_files,
        ffree: statfs.f_ffree,
        bsize: statfs.f_bsize as u32,
        namelen: statfs.f_namelen as u32,
        frsize: statfs.f_frsize as u32,
    }
}

impl ModcrabFS {
    /// Builds a new filesystem by taking a list of paths to overlay.
    /// The resulting filesystem will layer each directory on top of each other.
    /// The first directory is the base, and the last is the upper-most directory.
    /// This method will return an error if passed an empty list.
    pub fn new(
        mountpoint: impl AsRef<Path>,
        cache: impl AsRef<Path>,
        mut overlay: Vec<PathBuf>,
    ) -> io::Result<Self> {
        let surface = overlay
            .pop()
            .ok_or(io::Error::other("Cannot create ModcrabFS from empty list"))?
            .canonicalize()?;

        // The mountpoint cannot be the top-most directory in the overlay.
        if mountpoint.as_ref().canonicalize()? == surface {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }

        // Creates the transformation cache if it doesn't already exist.
        if let Ok(false) = cache.as_ref().try_exists() {
            _ = File::create(&cache);
        }

        let fs = Self {
            surface: surface.clone().into(),
            tree: RwLock::new(VirtualFileTree::new(&surface)),
            cache: cache.as_ref().canonicalize()?,
            shadowed: ShadowedDirectory::new(mountpoint)
                .map_err(io::Error::from_raw_os_error)?,
        };

        let mut tree = fs.tree.write().expect("VFT was poisoned!");

        // Maps all but the top-most directory in the primary overlay.
        // The surface directory is mapped in either mounting methods.
        // This is done so it properly overwrites any secondary overlays the caller may attach.
        for layer in overlay.into_iter().filter_map(|l| l.canonicalize().ok()) {
            tree.map_directory(&layer, None)?;
        }

        mem::drop(tree); // Releases the write lock on the tree

        fs.apply_cache()?;
        Ok(fs)
    }

    /// Attaches a secondary overlay to the directory tree.
    /// This secondary overlay will be accessible under the provided path for the tree.
    /// This path does not have to point to an existing node, but it must be directly under one.
    /// Note that all secondary overlays are overwritten by the top-most directory in the primary overlay.
    pub fn attach(self, attach_point: impl AsRef<Path>, overlay: Vec<PathBuf>) -> io::Result<Self> {
        let mut tree = self.tree.write().expect("VFT was poisoned!");
        let idx = match tree.find_index(&attach_point) {
            Some(idx) => idx,
            None => tree.add_node(&attach_point)?,
        };

        for layer in overlay.into_iter().filter_map(|l| l.canonicalize().ok()) {
            tree.map_directory(&layer, Some(idx))?;
        }

        mem::drop(tree); // Releases the write lock on the tree
        Ok(self)
    }

    /// Mounts this filesystem on the current thread.
    /// This will block until the filesystem is unmounted.
    pub fn mount(self) -> io::Result<()> {
        let mut tree = self.tree.write().expect("VFT was poisoned!");
        let target = self.shadowed.path().to_path_buf();

        tree.map_directory(&self.surface, None)?;
        mem::drop(tree); // Releases write lock

        let args = ["fsname=modcrabfs"].map(OsStr::new);
        fuse_mt::mount(FuseMT::new(self, 1), target, &args)
    }

    /// Mounts this filesystem on a newly spawned thread.
    /// This method returns a handle that will unmount the filesystem when dropped.
    pub fn spawn_mount(self) -> io::Result<fuser::BackgroundSession> {
        let mut tree = self.tree.write().expect("VFT was poisoned!");
        let target = self.shadowed.path().to_path_buf();

        tree.map_directory(&self.surface, None)?;
        mem::drop(tree); // Releases write lock

        let args = ["fsname=modcrabfs"].map(OsStr::new);
        fuse_mt::spawn_mount(FuseMT::new(self, 1), target, &args)
    }

    /// Registers a new path into the filesystem, making it accessible through a virtual path.
    fn register_path(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let real = path.as_ref().canonicalize()?;

        let virt = real
            .strip_prefix(&self.surface)
            .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?
            .to_path_buf()
            .into_os_string()
            .to_ascii_lowercase();

        let mut tree = self.tree.write().expect("VFT was poisoned!");
        debug!("Mapped path: '{:?}' => '{}'", virt, real.display());
        tree.map_file(virt, real)?;
        Ok(())
    }

    /// Returns `true` if a virtual path is valid.
    fn contains(&self, virt: impl AsRef<Path>) -> bool {
        let tree = self.tree.read().expect("VFT was poisoned!");
        tree.contains(virt)
    }

    /// Checks if a path points to a directory in the VFT.
    fn is_dir(&self, path: impl AsRef<Path>) -> bool {
        let tree = self.tree.read().expect("VFT was poisoned!");
        tree.is_dir(path)
    }

    /// Takes a virtual path and returns its real equivalent.
    fn real_path(&self, partial: impl AsRef<Path>) -> io::Result<OsString> {
        let partial = partial.as_ref();
        let tree = self.tree.read().expect("VRT was poisoned!");

        let real = match tree.translate_path(partial) {
            Some(path) => path.as_os_str().to_os_string(),
            None => return Err(io::Error::from(io::ErrorKind::NotFound)),
        };

        debug!("Translated path: '{partial:?}' => '{real:?}'");
        Ok(real)
    }

    pub fn is_shadowing(&self, real: impl AsRef<Path>) -> bool {
        real.as_ref().starts_with(self.shadowed.path())
    }

    /// Reads the transformation cache for this filesystem.
    fn read_cache(&self) -> io::Result<Vec<VirtualFileTransformation>> {
        let cache = fs::read(&self.cache)?;

        let transformations = bincode::deserialize(&cache)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        Ok(transformations)
    }

    /// Updates the transformation cache.
    fn update_cache(&self, transformations: Vec<VirtualFileTransformation>) -> io::Result<()> {
        let data = bincode::serialize(&transformations)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        fs::write(&self.cache, data)
    }

    /// Cleans and then applies all transformations in the cache.
    /// This method assumes the tree is mostly untouched, with no transformations previously applied to it.
    fn apply_cache(&self) -> io::Result<()> {
        let mut tree = self.tree.write().expect("VFT was poisoned!");
        let mut cached = match self.read_cache() {
            Ok(v) => v,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) if e.kind() == io::ErrorKind::InvalidData => Vec::new(),
            Err(e) => {
                error!("Failed to apply cache due to: {e}");
                return Err(e);
            }
        };

        cached.retain(|t| t.is_valid(&tree));

        for transformation in cached.iter() {
            transformation.apply(&mut tree)?;
        }

        self.update_cache(cached)
    }

    /// Applies a transformation to the directory tree, and then puts that transformation into the cache.
    fn transform(&self, transformation: VirtualFileTransformation) -> io::Result<()> {
        let mut tree = self.tree.write().expect("VFT was poisoned!");
        transformation.apply(&mut tree)?;

        let mut cached = match self.read_cache() {
            Ok(v) => v,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };

        cached.push(transformation);
        self.update_cache(cached)
    }
}

pub const TTL: Duration = Duration::from_secs(1);

// TODO: Check the logic for each of these methods.
impl FilesystemMT for ModcrabFS {
    fn init(&self, _req: RequestInfo) -> ResultEmpty {
        info!("ModcrabFS has been initialized!");
        Ok(())
    }

    fn destroy(&self) {
        debug!("Shutting down ModcrabFS...");
    }

    fn getattr(&self, _req: RequestInfo, path: &Path, fh: Option<u64>) -> ResultEntry {
        debug!("getattr: {:?}", path);
        let tree = self.tree.read().expect("VFT was poisoned!");

        if let Ok(real) = self.real_path(path) {
            if self.is_shadowing(&real) {
                return self.shadowed.stat(&real);
            }
        }

        match fh {
            Some(fh) => tree.fstat(fh),
            None => tree.stat(path),
        }
    }

    fn opendir(&self, _req: RequestInfo, path: &Path, _flags: u32) -> ResultOpen {
        debug!("opendir: {:?} (flags = {:#o})", path, _flags);
        let mut tree = self.tree.write().expect("VFT was poisoned!");

        match tree.open_dir(path) {
            Ok(fh) => Ok((fh, 0)),
            Err(e) => {
                error!("opendir({:?}): {}", path, e);
                Err(e.raw_os_error().unwrap())
            }
        }
    }

    fn releasedir(&self, _req: RequestInfo, path: &Path, fh: u64, _flags: u32) -> ResultEmpty {
        debug!("releasedir: {:?}", path);
        let mut tree = self.tree.write().expect("VFT was poisoned!");

        tree.close_dir(fh);
        Ok(())
    }

    fn readdir(&self, _req: RequestInfo, path: &Path, fh: u64) -> ResultReaddir {
        debug!("readdir: {:?}", path);

        if fh == 0 {
            error!("readdir: missing fh");
            return Err(libc::EINVAL);
        }

        let tree = self.tree.read().expect("VFT was poisoned!");
        tree.view_dir(fh).map_err(io_to_libc_error)
    }

    fn open(&self, _req: RequestInfo, path: &Path, flags: u32) -> ResultOpen {
        debug!("open: {:?} flags={:#x}", path, flags);

        let real = self.real_path(path).map_err(io_to_libc_error)?;

        if self.is_shadowing(&real) {
            return self.shadowed.open(&real, flags);
        };

        match libc_wrappers::open(real, flags as libc::c_int) {
            Ok(fh) => Ok((fh, flags)),
            Err(e) => {
                error!("open({:?}): {}", path, io::Error::from_raw_os_error(e));
                Err(e)
            }
        }
    }

    fn release(
        &self,
        _req: RequestInfo,
        path: &Path,
        fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
    ) -> ResultEmpty {
        debug!("release: {:?}", path);
        libc_wrappers::close(fh)
    }

    fn read(
        &self,
        _req: RequestInfo,
        path: &Path,
        fh: u64,
        offset: u64,
        size: u32,
        callback: impl FnOnce(ResultSlice<'_>) -> CallbackResult,
    ) -> CallbackResult {
        debug!("read: {:?} {:#x} @ {:#x}", path, size, offset);
        let mut file = unsafe { UnmanagedFile::new(fh) };

        let mut data = Vec::<u8>::with_capacity(size as usize);

        if let Err(e) = file.seek(SeekFrom::Start(offset)) {
            error!("seek({:?}, {}): {}", path, offset, e);
            return callback(Err(e.raw_os_error().unwrap()));
        }
        match file.read(unsafe { mem::transmute(data.spare_capacity_mut()) }) {
            Ok(n) => {
                unsafe { data.set_len(n) };
            }
            Err(e) => {
                error!("read {:?}, {:#x} @ {:#x}: {}", path, size, offset, e);
                return callback(Err(e.raw_os_error().unwrap()));
            }
        }

        callback(Ok(&data))
    }

    fn write(
        &self,
        _req: RequestInfo,
        path: &Path,
        fh: u64,
        offset: u64,
        data: Vec<u8>,
        _flags: u32,
    ) -> ResultWrite {
        debug!("write: {:?} {:#x} @ {:#x}", path, data.len(), offset);
        let mut file = unsafe { UnmanagedFile::new(fh) };

        if let Err(e) = file.seek(SeekFrom::Start(offset)) {
            error!("seek({:?}, {}): {}", path, offset, e);
            return Err(e.raw_os_error().unwrap());
        }
        let nwritten: u32 = match file.write(&data) {
            Ok(n) => n as u32,
            Err(e) => {
                error!("write {:?}, {:#x} @ {:#x}: {}", path, data.len(), offset, e);
                return Err(e.raw_os_error().unwrap());
            }
        };

        Ok(nwritten)
    }

    fn flush(&self, _req: RequestInfo, path: &Path, fh: u64, _lock_owner: u64) -> ResultEmpty {
        debug!("flush: {:?}", path);
        let mut file = unsafe { UnmanagedFile::new(fh) };

        if let Err(e) = file.flush() {
            error!("flush({:?}): {}", path, e);
            return Err(e.raw_os_error().unwrap());
        }

        Ok(())
    }

    fn fsync(&self, _req: RequestInfo, path: &Path, fh: u64, datasync: bool) -> ResultEmpty {
        debug!("fsync: {:?}, data={:?}", path, datasync);
        let file = unsafe { UnmanagedFile::new(fh) };

        if let Err(e) = if datasync {
            file.sync_data()
        } else {
            file.sync_all()
        } {
            error!("fsync({:?}, {:?}): {}", path, datasync, e);
            return Err(e.raw_os_error().unwrap());
        }

        Ok(())
    }

    fn chmod(&self, _req: RequestInfo, path: &Path, fh: Option<u64>, mode: u32) -> ResultEmpty {
        debug!("chmod: {:?} to {:#o}", path, mode);

        // Virtual directories cannot change permissions.
        if self.is_dir(path) {
            return Err(libc::ENOTSUP);
        }

        let result = if let Some(fh) = fh {
            unsafe { libc::fchmod(fh as libc::c_int, mode as libc::mode_t) }
        } else {
            let real = self
                .real_path(path)
                .map_err(|e| e.raw_os_error().unwrap())?;

            if self.is_shadowing(&real) {
                return self.shadowed.chmod(&real, mode);
            }
            unsafe {
                let path_c = CString::from_vec_unchecked(real.into_vec());
                libc::chmod(path_c.as_ptr(), mode as libc::mode_t)
            }
        };

        if result == -1 {
            let e = io::Error::last_os_error();
            error!("chmod({:?}, {:#o}): {}", path, mode, e);
            Err(e.raw_os_error().unwrap())
        } else {
            Ok(())
        }
    }

    fn chown(
        &self,
        _req: RequestInfo,
        path: &Path,
        fh: Option<u64>,
        uid: Option<u32>,
        gid: Option<u32>,
    ) -> ResultEmpty {
        let unwrapped_uid = uid.unwrap_or(::std::u32::MAX); // docs say "-1", but uid_t is unsigned
        let unwrapped_gid = gid.unwrap_or(::std::u32::MAX); // ditto for gid_t
        debug!("chown: {:?} to {}:{}", path, unwrapped_uid, unwrapped_gid);

        // Virtual directories cannot change owners.
        if self.is_dir(path) {
            return Err(libc::ENOTSUP);
        }

        let result = if let Some(fd) = fh {
            unsafe { libc::fchown(fd as libc::c_int, unwrapped_uid, unwrapped_gid) }
        } else {
            let real = self.real_path(path).map_err(io_to_libc_error)?;

            if self.is_shadowing(&real) {
                return self.shadowed.chown(&real, uid, gid);
            }
            unsafe {
                let path_c = CString::from_vec_unchecked(real.into_vec());
                libc::chown(path_c.as_ptr(), unwrapped_uid, unwrapped_gid)
            }
        };

        if result == -1 {
            let e = io::Error::last_os_error();
            error!(
                "chown({:?}, {}, {}): {}",
                path, unwrapped_uid, unwrapped_gid, e
            );
            Err(e.raw_os_error().unwrap())
        } else {
            Ok(())
        }
    }

    fn truncate(&self, _req: RequestInfo, path: &Path, fh: Option<u64>, size: u64) -> ResultEmpty {
        debug!("truncate: {:?} to {:#x}", path, size);

        let result = if let Some(fd) = fh {
            unsafe { libc::ftruncate64(fd as libc::c_int, size as i64) }
        } else {
            let real = self.real_path(path).map_err(io_to_libc_error)?;

            if self.is_shadowing(&real) {
                return self.shadowed.truncate(&real, size);
            }

            unsafe {
                let path_c = CString::from_vec_unchecked(real.into_vec());
                libc::truncate64(path_c.as_ptr(), size as i64)
            }
        };

        if result == -1 {
            let e = io::Error::last_os_error();
            error!("truncate({:?}, {}): {}", path, size, e);
            Err(e.raw_os_error().unwrap())
        } else {
            Ok(())
        }
    }

    fn utimens(
        &self,
        _req: RequestInfo,
        path: &Path,
        fh: Option<u64>,
        atime: Option<SystemTime>,
        mtime: Option<SystemTime>,
    ) -> ResultEmpty {
        debug!("utimens: {:?}: {:?}, {:?}", path, atime, mtime);

        // Virtual directories cannot change timestamps.
        if self.is_dir(path) {
            return Err(libc::ENOTSUP);
        }

        let systemtime_to_libc = |time: Option<SystemTime>| -> libc::timespec {
            if let Some(time) = time {
                let (secs, nanos) = match time.duration_since(SystemTime::UNIX_EPOCH) {
                    Ok(duration) => (duration.as_secs() as i64, duration.subsec_nanos()),
                    Err(in_past) => {
                        let duration = in_past.duration();
                        (-(duration.as_secs() as i64), duration.subsec_nanos())
                    }
                };

                libc::timespec {
                    tv_sec: secs,
                    tv_nsec: i64::from(nanos),
                }
            } else {
                libc::timespec {
                    tv_sec: 0,
                    tv_nsec: libc::UTIME_OMIT,
                }
            }
        };

        let times = [systemtime_to_libc(atime), systemtime_to_libc(mtime)];

        let result = if let Some(fd) = fh {
            unsafe { libc::futimens(fd as libc::c_int, &times as *const libc::timespec) }
        } else {
            let real = self.real_path(path).map_err(io_to_libc_error)?;

            if self.is_shadowing(&real) {
                return self.shadowed.utimens(&real, times[0], times[1]);
            }

            unsafe {
                let path_c = CString::from_vec_unchecked(real.into_vec());
                libc::utimensat(
                    libc::AT_FDCWD,
                    path_c.as_ptr(),
                    &times as *const libc::timespec,
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            }
        };

        if result == -1 {
            let e = io::Error::last_os_error();
            error!("utimens({:?}, {:?}, {:?}): {}", path, atime, mtime, e);
            Err(e.raw_os_error().unwrap())
        } else {
            Ok(())
        }
    }

    fn readlink(&self, _req: RequestInfo, path: &Path) -> ResultData {
        debug!("readlink: {:?}", path);
        let real = self.real_path(path).map_err(io_to_libc_error)?;

        if self.is_shadowing(&real) {
            return self.shadowed.readlink(&real);
        }

        match ::std::fs::read_link(real) {
            Ok(target) => Ok(target.into_os_string().into_vec()),
            Err(e) => Err(e.raw_os_error().unwrap()),
        }
    }

    // Simply returns the statfs of the overwrite directory on the parent filesystem.
    fn statfs(&self, _req: RequestInfo, path: &Path) -> ResultStatfs {
        let surface = self.surface.to_os_string();
        let mut buf: libc::statfs = unsafe { ::std::mem::zeroed() };
        let result = unsafe {
            let path_c = CString::from_vec_unchecked(surface.into_vec());
            libc::statfs(path_c.as_ptr(), &mut buf)
        };

        if result == -1 {
            let e = io::Error::last_os_error();
            error!("statfs({:?}): {}", path, e);
            Err(e.raw_os_error().unwrap())
        } else {
            Ok(statfs_to_fuse(buf))
        }
    }

    fn fsyncdir(&self, _req: RequestInfo, path: &Path, _fh: u64, datasync: bool) -> ResultEmpty {
        debug!("fsyncdir: {:?} (datasync = {:?})", path, datasync);
        Ok(()) // Silently succeed as directories are virtual.
    }

    fn mknod(
        &self,
        _req: RequestInfo,
        parent_path: &Path,
        name: &OsStr,
        mode: u32,
        rdev: u32,
    ) -> ResultEntry {
        debug!(
            "mknod: {:?}/{:?} (mode={:#o}, rdev={})",
            parent_path, name, mode, rdev
        );

        let real = self
            .real_path(parent_path.join(name))
            .map_err(io_to_libc_error)?;
        let result = unsafe {
            let path_c = CString::from_vec_unchecked(real.as_os_str().as_bytes().to_vec());
            libc::mknod(path_c.as_ptr(), mode as libc::mode_t, rdev as libc::dev_t)
        };

        if result == -1 {
            let e = io::Error::last_os_error();
            error!("mknod({:?}, {}, {}): {}", real, mode, rdev, e);
            Err(e.raw_os_error().unwrap())
        } else {
            self.register_path(&real).unwrap();

            match libc_wrappers::lstat(real) {
                Ok(attr) => Ok((TTL, stat_to_fuse(attr))),
                Err(e) => Err(e), // if this happens, yikes
            }
        }
    }

    fn mkdir(&self, _req: RequestInfo, parent_path: &Path, name: &OsStr, mode: u32) -> ResultEntry {
        debug!("mkdir {:?}/{:?} (mode={:#o})", parent_path, name, mode);

        let virt = parent_path.join(name);
        let real = PathBuf::from(&self.surface)
            .join(virt.strip_prefix("/").unwrap())
            .into_os_string();

        info!("Real: {real:?}");

        fs::create_dir_all(&real).map_err(io_to_libc_error)?;
        self.register_path(real).map_err(io_to_libc_error)?;

        let tree = self.tree.read().expect("VFT was poisoned!");
        tree.stat(&virt)
    }

    fn unlink(&self, _req: RequestInfo, parent_path: &Path, name: &OsStr) -> ResultEmpty {
        debug!("unlink {:?}/{:?}", parent_path, name);

        let virt = parent_path.join(name);

        self.transform(VirtualFileTransformation::Deletion {
            target: virt.to_path_buf(),
        })
        .map_err(io_to_libc_error)
    }

    fn rmdir(&self, _req: RequestInfo, parent_path: &Path, name: &OsStr) -> ResultEmpty {
        debug!("rmdir: {:?}/{:?}", parent_path, name);

        let virt = parent_path.join(name);

        self.transform(VirtualFileTransformation::Deletion {
            target: virt.to_path_buf(),
        })
        .map_err(io_to_libc_error)
    }

    fn symlink(
        &self,
        _req: RequestInfo,
        parent_path: &Path,
        name: &OsStr,
        target: &Path,
    ) -> ResultEntry {
        debug!("symlink: {:?}/{:?} -> {:?}", parent_path, name, target);

        let virt = parent_path.join(name);
        let real = PathBuf::from(&self.surface)
            .join(virt.strip_prefix("/").unwrap())
            .into_os_string();

        match ::std::os::unix::fs::symlink(target, &real) {
            Ok(()) => {
                if !self.contains(&virt) {
                    self.register_path(&real).map_err(io_to_libc_error)?;
                }

                match libc_wrappers::lstat(real.clone()) {
                    Ok(attr) => Ok((TTL, stat_to_fuse(attr))),
                    Err(e) => {
                        error!("lstat after symlink({:?}, {:?}): {}", real, target, e);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                error!("symlink({:?}, {:?}): {}", real, target, e);
                Err(e.raw_os_error().unwrap())
            }
        }
    }

    fn rename(
        &self,
        _req: RequestInfo,
        parent_path: &Path,
        name: &OsStr,
        newparent_path: &Path,
        newname: &OsStr,
    ) -> ResultEmpty {
        debug!(
            "rename: {:?}/{:?} -> {:?}/{:?}",
            parent_path, name, newparent_path, newname
        );

        let virt = parent_path.join(name);
        let new_virt = newparent_path.join(newname);

        self.transform(VirtualFileTransformation::Relocation {
            from: virt.to_path_buf(),
            to: new_virt.to_path_buf(),
        })
        .map_err(io_to_libc_error)
    }

    fn link(
        &self,
        _req: RequestInfo,
        path: &Path,
        newparent: &Path,
        newname: &OsStr,
    ) -> ResultEntry {
        debug!("link: {:?} -> {:?}/{:?}", path, newparent, newname);

        let newvirt = newparent.join(newname);

        let real = PathBuf::from(&self.surface)
            .join(path.strip_prefix("/").unwrap())
            .into_os_string();

        let newreal = PathBuf::from(&self.surface)
            .join(newvirt.strip_prefix("/").unwrap())
            .into_os_string();

        match fs::hard_link(&real, &newreal) {
            Ok(()) => {
                if !self.contains(&newvirt) {
                    self.register_path(&real).map_err(io_to_libc_error)?;
                }

                match libc_wrappers::lstat(real.clone()) {
                    Ok(attr) => Ok((TTL, stat_to_fuse(attr))),
                    Err(e) => {
                        error!("lstat after link({:?}, {:?}): {}", real, newreal, e);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                error!("link({:?}, {:?}): {}", real, newreal, e);
                Err(e.raw_os_error().unwrap())
            }
        }
    }

    fn create(
        &self,
        _req: RequestInfo,
        parent: &Path,
        name: &OsStr,
        mode: u32,
        flags: u32,
    ) -> ResultCreate {
        debug!(
            "create: {:?}/{:?} (mode={:#o}, flags={:#x})",
            parent, name, mode, flags
        );

        let virt = parent.join(name);

        let real_parent = PathBuf::from(&self.surface)
            .join(parent.strip_prefix("/").unwrap())
            .to_path_buf();

        let real = PathBuf::from(&self.surface)
            .join(virt.strip_prefix("/").unwrap())
            .into_os_string();

        fs::create_dir_all(real_parent).map_err(io_to_libc_error)?;

        let fd = unsafe {
            let real_c = CString::from_vec_unchecked(real.clone().into_vec());
            libc::open(
                real_c.as_ptr(),
                flags as i32 | libc::O_CREAT | libc::O_EXCL,
                mode,
            )
        };

        if -1 == fd {
            let ioerr = io::Error::last_os_error();
            error!("create({:?}): {}", real, ioerr);
            Err(ioerr.raw_os_error().unwrap())
        } else {
            if !self.contains(&virt) {
                self.register_path(&real).map_err(io_to_libc_error)?;
            }

            match libc_wrappers::lstat(real.clone()) {
                Ok(attr) => Ok(CreatedEntry {
                    ttl: TTL,
                    attr: stat_to_fuse(attr),
                    fh: fd as u64,
                    flags,
                }),
                Err(e) => {
                    error!(
                        "lstat after create({:?}): {}",
                        real,
                        io::Error::from_raw_os_error(e)
                    );
                    Err(e)
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn setvolname(&self, _req: RequestInfo, name: &OsStr) -> ResultEmpty {
        info!("setvolname: {:?}", name);
        Err(libc::ENOTSUP)
    }

    #[cfg(target_os = "macos")]
    fn getxtimes(&self, _req: RequestInfo, path: &Path) -> ResultXTimes {
        debug!("getxtimes: {:?}", path);
        let xtimes = XTimes {
            bkuptime: SystemTime::UNIX_EPOCH,
            crtime: SystemTime::UNIX_EPOCH,
        };
        Ok(xtimes)
    }
}

/// A file that is not closed upon leaving scope.
struct UnmanagedFile {
    inner: Option<File>,
}

impl UnmanagedFile {
    unsafe fn new(fd: u64) -> UnmanagedFile {
        UnmanagedFile {
            inner: Some(File::from_raw_fd(fd as i32)),
        }
    }
    fn sync_all(&self) -> io::Result<()> {
        self.inner.as_ref().unwrap().sync_all()
    }
    fn sync_data(&self) -> io::Result<()> {
        self.inner.as_ref().unwrap().sync_data()
    }
}

impl Drop for UnmanagedFile {
    fn drop(&mut self) {
        // Release control of the file descriptor so it is not closed.
        let file = self.inner.take().unwrap();
        file.into_raw_fd();
    }
}

impl Read for UnmanagedFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.as_ref().unwrap().read(buf)
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.inner.as_ref().unwrap().read_to_end(buf)
    }
}

impl Write for UnmanagedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.as_ref().unwrap().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.as_ref().unwrap().flush()
    }
}

impl Seek for UnmanagedFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.as_ref().unwrap().seek(pos)
    }
}
