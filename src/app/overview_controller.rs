//! Testable Overview-tab controller and worker payload reducer.
//!
//! The controller owns no Slint handles and performs no filesystem, registry,
//! process, network, or desktop work. It only turns UI intents and owned worker
//! events into a current [`OverviewSnapshot`](crate::domain::overview::OverviewSnapshot)
//! so production code can keep slow work on background workers while tests can
//! exercise every state transition with in-memory values.

use crate::{
    domain::{
        overview::{
            OverviewActionError, OverviewDeferredAction, OverviewDeferredActionKind,
            OverviewDeferredActionTarget, OverviewGamePathStatus, OverviewSnapshot,
            UpdateBannerState, UpdateProvider,
        },
        settings::UpdateSource,
    },
    workers::{
        OverviewWorkerPayload, WorkerEvent, WorkerFailure, WorkerPayload, WorkerSpawnError,
        WorkerTask, WorkerTaskId, WorkerTaskKind, WorkerTaskStatus,
    },
};

/// Stable prefix for Overview refresh worker task identifiers.
pub const OVERVIEW_REFRESH_TASK_PREFIX: &str = "overview-refresh";
/// Stable prefix for Overview update-check worker task identifiers.
pub const OVERVIEW_UPDATE_TASK_PREFIX: &str = "overview-update";
/// Stable prefix for Overview desktop-action worker task identifiers.
pub const OVERVIEW_DESKTOP_TASK_PREFIX: &str = "overview-desktop";

/// Monotonic identity assigned to each Overview refresh request.
pub type OverviewRefreshId = u64;

/// Result of applying a controller transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverviewTransitionResult {
    /// The event matched the active controller state and changed the snapshot or diagnostics.
    Applied,
    /// The event belonged to an older refresh and was intentionally ignored.
    StaleIgnored,
    /// The event was not an Overview event or was not relevant to this reducer.
    Ignored,
}

impl OverviewTransitionResult {
    /// Returns true when the event changed controller state.
    pub const fn is_applied(self) -> bool {
        matches!(self, Self::Applied)
    }
}

/// Work request returned by refresh intents and consumed by worker wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverviewRefreshRequest {
    /// Monotonic refresh request id used to reject stale worker results.
    pub refresh_id: OverviewRefreshId,
    /// Update source snapshot captured at request time.
    pub update_source: UpdateSource,
}

impl OverviewRefreshRequest {
    /// Returns whether this refresh should schedule network update work.
    pub const fn should_check_updates(self) -> bool {
        !matches!(self.update_source, UpdateSource::None)
    }

    /// Builds the worker metadata used for the blocking discovery/collection refresh.
    pub fn refresh_task(self) -> WorkerTask {
        overview_refresh_task(self.refresh_id)
    }

    /// Builds the worker metadata used for the async update check tied to this refresh.
    pub fn update_task(self) -> WorkerTask {
        overview_update_task(self.refresh_id)
    }
}

/// Pure reducer for Overview UI state and owned worker events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewController {
    snapshot: OverviewSnapshot,
    next_refresh_id: OverviewRefreshId,
    latest_refresh_id: Option<OverviewRefreshId>,
    pending_update_banner: Option<(OverviewRefreshId, UpdateBannerState)>,
}

impl Default for OverviewController {
    fn default() -> Self {
        Self::new()
    }
}

impl OverviewController {
    /// Creates an idle controller with an empty Overview snapshot.
    pub fn new() -> Self {
        Self {
            snapshot: OverviewSnapshot::empty(),
            next_refresh_id: 1,
            latest_refresh_id: None,
            pending_update_banner: None,
        }
    }

    /// Returns the current render-ready Overview snapshot.
    pub fn snapshot(&self) -> &OverviewSnapshot {
        &self.snapshot
    }

    /// Returns the current safe last-action error, if any.
    pub fn last_safe_action_error(&self) -> Option<&OverviewActionError> {
        self.snapshot.last_action_error.as_ref()
    }

    /// Starts the initial loading transition using the supplied settings snapshot.
    pub fn initial_loading(&mut self, update_source: UpdateSource) -> OverviewRefreshRequest {
        self.request_refresh(update_source)
    }

