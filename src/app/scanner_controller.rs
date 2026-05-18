//! Slint-free Scanner-tab controller and worker-payload reducer.
//!
//! The controller owns transient checkbox state, scan lifecycle state, result
//! selection, file-list visibility, and read-only action feedback. It performs
//! no filesystem, clipboard, desktop, settings, or Slint work; production UI
//! code should save scanner settings at scan start, schedule the returned worker
//! request off the event loop, and feed owned worker events back through this
//! reducer.

use crate::{
    domain::{
        scanner::{
            PROGRESS_REFRESHING_OVERVIEW_TEXT, SCAN_BUTTON_LABEL, SCANNING_BUTTON_LABEL,
            ScannerActionDescriptor, ScannerActionFeedback, ScannerActionKind, ScannerCategoryKind,
            ScannerCategoryProjection, ScannerDetailRecord, ScannerFileList, ScannerResult,
            ScannerResultGroup, ScannerScanSnapshot, scanner_category_projection,
            scanner_result_count_text,
        },
        settings::ScannerSettings,
    },
    workers::{
        ScannerWorkerPayload, WorkerEvent, WorkerFailure, WorkerPayload, WorkerProgress,
        WorkerSpawnError, WorkerTask, WorkerTaskId, WorkerTaskKind, WorkerTaskStatus,
    },
};

/// Stable prefix for S07 Scanner scan worker task identifiers.
pub const SCANNER_SCAN_TASK_PREFIX: &str = "s07-scanner-scan:";
/// Safe status shown when a Scanner worker cannot be scheduled.
pub const SCANNER_SCAN_START_FAILED_MESSAGE: &str = "Scanner scan could not be started.";
/// Safe status shown when every scanner checkbox is disabled.
pub const SCANNER_NO_ENABLED_CATEGORIES_MESSAGE: &str = "No scanner categories are enabled.";
/// Safe generic text for invalid or unavailable scanner actions.
pub const SCANNER_ACTION_UNAVAILABLE_MESSAGE: &str = "Scanner action is not available.";
/// Safe status shown when a file-list action is invoked without file-list data.
pub const SCANNER_FILE_LIST_UNAVAILABLE_MESSAGE: &str =
    "No file list is available for the selected result.";

/// Monotonic identity assigned to each Scanner scan request.
pub type ScannerScanId = u64;

/// Render-relevant lifecycle state for the Scanner tab.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScannerControllerPhase {
    /// No scan has been requested yet.
    #[default]
    Idle,
    /// A worker is currently scanning.
    Scanning,
    /// A scan completed successfully, possibly with zero rows.
    Ready,
    /// A scan failed or could not start.
    Failed,
}

/// Result of applying a Scanner controller transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScannerTransitionResult {
    /// The event or intent matched current state and changed renderable data.
    Applied,
    /// The event belonged to an older scan and was intentionally ignored.
    StaleIgnored,
    /// The event was not relevant to this reducer.
    Ignored,
    /// The intent was recognized but unavailable in the current UI state.
    Rejected,
}

impl ScannerTransitionResult {
    /// Returns true when the transition changed controller state.
    pub const fn is_applied(self) -> bool {
        matches!(self, Self::Applied)
    }
}

/// Work request returned by Scanner scan intents and consumed by worker wiring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerScanWorkerRequest {
    /// Monotonic scan request id used to reject stale worker results.
    pub scan_id: ScannerScanId,
    /// Persisted scanner settings snapshot captured at scan start.
    pub settings_snapshot: ScannerSettings,
    /// Worker task metadata that must accompany lifecycle events.
    pub task: WorkerTask,
}

impl ScannerScanWorkerRequest {
    /// Creates a worker request for the supplied scan id and settings snapshot.
    pub fn new(scan_id: ScannerScanId, settings_snapshot: ScannerSettings) -> Self {
        Self {
            scan_id,
            settings_snapshot,
            task: scanner_scan_task(scan_id),
        }
    }
}

/// Render-ready detail state for the currently selected scanner result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerSelectedDetail {
    /// Flat result index selected by the UI.
    pub result_index: usize,
    /// Tree label for the selected result.
    pub tree_label: String,
    /// Detail-pane rows in reference order.
    pub records: Vec<ScannerDetailRecord>,
    /// Text copied by the Copy Details action.
    pub copy_details_text: String,
    /// Read-only actions available for the selected result.
    pub actions: Vec<ScannerActionDescriptor>,
    /// Optional file-list metadata attached to the selected result.
    pub file_list: Option<ScannerFileList>,
}

impl ScannerSelectedDetail {
    fn from_result(result_index: usize, result: &ScannerResult) -> Self {
        Self {
            result_index,
            tree_label: result.tree_label.clone(),
            records: result.detail_records(true),
            copy_details_text: result.copy_details_text(true),
            actions: result.read_only_actions(),
            file_list: result.file_list.clone(),
        }
    }

