//! Injectable desktop open and external-tool launch adapters.
//!
//! UI callbacks should receive [`DesktopActionResult`] values from this module
//! instead of opening dialogs, silently logging failures, or directly launching
//! processes on the Slint event thread.

use std::path::Path;

use crate::platform::{PlatformError, PlatformErrorKind, PlatformOperation};

/// Result value for URL/path open and external-tool launch requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopActionResult {
    /// Kind of action that was attempted.
    pub operation: PlatformOperation,
    /// User-selected or adapter-supplied action target.
    pub target: String,
    /// Success or typed failure state.
    pub outcome: DesktopActionOutcome,
    safe_message: String,
    diagnostic: Option<String>,
}

impl DesktopActionResult {
    /// Creates a successful action result.
    pub fn success(operation: PlatformOperation, target: impl Into<String>) -> Self {
        Self {
            operation,
            target: target.into(),
            outcome: DesktopActionOutcome::Succeeded,
            safe_message: operation.success_message().to_owned(),
            diagnostic: None,
        }
    }

    /// Creates a failed action result from a platform error.
    pub fn failure(error: PlatformError) -> Self {
        Self {
            operation: error.operation,
            target: error.target.clone(),
            outcome: DesktopActionOutcome::Failed(error.kind),
            safe_message: error.user_message().to_owned(),
            diagnostic: error.diagnostic().map(ToOwned::to_owned),
        }
    }

    /// Returns true when the action was started successfully.
    pub const fn is_success(&self) -> bool {
        matches!(self.outcome, DesktopActionOutcome::Succeeded)
    }

    /// Returns the typed failure kind when the action failed.
    pub const fn failure_kind(&self) -> Option<PlatformErrorKind> {
        match self.outcome {
            DesktopActionOutcome::Succeeded => None,
            DesktopActionOutcome::Failed(kind) => Some(kind),
        }
    }

    /// Returns safe user-facing text for status surfaces.
    pub fn safe_message(&self) -> &str {
        &self.safe_message
    }

    /// Returns optional diagnostic detail for logs or tests, never UI text.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
}

/// Success or typed failure state for a desktop action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DesktopActionOutcome {
    /// The adapter successfully handed the action to the OS.
    Succeeded,
    /// The adapter could not perform the action and classified why.
    Failed(PlatformErrorKind),
}

/// Fakeable desktop action boundary.
pub trait DesktopActions {
    /// Opens a URL through the platform desktop handler.
    fn open_url(&self, url: &str) -> DesktopActionResult;

    /// Opens a file or folder through the platform desktop handler.
    fn open_path(&self, path: &Path) -> DesktopActionResult;

    /// Launches an executable tool with arguments.
    fn launch_tool(&self, executable: &Path, args: &[String]) -> DesktopActionResult;
}

/// Production desktop action adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealDesktopActions;

impl RealDesktopActions {
    /// Creates the production desktop action adapter without launching anything.
    pub const fn new() -> Self {
        Self
    }
}

impl DesktopActions for RealDesktopActions {
    fn open_url(&self, url: &str) -> DesktopActionResult {
        open_real_url(url)
    }

    fn open_path(&self, path: &Path) -> DesktopActionResult {
        open_real_path(path)
    }

    fn launch_tool(&self, executable: &Path, args: &[String]) -> DesktopActionResult {
        launch_real_tool(executable, args)
    }
}

#[cfg(not(windows))]
fn open_real_url(url: &str) -> DesktopActionResult {
    DesktopActionResult::failure(PlatformError::unsupported(
        PlatformOperation::OpenUrl,
        url.to_owned(),
    ))
}

#[cfg(not(windows))]
fn open_real_path(path: &Path) -> DesktopActionResult {
    DesktopActionResult::failure(PlatformError::unsupported(
        PlatformOperation::OpenPath,
        path.display().to_string(),
    ))
}

#[cfg(not(windows))]
fn launch_real_tool(executable: &Path, _args: &[String]) -> DesktopActionResult {
    DesktopActionResult::failure(PlatformError::unsupported(
        PlatformOperation::LaunchTool,
        executable.display().to_string(),
    ))
}

#[cfg(windows)]
fn open_real_url(url: &str) -> DesktopActionResult {
    hand_to_windows_shell(PlatformOperation::OpenUrl, url)
}

#[cfg(windows)]
fn open_real_path(path: &Path) -> DesktopActionResult {
    hand_to_windows_shell(PlatformOperation::OpenPath, &path.display().to_string())
}

