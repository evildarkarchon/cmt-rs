//! Tools/About action orchestration over fakeable platform adapters.
//!
//! Slint callback strings are untrusted input. This service parses them against
//! the inert reference contracts in [`crate::domain::tools`] before handing any
//! static URL or copied text to platform adapters.

use crate::{
    domain::tools::{
        ABOUT_LINKS, AboutActionId, AboutLink, AboutLinkId, TOOL_GROUPS, ToolActionId, ToolEntry,
    },
    platform::{
        PlatformErrorKind, PlatformOperation,
        clipboard::{ClipboardActionResult, ClipboardActions},
        desktop::{DesktopActionResult, DesktopActions},
    },
};

/// UI surface that requested a static Tools/About action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionSurface {
    /// The Tools tab requested the action.
    Tools,
    /// The About tab requested the action.
    About,
}

impl ActionSurface {
    /// Returns a stable label suitable for structured logs and tests.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Tools => "tools",
            Self::About => "about",
        }
    }
}

/// Parsed Tools-tab action identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolsActionKind {
    /// An enabled in-app utility entry such as the Downgrade Manager modal.
    InternalUtility(ToolActionId),
    /// An enabled static external-link entry.
    ExternalLink(ToolActionId),
    /// A known utility entry that is intentionally disabled/deferred.
    DeferredUtility(ToolActionId),
}

/// Parsed About-tab action identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AboutActionKind {
    /// Open a known About link.
    Open {
        /// Link row being opened.
        link_id: AboutLinkId,
        /// Stable callback id for the open button.
        action_id: AboutActionId,
    },
    /// Copy a known About link.
    Copy {
        /// Link row being copied.
        link_id: AboutLinkId,
        /// Stable callback id for the copy button.
        action_id: AboutActionId,
    },
}

impl AboutActionKind {
    /// Returns the link row associated with this action.
    pub const fn link_id(self) -> AboutLinkId {
        match self {
            Self::Open { link_id, .. } | Self::Copy { link_id, .. } => link_id,
        }
    }

    /// Returns the stable callback action id associated with this action.
    pub const fn action_id(self) -> AboutActionId {
        match self {
            Self::Open { action_id, .. } | Self::Copy { action_id, .. } => action_id,
        }
    }

    /// Returns true when this action is a copy-button request.
    pub const fn is_copy(self) -> bool {
        matches!(self, Self::Copy { .. })
    }
}

/// Failure metadata for a platform adapter call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionPlatformFailure {
    /// Platform operation that failed.
    pub operation: PlatformOperation,
    /// Typed failure category reported by the adapter.
    pub kind: PlatformErrorKind,
}

/// Fail-closed rejection category before a platform adapter is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionRejectionKind {
    /// Callback id did not match any known Tools/About action.
    UnknownAction,
    /// Callback id matched a known utility intentionally disabled in this slice.
    DisabledUtility,
    /// Callback id matched an action with no external target.
    InternalUtility,
    /// Known action had an invalid static input.
    InvalidInput,
    /// The background worker boundary could not accept or finish the action.
    WorkerUnavailable,
}

/// Safe action outcome returned by the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionOutcome {
    /// The platform adapter accepted the request.
    Succeeded,
    /// The request was rejected before touching an adapter.
    Rejected(ActionRejectionKind),
    /// The platform adapter rejected the request.
    Failed(ActionPlatformFailure),
}

impl ActionOutcome {
    /// Returns true when the action completed successfully.
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Succeeded)
    }
}

/// Tools-tab action feedback with safe UI text and diagnostic-only details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolsActionFeedback {
    /// Surface that produced this feedback.
    pub surface: ActionSurface,
    /// Stable action id supplied by the callback, or raw unknown id for diagnostics.
    pub action_id: String,
    /// Parsed known action identity when available.
    pub action: Option<ToolsActionKind>,
    /// Safe success/failure/rejection outcome.
    pub outcome: ActionOutcome,
    /// User-safe text for banners or disabled utility status labels.
    pub safe_message: String,
    /// Optional diagnostic detail for logs/tests, never UI text.
    pub diagnostic: Option<String>,
}

