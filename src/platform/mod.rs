//! Platform adapter boundary for operating-system integrations.
//!
//! Discovery, scans, tool launching, and later UI actions should depend on the
//! traits in this module rather than touching the real filesystem, registry,
//! process table, or desktop handlers directly. Real adapters return explicit
//! typed failures and tests can provide fake implementations without requiring a
//! Fallout 4 install, Windows registry state, running mod manager, or visible
//! desktop handler.

use std::{fmt, io};

pub mod clipboard;
pub mod desktop;
pub mod filesystem;
pub mod process;
pub mod registry;
pub mod settings_store;

/// Result type returned by platform adapters.
pub type PlatformResult<T> = Result<T, PlatformError>;

/// OS-facing operation that can fail in a typed, user-safe way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlatformOperation {
    /// Read filesystem metadata for a path.
    ReadMetadata,
    /// Read bytes or text from a file.
    ReadFile,
    /// Enumerate direct children of a directory.
    ReadDirectory,
    /// Recursively traverse a directory tree.
    WalkDirectory,
    /// Write bytes to a file.
    WriteFile,
    /// Copy one file to another path.
    CopyFile,
    /// Rename or move one file to another path.
    RenameFile,
    /// Remove a file from the filesystem.
    RemoveFile,
    /// Query a string value from the platform registry.
    ReadRegistry,
    /// Inspect the process table.
    ListProcesses,
    /// Read executable or DLL version metadata.
    ReadVersionMetadata,
    /// Read PC specs or operating-system metadata.
    ReadSystemMetadata,
    /// Open a URL through the desktop handler.
    OpenUrl,
    /// Open a file or folder through the desktop handler.
    OpenPath,
    /// Copy static text to the host clipboard.
    CopyToClipboard,
    /// Launch an external tool executable.
    LaunchTool,
}

impl PlatformOperation {
    /// Returns a concise user-safe label for this operation.
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadMetadata => "Filesystem metadata read",
            Self::ReadFile => "File read",
            Self::ReadDirectory => "Directory read",
            Self::WalkDirectory => "Directory traversal",
            Self::WriteFile => "File write",
            Self::CopyFile => "File copy",
            Self::RenameFile => "File rename",
            Self::RemoveFile => "File removal",
            Self::ReadRegistry => "Registry access",
            Self::ListProcesses => "Process inspection",
            Self::ReadVersionMetadata => "Version metadata read",
            Self::ReadSystemMetadata => "System metadata read",
            Self::OpenUrl => "URL open",
            Self::OpenPath => "Path open",
            Self::CopyToClipboard => "Clipboard copy",
            Self::LaunchTool => "Tool launch",
        }
    }

    /// Returns the success message used by desktop/action adapters.
    pub const fn success_message(self) -> &'static str {
        match self {
            Self::OpenUrl => "Opened URL.",
            Self::OpenPath => "Opened path.",
            Self::CopyToClipboard => "Copied to clipboard.",
            Self::LaunchTool => "Launched tool.",
            Self::WriteFile
            | Self::CopyFile
            | Self::RenameFile
            | Self::RemoveFile
            | Self::ReadMetadata
            | Self::ReadFile
            | Self::ReadDirectory
            | Self::WalkDirectory
            | Self::ReadRegistry
            | Self::ListProcesses
            | Self::ReadVersionMetadata
            | Self::ReadSystemMetadata => "Operation completed.",
        }
    }
}

impl fmt::Display for PlatformOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Typed adapter failure categories safe for callers to branch on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlatformErrorKind {
    /// Operation is intentionally unavailable on the current platform.
    UnsupportedPlatform,
    /// Requested target was not found.
    NotFound,
    /// The OS denied access to the requested target.
    PermissionDenied,
    /// Caller supplied a malformed target or unsupported input value.
    InvalidInput,
    /// A child process or system command failed.
    CommandFailed,
    /// Adapter output could not be parsed into the typed contract.
    ParseError,
    /// Generic IO failure not covered by a more specific kind.
    Io,
}

impl PlatformErrorKind {
    /// Maps a standard IO error into the nearest public platform failure kind.
    pub fn from_io_error(error: &io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::InvalidInput | io::ErrorKind::InvalidData => Self::InvalidInput,
            _ => Self::Io,
        }
    }
}

/// Typed platform failure with separated safe user text and diagnostic detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformError {
    /// Operation that failed.
    pub operation: PlatformOperation,
    /// Adapter-supplied target, such as a path, URL, process query, or registry value.
    pub target: String,
    /// Typed failure category.
    pub kind: PlatformErrorKind,
    user_message: String,
    diagnostic: Option<String>,
}

