//! Fakeable filesystem adapter contracts for discovery and scan code.
//!
//! The real adapter performs ordinary filesystem reads and deterministic
//! directory traversal. Callers should depend on [`Filesystem`] so tests can use
//! in-memory fakes instead of a real Fallout 4 installation tree.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::platform::{PlatformError, PlatformOperation, PlatformResult};

/// File type information exposed by filesystem adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileType {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symbolic link.
    Symlink,
    /// Any platform-specific file type not otherwise classified.
    Other,
}

impl FileType {
    fn from_std(file_type: fs::FileType) -> Self {
        if file_type.is_file() {
            Self::File
        } else if file_type.is_dir() {
            Self::Directory
        } else if file_type.is_symlink() {
            Self::Symlink
        } else {
            Self::Other
        }
    }
}

/// Metadata returned for a single filesystem path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileMetadata {
    /// Classified file type.
    pub file_type: FileType,
    /// Byte length reported by the OS for files and other entries.
    pub len: u64,
}

impl FileMetadata {
    /// Returns true when this metadata describes a regular file.
    pub const fn is_file(&self) -> bool {
        matches!(self.file_type, FileType::File)
    }

    /// Returns true when this metadata describes a directory.
    pub const fn is_dir(&self) -> bool {
        matches!(self.file_type, FileType::Directory)
    }
}

/// Directory entry returned by direct and recursive enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DirectoryEntry {
    /// Full path to the entry as supplied by the adapter.
    pub path: PathBuf,
    /// Classified file type for the entry.
    pub file_type: FileType,
}

impl DirectoryEntry {
    /// Creates a typed directory entry.
    pub fn new(path: impl Into<PathBuf>, file_type: FileType) -> Self {
        Self {
            path: path.into(),
            file_type,
        }
    }
}

/// Filesystem operations needed by discovery, scan, parser, and patch code.
pub trait Filesystem {
    /// Reads metadata for a path.
    fn metadata(&self, path: &Path) -> PlatformResult<FileMetadata>;

    /// Returns true when a path exists, false for not-found, and errors for unsafe ambiguity.
    fn exists(&self, path: &Path) -> PlatformResult<bool> {
        match self.metadata(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind == crate::platform::PlatformErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Returns true when a path is a regular file.
    fn is_file(&self, path: &Path) -> PlatformResult<bool> {
        match self.metadata(path) {
            Ok(metadata) => Ok(metadata.is_file()),
            Err(error) if error.kind == crate::platform::PlatformErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Returns true when a path is a directory.
    fn is_dir(&self, path: &Path) -> PlatformResult<bool> {
        match self.metadata(path) {
            Ok(metadata) => Ok(metadata.is_dir()),
            Err(error) if error.kind == crate::platform::PlatformErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Reads a whole file as bytes.
    fn read_bytes(&self, path: &Path) -> PlatformResult<Vec<u8>>;

    /// Reads at most `max_len` bytes from the start of a file.
    ///
    /// Real adapters should override this to avoid full-file reads for parser
    /// probes. The default keeps existing fakes small while preserving the same
    /// error behavior as [`Filesystem::read_bytes`].
    fn read_prefix(&self, path: &Path, max_len: usize) -> PlatformResult<Vec<u8>> {
        let mut bytes = self.read_bytes(path)?;
        bytes.truncate(max_len);
        Ok(bytes)
    }

    /// Reads a whole UTF-8 text file.
    fn read_to_string(&self, path: &Path) -> PlatformResult<String>;

    /// Lists direct children of a directory in deterministic path order.
    fn read_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>>;

    /// Recursively lists a directory tree in deterministic path order.
    fn walk_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>>;
}

/// Filesystem mutation operations needed by confirmed patch/download workflows.
///
/// This trait is intentionally separate from [`Filesystem`] so read-only discovery,
/// scan, and preview-plan code cannot accidentally gain write capabilities.
pub trait WritableFilesystem {
    /// Writes a whole file from bytes.
    fn write_bytes(&self, path: &Path, bytes: &[u8]) -> PlatformResult<()>;

    /// Copies one file to another path.
    fn copy_file(&self, from: &Path, to: &Path) -> PlatformResult<()>;

    /// Renames or moves one file to another path.
    fn rename_file(&self, from: &Path, to: &Path) -> PlatformResult<()>;

    /// Removes a single file.
    fn remove_file(&self, path: &Path) -> PlatformResult<()>;
}

/// Production filesystem adapter backed by `std::fs` and deterministic `walkdir` traversal.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealFilesystem;

impl RealFilesystem {
    /// Creates the production filesystem adapter without touching the filesystem.
    pub const fn new() -> Self {
        Self
    }
}

impl Filesystem for RealFilesystem {
    fn metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
        fs::metadata(path)
            .map(|metadata| FileMetadata {
                file_type: FileType::from_std(metadata.file_type()),
                len: metadata.len(),
            })
            .map_err(|error| {
                PlatformError::from_io(
                    PlatformOperation::ReadMetadata,
                    path.display().to_string(),
                    &error,
                )
            })
    }

    fn read_bytes(&self, path: &Path) -> PlatformResult<Vec<u8>> {
        fs::read(path).map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::ReadFile,
                path.display().to_string(),
                &error,
            )
        })
    }

    fn read_prefix(&self, path: &Path, max_len: usize) -> PlatformResult<Vec<u8>> {
        let mut file = fs::File::open(path).map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::ReadFile,
                path.display().to_string(),
                &error,
            )
        })?;
        let mut bytes = Vec::with_capacity(max_len);
        file.by_ref()
            .take(max_len as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                PlatformError::from_io(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    &error,
                )
            })?;
        Ok(bytes)
    }

    fn read_to_string(&self, path: &Path) -> PlatformResult<String> {
        fs::read_to_string(path).map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::ReadFile,
                path.display().to_string(),
                &error,
            )
        })
    }

    fn read_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
        let mut entries = Vec::new();
        let iterator = fs::read_dir(path).map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::ReadDirectory,
                path.display().to_string(),
                &error,
            )
        })?;

        for entry in iterator {
            let entry = entry.map_err(|error| {
                PlatformError::from_io(
                    PlatformOperation::ReadDirectory,
                    path.display().to_string(),
                    &error,
                )
            })?;
            let entry_path = entry.path();
            let file_type = entry.file_type().map_err(|error| {
                PlatformError::from_io(
                    PlatformOperation::ReadMetadata,
                    entry_path.display().to_string(),
                    &error,
                )
            })?;
            entries.push(DirectoryEntry::new(
                entry_path,
                FileType::from_std(file_type),
            ));
        }

        entries.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(entries)
    }

    fn walk_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
        let mut entries = Vec::new();
        for entry in WalkDir::new(path).sort_by_file_name() {
            let entry = entry.map_err(|error| walkdir_error(path, error))?;
            let file_type = entry.file_type();
            let file_type = if file_type.is_file() {
                FileType::File
            } else if file_type.is_dir() {
                FileType::Directory
            } else if file_type.is_symlink() {
                FileType::Symlink
            } else {
                FileType::Other
            };
            entries.push(DirectoryEntry::new(entry.path().to_path_buf(), file_type));
        }
        Ok(entries)
    }
}