impl ToolsActionFeedback {
    /// Creates a successful Tools feedback value.
    pub fn succeeded(
        action_id: impl Into<String>,
        action: ToolsActionKind,
        safe_message: impl Into<String>,
    ) -> Self {
        Self {
            surface: ActionSurface::Tools,
            action_id: action_id.into(),
            action: Some(action),
            outcome: ActionOutcome::Succeeded,
            safe_message: safe_message.into(),
            diagnostic: None,
        }
    }

    /// Creates a fail-closed Tools rejection.
    pub fn rejected(
        action_id: impl Into<String>,
        action: Option<ToolsActionKind>,
        kind: ActionRejectionKind,
        safe_message: impl Into<String>,
        diagnostic: impl Into<Option<String>>,
    ) -> Self {
        Self {
            surface: ActionSurface::Tools,
            action_id: action_id.into(),
            action,
            outcome: ActionOutcome::Rejected(kind),
            safe_message: safe_message.into(),
            diagnostic: diagnostic.into(),
        }
    }

    /// Creates a failed Tools feedback value from a platform result.
    pub fn failed(
        action_id: impl Into<String>,
        action: ToolsActionKind,
        operation: PlatformOperation,
        kind: PlatformErrorKind,
        safe_message: impl Into<String>,
        diagnostic: impl Into<Option<String>>,
    ) -> Self {
        Self {
            surface: ActionSurface::Tools,
            action_id: action_id.into(),
            action: Some(action),
            outcome: ActionOutcome::Failed(ActionPlatformFailure { operation, kind }),
            safe_message: safe_message.into(),
            diagnostic: diagnostic.into(),
        }
    }

    /// Returns true when the action completed successfully.
    pub const fn is_success(&self) -> bool {
        self.outcome.is_success()
    }

    /// Returns safe user-facing text for status surfaces.
    pub fn safe_message(&self) -> &str {
        &self.safe_message
    }

    /// Returns diagnostic detail for logs/tests only.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
}

/// About-tab action feedback with safe UI text and diagnostic-only details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutActionFeedback {
    /// Surface that produced this feedback.
    pub surface: ActionSurface,
    /// Stable action id supplied by the callback, or raw unknown id for diagnostics.
    pub action_id: String,
    /// Parsed known action identity when available.
    pub action: Option<AboutActionKind>,
    /// Safe success/failure/rejection outcome.
    pub outcome: ActionOutcome,
    /// User-safe text for banners or status labels.
    pub safe_message: String,
    /// Optional diagnostic detail for logs/tests, never UI text.
    pub diagnostic: Option<String>,
}

impl AboutActionFeedback {
    /// Creates a successful About feedback value.
    pub fn succeeded(
        action_id: impl Into<String>,
        action: AboutActionKind,
        safe_message: impl Into<String>,
    ) -> Self {
        Self {
            surface: ActionSurface::About,
            action_id: action_id.into(),
            action: Some(action),
            outcome: ActionOutcome::Succeeded,
            safe_message: safe_message.into(),
            diagnostic: None,
        }
    }

    /// Creates a fail-closed About rejection.
    pub fn rejected(
        action_id: impl Into<String>,
        action: Option<AboutActionKind>,
        kind: ActionRejectionKind,
        safe_message: impl Into<String>,
        diagnostic: impl Into<Option<String>>,
    ) -> Self {
        Self {
            surface: ActionSurface::About,
            action_id: action_id.into(),
            action,
            outcome: ActionOutcome::Rejected(kind),
            safe_message: safe_message.into(),
            diagnostic: diagnostic.into(),
        }
    }

