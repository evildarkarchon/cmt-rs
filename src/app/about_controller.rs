//! Pure About-tab action feedback reducer.
//!
//! This controller owns no Slint handles and performs no desktop or clipboard
//! work. It turns owned About action feedback/worker payloads into render-ready
//! error state and copy-button labels/enabled states.

use crate::{
    domain::tools::{ABOUT_COPY_SUCCESS_LABEL, ABOUT_LINKS, AboutActionId, AboutLinkId},
    services::tools::{AboutActionFeedback, AboutActionKind, ActionOutcome},
    workers::{WorkerEvent, WorkerPayload},
};

/// Result of applying an About controller transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AboutTransitionResult {
    /// The feedback belonged to the About surface and changed render state.
    Applied,
    /// The event/reset id was not relevant to this reducer.
    Ignored,
}

impl AboutTransitionResult {
    /// Returns true when the transition changed render state.
    pub const fn is_applied(self) -> bool {
        matches!(self, Self::Applied)
    }
}

/// User-safe About action error for visible banners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutActionError {
    /// Stable action id associated with the error.
    pub action_id: String,
    /// Safe summary text shown to the user.
    pub summary: String,
}

impl AboutActionError {
    /// Creates a user-safe About action error.
    pub fn new(action_id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            summary: summary.into(),
        }
    }
}

/// Render-ready copy-button state for one About link row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutCopyButtonState {
    /// Stable About link row id.
    pub link_id: AboutLinkId,
    /// Stable copy action id for this button.
    pub action_id: AboutActionId,
    /// Current button label.
    pub label: String,
    /// Whether the copy button should be enabled.
    pub enabled: bool,
}

/// Render-ready About-tab state owned by the controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutState {
    /// Last safe action error for the visible banner.
    pub last_safe_error: Option<AboutActionError>,
    /// Copy buttons in reference display order.
    pub copy_buttons: Vec<AboutCopyButtonState>,
}

impl Default for AboutState {
    fn default() -> Self {
        Self {
            last_safe_error: None,
            copy_buttons: ABOUT_LINKS
                .iter()
                .map(|link| AboutCopyButtonState {
                    link_id: link.id,
                    action_id: link.copy_action_id,
                    label: link.copy_button_label.to_owned(),
                    enabled: true,
                })
                .collect(),
        }
    }
}

/// Pure reducer for About action feedback and copy-label reset transitions.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AboutController {
    state: AboutState,
}

impl AboutController {
    /// Creates an idle About controller with reference copy labels.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current render-ready state.
    pub fn state(&self) -> &AboutState {
        &self.state
    }

    /// Returns the current safe last-action error, if any.
    pub fn last_safe_error(&self) -> Option<&AboutActionError> {
        self.state.last_safe_error.as_ref()
    }

    /// Returns copy-button states in reference display order.
    pub fn copy_buttons(&self) -> &[AboutCopyButtonState] {
        &self.state.copy_buttons
    }

    /// Returns the copy-button state for one link row.
    pub fn copy_button(&self, link_id: AboutLinkId) -> Option<&AboutCopyButtonState> {
        self.state
            .copy_buttons
            .iter()
            .find(|button| button.link_id == link_id)
    }