    fn action(&self, kind: ScannerActionKind) -> Option<ScannerActionDescriptor> {
        self.actions
            .iter()
            .find(|action| action.kind == kind && action.enabled)
            .cloned()
    }
}

/// Pure reducer for Scanner UI state and owned worker events.
#[derive(Debug, Clone, PartialEq)]
pub struct ScannerController {
    settings: ScannerSettings,
    phase: ScannerControllerPhase,
    next_scan_id: ScannerScanId,
    active_scan_id: Option<ScannerScanId>,
    latest_scan_id: Option<ScannerScanId>,
    progress_text: String,
    progress_current: Option<u64>,
    progress_total: Option<u64>,
    progress_percent: f32,
    status_text: String,
    result_count_text: String,
    results: Vec<ScannerResult>,
    groups: Vec<ScannerResultGroup>,
    selected_detail: Option<ScannerSelectedDetail>,
    file_list_visible: bool,
    visible_file_list: Option<ScannerFileList>,
    last_action_feedback: Option<ScannerActionFeedback>,
}

impl Default for ScannerController {
    fn default() -> Self {
        Self::new(ScannerSettings::default())
    }
}

impl ScannerController {
    /// Creates an idle Scanner controller from the current visible scanner settings.
    pub fn new(settings: ScannerSettings) -> Self {
        let scan_enabled = any_scanner_category_enabled(&settings);
        Self {
            settings,
            phase: ScannerControllerPhase::Idle,
            next_scan_id: 1,
            active_scan_id: None,
            latest_scan_id: None,
            progress_text: String::new(),
            progress_current: None,
            progress_total: None,
            progress_percent: 0.0,
            status_text: if scan_enabled {
                String::new()
            } else {
                SCANNER_NO_ENABLED_CATEGORIES_MESSAGE.to_owned()
            },
            result_count_text: scanner_result_count_text(0),
            results: Vec::new(),
            groups: Vec::new(),
            selected_detail: None,
            file_list_visible: false,
            visible_file_list: None,
            last_action_feedback: None,
        }
    }

    /// Returns the current transient scanner settings snapshot.
    pub fn settings(&self) -> &ScannerSettings {
        &self.settings
    }

    /// Replaces transient scanner settings with the snapshot Slint should display.
    pub fn replace_settings(&mut self, settings: ScannerSettings) {
        self.settings = settings;
        if !any_scanner_category_enabled(&self.settings)
            && self.phase != ScannerControllerPhase::Scanning
        {
            self.status_text = SCANNER_NO_ENABLED_CATEGORIES_MESSAGE.to_owned();
        }
    }

    /// Returns read-only category projections in reference display order.
    pub fn category_projection(&self) -> Vec<ScannerCategoryProjection> {
        scanner_category_projection(&self.settings)
    }

    /// Returns the current controller lifecycle phase.
    pub const fn phase(&self) -> ScannerControllerPhase {
        self.phase
    }

    /// Returns the active scan id, if a scan is currently in flight.
    pub const fn active_scan_id(&self) -> Option<ScannerScanId> {
        self.active_scan_id
    }

    /// Returns the latest scan id assigned by this controller.
    pub const fn latest_scan_id(&self) -> Option<ScannerScanId> {
        self.latest_scan_id
    }

    /// Returns the next scan id that will be assigned.
    pub const fn next_scan_id(&self) -> ScannerScanId {
        self.next_scan_id
    }