    /// Creates a failed About feedback value from a platform result.
    pub fn failed(
        action_id: impl Into<String>,
        action: AboutActionKind,
        operation: PlatformOperation,
        kind: PlatformErrorKind,
        safe_message: impl Into<String>,
        diagnostic: impl Into<Option<String>>,
    ) -> Self {
        Self {
            surface: ActionSurface::About,
            action_id: action_id.into(),
            action: Some(action),
            outcome: ActionOutcome::Failed(ActionPlatformFailure { operation, kind }),
            safe_message: safe_message.into(),
            diagnostic: diagnostic.into(),
        }
    }

    /// Returns true when the action completed successfully.
    pub const fn is_success(&self) -> bool {
        self.outcome.is_success()
    }

    /// Returns safe user-facing text for status surfaces.
    pub fn safe_message(&self) -> &str {
        &self.safe_message
    }

    /// Returns diagnostic detail for logs/tests only.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
}

/// Parses a Tools-tab callback id without touching platform adapters.
///
/// Unknown ids and disabled utilities return the same safe feedback the full
/// service would have produced, allowing UI callbacks to fail closed before
/// scheduling background work. Enabled internal utilities are returned so the UI
/// layer can route them to in-app workflow windows without desktop handoff.
pub fn tools_action_for_id(action_id: &str) -> Result<ToolsActionKind, ToolsActionFeedback> {
    let Some(entry) = find_tool_entry(action_id) else {
        tracing::warn!(
            event = "s05-tools-action-rejected",
            surface = ActionSurface::Tools.label(),
            action_id,
            reason = "unknown-action",
            "Tools action rejected because the id is unknown"
        );
        return Err(ToolsActionFeedback::rejected(
            action_id,
            None,
            ActionRejectionKind::UnknownAction,
            "Tools action is not available.",
            Some(format!("unknown Tools action id: {action_id}")),
        ));
    };

    if let Some(utility) = entry.deferred_utility() {
        tracing::warn!(
            event = "s05-tools-action-rejected",
            surface = ActionSurface::Tools.label(),
            action_id = entry.id.as_str(),
            utility_key = utility.key,
            reason = "disabled-utility",
            "Tools utility action rejected because it is deferred"
        );
        return Err(ToolsActionFeedback::rejected(
            entry.id.as_str(),
            Some(ToolsActionKind::DeferredUtility(entry.id)),
            ActionRejectionKind::DisabledUtility,
            utility.status_text,
            Some(format!("deferred Tools utility: {}", utility.key)),
        ));
    }

    if let Some(utility) = entry.internal_utility() {
        tracing::info!(
            event = "s09-tools-internal-utility-routed",
            surface = ActionSurface::Tools.label(),
            action_id = entry.id.as_str(),
            utility_key = utility.key,
            "Tools internal utility action parsed for in-app routing"
        );
        return Ok(ToolsActionKind::InternalUtility(entry.id));
    }

    if entry.external_link().is_some() {
        Ok(ToolsActionKind::ExternalLink(entry.id))
    } else {
        tracing::warn!(
            event = "s05-tools-action-rejected",
            surface = ActionSurface::Tools.label(),
            action_id = entry.id.as_str(),
            reason = "internal-utility",
            "Tools action rejected because it has no external target"
        );
        Err(ToolsActionFeedback::rejected(
            entry.id.as_str(),
            Some(ToolsActionKind::DeferredUtility(entry.id)),
            ActionRejectionKind::InternalUtility,
            "Tools action does not have an external target.",
            Some("known Tools action has no external link".to_owned()),
        ))
    }
}

/// Parses an About-tab callback id without touching desktop or clipboard adapters.
pub fn about_action_for_id(action_id: &str) -> Result<AboutActionKind, AboutActionFeedback> {
    let Some((_link, action)) = find_about_action(action_id) else {
        tracing::warn!(
            event = "s05-about-action-rejected",
            surface = ActionSurface::About.label(),
            action_id,
            reason = "unknown-action",
            "About action rejected because the id is unknown"
        );
        return Err(AboutActionFeedback::rejected(
            action_id,
            None,
            ActionRejectionKind::UnknownAction,
            "About action is not available.",
            Some(format!("unknown About action id: {action_id}")),
        ));
    };

    Ok(action)
}

