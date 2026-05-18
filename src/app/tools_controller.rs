//! Pure Tools-tab action feedback reducer.
//!
//! This controller owns no Slint handles and performs no desktop, clipboard, or
//! filesystem work. It only turns owned action feedback/worker payloads into
//! render-ready banner and disabled-utility status state.

use crate::{
    services::tools::{ActionOutcome, ActionRejectionKind, ToolsActionFeedback, ToolsActionKind},
    workers::{WorkerEvent, WorkerPayload},
};

/// Default status text shown before a specific deferred utility action is requested.
pub const TOOLS_DEFAULT_DISABLED_UTILITY_STATUS: &str =
    "Downgrade Manager is deferred until S09; Archive Patcher is deferred until S10.";

/// Result of applying a Tools controller transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsTransitionResult {
    /// The feedback belonged to the Tools surface and changed render state.
    Applied,
    /// The event was not a Tools action-completion payload.
    Ignored,
}

impl ToolsTransitionResult {
    /// Returns true when the transition changed render state.
    pub const fn is_applied(self) -> bool {
        matches!(self, Self::Applied)
    }
}

/// User-safe Tools action error for visible banners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolsActionError {
    /// Stable action id associated with the error.
    pub action_id: String,
    /// Safe summary text shown to the user.
    pub summary: String,
}

impl ToolsActionError {
    /// Creates a user-safe Tools action error.
    pub fn new(action_id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            summary: summary.into(),
        }
    }
}

/// Render-ready Tools-tab state owned by the controller.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ToolsState {
    /// Last safe action error for the visible banner.
    pub last_safe_error: Option<ToolsActionError>,
    /// Last deferred utility status text, shown separately from error banners.
    pub disabled_utility_status: Option<String>,
}

/// Pure reducer for Tools action feedback.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ToolsController {
    state: ToolsState,
}

impl ToolsController {
    /// Creates an idle Tools controller with no visible status.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current render-ready state.
    pub fn state(&self) -> &ToolsState {
        &self.state
    }

    /// Returns the current safe last-action error, if any.
    pub fn last_safe_error(&self) -> Option<&ToolsActionError> {
        self.state.last_safe_error.as_ref()
    }

    /// Returns the current disabled utility status text, if any.
    pub fn disabled_utility_status(&self) -> Option<&str> {
        self.state.disabled_utility_status.as_deref()
    }

    /// Applies service feedback directly to render-ready Tools state.
    pub fn handle_feedback(&mut self, feedback: ToolsActionFeedback) -> ToolsTransitionResult {
        match feedback.outcome {
            ActionOutcome::Succeeded => {
                tracing::info!(
                    event = "s05-tools-reducer-applied",
                    action_id = feedback.action_id.as_str(),
                    outcome = "succeeded",
                    "Tools action success cleared visible error state"
                );
                self.state.last_safe_error = None;
                self.state.disabled_utility_status = None;
            }
            ActionOutcome::Rejected(ActionRejectionKind::DisabledUtility)
            | ActionOutcome::Rejected(ActionRejectionKind::InternalUtility)
                if is_deferred_tool_action(feedback.action) =>
            {
                tracing::info!(
                    event = "s05-tools-disabled-utility-status-applied",
                    action_id = feedback.action_id.as_str(),
                    "Tools deferred utility status applied"
                );
                self.state.last_safe_error = None;
                self.state.disabled_utility_status = Some(feedback.safe_message);
            }
            ActionOutcome::Rejected(kind) => {
                tracing::warn!(
                    event = "s05-tools-reducer-applied",
                    action_id = feedback.action_id.as_str(),
                    rejection_kind = ?kind,
                    safe_message = feedback.safe_message.as_str(),
                    diagnostic = feedback.diagnostic.as_deref().unwrap_or(""),
                    "Tools action rejection applied as safe error"
                );
                self.state.last_safe_error = Some(ToolsActionError::new(
                    feedback.action_id,
                    feedback.safe_message,
                ));
                self.state.disabled_utility_status = None;
            }
            ActionOutcome::Failed(failure) => {
                tracing::warn!(
                    event = "s05-tools-reducer-applied",
                    action_id = feedback.action_id.as_str(),
                    operation = failure.operation.label(),
                    failure_kind = ?failure.kind,
                    safe_message = feedback.safe_message.as_str(),
                    diagnostic = feedback.diagnostic.as_deref().unwrap_or(""),
                    "Tools action failure applied as safe error"
                );
                self.state.last_safe_error = Some(ToolsActionError::new(
                    feedback.action_id,
                    feedback.safe_message,
                ));
                self.state.disabled_utility_status = None;
            }
        }

        ToolsTransitionResult::Applied
    }