impl PlatformError {
    /// Creates a platform error from already-classified data.
    pub fn new(
        operation: PlatformOperation,
        target: impl Into<String>,
        kind: PlatformErrorKind,
        user_message: impl Into<String>,
    ) -> Self {
        Self {
            operation,
            target: target.into(),
            kind,
            user_message: user_message.into(),
            diagnostic: None,
        }
    }

    /// Creates an unsupported-platform error for a real adapter operation.
    pub fn unsupported(operation: PlatformOperation, target: impl Into<String>) -> Self {
        Self::new(
            operation,
            target,
            PlatformErrorKind::UnsupportedPlatform,
            format!("{} is not supported on this platform.", operation.label()),
        )
    }

    /// Creates a platform error from an IO failure without exposing raw OS text to users.
    pub fn from_io(
        operation: PlatformOperation,
        target: impl Into<String>,
        error: &io::Error,
    ) -> Self {
        let kind = PlatformErrorKind::from_io_error(error);
        let user_message = match kind {
            PlatformErrorKind::NotFound => format!("{} target was not found.", operation.label()),
            PlatformErrorKind::PermissionDenied => {
                format!(
                    "{} target could not be accessed because permission was denied.",
                    operation.label()
                )
            }
            PlatformErrorKind::InvalidInput => format!("{} target is invalid.", operation.label()),
            PlatformErrorKind::UnsupportedPlatform
            | PlatformErrorKind::CommandFailed
            | PlatformErrorKind::ParseError
            | PlatformErrorKind::Io => format!("{} failed.", operation.label()),
        };

        Self::new(operation, target, kind, user_message).with_diagnostic(error.to_string())
    }

    /// Creates an adapter command failure with diagnostic detail kept separate from user text.
    pub fn command_failed(
        operation: PlatformOperation,
        target: impl Into<String>,
        diagnostic: impl Into<String>,
    ) -> Self {
        Self::new(
            operation,
            target,
            PlatformErrorKind::CommandFailed,
            format!("{} failed.", operation.label()),
        )
        .with_diagnostic(diagnostic)
    }

    /// Creates a parse failure with diagnostic detail kept separate from user text.
    pub fn parse_error(
        operation: PlatformOperation,
        target: impl Into<String>,
        diagnostic: impl Into<String>,
    ) -> Self {
        Self::new(
            operation,
            target,
            PlatformErrorKind::ParseError,
            format!(
                "{} returned data that could not be understood.",
                operation.label()
            ),
        )
        .with_diagnostic(diagnostic)
    }

    /// Adds non-user-facing diagnostic text for logs or tests.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }

    /// Returns safe user-facing text for this failure.
    pub fn user_message(&self) -> &str {
        &self.user_message
    }

    /// Returns diagnostic detail suitable for logs, never for modal/dialog text.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
}

impl fmt::Display for PlatformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ({:?} during {})",
            self.user_message, self.kind, self.operation
        )
    }
}

impl std::error::Error for PlatformError {}

/// Platform services marker for dependency injection boundaries.
///
/// Constructing this marker does not read paths, query the registry, inspect the
/// environment, launch processes, or disclose filesystem state.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlatformServices;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_error_keeps_user_message_separate_from_diagnostics() {
        let error = PlatformError::command_failed(
            PlatformOperation::LaunchTool,
            "C:/Tools/Some Tool.exe",
            "raw OS error containing implementation detail",
        );

        assert_eq!(error.kind, PlatformErrorKind::CommandFailed);
        assert_eq!(error.user_message(), "Tool launch failed.");
        assert_eq!(
            error.diagnostic(),
            Some("raw OS error containing implementation detail")
        );
        assert!(!error.user_message().contains("raw OS error"));
    }

    #[test]
    fn platform_adapter_types_are_publicly_importable() {
        fn assert_type<T>() {}

        assert_type::<crate::platform::filesystem::RealFilesystem>();
        assert_type::<crate::platform::filesystem::DirectoryEntry>();
        assert_type::<crate::platform::registry::RealRegistry>();
        assert_type::<crate::platform::registry::RegistryHive>();
        assert_type::<crate::platform::process::RealProcessInspector>();
        assert_type::<crate::platform::process::ProcessInfo>();
        assert_type::<crate::platform::desktop::RealDesktopActions>();
        assert_type::<crate::platform::desktop::DesktopActionResult>();
        assert_type::<crate::platform::clipboard::RealClipboardActions>();
        assert_type::<crate::platform::clipboard::ClipboardActionResult>();
    }
}