/// Executes known Tools/About action ids through injected platform adapters.
#[derive(Debug, Clone)]
pub struct ToolsActionService<D, C> {
    desktop: D,
    clipboard: C,
}

impl<D, C> ToolsActionService<D, C> {
    /// Creates a service with injected desktop and clipboard adapters.
    pub fn new(desktop: D, clipboard: C) -> Self {
        Self { desktop, clipboard }
    }
}

impl<D: DesktopActions, C: ClipboardActions> ToolsActionService<D, C> {
    /// Executes a Tools-tab callback id after parsing it against known reference entries.
    pub fn execute_tools_action(&self, action_id: &str) -> ToolsActionFeedback {
        let Some(entry) = find_tool_entry(action_id) else {
            tracing::warn!(
                event = "s05-tools-action-rejected",
                surface = ActionSurface::Tools.label(),
                action_id,
                reason = "unknown-action",
                "Tools action rejected because the id is unknown"
            );
            return ToolsActionFeedback::rejected(
                action_id,
                None,
                ActionRejectionKind::UnknownAction,
                "Tools action is not available.",
                Some(format!("unknown Tools action id: {action_id}")),
            );
        };

        if let Some(utility) = entry.deferred_utility() {
            tracing::warn!(
                event = "s05-tools-action-rejected",
                surface = ActionSurface::Tools.label(),
                action_id = entry.id.as_str(),
                utility_key = utility.key,
                reason = "disabled-utility",
                "Tools utility action rejected because it is deferred"
            );
            return ToolsActionFeedback::rejected(
                entry.id.as_str(),
                Some(ToolsActionKind::DeferredUtility(entry.id)),
                ActionRejectionKind::DisabledUtility,
                utility.status_text,
                Some(format!("deferred Tools utility: {}", utility.key)),
            );
        }

        if let Some(utility) = entry.internal_utility() {
            tracing::info!(
                event = "s09-tools-internal-utility-routed",
                surface = ActionSurface::Tools.label(),
                action_id = entry.id.as_str(),
                utility_key = utility.key,
                "Tools internal utility action completed by in-app routing"
            );
            return ToolsActionFeedback::succeeded(
                entry.id.as_str(),
                ToolsActionKind::InternalUtility(entry.id),
                utility.status_text,
            );
        }

        let Some(link) = entry.external_link() else {
            tracing::warn!(
                event = "s05-tools-action-rejected",
                surface = ActionSurface::Tools.label(),
                action_id = entry.id.as_str(),
                reason = "internal-utility",
                "Tools action rejected because it has no external target"
            );
            return ToolsActionFeedback::rejected(
                entry.id.as_str(),
                Some(ToolsActionKind::DeferredUtility(entry.id)),
                ActionRejectionKind::InternalUtility,
                "Tools action does not have an external target.",
                Some("known Tools action has no external link".to_owned()),
            );
        };

        tracing::info!(
            event = "s05-tools-action-started",
            surface = ActionSurface::Tools.label(),
            action_id = entry.id.as_str(),
            operation = PlatformOperation::OpenUrl.label(),
            host_hint = link.host_hint,
            "Tools external link action started"
        );
        tools_desktop_feedback(entry.id, self.desktop.open_url(link.url))
    }

