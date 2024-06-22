// VirtualFileTree :: An in-memory directory tree that represents a merged view into several real directories.
// This file is completely new, and was written for use with ModcrabFS.

use std::{collections::HashMap, ffi::{CStr, OsStr, OsString}, fs, io, path::{Path, PathBuf}, time::SystemTime};
use std::os::unix::ffi::OsStrExt; 

use fuse_mt::{DirectoryEntry, FileAttr, FileType};
use nix::unistd::{Gid, Uid};
use petgraph::{algo::has_path_connecting, graph::NodeIndex, stable_graph::StableDiGraph, visit::EdgeRef};
use rand::{thread_rng, Rng};

use crate::{libc_extras::libc, filesystem::{mode_to_filetype, stat_to_fuse, TTL}};
use crate::libc_wrappers;

/// The maximum number of attempts when randomly generating a unique file handle.
/// This should rarely, if ever, be hit under normal circumstances.
const MAX_HANDLE_GENERATION_TRIES: u8 = 100;

/// A tree representing a case-insensitive, overlay filesystem.
pub struct VirtualFileTree {
	/// The actual tree graph itself.
	graph: StableDiGraph<VirtualFileData, OsString>,

	/// File handles mapped to directories are stored here.
	/// The handles are all unique, as they are generated via RNG.
	handles: HashMap<u64, NodeIndex>,
}

/// Represents a file within a *VirtualFileTree*.
/// This struct simply tracks a file's real path and its Linux file type.
pub struct VirtualFileData {
	/// The real path this node points to.
	pub path: PathBuf,

	/// The Linux file type of the real file.
	pub kind: FileType,

	/// Determines if this node is treated as the root of the VFT.
	pub is_root: bool,
}

impl VirtualFileData {
	/// Returns this file's real path.
	pub fn real_path(&self) -> &Path {
		&self.path
	}
}

impl From<VirtualFileData> for DirectoryEntry {
    fn from(val: VirtualFileData) -> Self {
		DirectoryEntry {
			name: val.path
				.file_name()
				.unwrap()
				.to_os_string(),
			
			kind: val.kind,
		}
    }
}

/// Retrieves a real file's type using lstat.
fn query_file_type(path: impl AsRef<Path>) -> io::Result<FileType> {
	let path = path.as_ref()
		.as_os_str()
		.to_os_string();

	let stat = libc_wrappers::lstat(path)
		.map_err(io::Error::from_raw_os_error)?;

	Ok(mode_to_filetype(stat.st_mode))
}

/// Reads a directory to prepare it for being mapped into a *VirtualFileTree*.
fn read_dir(path: impl AsRef<Path>) -> io::Result<Vec<VirtualFileData>> {
	let path = path.as_ref();
	let mut full_paths: HashMap<OsString, PathBuf> = HashMap::new();

	let mut entry_path;
	for entry in fs::read_dir(path)? {
		entry_path = entry?.path();
		full_paths.insert(entry_path.file_name().unwrap().to_os_string(), entry_path.to_path_buf());
	}

	let fh = libc_wrappers::opendir(path.as_os_str().to_os_string())
		.map_err(io::Error::from_raw_os_error)?;

	// Code after this point is mostly borrowed from PassthroughFS's 'readdir' method.
	let mut entries: Vec<DirectoryEntry> = vec![];

	if fh == 0 {
		error!("readdir: missing fh");
		return Err(io::Error::from_raw_os_error(libc::EINVAL));
	}

    loop {
        match libc_wrappers::readdir(fh) {
            Ok(Some(entry)) => {
                let name_c = unsafe { CStr::from_ptr(entry.d_name.as_ptr()) };
                let name = OsStr::from_bytes(name_c.to_bytes()).to_owned();

                let filetype = match entry.d_type {
                    libc::DT_DIR => FileType::Directory,
                    libc::DT_REG => FileType::RegularFile,
                    libc::DT_LNK => FileType::Symlink,
                    libc::DT_BLK => FileType::BlockDevice,
                    libc::DT_CHR => FileType::CharDevice,
                    libc::DT_FIFO => FileType::NamedPipe,
                    libc::DT_SOCK => {
                        warn!("FUSE doesn't support Socket file type; translating to NamedPipe instead.");
                        FileType::NamedPipe
                    },
                    _ => panic!("Can't get file type of: {name:?}"),
                };

                entries.push(DirectoryEntry {
                    name,
                    kind: filetype,
                })
            },
            Ok(None) => { break; },
            Err(e) => {
                error!("readdir: {:?}: {}", path, e);
                return Err(io::Error::from_raw_os_error(e));
            }
        }
    }

	println!("{entries:?}");
	println!("{full_paths:?}");

	let final_contents = entries.into_iter()
		.filter(|e| !(&e.name == "." || &e.name == ".."))
		.map(|e| VirtualFileData {path: full_paths.remove(&e.name).unwrap(), kind: e.kind, is_root: false})
		.collect();

	Ok(final_contents)
}