    /// Returns the user-facing scan button label.
    pub const fn scan_button_text(&self) -> &'static str {
        if matches!(self.phase, ScannerControllerPhase::Scanning) {
            SCANNING_BUTTON_LABEL
        } else {
            SCAN_BUTTON_LABEL
        }
    }

    /// Returns whether the Scan Game button should be enabled.
    pub fn scan_button_enabled(&self) -> bool {
        !matches!(self.phase, ScannerControllerPhase::Scanning)
            && any_scanner_category_enabled(&self.settings)
    }

    /// Returns the current safe status text.
    pub fn status_text(&self) -> &str {
        self.status_text.as_str()
    }

    /// Returns the current safe progress text.
    pub fn progress_text(&self) -> &str {
        self.progress_text.as_str()
    }

    /// Returns current optional progress counts.
    pub const fn progress_counts(&self) -> (Option<u64>, Option<u64>) {
        (self.progress_current, self.progress_total)
    }

    /// Returns the current progress percent in the 0-100 range.
    pub const fn progress_percent(&self) -> f32 {
        self.progress_percent
    }

    /// Returns reference-shaped result-count text.
    pub fn result_count_text(&self) -> &str {
        self.result_count_text.as_str()
    }

    /// Returns flat result rows.
    pub fn results(&self) -> &[ScannerResult] {
        &self.results
    }

    /// Returns grouped result rows.
    pub fn groups(&self) -> &[ScannerResultGroup] {
        &self.groups
    }

    /// Returns the selected detail state, if a result row is selected.
    pub fn selected_detail(&self) -> Option<&ScannerSelectedDetail> {
        self.selected_detail.as_ref()
    }

    /// Returns whether a file-list panel/dialog should be visible.
    pub const fn file_list_visible(&self) -> bool {
        self.file_list_visible
    }

    /// Returns the currently visible file-list metadata, if any.
    pub fn visible_file_list(&self) -> Option<&ScannerFileList> {
        self.visible_file_list.as_ref()
    }

    /// Returns the last safe read-only action feedback, if any.
    pub fn last_action_feedback(&self) -> Option<&ScannerActionFeedback> {
        self.last_action_feedback.as_ref()
    }

    /// Applies a transient checkbox toggle without persisting settings.
    pub fn toggle_category(
        &mut self,
        category: ScannerCategoryKind,
        enabled: bool,
    ) -> ScannerTransitionResult {
        let target = scanner_setting_mut(&mut self.settings, category);
        if *target == enabled {
            return ScannerTransitionResult::Ignored;
        }

        *target = enabled;
        if !any_scanner_category_enabled(&self.settings)
            && !matches!(self.phase, ScannerControllerPhase::Scanning)
        {
            self.status_text = SCANNER_NO_ENABLED_CATEGORIES_MESSAGE.to_owned();
        } else if self.status_text == SCANNER_NO_ENABLED_CATEGORIES_MESSAGE {
            self.status_text.clear();
        }

        tracing::debug!(
            event = "s07-scanner-toggle-updated",
            category = category.label(),
            enabled,
            scan_button_enabled = self.scan_button_enabled(),
            "Scanner checkbox updated as transient state"
        );
        ScannerTransitionResult::Applied
    }

    /// Requests a new scan and returns the worker request to schedule.
    pub fn request_scan(&mut self) -> Option<ScannerScanWorkerRequest> {
        if matches!(self.phase, ScannerControllerPhase::Scanning) {
            tracing::debug!(
                event = "s07-scanner-scan-request-ignored",
                active_scan_id = ?self.active_scan_id,
                "Scanner scan request ignored because a scan is already active"
            );
            return None;
        }

        if !any_scanner_category_enabled(&self.settings) {
            self.status_text = SCANNER_NO_ENABLED_CATEGORIES_MESSAGE.to_owned();
            tracing::warn!(
                event = "s07-scanner-scan-request-rejected",
                reason = "no-enabled-categories",
                "Scanner scan request rejected because all categories are disabled"
            );
            return None;
        }

        let scan_id = self.next_scan_id;
        self.next_scan_id = self.next_scan_id.saturating_add(1);
        self.active_scan_id = Some(scan_id);
        self.latest_scan_id = Some(scan_id);
        self.phase = ScannerControllerPhase::Scanning;
        self.progress_text = PROGRESS_REFRESHING_OVERVIEW_TEXT.to_owned();
        self.progress_current = None;
        self.progress_total = None;
        self.progress_percent = 0.0;
        self.status_text = "Scanning...".to_owned();
        self.clear_selection();
        self.results.clear();
        self.groups.clear();
        self.result_count_text = scanner_result_count_text(0);
        self.last_action_feedback = None;

        let request = ScannerScanWorkerRequest::new(scan_id, self.settings.clone());
        tracing::info!(
            event = "s07-scanner-scan-requested",
            scan_id,
            task_id = %request.task.id,
            "Scanner scan requested"
        );
        Some(request)
    }

    /// Applies the running transition once worker scheduling succeeds.
    pub fn scan_started(&mut self, scan_id: ScannerScanId) -> ScannerTransitionResult {
        if !self.is_active_scan(scan_id) {
            tracing::debug!(
                event = "s07-scanner-scan-start-stale-ignored",
                scan_id,
                active_scan_id = ?self.active_scan_id,
                "Ignoring stale Scanner scan start"
            );
            return ScannerTransitionResult::StaleIgnored;
        }

        self.phase = ScannerControllerPhase::Scanning;
        self.status_text = "Scanning...".to_owned();
        if self.progress_text.is_empty() {
            self.progress_text = PROGRESS_REFRESHING_OVERVIEW_TEXT.to_owned();
        }
        tracing::info!(
            event = "s07-scanner-scan-started",
            scan_id,
            "Scanner scan started"
        );
        ScannerTransitionResult::Applied
    }

    /// Applies progress text/counts for the active scan.
    pub fn scan_progress(
        &mut self,
        scan_id: ScannerScanId,
        progress: WorkerProgress,
    ) -> ScannerTransitionResult {
        if !self.is_active_scan(scan_id) {
            tracing::debug!(
                event = "s07-scanner-progress-stale-ignored",
                scan_id,
                active_scan_id = ?self.active_scan_id,
                "Ignoring stale Scanner progress"
            );
            return ScannerTransitionResult::StaleIgnored;
        }

        if let Some(message) = progress.message {
            self.progress_text = message;
        }
        self.progress_current = progress.current;
        self.progress_total = progress.total;
        self.progress_percent = match (progress.current, progress.total) {
            (Some(current), Some(total)) if total > 0 => (current as f32 / total as f32) * 100.0,
            _ => self.progress_percent,
        };

        tracing::debug!(
            event = "s07-scanner-progress-applied",
            scan_id,
            current = ?self.progress_current,
            total = ?self.progress_total,
            progress_text = self.progress_text.as_str(),
            "Scanner progress applied"
        );
        ScannerTransitionResult::Applied
    }

    /// Applies a worker snapshot if it belongs to the active Scanner scan.
    pub fn scan_completed(
        &mut self,
        scan_id: ScannerScanId,
        snapshot: ScannerScanSnapshot,
    ) -> ScannerTransitionResult {
        if !self.is_active_scan(scan_id) || snapshot.scan_id != scan_id {
            tracing::debug!(
                event = "s07-scanner-completion-stale-ignored",
                scan_id,
                snapshot_scan_id = snapshot.scan_id,
                active_scan_id = ?self.active_scan_id,
                "Ignoring stale Scanner completion"
            );
            return ScannerTransitionResult::StaleIgnored;
        }

        let result_count = snapshot.result_count();
        self.phase = ScannerControllerPhase::Ready;
        self.active_scan_id = None;
        self.status_text = snapshot.status_text.clone();
        self.progress_text = snapshot.status_text.clone();
        self.progress_current = Some(result_count as u64);
        self.progress_total = Some(result_count as u64);
        self.progress_percent = 100.0;
        self.result_count_text = snapshot.result_count_text;
        self.results = snapshot.results;
        self.groups = snapshot.groups;
        self.clear_selection();

        tracing::info!(
            event = "s07-scanner-scan-completed",
            scan_id,
            result_count,
            group_count = self.groups.len(),
            status_text = self.status_text.as_str(),
            "Scanner scan completed"
        );
        ScannerTransitionResult::Applied
    }

    /// Maps a worker failure into a safe failed Scanner state for the active scan.
    pub fn scan_failed(
        &mut self,
        scan_id: ScannerScanId,
        failure: WorkerFailure,
    ) -> ScannerTransitionResult {
        if !self.is_active_scan(scan_id) {
            tracing::debug!(
                event = "s07-scanner-failure-stale-ignored",
                scan_id,
                active_scan_id = ?self.active_scan_id,
                "Ignoring stale Scanner failure"
            );
            return ScannerTransitionResult::StaleIgnored;
        }

        tracing::error!(
            event = "s07-scanner-scan-failed",
            scan_id,
            safe_message = failure.safe_message(),
            diagnostic = failure.diagnostic().unwrap_or(""),
            "Scanner scan failed"
        );
        self.phase = ScannerControllerPhase::Failed;
        self.active_scan_id = None;
        self.status_text = failure.safe_message().to_owned();
        self.progress_text = failure.safe_message().to_owned();
        self.progress_current = None;
        self.progress_total = None;
        self.progress_percent = 0.0;
        self.clear_selection();
        ScannerTransitionResult::Applied
    }

    /// Maps a worker spawn failure into a safe failed Scanner state.
    pub fn spawn_failed(
        &mut self,
        scan_id: ScannerScanId,
        error: WorkerSpawnError,
    ) -> ScannerTransitionResult {
        tracing::error!(
            event = "s07-scanner-spawn-failed",
            scan_id,
            diagnostic = %error,
            "Scanner scan worker could not be scheduled"
        );
        self.scan_failed(
            scan_id,
            WorkerFailure::new(SCANNER_SCAN_START_FAILED_MESSAGE)
                .with_diagnostic(error.to_string()),
        )
    }

    /// Selects a flat result row and prepares details/actions; missing rows clear details safely.
    pub fn select_result(&mut self, result_index: usize) -> ScannerTransitionResult {
        let Some(result) = self.results.get(result_index) else {
            self.clear_selection();
            tracing::debug!(
                event = "s07-scanner-selection-cleared",
                result_index,
                "Scanner selection cleared because the row was missing"
            );
            return ScannerTransitionResult::Applied;
        };

        self.selected_detail = Some(ScannerSelectedDetail::from_result(result_index, result));
        self.file_list_visible = false;
        self.visible_file_list = None;
        tracing::debug!(
            event = "s07-scanner-result-selected",
            result_index,
            problem_type = result.problem_type.label(),
            action_count = self
                .selected_detail
                .as_ref()
                .map(|detail| detail.actions.len())
                .unwrap_or_default(),
            "Scanner result selected"
        );
        ScannerTransitionResult::Applied
    }

    /// Returns an enabled selected action for a stable action id, recording safe feedback if absent.
    pub fn request_selected_action(&mut self, action_id: &str) -> Option<ScannerActionDescriptor> {
        let Some(kind) = ScannerActionKind::from_id(action_id) else {
            self.last_action_feedback = Some(ScannerActionFeedback::failed(
                self.latest_scan_id,
                ScannerActionKind::CopyDetails,
                SCANNER_ACTION_UNAVAILABLE_MESSAGE,
            ));
            tracing::warn!(
                event = "s07-scanner-action-invalid",
                action_id,
                "Scanner action id was invalid"
            );
            return None;
        };

        let action = self
            .selected_detail
            .as_ref()
            .and_then(|detail| detail.action(kind));
        if action.is_none() {
            self.last_action_feedback = Some(ScannerActionFeedback::failed(
                self.latest_scan_id,
                kind,
                SCANNER_ACTION_UNAVAILABLE_MESSAGE,
            ));
            tracing::warn!(
                event = "s07-scanner-action-unavailable",
                action = kind.as_id(),
                "Scanner action is unavailable for the current selection"
            );
        }
        action
    }

    /// Toggles the selected result's file-list visibility when file-list data exists.
    pub fn toggle_file_list(&mut self) -> ScannerTransitionResult {
        let Some(file_list) = self
            .selected_detail
            .as_ref()
            .and_then(|detail| detail.file_list.clone())
            .filter(|file_list| !file_list.is_empty())
        else {
            self.file_list_visible = false;
            self.visible_file_list = None;
            self.last_action_feedback = Some(ScannerActionFeedback::failed(
                self.latest_scan_id,
                ScannerActionKind::ShowFileList,
                SCANNER_FILE_LIST_UNAVAILABLE_MESSAGE,
            ));
            tracing::warn!(
                event = "s07-scanner-file-list-unavailable",
                "Scanner file-list toggle rejected because no file list is selected"
            );
            return ScannerTransitionResult::Rejected;
        };

        if self.file_list_visible {
            self.file_list_visible = false;
            self.visible_file_list = None;
        } else {
            self.file_list_visible = true;
            self.visible_file_list = Some(file_list);
        }
        self.last_action_feedback = Some(ScannerActionFeedback::succeeded(
            self.latest_scan_id,
            ScannerActionKind::ShowFileList,
            "File list updated.",
        ));
        ScannerTransitionResult::Applied
    }

    /// Applies safe read-only action feedback, ignoring stale scan ids.
    pub fn action_completed(&mut self, feedback: ScannerActionFeedback) -> ScannerTransitionResult {
        if feedback
            .scan_id
            .is_some_and(|scan_id| Some(scan_id) != self.latest_scan_id)
        {
            tracing::debug!(
                event = "s07-scanner-action-stale-ignored",
                feedback_scan_id = ?feedback.scan_id,
                latest_scan_id = ?self.latest_scan_id,
                "Ignoring stale Scanner action feedback"
            );
            return ScannerTransitionResult::StaleIgnored;
        }

        if feedback.succeeded {
            tracing::info!(
                event = "s07-scanner-action-completed",
                action = feedback.action.as_id(),
                scan_id = ?feedback.scan_id,
                safe_message = feedback.safe_message(),
                "Scanner read-only action completed"
            );
        } else {
            tracing::warn!(
                event = "s07-scanner-action-failed",
                action = feedback.action.as_id(),
                scan_id = ?feedback.scan_id,
                safe_message = feedback.safe_message(),
                diagnostic = feedback.diagnostic.as_deref().unwrap_or(""),
                "Scanner read-only action failed"
            );
        }
        self.last_action_feedback = Some(feedback);
        ScannerTransitionResult::Applied
    }

    /// Applies an owned worker event if it carries a Scanner scan/action payload.
    pub fn handle_worker_event(&mut self, event: WorkerEvent) -> ScannerTransitionResult {
        let task = event.task;
        let status = event.status;
        match event.payload {
            WorkerPayload::Scanner(payload) => self.handle_scanner_payload(task, status, payload),
            WorkerPayload::Progress(progress)
                if task.kind == WorkerTaskKind::Scan && status == WorkerTaskStatus::Progress =>
            {
                match scanner_scan_id_from_task_id(&task.id) {
                    Some(scan_id) => self.scan_progress(scan_id, progress),
                    None => ScannerTransitionResult::Ignored,
                }
            }
            WorkerPayload::Error(failure)
                if task.kind == WorkerTaskKind::Scan && status == WorkerTaskStatus::Failed =>
            {
                match scanner_scan_id_from_task_id(&task.id) {
                    Some(scan_id) => self.scan_failed(scan_id, failure),
                    None => ScannerTransitionResult::Ignored,
                }
            }
            WorkerPayload::None
                if task.kind == WorkerTaskKind::Scan && status == WorkerTaskStatus::Running =>
            {
                match scanner_scan_id_from_task_id(&task.id) {
                    Some(scan_id) => self.scan_started(scan_id),
                    None => ScannerTransitionResult::Ignored,
                }
            }
            _ => ScannerTransitionResult::Ignored,
        }
    }

    fn handle_scanner_payload(
        &mut self,
        task: WorkerTask,
        status: WorkerTaskStatus,
        payload: ScannerWorkerPayload,
    ) -> ScannerTransitionResult {
        match payload {
            ScannerWorkerPayload::ScanCompleted { scan_id, snapshot }
                if task.kind == WorkerTaskKind::Scan
                    && status == WorkerTaskStatus::Completed
                    && scanner_scan_id_from_task_id(&task.id) == Some(scan_id) =>
            {
                self.scan_completed(scan_id, *snapshot)
            }
            ScannerWorkerPayload::ActionCompleted { feedback }
                if status == WorkerTaskStatus::Completed =>
            {
                self.action_completed(feedback)
            }
            _ => ScannerTransitionResult::Ignored,
        }
    }

    fn is_active_scan(&self, scan_id: ScannerScanId) -> bool {
        self.active_scan_id == Some(scan_id)
    }

    fn clear_selection(&mut self) {
        self.selected_detail = None;
        self.file_list_visible = false;
        self.visible_file_list = None;
    }
}