    /// Executes an About-tab callback id after parsing it against known reference buttons.
    pub fn execute_about_action(&self, action_id: &str) -> AboutActionFeedback {
        let Some((link, action)) = find_about_action(action_id) else {
            tracing::warn!(
                event = "s05-about-action-rejected",
                surface = ActionSurface::About.label(),
                action_id,
                reason = "unknown-action",
                "About action rejected because the id is unknown"
            );
            return AboutActionFeedback::rejected(
                action_id,
                None,
                ActionRejectionKind::UnknownAction,
                "About action is not available.",
                Some(format!("unknown About action id: {action_id}")),
            );
        };

        match action {
            AboutActionKind::Open { .. } => {
                tracing::info!(
                    event = "s05-about-action-started",
                    surface = ActionSurface::About.label(),
                    action_id = action.action_id().as_str(),
                    link_id = action.link_id().as_str(),
                    operation = PlatformOperation::OpenUrl.label(),
                    "About open-link action started"
                );
                about_desktop_feedback(action, self.desktop.open_url(link.url))
            }
            AboutActionKind::Copy { .. } => {
                if link.url.is_empty() {
                    tracing::error!(
                        event = "s05-about-action-rejected",
                        surface = ActionSurface::About.label(),
                        action_id = action.action_id().as_str(),
                        link_id = action.link_id().as_str(),
                        reason = "invalid-input",
                        "About copy action rejected because the reference text is empty"
                    );
                    return AboutActionFeedback::rejected(
                        action.action_id().as_str(),
                        Some(action),
                        ActionRejectionKind::InvalidInput,
                        "Clipboard text is invalid.",
                        Some("empty reference copy text".to_owned()),
                    );
                }

                tracing::info!(
                    event = "s05-about-action-started",
                    surface = ActionSurface::About.label(),
                    action_id = action.action_id().as_str(),
                    link_id = action.link_id().as_str(),
                    operation = PlatformOperation::CopyToClipboard.label(),
                    "About copy-link action started"
                );
                about_clipboard_feedback(action, self.clipboard.copy_text(link.url))
            }
        }
    }
}

fn tools_desktop_feedback(
    action_id: ToolActionId,
    result: DesktopActionResult,
) -> ToolsActionFeedback {
    let action = ToolsActionKind::ExternalLink(action_id);
    if result.is_success() {
        tracing::info!(
            event = "s05-tools-action-completed",
            surface = ActionSurface::Tools.label(),
            action_id = action_id.as_str(),
            operation = result.operation.label(),
            "Tools external link action completed"
        );
        return ToolsActionFeedback::succeeded(action_id.as_str(), action, result.safe_message());
    }

    let failure_kind = result
        .failure_kind()
        .unwrap_or(PlatformErrorKind::CommandFailed);
    tracing::warn!(
        event = "s05-tools-action-failed",
        surface = ActionSurface::Tools.label(),
        action_id = action_id.as_str(),
        operation = result.operation.label(),
        failure_kind = ?failure_kind,
        diagnostic = result.diagnostic().unwrap_or(""),
        "Tools external link action failed"
    );
    ToolsActionFeedback::failed(
        action_id.as_str(),
        action,
        result.operation,
        failure_kind,
        result.safe_message(),
        result.diagnostic().map(ToOwned::to_owned),
    )
}

fn about_desktop_feedback(
    action: AboutActionKind,
    result: DesktopActionResult,
) -> AboutActionFeedback {
    if result.is_success() {
        tracing::info!(
            event = "s05-about-action-completed",
            surface = ActionSurface::About.label(),
            action_id = action.action_id().as_str(),
            link_id = action.link_id().as_str(),
            operation = result.operation.label(),
            "About open-link action completed"
        );
        return AboutActionFeedback::succeeded(
            action.action_id().as_str(),
            action,
            result.safe_message(),
        );
    }

    let failure_kind = result
        .failure_kind()
        .unwrap_or(PlatformErrorKind::CommandFailed);
    tracing::warn!(
        event = "s05-about-action-failed",
        surface = ActionSurface::About.label(),
        action_id = action.action_id().as_str(),
        link_id = action.link_id().as_str(),
        operation = result.operation.label(),
        failure_kind = ?failure_kind,
        diagnostic = result.diagnostic().unwrap_or(""),
        "About open-link action failed"
    );
    AboutActionFeedback::failed(
        action.action_id().as_str(),
        action,
        result.operation,
        failure_kind,
        result.safe_message(),
        result.diagnostic().map(ToOwned::to_owned),
    )
}