    /// Applies service feedback directly to render-ready About state.
    pub fn handle_feedback(&mut self, feedback: AboutActionFeedback) -> AboutTransitionResult {
        match feedback.outcome {
            ActionOutcome::Succeeded => {
                if let Some(action @ AboutActionKind::Copy { .. }) = feedback.action {
                    self.apply_copy_success(action);
                }

                tracing::info!(
                    event = "s05-about-reducer-applied",
                    action_id = feedback.action_id.as_str(),
                    outcome = "succeeded",
                    copied = feedback.action.is_some_and(AboutActionKind::is_copy),
                    "About action success cleared visible error state"
                );
                self.state.last_safe_error = None;
            }
            ActionOutcome::Rejected(kind) => {
                tracing::warn!(
                    event = "s05-about-reducer-applied",
                    action_id = feedback.action_id.as_str(),
                    rejection_kind = ?kind,
                    safe_message = feedback.safe_message.as_str(),
                    diagnostic = feedback.diagnostic.as_deref().unwrap_or(""),
                    "About action rejection applied as safe error"
                );
                self.state.last_safe_error = Some(AboutActionError::new(
                    feedback.action_id,
                    feedback.safe_message,
                ));
            }
            ActionOutcome::Failed(failure) => {
                tracing::warn!(
                    event = "s05-about-reducer-applied",
                    action_id = feedback.action_id.as_str(),
                    operation = failure.operation.label(),
                    failure_kind = ?failure.kind,
                    safe_message = feedback.safe_message.as_str(),
                    diagnostic = feedback.diagnostic.as_deref().unwrap_or(""),
                    "About action failure applied as safe error"
                );
                self.state.last_safe_error = Some(AboutActionError::new(
                    feedback.action_id,
                    feedback.safe_message,
                ));
            }
        }

        AboutTransitionResult::Applied
    }

    /// Resets a copied button label back to the reference copy label and re-enables it.
    pub fn reset_copy_label(&mut self, action_id: &str) -> AboutTransitionResult {
        let Some(reference) = ABOUT_LINKS
            .iter()
            .find(|link| link.copy_action_id.as_str() == action_id)
        else {
            tracing::warn!(
                event = "s05-about-copy-label-reset-ignored",
                action_id,
                "About copy-label reset ignored because the id is unknown"
            );
            return AboutTransitionResult::Ignored;
        };

        if let Some(button) = self
            .state
            .copy_buttons
            .iter_mut()
            .find(|button| button.action_id == reference.copy_action_id)
        {
            button.label = reference.copy_button_label.to_owned();
            button.enabled = true;
        }

        tracing::info!(
            event = "s05-about-copy-label-reset",
            action_id = reference.copy_action_id.as_str(),
            link_id = reference.id.as_str(),
            "About copy button label reset to reference text"
        );
        AboutTransitionResult::Applied
    }

    /// Applies an owned worker event if it carries an About action-completion payload.
    pub fn handle_worker_event(&mut self, event: WorkerEvent) -> AboutTransitionResult {
        match event.payload {
            WorkerPayload::AboutAction(payload) => self.handle_feedback(payload.feedback),
            _ => AboutTransitionResult::Ignored,
        }
    }