/// Generates fake file attributes for virtual directories.
fn generate_fake_attr() -> FileAttr {
	let now = SystemTime::now();
	
	FileAttr {
		size: 0,
		blocks: 0,
		atime: now,
		mtime: now,
		ctime: now,
		kind: FileType::Directory,
		perm: 0o755, // Everyone can read and execute, only owner can write.
		nlink: 2,
		uid: Uid::current().as_raw(),
		gid: Gid::current().as_raw(),
		rdev: 0, //  Not a special file.

		// Mac-only fields:
		crtime: SystemTime::now(),
		flags: 0,
	}
}

impl VirtualFileTree {
	/// Constructs a new *VirtualFileTree*.
	pub fn new(real_root: impl AsRef<Path>) -> Self {
		let real_root = real_root.as_ref();
		assert!(real_root.is_dir());

		let mut fs = Self {
			graph: StableDiGraph::new(),
			handles: HashMap::new(),
		};
		
		fs.graph.add_node(VirtualFileData {
			path: real_root.to_path_buf(),
			kind: FileType::Directory,
			is_root: true,
		});

		fs
	}

	/// Finds the index of the node corresponding to the requested virtual path.
	/// Returns *None* if the virtual path does not exist.
	pub fn find_index(&self, path:  impl AsRef<Path>) -> Option<NodeIndex> {
		let virt = match path.as_ref().has_root() {
			true => path.as_ref().strip_prefix("/").unwrap(),
			false => path.as_ref(),
		};

		let mut idx = NodeIndex::new(0);
		for step in virt.components()
			.map(|c| c.as_os_str().to_ascii_lowercase()) {
				match self.graph.edges(idx).find(|e| e.weight() == &step) {
					Some(edge) => idx = edge.target(),
					None => return None,
				}
			}

		Some(idx)
	}

	/// Translates a virtual path into its real equivalent.
	pub fn translate_path(&self, virt: impl AsRef<Path>) -> Option<&Path> {
		let idx = self.find_index(virt)?;
		Some(self.graph[idx].real_path())
	}

	/// Checks if the tree contains a requested path.
	pub fn contains(&self, path: impl AsRef<Path>) -> bool {
		self.find_index(path).is_some()
	}

	/// Maps a real directory to the tree.
	/// You can optionally provide an attachment point to map the directory to.
	/// If the attachment point is *None*, the root of the tree will be used.
	pub fn map_directory(&mut self, path: impl AsRef<Path>, attach_point: Option<NodeIndex>) -> io::Result<()> {
		let mut roots: Vec<NodeIndex> = Vec::new();
		let mut dirs: Vec<Vec<VirtualFileData>> = Vec::new();
		let mut depth = 0;

		let attach_point = match attach_point {
			Some(point) => point,
			None => NodeIndex::new(0)
		};
		
		roots.push(attach_point);
		dirs.push(read_dir(path)?);
		
		'outer: loop {
			while let Some(item) = dirs[depth].pop() {
				if item.kind == FileType::Directory {
					dirs.push(read_dir(&item.path)?);
					
					let dir = self.update_child(roots[depth], item);
					roots.push(dir);
					
					depth += 1;
					continue 'outer;
				}

				else { self.update_child(roots[depth], item); }

			}			

			roots.pop();
			dirs.pop();
			
			match depth == 0 {
				true => break,
				false => depth -= 1,
			}
		}
		