fn about_clipboard_feedback(
    action: AboutActionKind,
    result: ClipboardActionResult,
) -> AboutActionFeedback {
    if result.is_success() {
        tracing::info!(
            event = "s05-about-action-completed",
            surface = ActionSurface::About.label(),
            action_id = action.action_id().as_str(),
            link_id = action.link_id().as_str(),
            operation = result.operation.label(),
            "About copy-link action completed"
        );
        return AboutActionFeedback::succeeded(
            action.action_id().as_str(),
            action,
            result.safe_message(),
        );
    }

    let failure_kind = result
        .failure_kind()
        .unwrap_or(PlatformErrorKind::CommandFailed);
    tracing::warn!(
        event = "s05-about-action-failed",
        surface = ActionSurface::About.label(),
        action_id = action.action_id().as_str(),
        link_id = action.link_id().as_str(),
        operation = result.operation.label(),
        failure_kind = ?failure_kind,
        diagnostic = result.diagnostic().unwrap_or(""),
        "About copy-link action failed"
    );
    AboutActionFeedback::failed(
        action.action_id().as_str(),
        action,
        result.operation,
        failure_kind,
        result.safe_message(),
        result.diagnostic().map(ToOwned::to_owned),
    )
}

fn find_tool_entry(action_id: &str) -> Option<ToolEntry> {
    TOOL_GROUPS
        .iter()
        .flat_map(|group| group.entries.iter())
        .copied()
        .find(|entry| entry.id.as_str() == action_id)
}