/// Builds worker metadata for a blocking Scanner scan.
pub fn scanner_scan_task(scan_id: ScannerScanId) -> WorkerTask {
    WorkerTask::new(
        format!("{SCANNER_SCAN_TASK_PREFIX}{scan_id}"),
        WorkerTaskKind::Scan,
    )
    .with_label("Scan Game")
}

/// Converts a completed scan snapshot into the Scanner worker payload shape.
pub fn scanner_scan_completed_payload(
    scan_id: ScannerScanId,
    snapshot: ScannerScanSnapshot,
) -> WorkerPayload {
    WorkerPayload::Scanner(ScannerWorkerPayload::scan_completed(scan_id, snapshot))
}

/// Converts read-only action feedback into the Scanner worker payload shape.
pub fn scanner_action_completed_payload(feedback: ScannerActionFeedback) -> WorkerPayload {
    WorkerPayload::Scanner(ScannerWorkerPayload::action_completed(feedback))
}

/// Parses an S07 Scanner scan id from a worker task id.
pub fn scanner_scan_id_from_task_id(task_id: &WorkerTaskId) -> Option<ScannerScanId> {
    task_id
        .as_str()
        .strip_prefix(SCANNER_SCAN_TASK_PREFIX)
        .and_then(|value| value.parse::<ScannerScanId>().ok())
}