#[cfg(windows)]
fn hand_to_windows_shell(operation: PlatformOperation, target: &str) -> DesktopActionResult {
    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        core::{HSTRING, w},
    };

    let target_hstring = HSTRING::from(target);
    let result =
        unsafe { ShellExecuteW(None, w!("open"), &target_hstring, None, None, SW_SHOWNORMAL) };

    if (result.0 as isize) > 32 {
        DesktopActionResult::success(operation, target.to_owned())
    } else {
        DesktopActionResult::failure(PlatformError::command_failed(
            operation,
            target.to_owned(),
            format!("ShellExecuteW failed with code {}", result.0 as isize),
        ))
    }
}

#[cfg(windows)]
fn launch_real_tool(executable: &Path, args: &[String]) -> DesktopActionResult {
    use std::process::Command;

    match Command::new(executable).args(args).spawn() {
        Ok(_) => DesktopActionResult::success(
            PlatformOperation::LaunchTool,
            executable.display().to_string(),
        ),
        Err(error) => DesktopActionResult::failure(PlatformError::from_io(
            PlatformOperation::LaunchTool,
            executable.display().to_string(),
            &error,
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[derive(Debug, Default)]
    struct FakeDesktopActions {
        failures: BTreeMap<(PlatformOperation, String), PlatformErrorKind>,
    }

    impl FakeDesktopActions {
        fn fail(
            mut self,
            operation: PlatformOperation,
            target: impl Into<String>,
            kind: PlatformErrorKind,
        ) -> Self {
            self.failures.insert((operation, target.into()), kind);
            self
        }

        fn run(&self, operation: PlatformOperation, target: String) -> DesktopActionResult {
            if let Some(kind) = self.failures.get(&(operation, target.clone())) {
                DesktopActionResult::failure(PlatformError::new(
                    operation,
                    target,
                    *kind,
                    format!("{} failed.", operation.label()),
                ))
            } else {
                DesktopActionResult::success(operation, target)
            }
        }
    }

    impl DesktopActions for FakeDesktopActions {
        fn open_url(&self, url: &str) -> DesktopActionResult {
            self.run(PlatformOperation::OpenUrl, url.to_owned())
        }

        fn open_path(&self, path: &Path) -> DesktopActionResult {
            self.run(PlatformOperation::OpenPath, path.display().to_string())
        }

        fn launch_tool(&self, executable: &Path, _args: &[String]) -> DesktopActionResult {
            self.run(
                PlatformOperation::LaunchTool,
                executable.display().to_string(),
            )
        }
    }

    #[test]
    fn fake_desktop_actions_return_typed_success_values() {
        let desktop = FakeDesktopActions::default();

        let result = desktop.open_url("https://example.invalid/tool");

        assert!(result.is_success());
        assert_eq!(result.operation, PlatformOperation::OpenUrl);
        assert_eq!(result.target, "https://example.invalid/tool");
        assert_eq!(result.safe_message(), "Opened URL.");
        assert_eq!(result.failure_kind(), None);
    }

    #[test]
    fn fake_desktop_actions_return_typed_failures_without_launching() {
        let desktop = FakeDesktopActions::default().fail(
            PlatformOperation::LaunchTool,
            r"C:\Tools\BSArch.exe",
            PlatformErrorKind::NotFound,
        );

        let result = desktop.launch_tool(Path::new(r"C:\Tools\BSArch.exe"), &[]);

        assert!(!result.is_success());
        assert_eq!(result.operation, PlatformOperation::LaunchTool);
        assert_eq!(result.failure_kind(), Some(PlatformErrorKind::NotFound));
        assert_eq!(result.safe_message(), "Tool launch failed.");
    }

    #[cfg(not(windows))]
    #[test]
    fn real_desktop_actions_are_explicitly_unsupported_off_windows() {
        let desktop = RealDesktopActions::new();

        let url_result = desktop.open_url("https://example.invalid");
        let path_result = desktop.open_path(Path::new("Data"));
        let launch_result = desktop.launch_tool(Path::new("xEdit.exe"), &[]);

        assert_eq!(
            url_result.failure_kind(),
            Some(PlatformErrorKind::UnsupportedPlatform)
        );
        assert_eq!(
            path_result.failure_kind(),
            Some(PlatformErrorKind::UnsupportedPlatform)
        );
        assert_eq!(
            launch_result.failure_kind(),
            Some(PlatformErrorKind::UnsupportedPlatform)
        );
        assert_eq!(
            url_result.safe_message(),
            "URL open is not supported on this platform."
        );
    }
}