    /// Starts a new refresh and returns the worker request to schedule.
    pub fn request_refresh(&mut self, update_source: UpdateSource) -> OverviewRefreshRequest {
        let refresh_id = self.next_refresh_id;
        self.next_refresh_id = self.next_refresh_id.saturating_add(1);
        self.latest_refresh_id = Some(refresh_id);
        self.pending_update_banner = None;

        let last_action_error = self.snapshot.last_action_error.clone();
        self.snapshot = OverviewSnapshot::loading("Refreshing Overview...");
        self.snapshot.update_banner = initial_update_banner(update_source);
        self.snapshot.last_action_error = last_action_error;

        tracing::info!(
            event = "overview-refresh-requested",
            refresh_id,
            update_source = update_source.as_wire_value(),
            "Overview refresh requested"
        );

        OverviewRefreshRequest {
            refresh_id,
            update_source,
        }
    }

    /// Applies a successful refresh snapshot if it belongs to the latest request.
    pub fn refresh_completed(
        &mut self,
        refresh_id: OverviewRefreshId,
        mut snapshot: OverviewSnapshot,
    ) -> OverviewTransitionResult {
        if !self.is_latest_refresh(refresh_id) {
            tracing::debug!(
                event = "overview-refresh-stale-ignored",
                refresh_id,
                latest_refresh_id = ?self.latest_refresh_id,
                "Ignoring stale Overview refresh result"
            );
            return OverviewTransitionResult::StaleIgnored;
        }

        snapshot.last_action_error = self.snapshot.last_action_error.clone();
        if let Some((pending_refresh_id, update_banner)) = self.pending_update_banner.take() {
            if pending_refresh_id == refresh_id {
                snapshot.update_banner = update_banner;
            } else {
                self.pending_update_banner = Some((pending_refresh_id, update_banner));
            }
        }

        tracing::info!(
            event = "overview-refresh-completed",
            refresh_id,
            phase = ?snapshot.refresh.phase,
            problems = snapshot.problems.len(),
            "Overview refresh completed"
        );
        self.snapshot = snapshot;
        OverviewTransitionResult::Applied
    }

    /// Applies a safe refresh failure if it belongs to the latest request.
    pub fn refresh_failed(
        &mut self,
        refresh_id: OverviewRefreshId,
        failure: WorkerFailure,
    ) -> OverviewTransitionResult {
        if !self.is_latest_refresh(refresh_id) {
            tracing::debug!(
                event = "overview-refresh-failure-stale-ignored",
                refresh_id,
                latest_refresh_id = ?self.latest_refresh_id,
                "Ignoring stale Overview refresh failure"
            );
            return OverviewTransitionResult::StaleIgnored;
        }

        tracing::error!(
            event = "overview-refresh-failed",
            refresh_id,
            safe_message = failure.safe_message(),
            diagnostic = failure.diagnostic().unwrap_or(""),
            "Overview refresh failed"
        );
        let last_action_error = self.snapshot.last_action_error.clone();
        let update_banner = self.snapshot.update_banner.clone();
        self.snapshot = OverviewSnapshot::error(failure.safe_message().to_owned());
        self.snapshot.update_banner = update_banner;
        self.snapshot.last_action_error = last_action_error;
        OverviewTransitionResult::Applied
    }

    /// Maps a worker spawn failure into the safe refresh-failed state.
    pub fn refresh_spawn_failed(
        &mut self,
        refresh_id: OverviewRefreshId,
        error: WorkerSpawnError,
    ) -> OverviewTransitionResult {
        let failure = WorkerFailure::new("Overview refresh could not be started.")
            .with_diagnostic(error.to_string());
        self.refresh_failed(refresh_id, failure)
    }

