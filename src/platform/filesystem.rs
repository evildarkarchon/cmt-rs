//! Fakeable filesystem adapter contracts for discovery and scan code.
//!
//! The real adapter performs ordinary filesystem reads and deterministic
//! directory traversal. Callers should depend on [`Filesystem`] so tests can use
//! in-memory fakes instead of a real Fallout 4 installation tree.

use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
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
        if file_type.is_symlink() {
            Self::Symlink
        } else if file_type.is_file() {
            Self::File
        } else if file_type.is_dir() {
            Self::Directory
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
    /// Whether the path itself is a platform reparse point such as a Windows junction.
    pub is_reparse_point: bool,
}

impl FileMetadata {
    /// Creates ordinary metadata for a non-reparse path.
    pub const fn new(file_type: FileType, len: u64) -> Self {
        Self {
            file_type,
            len,
            is_reparse_point: false,
        }
    }

    /// Creates metadata for a path the platform reports as a reparse point.
    pub const fn reparse_point(file_type: FileType, len: u64) -> Self {
        Self {
            file_type,
            len,
            is_reparse_point: true,
        }
    }

    /// Returns true when this metadata describes a regular file.
    pub const fn is_file(&self) -> bool {
        matches!(self.file_type, FileType::File)
    }

    /// Returns true when this metadata describes a directory.
    pub const fn is_dir(&self) -> bool {
        matches!(self.file_type, FileType::Directory)
    }

    /// Returns true when a no-follow metadata read found a link or reparse point.
    pub const fn is_symlink_or_reparse_point(&self) -> bool {
        matches!(self.file_type, FileType::Symlink) || self.is_reparse_point
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

    /// Reads metadata for a path without following the final symbolic link or reparse point.
    ///
    /// The default delegates to [`Filesystem::metadata`] so existing fake adapters remain small;
    /// real adapters should override this when the platform can expose no-follow metadata.
    fn symlink_metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
        self.metadata(path)
    }

    /// Resolves a path through the platform canonicalization API.
    ///
    /// The default returns the input path unchanged for in-memory fakes. Real adapters should
    /// override this to collapse links, junctions, and relative components before containment
    /// checks that guard destructive mutation.
    fn canonicalize_path(&self, path: &Path) -> PlatformResult<PathBuf> {
        Ok(path.to_path_buf())
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

    /// Writes `bytes` at `offset` without replacing or reading the whole file.
    ///
    /// Confirmed patchers use this for small fixed header fields in very large files. Callers
    /// remain responsible for validating the target path, file type, and byte range before write.
    fn write_byte_range(&self, path: &Path, offset: u64, bytes: &[u8]) -> PlatformResult<()>;

    /// Replaces a file with bytes written to a temporary file in the same directory.
    ///
    /// Implementations should keep the active destination intact until the replacement bytes have
    /// been fully written and are ready for a same-directory rename/replace operation.
    fn replace_file_bytes(&self, path: &Path, bytes: &[u8]) -> PlatformResult<()>;

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
            .map(file_metadata_from_std)
            .map_err(|error| {
                PlatformError::from_io(
                    PlatformOperation::ReadMetadata,
                    path.display().to_string(),
                    &error,
                )
            })
    }