fn find_about_action(action_id: &str) -> Option<(AboutLink, AboutActionKind)> {
    ABOUT_LINKS.iter().copied().find_map(|link| {
        if link.open_action_id.as_str() == action_id {
            Some((
                link,
                AboutActionKind::Open {
                    link_id: link.id,
                    action_id: link.open_action_id,
                },
            ))
        } else if link.copy_action_id.as_str() == action_id {
            Some((
                link,
                AboutActionKind::Copy {
                    link_id: link.id,
                    action_id: link.copy_action_id,
                },
            ))
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        path::Path,
        sync::{Arc, Mutex},
    };

    use super::*;
    use crate::{
        domain::tools::{DISCORD_INVITE, GITHUB_LINK, NEXUS_LINK},
        platform::{PlatformError, clipboard::ClipboardActions, desktop::DesktopActions},
    };

    #[derive(Debug, Clone, Default)]
    struct FakeDesktopActions {
        calls: Arc<Mutex<Vec<(PlatformOperation, String)>>>,
        failures: Arc<Mutex<BTreeMap<(PlatformOperation, String), crate::platform::PlatformError>>>,
    }

    impl FakeDesktopActions {
        fn fail_with(
            self,
            operation: PlatformOperation,
            target: impl Into<String>,
            error: crate::platform::PlatformError,
        ) -> Self {
            self.failures
                .lock()
                .expect("fake desktop failure map should be writable")
                .insert((operation, target.into()), error);
            self
        }

        fn calls(&self) -> Vec<(PlatformOperation, String)> {
            self.calls
                .lock()
                .expect("fake desktop calls should be readable")
                .clone()
        }

        fn run(&self, operation: PlatformOperation, target: String) -> DesktopActionResult {
            self.calls
                .lock()
                .expect("fake desktop calls should be writable")
                .push((operation, target.clone()));
            if let Some(error) = self
                .failures
                .lock()
                .expect("fake desktop failures should be readable")
                .get(&(operation, target.clone()))
                .cloned()
            {
                DesktopActionResult::failure(error)
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

    #[derive(Debug, Clone, Default)]
    struct FakeClipboardActions {
        copied: Arc<Mutex<Vec<String>>>,
        failures: Arc<Mutex<BTreeMap<String, crate::platform::PlatformError>>>,
    }

    impl FakeClipboardActions {
        fn fail_with(self, text: impl Into<String>, error: crate::platform::PlatformError) -> Self {
            self.failures
                .lock()
                .expect("fake clipboard failure map should be writable")
                .insert(text.into(), error);
            self
        }

        fn copied(&self) -> Vec<String> {
            self.copied
                .lock()
                .expect("fake clipboard calls should be readable")
                .clone()
        }
    }

    impl ClipboardActions for FakeClipboardActions {
        fn copy_text(&self, text: &str) -> ClipboardActionResult {
            self.copied
                .lock()
                .expect("fake clipboard calls should be writable")
                .push(text.to_owned());
            if let Some(error) = self
                .failures
                .lock()
                .expect("fake clipboard failures should be readable")
                .get(text)
                .cloned()
            {
                ClipboardActionResult::failure(error)
            } else {
                ClipboardActionResult::success("system clipboard")
            }
        }
    }

    #[test]
    fn s05_actions_tools_open_success_uses_static_url() {
        let desktop = FakeDesktopActions::default();
        let clipboard = FakeClipboardActions::default();
        let service = ToolsActionService::new(desktop.clone(), clipboard.clone());

        let feedback = service.execute_tools_action(ToolActionId::BethiniPie.as_str());

        assert!(feedback.is_success());
        assert_eq!(feedback.surface, ActionSurface::Tools);
        assert_eq!(
            feedback.action,
            Some(ToolsActionKind::ExternalLink(ToolActionId::BethiniPie))
        );
        assert_eq!(feedback.safe_message(), "Opened URL.");
        assert_eq!(
            desktop.calls(),
            vec![(
                PlatformOperation::OpenUrl,
                "https://www.nexusmods.com/site/mods/631".to_owned()
            )]
        );
        assert!(clipboard.copied().is_empty());
    }

    #[test]
    fn s05_actions_tools_desktop_open_failure_returns_safe_message_and_diagnostic() {
        let desktop = FakeDesktopActions::default().fail_with(
            PlatformOperation::OpenUrl,
            "https://www.nexusmods.com/site/mods/631",
            PlatformError::command_failed(
                PlatformOperation::OpenUrl,
                "https://www.nexusmods.com/site/mods/631",
                "raw OS browser diagnostic",
            ),
        );
        let service = ToolsActionService::new(desktop, FakeClipboardActions::default());

        let feedback = service.execute_tools_action(ToolActionId::BethiniPie.as_str());

        assert_eq!(
            feedback.outcome,
            ActionOutcome::Failed(ActionPlatformFailure {
                operation: PlatformOperation::OpenUrl,
                kind: PlatformErrorKind::CommandFailed,
            })
        );
        assert_eq!(feedback.safe_message(), "URL open failed.");
        assert_eq!(feedback.diagnostic(), Some("raw OS browser diagnostic"));
        assert!(!feedback.safe_message().contains("raw OS"));
    }

    #[test]
    fn s05_actions_about_clipboard_success_copies_only_reference_url() {
        let desktop = FakeDesktopActions::default();
        let clipboard = FakeClipboardActions::default();
        let service = ToolsActionService::new(desktop.clone(), clipboard.clone());

        let feedback = service.execute_about_action(AboutActionId::CopyDiscord.as_str());

        assert!(feedback.is_success());
        assert_eq!(
            feedback.action,
            Some(AboutActionKind::Copy {
                link_id: AboutLinkId::Discord,
                action_id: AboutActionId::CopyDiscord,
            })
        );
        assert_eq!(clipboard.copied(), vec![DISCORD_INVITE.to_owned()]);
        assert!(desktop.calls().is_empty());
    }

    #[test]
    fn s05_actions_about_clipboard_failure_returns_safe_message_and_diagnostic() {
        let clipboard = FakeClipboardActions::default().fail_with(
            GITHUB_LINK,
            PlatformError::command_failed(
                PlatformOperation::CopyToClipboard,
                "system clipboard",
                "raw clipboard diagnostic",
            ),
        );
        let service = ToolsActionService::new(FakeDesktopActions::default(), clipboard);

        let feedback = service.execute_about_action(AboutActionId::CopyGithub.as_str());

        assert_eq!(
            feedback.outcome,
            ActionOutcome::Failed(ActionPlatformFailure {
                operation: PlatformOperation::CopyToClipboard,
                kind: PlatformErrorKind::CommandFailed,
            })
        );
        assert_eq!(feedback.safe_message(), "Clipboard copy failed.");
        assert_eq!(feedback.diagnostic(), Some("raw clipboard diagnostic"));
        assert!(!feedback.safe_message().contains("raw clipboard"));
    }

    #[test]
    fn s05_actions_unsupported_clipboard_adapter_failure_is_safe() {
        let clipboard = FakeClipboardActions::default().fail_with(
            NEXUS_LINK,
            PlatformError::unsupported(PlatformOperation::CopyToClipboard, "system clipboard"),
        );
        let service = ToolsActionService::new(FakeDesktopActions::default(), clipboard);

        let feedback = service.execute_about_action(AboutActionId::CopyNexus.as_str());

        assert_eq!(
            feedback.outcome,
            ActionOutcome::Failed(ActionPlatformFailure {
                operation: PlatformOperation::CopyToClipboard,
                kind: PlatformErrorKind::UnsupportedPlatform,
            })
        );
        assert_eq!(
            feedback.safe_message(),
            "Clipboard copy is not supported on this platform."
        );
    }

    #[test]
    fn s05_actions_unknown_action_ids_are_rejected_without_adapter_calls() {
        let desktop = FakeDesktopActions::default();
        let clipboard = FakeClipboardActions::default();
        let service = ToolsActionService::new(desktop.clone(), clipboard.clone());

        let tools_feedback = service.execute_tools_action("tools.open_arbitrary_url");
        let about_feedback = service.execute_about_action("about.github.copy.trailing");

        assert_eq!(
            tools_feedback.outcome,
            ActionOutcome::Rejected(ActionRejectionKind::UnknownAction)
        );
        assert_eq!(
            tools_feedback.safe_message(),
            "Tools action is not available."
        );
        assert_eq!(
            about_feedback.outcome,
            ActionOutcome::Rejected(ActionRejectionKind::UnknownAction)
        );
        assert_eq!(
            about_feedback.safe_message(),
            "About action is not available."
        );
        assert!(desktop.calls().is_empty());
        assert!(clipboard.copied().is_empty());
    }

    #[test]
    fn s09_actions_downgrade_manager_and_archive_patcher_are_routed_without_desktop_handoff() {
        let desktop = FakeDesktopActions::default();
        let clipboard = FakeClipboardActions::default();
        let service = ToolsActionService::new(desktop.clone(), clipboard.clone());

        let downgrade_feedback =
            service.execute_tools_action(ToolActionId::DowngradeManager.as_str());

        assert_eq!(downgrade_feedback.outcome, ActionOutcome::Succeeded);
        assert_eq!(
            downgrade_feedback.action,
            Some(ToolsActionKind::InternalUtility(
                ToolActionId::DowngradeManager
            ))
        );
        assert_eq!(
            downgrade_feedback.safe_message(),
            "Open the Downgrade Manager workflow."
        );
        assert!(desktop.calls().is_empty());
        assert!(clipboard.copied().is_empty());

        let archive_feedback = service.execute_tools_action(ToolActionId::ArchivePatcher.as_str());

        assert_eq!(archive_feedback.outcome, ActionOutcome::Succeeded);
        assert_eq!(
            archive_feedback.action,
            Some(ToolsActionKind::InternalUtility(
                ToolActionId::ArchivePatcher
            ))
        );
        assert_eq!(
            archive_feedback.safe_message(),
            "Open the Archive Patcher workflow."
        );
        assert!(desktop.calls().is_empty());
        assert!(clipboard.copied().is_empty());
    }
}