    /// Applies an update-check banner if it belongs to the latest refresh.
    pub fn update_check_completed(
        &mut self,
        refresh_id: OverviewRefreshId,
        update_banner: UpdateBannerState,
    ) -> OverviewTransitionResult {
        if !self.is_latest_refresh(refresh_id) {
            tracing::debug!(
                event = "overview-update-stale-ignored",
                refresh_id,
                latest_refresh_id = ?self.latest_refresh_id,
                "Ignoring stale Overview update-check result"
            );
            return OverviewTransitionResult::StaleIgnored;
        }

        tracing::info!(
            event = "overview-update-completed",
            refresh_id,
            visible = update_banner.is_visible(),
            "Overview update check completed"
        );

        if self.snapshot.refresh.is_busy() {
            self.pending_update_banner = Some((refresh_id, update_banner));
        } else {
            self.snapshot.update_banner = update_banner;
        }
        OverviewTransitionResult::Applied
    }

    /// Applies desktop action feedback to the safe visible last-action error state.
    pub fn desktop_action_completed(
        &mut self,
        action: OverviewDeferredActionKind,
        error: Option<OverviewActionError>,
    ) -> OverviewTransitionResult {
        if let Some(error) = error {
            tracing::warn!(
                event = "overview-desktop-action-failed",
                action = ?action,
                safe_message = error.summary.as_str(),
                "Overview desktop action failed"
            );
            self.snapshot.last_action_error = Some(error);
        } else {
            tracing::info!(
                event = "overview-desktop-action-completed",
                action = ?action,
                "Overview desktop action completed"
            );
            self.snapshot.last_action_error = None;
        }
        OverviewTransitionResult::Applied
    }

    /// Applies an owned worker event if it carries an Overview payload.
    pub fn handle_worker_event(&mut self, event: WorkerEvent) -> OverviewTransitionResult {
        match event.payload {
            WorkerPayload::Overview(payload) => self.handle_overview_payload(payload),
            WorkerPayload::Error(failure)
                if event.task.kind == WorkerTaskKind::Overview
                    && event.status == WorkerTaskStatus::Failed =>
            {
                let refresh_id = refresh_id_from_task_id(&event.task.id)
                    .or(self.latest_refresh_id)
                    .unwrap_or_default();
                self.refresh_failed(refresh_id, failure)
            }
            _ => OverviewTransitionResult::Ignored,
        }
    }

    /// Returns the current game-path open action when discovery found one.
    pub fn game_path_action(&self) -> Option<OverviewDeferredAction> {
        match &self.snapshot.top.game_path {
            OverviewGamePathStatus::Found(path) => Some(OverviewDeferredAction::open_path(
                OverviewDeferredActionKind::OpenGamePath,
                "Game Path",
                path.clone(),
            )),
            OverviewGamePathStatus::NotFound => None,
        }
    }

    /// Returns the current update-provider action when an update banner is visible.
    pub fn update_provider_action(
        &self,
        provider: UpdateProvider,
    ) -> Option<OverviewDeferredAction> {
        match &self.snapshot.update_banner {
            UpdateBannerState::Available { releases, .. } => releases
                .iter()
                .find(|release| release.provider == provider)
                .map(|release| release.action.clone()),
            UpdateBannerState::Disabled
            | UpdateBannerState::NotChecked { .. }
            | UpdateBannerState::Checking { .. }
            | UpdateBannerState::NoUpdate { .. }
            | UpdateBannerState::FailedSilently { .. } => None,
        }
    }

    fn handle_overview_payload(
        &mut self,
        payload: OverviewWorkerPayload,
    ) -> OverviewTransitionResult {
        match payload {
            OverviewWorkerPayload::RefreshCompleted {
                refresh_id,
                snapshot,
            } => self.refresh_completed(refresh_id, *snapshot),
            OverviewWorkerPayload::UpdateCheckCompleted {
                refresh_id,
                update_banner,
            } => self.update_check_completed(refresh_id, update_banner),
            OverviewWorkerPayload::DesktopActionCompleted { action, error } => {
                self.desktop_action_completed(action, error)
            }
        }
    }

    fn is_latest_refresh(&self, refresh_id: OverviewRefreshId) -> bool {
        self.latest_refresh_id == Some(refresh_id)
    }
}

/// Builds worker metadata for a blocking Overview refresh.
pub fn overview_refresh_task(refresh_id: OverviewRefreshId) -> WorkerTask {
    WorkerTask::new(
        format!("{OVERVIEW_REFRESH_TASK_PREFIX}-{refresh_id}"),
        WorkerTaskKind::Overview,
    )
    .with_label("Refresh Overview")
}