/// Returns true when at least one scanner category is enabled.
pub const fn any_scanner_category_enabled(settings: &ScannerSettings) -> bool {
    settings.overview_issues
        || settings.errors
        || settings.wrong_format
        || settings.loose_previs
        || settings.junk_files
        || settings.problem_overrides
        || settings.race_subgraphs
}

fn scanner_setting_mut(settings: &mut ScannerSettings, category: ScannerCategoryKind) -> &mut bool {
    match category {
        ScannerCategoryKind::OverviewIssues => &mut settings.overview_issues,
        ScannerCategoryKind::Errors => &mut settings.errors,
        ScannerCategoryKind::WrongFormat => &mut settings.wrong_format,
        ScannerCategoryKind::LoosePrevis => &mut settings.loose_previs,
        ScannerCategoryKind::JunkFiles => &mut settings.junk_files,
        ScannerCategoryKind::ProblemOverrides => &mut settings.problem_overrides,
        ScannerCategoryKind::RaceSubgraphs => &mut settings.race_subgraphs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::scanner::{
        ACTION_COPY_DETAILS_LABEL, ScannerExtraData, ScannerFileList, ScannerFileListEntry,
        ScannerProblemType, ScannerSolutionKind, group_scanner_results,
    };

    fn example_result(detail_path: &str) -> ScannerResult {
        ScannerResult::with_path(
            ScannerProblemType::JunkFile,
            format!("C:/Games/Fallout 4/Data/{detail_path}"),
            detail_path,
            "This is a junk file not used by the game or mod managers.",
            Some(ScannerSolutionKind::DeleteOrIgnoreFile.into_solution_text()),
        )
    }

    fn completed_snapshot(
        scan_id: ScannerScanId,
        results: Vec<ScannerResult>,
    ) -> ScannerScanSnapshot {
        ScannerScanSnapshot::from_results(scan_id, results, "Scanner completed.")
    }

    #[test]
    fn scanner_controller_toggles_are_transient_and_request_uses_owned_snapshot() {
        let mut controller = ScannerController::new(ScannerSettings::default());

        assert_eq!(
            controller.toggle_category(ScannerCategoryKind::WrongFormat, false),
            ScannerTransitionResult::Applied
        );
        assert!(!controller.settings().wrong_format);

        let request = controller.request_scan().expect("scan should be requested");

        assert_eq!(request.scan_id, 1);
        assert_eq!(request.task.id.as_str(), "s07-scanner-scan:1");
        assert_eq!(request.task.kind, WorkerTaskKind::Scan);
        assert!(!request.settings_snapshot.wrong_format);
        assert_eq!(controller.scan_button_text(), SCANNING_BUTTON_LABEL);
        assert!(!controller.scan_button_enabled());
    }

    #[test]
    fn scanner_controller_all_toggles_off_disables_scan() {
        let settings = ScannerSettings {
            overview_issues: false,
            errors: false,
            wrong_format: false,
            loose_previs: false,
            junk_files: false,
            problem_overrides: false,
            race_subgraphs: false,
        };
        let mut controller = ScannerController::new(settings);

        assert!(!controller.scan_button_enabled());
        assert_eq!(
            controller.status_text(),
            SCANNER_NO_ENABLED_CATEGORIES_MESSAGE
        );
        assert!(controller.request_scan().is_none());
        assert_eq!(controller.next_scan_id(), 1);
    }

    #[test]
    fn scanner_controller_completion_sets_zero_result_text_and_restores_button() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        controller.scan_started(request.scan_id);
        let snapshot = ScannerScanSnapshot::empty(request.scan_id, "Scanner completed.");

        let result = controller.scan_completed(request.scan_id, snapshot);

        assert_eq!(result, ScannerTransitionResult::Applied);
        assert_eq!(controller.phase(), ScannerControllerPhase::Ready);
        assert_eq!(controller.active_scan_id(), None);
        assert_eq!(controller.scan_button_text(), SCAN_BUTTON_LABEL);
        assert!(controller.scan_button_enabled());
        assert_eq!(
            controller.result_count_text(),
            "0 Results ~ Select an item for details"
        );
        assert_eq!(controller.progress_counts(), (Some(0), Some(0)));
        assert_eq!(controller.progress_percent(), 100.0);
    }

    #[test]
    fn scanner_controller_ignores_stale_progress_completion_and_action_events() {
        let mut controller = ScannerController::default();
        let first = controller.request_scan().expect("first scan should start");
        controller.scan_completed(
            first.scan_id,
            ScannerScanSnapshot::empty(first.scan_id, "Done."),
        );
        let second = controller.request_scan().expect("second scan should start");

        let stale_progress = controller.handle_worker_event(WorkerEvent::progress(
            first.task.clone(),
            WorkerProgress::new().with_message("old progress"),
        ));
        let stale_completion = controller.handle_worker_event(WorkerEvent::completed(
            first.task,
            scanner_scan_completed_payload(
                first.scan_id,
                ScannerScanSnapshot::empty(first.scan_id, "Old completion."),
            ),
        ));
        let stale_action = controller.action_completed(ScannerActionFeedback::succeeded(
            Some(first.scan_id),
            ScannerActionKind::CopyDetails,
            "Old action.",
        ));

        assert_eq!(second.scan_id, 2);
        assert_eq!(stale_progress, ScannerTransitionResult::StaleIgnored);
        assert_eq!(stale_completion, ScannerTransitionResult::StaleIgnored);
        assert_eq!(stale_action, ScannerTransitionResult::StaleIgnored);
        assert_ne!(controller.progress_text(), "old progress");
        assert!(controller.last_action_feedback().is_none());
    }

    #[test]
    fn scanner_controller_spawn_failure_restores_scan_button_and_safe_error_text() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        let raw_error = WorkerSpawnError::NoActiveRuntime {
            task_id: request.task.id.clone(),
        };

        let result = controller.spawn_failed(request.scan_id, raw_error);

        assert_eq!(result, ScannerTransitionResult::Applied);
        assert_eq!(controller.phase(), ScannerControllerPhase::Failed);
        assert_eq!(controller.active_scan_id(), None);
        assert_eq!(controller.scan_button_text(), SCAN_BUTTON_LABEL);
        assert!(controller.scan_button_enabled());
        assert_eq!(controller.status_text(), SCANNER_SCAN_START_FAILED_MESSAGE);
        assert!(!controller.status_text().contains("Tokio"));
    }

    #[test]
    fn scanner_controller_selecting_missing_result_clears_details_safely() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            request.scan_id,
            completed_snapshot(request.scan_id, vec![example_result("desktop.ini")]),
        );
        assert_eq!(
            controller.select_result(0),
            ScannerTransitionResult::Applied
        );
        assert!(controller.selected_detail().is_some());

        let result = controller.select_result(99);

        assert_eq!(result, ScannerTransitionResult::Applied);
        assert!(controller.selected_detail().is_none());
        assert!(!controller.file_list_visible());
        assert!(controller.visible_file_list().is_none());
    }

    #[test]
    fn scanner_controller_invalid_action_and_file_list_without_file_list_are_safe() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            request.scan_id,
            completed_snapshot(request.scan_id, vec![example_result("desktop.ini")]),
        );
        controller.select_result(0);

        assert!(
            controller
                .request_selected_action("not-an-action")
                .is_none()
        );
        assert_eq!(
            controller
                .last_action_feedback()
                .map(ScannerActionFeedback::safe_message),
            Some(SCANNER_ACTION_UNAVAILABLE_MESSAGE)
        );

        let result = controller.toggle_file_list();

        assert_eq!(result, ScannerTransitionResult::Rejected);
        assert!(!controller.file_list_visible());
        assert_eq!(
            controller
                .last_action_feedback()
                .map(ScannerActionFeedback::safe_message),
            Some(SCANNER_FILE_LIST_UNAVAILABLE_MESSAGE)
        );
    }

    #[test]
    fn scanner_controller_file_list_toggle_and_copy_open_action_feedback_are_safe() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        let result_with_file_list = ScannerResult::simple(
            ScannerProblemType::RaceSubgraphRecordCount,
            "Example.esp",
            "Example.esp has race animation subgraph records.",
            Some("Review the record count.".to_owned()),
        )
        .with_file_list(ScannerFileList::race_subgraph_records(vec![
            ScannerFileListEntry::new(101, "C:/Games/Fallout 4/Data/Example.esp"),
        ]))
        .with_extra_data(vec![ScannerExtraData::url("https://example.invalid/help")]);
        controller.scan_completed(
            request.scan_id,
            completed_snapshot(request.scan_id, vec![result_with_file_list]),
        );
        controller.select_result(0);

        let show_result = controller.toggle_file_list();
        let open_feedback = ScannerActionFeedback::failed(
            controller.latest_scan_id(),
            ScannerActionKind::OpenLocation,
            "Location could not be opened.",
        )
        .with_diagnostic("raw OS detail");
        let action_result = controller.action_completed(open_feedback);

        assert_eq!(show_result, ScannerTransitionResult::Applied);
        assert!(controller.file_list_visible());
        assert_eq!(
            controller
                .visible_file_list()
                .map(|file_list| file_list.title.as_str()),
            Some("Race Animation Subgraph Records")
        );
        assert_eq!(action_result, ScannerTransitionResult::Applied);
        assert_eq!(
            controller
                .last_action_feedback()
                .map(ScannerActionFeedback::safe_message),
            Some("Location could not be opened.")
        );
        assert!(
            !controller
                .last_action_feedback()
                .expect("feedback should exist")
                .safe_message()
                .contains("raw OS")
        );
    }

    #[test]
    fn scanner_controller_worker_events_apply_only_scanner_scan_payloads() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        let progress = WorkerEvent::progress(
            request.task.clone(),
            WorkerProgress::new()
                .with_message("Scanning... 1/2: meshes")
                .with_counts(Some(1), Some(2)),
        );
        let completion = WorkerEvent::completed(
            request.task,
            scanner_scan_completed_payload(
                request.scan_id,
                completed_snapshot(request.scan_id, vec![example_result("desktop.ini")]),
            ),
        );

        assert_eq!(
            controller.handle_worker_event(progress),
            ScannerTransitionResult::Applied
        );
        assert_eq!(controller.progress_text(), "Scanning... 1/2: meshes");
        assert_eq!(controller.progress_counts(), (Some(1), Some(2)));
        assert_eq!(
            controller.handle_worker_event(completion),
            ScannerTransitionResult::Applied
        );
        assert_eq!(controller.results().len(), 1);
        assert_eq!(controller.groups().len(), 1);
        assert_eq!(controller.groups()[0].label, "Junk File");
    }

    #[test]
    fn scanner_controller_scan_task_ids_are_stable_and_duplicate_free() {
        let mut controller = ScannerController::default();
        let first = controller.request_scan().expect("first scan should start");
        controller.scan_completed(
            first.scan_id,
            ScannerScanSnapshot::empty(first.scan_id, "Done."),
        );
        let second = controller.request_scan().expect("second scan should start");

        assert_eq!(first.scan_id, 1);
        assert_eq!(second.scan_id, 2);
        assert_ne!(first.task.id, second.task.id);
        assert_eq!(scanner_scan_id_from_task_id(&first.task.id), Some(1));
        assert_eq!(scanner_scan_id_from_task_id(&second.task.id), Some(2));
        assert_eq!(
            scanner_scan_id_from_task_id(&WorkerTaskId::new("scan:2")),
            None
        );
    }

    #[test]
    fn scanner_controller_copy_details_action_is_available_for_selected_result() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            request.scan_id,
            completed_snapshot(request.scan_id, vec![example_result("desktop.ini")]),
        );
        controller.select_result(0);

        let action = controller
            .request_selected_action(ScannerActionKind::CopyDetails.as_id())
            .expect("copy details should be available");

        assert_eq!(action.label, ACTION_COPY_DETAILS_LABEL);
        assert!(action.enabled);
        assert!(
            controller
                .selected_detail()
                .expect("detail should exist")
                .copy_details_text
                .contains("desktop.ini")
        );
    }

    #[test]
    fn scanner_controller_groups_completion_results_when_snapshot_is_flat() {
        let snapshot = ScannerScanSnapshot::from_results(
            7,
            vec![
                example_result("z-last.ini"),
                ScannerResult::simple(
                    ScannerProblemType::LimitExceeded,
                    "300 General Archives",
                    "You have 300 General Archives enabled. The limit is 256.",
                    None,
                ),
            ],
            "Scanner completed.",
        );

        assert_eq!(snapshot.result_count_text, scanner_result_count_text(2));
        assert_eq!(
            snapshot
                .groups
                .iter()
                .map(|group| group.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Junk File", "Limit Exceeded"]
        );
        assert_eq!(group_scanner_results(&snapshot.results), snapshot.groups);
    }
}