		Ok(())
	}

	/// Maps a single file into the tree.
	pub fn map_file(&mut self, virt: impl AsRef<Path>, real: impl AsRef<Path>) -> io::Result<()> {
		let real = real.as_ref();
		let Some(parent) = virt.as_ref().parent() else {
			return Err(io::Error::from(io::ErrorKind::InvalidInput));
		};

		let Some(root) = self.find_index(parent) else {
			return Err(io::Error::from(io::ErrorKind::NotFound));
		};

		let child_data = VirtualFileData {
			path: real.to_path_buf(),
			kind: query_file_type(real)?,
			is_root: false,
		};

		
		self.update_child(root, child_data);

		Ok(())
	}

	/// Adds a fully-virtual node to the tree.
	/// This is mostly useful when defining attachment points for other overlays.
	pub fn add_node(&mut self, virt: impl AsRef<Path>) -> io::Result<NodeIndex> {
		let mut virt = virt.as_ref();
		
		let Some(parent) = virt.parent() else {
			return Err(io::Error::from(io::ErrorKind::InvalidInput));
		};

		let Some(root) = self.find_index(parent) else {
			return Err(io::Error::from(io::ErrorKind::NotFound));
		};

		// Remove root prefix before joining path.
		virt = match virt.strip_prefix("/") {
			Ok(new) => new,
			Err(_) => virt,
		};
		
		let child_data = VirtualFileData {
			path: PathBuf::from("<VIRTUAL>").join(virt),
			kind: FileType::Directory,
			is_root: false,
		};

		
		let idx = self.update_child(root, child_data);
		Ok(idx)
	}


	/// Removes a file from the tree.
	/// This does not delete the physical file, it only hides it from the tree's view.
	pub fn remove_file(&mut self, path: impl AsRef<Path>) -> io::Result<VirtualFileData> {
		let Some(node) = self.find_index(path) else {
			return Err(io::Error::from(io::ErrorKind::NotFound))
		};

		let data = self.graph.remove_node(node).unwrap();
		self.clear_orphans();

		Ok(data)
	}

	/// Moves a file to a different place in the tree.
	/// This does not move the physical file.
	pub fn move_file(&mut self, old_path: impl AsRef<Path>, new_path: impl AsRef<Path>) -> io::Result<()> {
		let old_path = old_path.as_ref();
		let new_path = new_path.as_ref();

		let data = self.remove_file(old_path)?;

		let Some(Some(new_parent)) = new_path.parent().map(|p| self.find_index(p)) else {
			return Err(io::Error::from(io::ErrorKind::NotFound));
		};

		self.update_child(new_parent, data);
		Ok(())
	}

	/// Opens a virtual directory and returns a file handle that refers to it.
	/// Remember to properly discard your file handle using `VirtualFileTree::close_dir(handle)`.
	pub fn open_dir(&mut self, path: impl AsRef<Path>) -> io::Result<u64> {
		let Some(dir) = self.find_index(path) else {
			return Err(io::Error::from(io::ErrorKind::NotFound))
		};

		let mut rng = thread_rng();
		let mut handle: u64;
		let mut tries = 0;
		loop {
			if tries == MAX_HANDLE_GENERATION_TRIES {
				panic!("Could not generate new file handle in maximum number of attempts ({MAX_HANDLE_GENERATION_TRIES})");
			}

			handle = rng.gen();

			if let std::collections::hash_map::Entry::Vacant(e) = self.handles.entry(handle) {
				e.insert(dir);
				break;
			}

			tries += 1;
		}
		
		Ok(handle)
	}
	
	/// Builds a view into the directory specified by the provided handle.
	/// This is primarily used to expose a directory to FUSE.
	/// This method takes a file handle, which you will need to acquire using `VirtualFileTree::open_dir(path)`.
	pub fn view_dir(&self, handle: u64) -> io::Result<Vec<DirectoryEntry>> {
		let Some(dir) = self.handles.get(&handle).copied() else {
			return Err(io::Error::from(io::ErrorKind::NotFound))
		};

		let mut entries = Vec::new();
		for child in self.graph.neighbors(dir).map(|n| &self.graph[n]) {
			let entry = DirectoryEntry {
				name: child.path
					.file_name()
					.ok_or(io::Error::from(io::ErrorKind::InvalidInput))?
					.to_os_string(),
				
				kind: child.kind,
			};

			entries.push(entry);
		}

		Ok(entries)
	}

	/// Closes a virtual directory by discarding its handle.
	/// This should be called for every handle created by `VirtualFileTree::open_dir(path)`.
	/// This method will silently do nothing if the provided handle isn't valid.
	pub fn close_dir(&mut self, handle: u64) {
		self.handles.remove(&handle);
	}

	/// Returns *true* if the provided handle belongs to this filesystem.
	pub fn is_dir_open(&self, handle: u64) -> bool {
		self.handles.contains_key(&handle)
	}

	/// Returns *true* if the provided path is a directory.
	/// This will be *false* if the path isn't within the tree.
	pub fn is_dir(&self, path: impl AsRef<Path>) -> bool {
		let Some(idx) = self.find_index(path) else {
			return false;
		};

		self.graph[idx].kind == FileType::Directory
	}

	/// Retrieves file attributes to pass to FUSE.
	/// Generates fake attributes when given a directory.
	/// If given a normal file, it will fallback to the normal *lstat*.
	pub fn stat(&self, path: impl AsRef<Path>) -> fuse_mt::ResultEntry {
		let Some(idx) = self.find_index(path) else {
			return Err(libc::ENOENT);
		};

		match self.graph[idx].kind == FileType::Directory {
			true => Ok((TTL, generate_fake_attr())),
			false => libc_wrappers::lstat(self.graph[idx].path.as_os_str().to_os_string())
				.map(|s| (TTL, stat_to_fuse(s))),
		}
	}
	
	/// Like `VirtualFileTree::stat(path)` but for file handles.
	/// If the handle belongs to an open virtual directory, we generate fake attributes for it.
	/// If it isn't, we fall back to calling the real *fstat* on the handle.
	pub fn fstat(&self, handle: u64) -> fuse_mt::ResultEntry {
		match self.is_dir_open(handle) {
			true => Ok((TTL, generate_fake_attr())),
			false => libc_wrappers::fstat(handle).map(|s| (TTL, stat_to_fuse(s))),
		}
	}

	/// Adds or updates a child node.
	/// This method does not update the edge linking the parent and child.
	fn update_child(&mut self, parent: NodeIndex, weight: VirtualFileData) -> NodeIndex {
		let link = weight.path.file_name()
			.unwrap()
			.to_ascii_lowercase();

		let target = self.graph.edges(parent)
			.find(|e| e.weight() == &link)
			.map(|e| e.target());

		if let Some(old) = target {
			self.graph[old] = weight;
			old
		}

		else {
			let new = self.graph.add_node(weight);
			self.graph.add_edge(parent, new, link);
			new
		}
	}

	/// Removes all nodes that don't connect back to the tree's root.
	fn clear_orphans(&mut self) {
		let root = NodeIndex::new(0);
		
		self.graph.retain_nodes(|graph, node| has_path_connecting(&*graph, root, node, None));
	}
}