/// Builds worker metadata for an Overview update check.
pub fn overview_update_task(refresh_id: OverviewRefreshId) -> WorkerTask {
    WorkerTask::new(
        format!("{OVERVIEW_UPDATE_TASK_PREFIX}-{refresh_id}"),
        WorkerTaskKind::Overview,
    )
    .with_label("Check Overview updates")
}

/// Builds worker metadata for an Overview desktop action.
pub fn overview_desktop_task(action: OverviewDeferredActionKind) -> WorkerTask {
    WorkerTask::new(
        format!("{OVERVIEW_DESKTOP_TASK_PREFIX}-{}", action_label(action)),
        WorkerTaskKind::DesktopAction,
    )
    .with_label("Open Overview link")
}

/// Converts desktop-action worker output into the Overview payload shape.
pub fn overview_desktop_action_payload(
    action: OverviewDeferredActionKind,
    error: Option<OverviewActionError>,
) -> WorkerPayload {
    WorkerPayload::Overview(OverviewWorkerPayload::desktop_action_completed(
        action, error,
    ))
}

/// Creates a safe action error for an unavailable or unsupported action target.
pub fn unavailable_action_error(
    action: OverviewDeferredActionKind,
    message: impl Into<String>,
) -> OverviewActionError {
    OverviewActionError::new(action, message)
}

/// Returns a safe text representation of an action target for diagnostics.
pub fn action_target_label(action: &OverviewDeferredAction) -> String {
    match &action.target {
        OverviewDeferredActionTarget::Internal => "internal action".to_owned(),
        OverviewDeferredActionTarget::Path(path) => path.display().to_string(),
        OverviewDeferredActionTarget::Url(url) => url.clone(),
    }
}

fn initial_update_banner(update_source: UpdateSource) -> UpdateBannerState {
    if matches!(update_source, UpdateSource::None) {
        UpdateBannerState::Disabled
    } else {
        UpdateBannerState::Checking {
            selected_source: update_source,
        }
    }
}

fn refresh_id_from_task_id(task_id: &WorkerTaskId) -> Option<OverviewRefreshId> {
    task_id
        .as_str()
        .strip_prefix(OVERVIEW_REFRESH_TASK_PREFIX)
        .and_then(|suffix| suffix.strip_prefix('-'))
        .and_then(|value| value.parse::<OverviewRefreshId>().ok())
}

