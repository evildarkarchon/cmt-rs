//! Injectable clipboard adapter boundary.
//!
//! UI callbacks should not touch the host clipboard directly. This module keeps
//! clipboard writes behind a tiny fakeable trait and returns typed results with
//! safe user-facing text separated from adapter diagnostics.

use crate::platform::{PlatformError, PlatformErrorKind, PlatformOperation};

const CLIPBOARD_TARGET_LABEL: &str = "system clipboard";
const INVALID_CLIPBOARD_TEXT_MESSAGE: &str = "Clipboard text is invalid.";

/// Result value for a clipboard write request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardActionResult {
    /// Kind of action that was attempted.
    pub operation: PlatformOperation,
    /// Safe target label for diagnostics; this never contains copied text.
    pub target: String,
    /// Success or typed failure state.
    pub outcome: ClipboardActionOutcome,
    safe_message: String,
    diagnostic: Option<String>,
}

impl ClipboardActionResult {
    /// Creates a successful clipboard result.
    pub fn success(target: impl Into<String>) -> Self {
        Self {
            operation: PlatformOperation::CopyToClipboard,
            target: target.into(),
            outcome: ClipboardActionOutcome::Succeeded,
            safe_message: PlatformOperation::CopyToClipboard
                .success_message()
                .to_owned(),
            diagnostic: None,
        }
    }

    /// Creates a failed clipboard result from a platform error.
    pub fn failure(error: PlatformError) -> Self {
        Self {
            operation: error.operation,
            target: error.target.clone(),
            outcome: ClipboardActionOutcome::Failed(error.kind),
            safe_message: error.user_message().to_owned(),
            diagnostic: error.diagnostic().map(ToOwned::to_owned),
        }
    }

    /// Returns true when the clipboard write succeeded.
    pub const fn is_success(&self) -> bool {
        matches!(self.outcome, ClipboardActionOutcome::Succeeded)
    }

    /// Returns the typed failure kind when the clipboard write failed.
    pub const fn failure_kind(&self) -> Option<PlatformErrorKind> {
        match self.outcome {
            ClipboardActionOutcome::Succeeded => None,
            ClipboardActionOutcome::Failed(kind) => Some(kind),
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

/// Success or typed failure state for a clipboard action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipboardActionOutcome {
    /// The adapter successfully wrote the text to the clipboard.
    Succeeded,
    /// The adapter could not write the text and classified why.
    Failed(PlatformErrorKind),
}

/// Fakeable clipboard action boundary.
pub trait ClipboardActions {
    /// Copies text to the host clipboard.
    fn copy_text(&self, text: &str) -> ClipboardActionResult;
}

/// Production clipboard adapter backed by `arboard`.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealClipboardActions;

impl RealClipboardActions {
    /// Creates the production clipboard adapter without touching the OS clipboard.
    pub const fn new() -> Self {
        Self
    }
}

impl ClipboardActions for RealClipboardActions {
    fn copy_text(&self, text: &str) -> ClipboardActionResult {
        copy_real_text(text)
    }
}

fn copy_real_text(text: &str) -> ClipboardActionResult {
    if text.is_empty() {
        return ClipboardActionResult::failure(
            PlatformError::new(
                PlatformOperation::CopyToClipboard,
                CLIPBOARD_TARGET_LABEL,
                PlatformErrorKind::InvalidInput,
                INVALID_CLIPBOARD_TEXT_MESSAGE,
            )
            .with_diagnostic("empty clipboard text"),
        );
    }

    let mut clipboard = match arboard::Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(error) => {
            return ClipboardActionResult::failure(PlatformError::command_failed(
                PlatformOperation::CopyToClipboard,
                CLIPBOARD_TARGET_LABEL,
                error.to_string(),
            ));
        }
    };

    match clipboard.set_text(text.to_owned()) {
        Ok(()) => ClipboardActionResult::success(CLIPBOARD_TARGET_LABEL),
        Err(error) => ClipboardActionResult::failure(PlatformError::command_failed(
            PlatformOperation::CopyToClipboard,
            CLIPBOARD_TARGET_LABEL,
            error.to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s05_actions_clipboard_result_keeps_diagnostics_out_of_safe_text() {
        let result = ClipboardActionResult::failure(PlatformError::command_failed(
            PlatformOperation::CopyToClipboard,
            CLIPBOARD_TARGET_LABEL,
            "raw clipboard adapter detail",
        ));

        assert!(!result.is_success());
        assert_eq!(result.operation, PlatformOperation::CopyToClipboard);
        assert_eq!(result.target, CLIPBOARD_TARGET_LABEL);
        assert_eq!(
            result.failure_kind(),
            Some(PlatformErrorKind::CommandFailed)
        );
        assert_eq!(result.safe_message(), "Clipboard copy failed.");
        assert_eq!(result.diagnostic(), Some("raw clipboard adapter detail"));
        assert!(!result.safe_message().contains("raw clipboard"));
    }

    #[test]
    fn s05_actions_real_clipboard_rejects_empty_text_before_os_access() {
        let result = RealClipboardActions::new().copy_text("");

        assert_eq!(result.failure_kind(), Some(PlatformErrorKind::InvalidInput));
        assert_eq!(result.safe_message(), INVALID_CLIPBOARD_TEXT_MESSAGE);
        assert_eq!(result.diagnostic(), Some("empty clipboard text"));
    }
}