    fn symlink_metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
        fs::symlink_metadata(path)
            .map(file_metadata_from_std)
            .map_err(|error| {
                PlatformError::from_io(
                    PlatformOperation::ReadMetadata,
                    path.display().to_string(),
                    &error,
                )
            })
    }

    fn canonicalize_path(&self, path: &Path) -> PlatformResult<PathBuf> {
        fs::canonicalize(path).map_err(|error| {
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
        Read::by_ref(&mut file)
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
            let file_type = if file_type.is_symlink() {
                FileType::Symlink
            } else if file_type.is_file() {
                FileType::File
            } else if file_type.is_dir() {
                FileType::Directory
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

    fn write_byte_range(&self, path: &Path, offset: u64, bytes: &[u8]) -> PlatformResult<()> {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|error| {
                PlatformError::from_io(
                    PlatformOperation::WriteFile,
                    path.display().to_string(),
                    &error,
                )
            })?;
        file.seek(SeekFrom::Start(offset)).map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::WriteFile,
                path.display().to_string(),
                &error,
            )
        })?;
        file.write_all(bytes).map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::WriteFile,
                path.display().to_string(),
                &error,
            )
        })?;
        file.sync_all().map_err(|error| {
            PlatformError::from_io(
                PlatformOperation::WriteFile,
                path.display().to_string(),
                &error,
            )
        })
    }

    fn replace_file_bytes(&self, path: &Path, bytes: &[u8]) -> PlatformResult<()> {
        static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

        let parent = path.parent().ok_or_else(|| {
            PlatformError::new(
                PlatformOperation::WriteFile,
                path.display().to_string(),
                crate::platform::PlatformErrorKind::InvalidInput,
                "File write target is invalid.",
            )
        })?;
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                PlatformError::new(
                    PlatformOperation::WriteFile,
                    path.display().to_string(),
                    crate::platform::PlatformErrorKind::InvalidInput,
                    "File write target is invalid.",
                )
            })?;
        let temp_path = parent.join(format!(
            ".{file_name}.cmt-rs-{}-{}.tmp",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));

        let write_result = (|| -> PlatformResult<()> {
            let mut temp_file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)
                .map_err(|error| {
                    PlatformError::from_io(
                        PlatformOperation::WriteFile,
                        temp_path.display().to_string(),
                        &error,
                    )
                })?;
            temp_file.write_all(bytes).map_err(|error| {
                PlatformError::from_io(
                    PlatformOperation::WriteFile,
                    temp_path.display().to_string(),
                    &error,
                )
            })?;
            temp_file.sync_all().map_err(|error| {
                PlatformError::from_io(
                    PlatformOperation::WriteFile,
                    temp_path.display().to_string(),
                    &error,
                )
            })?;
            fs::rename(&temp_path, path).map_err(|error| {
                PlatformError::from_io(
                    PlatformOperation::RenameFile,
                    format!("{} -> {}", temp_path.display(), path.display()),
                    &error,
                )
            })?;
            Ok(())
        })();

        if write_result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        write_result
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

fn file_metadata_from_std(metadata: fs::Metadata) -> FileMetadata {
    let is_reparse_point = metadata_is_reparse_point(&metadata);
    FileMetadata {
        file_type: FileType::from_std(metadata.file_type()),
        len: metadata.len(),
        is_reparse_point,
    }
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
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
                    return Ok(FileMetadata::new(FileType::File, bytes.len() as u64));
                }
                FakeNode::Directory => FileType::Directory,
                FakeNode::Denied => unreachable!("denied nodes return before classification"),
            };
            Ok(FileMetadata::new(file_type, 0))
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

    fn unique_platform_test_file(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "cmt-rs-{name}-{}-{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos()
        ));
        path
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

    #[test]
    fn real_filesystem_write_byte_range_updates_only_requested_bytes() {
        let fs = RealFilesystem::new();
        let path = unique_platform_test_file("byte-range");
        fs.write_bytes(&path, b"BTDX\x08\0\0\0GNRLbody")
            .expect("test fixture should be written");

        fs.write_byte_range(&path, 4, &1_u32.to_le_bytes())
            .expect("byte range write should succeed");
        let bytes = fs
            .read_bytes(&path)
            .expect("patched fixture should be readable");
        let _ = fs.remove_file(&path);

        assert_eq!(&bytes[..4], b"BTDX");
        assert_eq!(&bytes[4..8], &1_u32.to_le_bytes());
        assert_eq!(&bytes[8..12], b"GNRL");
        assert_eq!(&bytes[12..], b"body");
    }
}