    fn apply_copy_success(&mut self, action: AboutActionKind) {
        let target_action_id = action.action_id();
        for button in &mut self.state.copy_buttons {
            if button.action_id == target_action_id {
                button.label = ABOUT_COPY_SUCCESS_LABEL.to_owned();
                button.enabled = false;
            } else if let Some(reference) = ABOUT_LINKS
                .iter()
                .find(|link| link.copy_action_id == button.action_id)
            {
                button.label = reference.copy_button_label.to_owned();
                button.enabled = true;
            }
        }

        tracing::info!(
            event = "s05-about-copy-success-label-applied",
            action_id = target_action_id.as_str(),
            link_id = action.link_id().as_str(),
            "About copy button label changed to success text"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::tools::{ABOUT_COPY_INVITE_LABEL, ABOUT_COPY_LINK_LABEL},
        services::tools::{ActionPlatformFailure, ToolsActionFeedback, ToolsActionKind},
        workers::{
            ToolsActionWorkerPayload, WorkerEvent, WorkerPayload, WorkerTask, WorkerTaskKind,
        },
    };

    #[test]
    fn s05_actions_about_controller_copy_success_sets_only_target_label_and_reset_restores() {
        let mut controller = AboutController::new();
        let feedback = AboutActionFeedback::succeeded(
            AboutActionId::CopyDiscord.as_str(),
            AboutActionKind::Copy {
                link_id: AboutLinkId::Discord,
                action_id: AboutActionId::CopyDiscord,
            },
            "Copied to clipboard.",
        );

        let result = controller.handle_feedback(feedback);

        assert!(result.is_applied());
        assert_eq!(controller.last_safe_error(), None);
        assert_eq!(
            controller.copy_button(AboutLinkId::Nexus),
            Some(&AboutCopyButtonState {
                link_id: AboutLinkId::Nexus,
                action_id: AboutActionId::CopyNexus,
                label: ABOUT_COPY_LINK_LABEL.to_owned(),
                enabled: true,
            })
        );
        assert_eq!(
            controller.copy_button(AboutLinkId::Discord),
            Some(&AboutCopyButtonState {
                link_id: AboutLinkId::Discord,
                action_id: AboutActionId::CopyDiscord,
                label: ABOUT_COPY_SUCCESS_LABEL.to_owned(),
                enabled: false,
            })
        );
        assert_eq!(
            controller.copy_button(AboutLinkId::Github),
            Some(&AboutCopyButtonState {
                link_id: AboutLinkId::Github,
                action_id: AboutActionId::CopyGithub,
                label: ABOUT_COPY_LINK_LABEL.to_owned(),
                enabled: true,
            })
        );

        let reset = controller.reset_copy_label(AboutActionId::CopyDiscord.as_str());

        assert_eq!(reset, AboutTransitionResult::Applied);
        assert_eq!(
            controller.copy_button(AboutLinkId::Discord),
            Some(&AboutCopyButtonState {
                link_id: AboutLinkId::Discord,
                action_id: AboutActionId::CopyDiscord,
                label: ABOUT_COPY_INVITE_LABEL.to_owned(),
                enabled: true,
            })
        );
    }

    #[test]
    fn s05_actions_about_controller_failure_sets_safe_error_without_raw_diagnostic() {
        let mut controller = AboutController::new();
        let feedback = AboutActionFeedback::failed(
            AboutActionId::CopyGithub.as_str(),
            AboutActionKind::Copy {
                link_id: AboutLinkId::Github,
                action_id: AboutActionId::CopyGithub,
            },
            crate::platform::PlatformOperation::CopyToClipboard,
            crate::platform::PlatformErrorKind::CommandFailed,
            "Clipboard copy failed.",
            Some("raw clipboard diagnostic".to_owned()),
        );

        let result = controller.handle_feedback(feedback);

        assert_eq!(result, AboutTransitionResult::Applied);
        assert_eq!(
            controller.last_safe_error(),
            Some(&AboutActionError::new(
                AboutActionId::CopyGithub.as_str(),
                "Clipboard copy failed."
            ))
        );
        assert!(
            !controller
                .last_safe_error()
                .expect("safe error")
                .summary
                .contains("raw clipboard")
        );
    }

    #[test]
    fn s05_actions_about_controller_ignores_tools_worker_payloads_and_unknown_resets() {
        let mut controller = AboutController::new();
        let tools_feedback = ToolsActionFeedback {
            surface: crate::services::tools::ActionSurface::Tools,
            action_id: crate::domain::tools::ToolActionId::BethiniPie
                .as_str()
                .to_owned(),
            action: Some(ToolsActionKind::ExternalLink(
                crate::domain::tools::ToolActionId::BethiniPie,
            )),
            outcome: ActionOutcome::Failed(ActionPlatformFailure {
                operation: crate::platform::PlatformOperation::OpenUrl,
                kind: crate::platform::PlatformErrorKind::CommandFailed,
            }),
            safe_message: "URL open failed.".to_owned(),
            diagnostic: Some("raw browser diagnostic".to_owned()),
        };
        let event = WorkerEvent::completed(
            WorkerTask::new("tools-open", WorkerTaskKind::DesktopAction),
            WorkerPayload::ToolsAction(ToolsActionWorkerPayload::action_completed(tools_feedback)),
        );

        let ignored_event = controller.handle_worker_event(event);
        let ignored_reset = controller.reset_copy_label("about.unknown.copy");

        assert_eq!(ignored_event, AboutTransitionResult::Ignored);
        assert_eq!(ignored_reset, AboutTransitionResult::Ignored);
        assert_eq!(controller.state(), &AboutState::default());
    }
}