fn action_label(action: OverviewDeferredActionKind) -> &'static str {
    match action {
        OverviewDeferredActionKind::OpenGamePath => "game-path",
        OverviewDeferredActionKind::OpenModManagerDetails => "mod-manager-details",
        OverviewDeferredActionKind::OpenUpdateProvider(UpdateProvider::NexusMods) => "nexus-update",
        OverviewDeferredActionKind::OpenUpdateProvider(UpdateProvider::Github) => "github-update",
        OverviewDeferredActionKind::OpenDowngradeManager => "downgrade-manager",
        OverviewDeferredActionKind::OpenArchivePatcher => "archive-patcher",
        OverviewDeferredActionKind::ShowInvalidModuleVersions => "invalid-module-versions",
        OverviewDeferredActionKind::OpenProblemLink => "problem-link",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::domain::{
        discovery::Fallout4InstallType,
        overview::{OverviewRefreshPhase, OverviewRefreshState, UpdateRelease},
    };

    fn ready_snapshot(message: &str) -> OverviewSnapshot {
        let mut snapshot = OverviewSnapshot::empty();
        snapshot.refresh = OverviewRefreshState::ready(Some(message.to_owned()));
        snapshot.top.version = Fallout4InstallType::OldGen;
        snapshot
    }

    #[test]
    fn overview_controller_initial_loading_sets_busy_snapshot_and_update_check_request() {
        let mut controller = OverviewController::new();

        let request = controller.initial_loading(UpdateSource::Github);

        assert_eq!(request.refresh_id, 1);
        assert!(request.should_check_updates());
        assert_eq!(
            controller.snapshot().refresh.phase,
            OverviewRefreshPhase::Loading
        );
        assert_eq!(
            controller.snapshot().update_banner,
            UpdateBannerState::Checking {
                selected_source: UpdateSource::Github
            }
        );
    }

    #[test]
    fn overview_controller_refresh_success_applies_latest_snapshot() {
        let mut controller = OverviewController::new();
        let request = controller.request_refresh(UpdateSource::Nexus);

        let result = controller.refresh_completed(request.refresh_id, ready_snapshot("ready"));

        assert_eq!(result, OverviewTransitionResult::Applied);
        assert_eq!(
            controller.snapshot().refresh.phase,
            OverviewRefreshPhase::Ready
        );
        assert_eq!(
            controller.snapshot().refresh.message.as_deref(),
            Some("ready")
        );
    }

    #[test]
    fn overview_controller_refresh_failure_maps_to_safe_error_snapshot() {
        let mut controller = OverviewController::new();
        let request = controller.request_refresh(UpdateSource::Nexus);

        let result = controller.refresh_failed(
            request.refresh_id,
            WorkerFailure::new("Overview refresh failed.").with_diagnostic("raw detail"),
        );

        assert_eq!(result, OverviewTransitionResult::Applied);
        assert_eq!(
            controller.snapshot().refresh.phase,
            OverviewRefreshPhase::Error
        );
        assert_eq!(
            controller.snapshot().refresh.message.as_deref(),
            Some("Overview refresh failed.")
        );
    }

    #[test]
    fn overview_controller_update_source_none_skips_update_work() {
        let mut controller = OverviewController::new();

        let request = controller.request_refresh(UpdateSource::None);

        assert!(!request.should_check_updates());
        assert_eq!(
            controller.snapshot().update_banner,
            UpdateBannerState::Disabled
        );
    }

    #[test]
    fn overview_controller_ignores_stale_refresh_when_second_request_is_newer() {
        let mut controller = OverviewController::new();
        let first = controller.request_refresh(UpdateSource::Github);
        let second = controller.request_refresh(UpdateSource::Github);

        let stale = controller.refresh_completed(first.refresh_id, ready_snapshot("first"));
        let latest = controller.refresh_completed(second.refresh_id, ready_snapshot("second"));

        assert_eq!(stale, OverviewTransitionResult::StaleIgnored);
        assert_eq!(latest, OverviewTransitionResult::Applied);
        assert_eq!(
            controller.snapshot().refresh.message.as_deref(),
            Some("second")
        );
    }

    #[test]
    fn overview_controller_update_completed_before_refresh_is_applied_after_refresh() {
        let mut controller = OverviewController::new();
        let request = controller.request_refresh(UpdateSource::Github);
        let update_banner = UpdateBannerState::Available {
            selected_source: UpdateSource::Github,
            releases: vec![UpdateRelease::new(UpdateProvider::Github, "9.9.9")],
        };

        let update_result = controller.update_check_completed(request.refresh_id, update_banner);
        assert_eq!(update_result, OverviewTransitionResult::Applied);
        assert!(matches!(
            controller.snapshot().update_banner,
            UpdateBannerState::Checking { .. }
        ));

        controller.refresh_completed(request.refresh_id, ready_snapshot("ready"));

        assert!(matches!(
            controller.snapshot().update_banner,
            UpdateBannerState::Available { ref releases, .. } if releases.len() == 1
        ));
    }

    #[test]
    fn overview_controller_desktop_action_success_clears_last_safe_error() {
        let mut controller = OverviewController::new();
        controller.desktop_action_completed(
            OverviewDeferredActionKind::OpenGamePath,
            Some(OverviewActionError::new(
                OverviewDeferredActionKind::OpenGamePath,
                "Path open failed.",
            )),
        );

        let result =
            controller.desktop_action_completed(OverviewDeferredActionKind::OpenGamePath, None);

        assert_eq!(result, OverviewTransitionResult::Applied);
        assert_eq!(controller.last_safe_action_error(), None);
    }

    #[test]
    fn overview_controller_desktop_action_failure_updates_last_safe_error() {
        let mut controller = OverviewController::new();

        let result = controller.desktop_action_completed(
            OverviewDeferredActionKind::OpenUpdateProvider(UpdateProvider::NexusMods),
            Some(OverviewActionError::new(
                OverviewDeferredActionKind::OpenUpdateProvider(UpdateProvider::NexusMods),
                "URL open failed.",
            )),
        );

        assert_eq!(result, OverviewTransitionResult::Applied);
        assert_eq!(
            controller
                .last_safe_action_error()
                .map(|error| error.summary.as_str()),
            Some("URL open failed.")
        );
    }

    #[test]
    fn overview_controller_spawn_failure_maps_to_safe_refresh_error() {
        let mut controller = OverviewController::new();
        let request = controller.request_refresh(UpdateSource::Nexus);

        let result = controller.refresh_spawn_failed(
            request.refresh_id,
            WorkerSpawnError::NoActiveRuntime {
                task_id: WorkerTaskId::new("overview-refresh-1"),
            },
        );

        assert_eq!(result, OverviewTransitionResult::Applied);
        assert_eq!(
            controller.snapshot().refresh.phase,
            OverviewRefreshPhase::Error
        );
        assert_eq!(
            controller.snapshot().refresh.message.as_deref(),
            Some("Overview refresh could not be started.")
        );
    }

    #[test]
    fn overview_controller_worker_panic_failure_event_maps_to_safe_refresh_error() {
        let mut controller = OverviewController::new();
        let request = controller.request_refresh(UpdateSource::Nexus);
        let event = WorkerEvent::failed(
            request.refresh_task(),
            WorkerFailure::new("Worker task panicked.").with_diagnostic("boom"),
        );

        let result = controller.handle_worker_event(event);

        assert_eq!(result, OverviewTransitionResult::Applied);
        assert_eq!(
            controller.snapshot().refresh.phase,
            OverviewRefreshPhase::Error
        );
        assert_eq!(
            controller.snapshot().refresh.message.as_deref(),
            Some("Worker task panicked.")
        );
    }

    #[test]
    fn overview_controller_worker_overview_payload_applies_refresh() {
        let mut controller = OverviewController::new();
        let request = controller.request_refresh(UpdateSource::Github);
        let event = WorkerEvent::completed(
            request.refresh_task(),
            WorkerPayload::Overview(OverviewWorkerPayload::refresh_completed(
                request.refresh_id,
                ready_snapshot("from worker"),
            )),
        );

        let result = controller.handle_worker_event(event);

        assert_eq!(result, OverviewTransitionResult::Applied);
        assert_eq!(
            controller.snapshot().refresh.message.as_deref(),
            Some("from worker")
        );
    }

    #[test]
    fn overview_controller_game_path_action_is_available_only_after_path_discovery() {
        let mut controller = OverviewController::new();
        let request = controller.request_refresh(UpdateSource::None);
        let mut snapshot = ready_snapshot("ready");
        snapshot.top.game_path =
            OverviewGamePathStatus::found(PathBuf::from(r"C:\Games\Fallout 4"));
        controller.refresh_completed(request.refresh_id, snapshot);

        let action = controller
            .game_path_action()
            .expect("discovered game path should be openable");

        assert_eq!(action.kind, OverviewDeferredActionKind::OpenGamePath);
        assert!(matches!(
            action.target,
            OverviewDeferredActionTarget::Path(_)
        ));
    }

    #[test]
    fn overview_controller_update_provider_action_tracks_visible_banner() {
        let mut controller = OverviewController::new();
        let request = controller.request_refresh(UpdateSource::Both);
        let mut snapshot = ready_snapshot("ready");
        snapshot.update_banner = UpdateBannerState::Available {
            selected_source: UpdateSource::Both,
            releases: vec![UpdateRelease::new(UpdateProvider::NexusMods, "1.2.3")],
        };
        controller.refresh_completed(request.refresh_id, snapshot);

        assert!(
            controller
                .update_provider_action(UpdateProvider::NexusMods)
                .is_some()
        );
        assert!(
            controller
                .update_provider_action(UpdateProvider::Github)
                .is_none()
        );
    }
}