    /// Applies an owned worker event if it carries a Tools action-completion payload.
    pub fn handle_worker_event(&mut self, event: WorkerEvent) -> ToolsTransitionResult {
        match event.payload {
            WorkerPayload::ToolsAction(payload) => self.handle_feedback(payload.feedback),
            _ => ToolsTransitionResult::Ignored,
        }
    }
}

fn is_deferred_tool_action(action: Option<ToolsActionKind>) -> bool {
    matches!(action, Some(ToolsActionKind::DeferredUtility(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::tools::{AboutActionId, AboutLinkId, ToolActionId},
        services::tools::{AboutActionFeedback, AboutActionKind},
        workers::{
            AboutActionWorkerPayload, WorkerEvent, WorkerPayload, WorkerTask, WorkerTaskKind,
        },
    };

    #[test]
    fn s05_actions_tools_controller_sets_disabled_status_without_error_banner() {
        let mut controller = ToolsController::new();
        let feedback = ToolsActionFeedback::rejected(
            ToolActionId::ArchivePatcher.as_str(),
            Some(ToolsActionKind::DeferredUtility(
                ToolActionId::ArchivePatcher,
            )),
            ActionRejectionKind::DisabledUtility,
            "Archive Patcher is not available in this Rust port yet.",
            Some("deferred utility".to_owned()),
        );

        let result = controller.handle_feedback(feedback);

        assert!(result.is_applied());
        assert_eq!(controller.last_safe_error(), None);
        assert_eq!(
            controller.disabled_utility_status(),
            Some("Archive Patcher is not available in this Rust port yet.")
        );
    }

    #[test]
    fn s05_actions_tools_controller_applies_failures_as_safe_error_banners() {
        let mut controller = ToolsController::new();
        let feedback = ToolsActionFeedback::failed(
            ToolActionId::BethiniPie.as_str(),
            ToolsActionKind::ExternalLink(ToolActionId::BethiniPie),
            crate::platform::PlatformOperation::OpenUrl,
            crate::platform::PlatformErrorKind::CommandFailed,
            "URL open failed.",
            Some("raw browser diagnostic".to_owned()),
        );

        let result = controller.handle_feedback(feedback);

        assert_eq!(result, ToolsTransitionResult::Applied);
        assert_eq!(controller.disabled_utility_status(), None);
        assert_eq!(
            controller.last_safe_error(),
            Some(&ToolsActionError::new(
                ToolActionId::BethiniPie.as_str(),
                "URL open failed."
            ))
        );
    }

    #[test]
    fn s05_actions_tools_controller_ignores_about_worker_payloads() {
        let mut controller = ToolsController::new();
        let about_feedback = AboutActionFeedback::succeeded(
            AboutActionId::CopyGithub.as_str(),
            AboutActionKind::Copy {
                link_id: AboutLinkId::Github,
                action_id: AboutActionId::CopyGithub,
            },
            "Copied to clipboard.",
        );
        let event = WorkerEvent::completed(
            WorkerTask::new("about-copy", WorkerTaskKind::DesktopAction),
            WorkerPayload::AboutAction(AboutActionWorkerPayload::action_completed(about_feedback)),
        );

        let result = controller.handle_worker_event(event);

        assert_eq!(result, ToolsTransitionResult::Ignored);
        assert_eq!(controller.state(), &ToolsState::default());
    }
}