impl WritableFilesystem for RealFilesystem {
    fn write_bytes(&self, path: &Path, bytes: &[u8]) -> PlatformResult<()> {
        fs::write(path, bytes).map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::WriteFile,
                path.display().to_string(),
                &error,
            )
        })
    }

    fn copy_file(&self, from: &Path, to: &Path) -> PlatformResult<()> {
        fs::copy(from, to).map(|_| ()).map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::CopyFile,
                format!("{} -> {}", from.display(), to.display()),
                &error,
            )
        })
    }

    fn rename_file(&self, from: &Path, to: &Path) -> PlatformResult<()> {
        fs::rename(from, to).map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::RenameFile,
                format!("{} -> {}", from.display(), to.display()),
                &error,
            )
        })
    }

    fn remove_file(&self, path: &Path) -> PlatformResult<()> {
        fs::remove_file(path).map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::RemoveFile,
                path.display().to_string(),
                &error,
            )
        })
    }
}

fn walkdir_error(root: &Path, error: walkdir::Error) -> PlatformError {
    let target = error.path().unwrap_or(root).display().to_string();
    if let Some(io_error) = error.io_error() {
        PlatformError::from_io(PlatformOperation::WalkDirectory, target, io_error)
    } else {
        PlatformError::command_failed(PlatformOperation::WalkDirectory, target, error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::platform::{PlatformErrorKind, PlatformOperation};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FakeNode {
        File(Vec<u8>),
        Directory,
        Denied,
    }

    #[derive(Debug, Default)]
    struct FakeFilesystem {
        nodes: BTreeMap<PathBuf, FakeNode>,
    }

    impl FakeFilesystem {
        fn with_file(mut self, path: &str, bytes: impl Into<Vec<u8>>) -> Self {
            self.nodes
                .insert(PathBuf::from(path), FakeNode::File(bytes.into()));
            self
        }

        fn with_dir(mut self, path: &str) -> Self {
            self.nodes.insert(PathBuf::from(path), FakeNode::Directory);
            self
        }

        fn with_denied(mut self, path: &str) -> Self {
            self.nodes.insert(PathBuf::from(path), FakeNode::Denied);
            self
        }

        fn node(&self, path: &Path, operation: PlatformOperation) -> PlatformResult<&FakeNode> {
            match self.nodes.get(path) {
                Some(FakeNode::Denied) => Err(PlatformError::new(
                    operation,
                    path.display().to_string(),
                    PlatformErrorKind::PermissionDenied,
                    format!(
                        "{} target could not be accessed because permission was denied.",
                        operation.label()
                    ),
                )),
                Some(node) => Ok(node),
                None => Err(PlatformError::new(
                    operation,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    format!("{} target was not found.", operation.label()),
                )),
            }
        }
    }

    impl Filesystem for FakeFilesystem {
        fn metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
            let node = self.node(path, PlatformOperation::ReadMetadata)?;
            let file_type = match node {
                FakeNode::File(bytes) => {
                    return Ok(FileMetadata {
                        file_type: FileType::File,
                        len: bytes.len() as u64,
                    });
                }
                FakeNode::Directory => FileType::Directory,
                FakeNode::Denied => unreachable!("denied nodes return before classification"),
            };
            Ok(FileMetadata { file_type, len: 0 })
        }

        fn read_bytes(&self, path: &Path) -> PlatformResult<Vec<u8>> {
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes) => Ok(bytes.clone()),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
                FakeNode::Denied => unreachable!("denied nodes return before reads"),
            }
        }

        fn read_to_string(&self, path: &Path) -> PlatformResult<String> {
            String::from_utf8(self.read_bytes(path)?).map_err(|error| {
                PlatformError::parse_error(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    error.to_string(),
                )
            })
        }

        fn read_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            self.node(path, PlatformOperation::ReadDirectory)?;
            let mut entries = Vec::new();
            for (candidate, node) in &self.nodes {
                if candidate.parent() == Some(path) {
                    let file_type = match node {
                        FakeNode::File(_) => FileType::File,
                        FakeNode::Directory => FileType::Directory,
                        FakeNode::Denied => FileType::Other,
                    };
                    entries.push(DirectoryEntry::new(candidate.clone(), file_type));
                }
            }
            Ok(entries)
        }

        fn walk_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            self.node(path, PlatformOperation::WalkDirectory)?;
            let mut entries = Vec::new();
            for (candidate, node) in &self.nodes {
                if candidate == path || candidate.starts_with(path) {
                    let file_type = match node {
                        FakeNode::File(_) => FileType::File,
                        FakeNode::Directory => FileType::Directory,
                        FakeNode::Denied => FileType::Other,
                    };
                    entries.push(DirectoryEntry::new(candidate.clone(), file_type));
                }
            }
            Ok(entries)
        }
    }

    fn collect_files(fs: &dyn Filesystem, root: &Path) -> PlatformResult<BTreeSet<PathBuf>> {
        Ok(fs
            .walk_dir(root)?
            .into_iter()
            .filter(|entry| entry.file_type == FileType::File)
            .map(|entry| entry.path)
            .collect())
    }

    #[test]
    fn fake_filesystem_drives_scan_style_directory_inputs() {
        let fs = FakeFilesystem::default()
            .with_dir("Data")
            .with_file("Data/Fallout4.esm", b"plugin".to_vec())
            .with_file("Data/Fallout4 - Main.ba2", b"archive".to_vec())
            .with_dir("Data/F4SE");

        let files = collect_files(&fs, Path::new("Data")).expect("fake traversal should work");

        assert_eq!(
            files,
            BTreeSet::from([
                PathBuf::from("Data/Fallout4 - Main.ba2"),
                PathBuf::from("Data/Fallout4.esm"),
            ])
        );
        assert!(fs.is_dir(Path::new("Data")).expect("fake dir metadata"));
        assert!(
            fs.is_file(Path::new("Data/Fallout4.esm"))
                .expect("fake file metadata")
        );
        assert_eq!(
            fs.read_to_string(Path::new("Data/Fallout4.esm"))
                .expect("fake text read"),
            "plugin"
        );
    }

    #[test]
    fn fake_filesystem_surfaces_typed_failure_without_real_paths() {
        let fs = FakeFilesystem::default()
            .with_dir("Data")
            .with_denied("Data/Private.ba2");

        let error = fs
            .metadata(Path::new("Data/Private.ba2"))
            .expect_err("denied fake path should fail");

        assert_eq!(error.operation, PlatformOperation::ReadMetadata);
        assert_eq!(error.kind, PlatformErrorKind::PermissionDenied);
        assert_eq!(
            error.user_message(),
            "Filesystem metadata read target could not be accessed because permission was denied."
        );
    }

    #[test]
    fn real_filesystem_missing_file_maps_to_typed_not_found() {
        let fs = RealFilesystem::new();
        let missing = Path::new("definitely-not-a-cmt-rs-platform-test-file.nope");

        let error = fs
            .read_bytes(missing)
            .expect_err("missing file should produce typed failure");

        assert_eq!(error.kind, PlatformErrorKind::NotFound);
        assert_eq!(error.operation, PlatformOperation::ReadFile);
        assert_eq!(error.user_message(), "File read target was not found.");
    }
}
