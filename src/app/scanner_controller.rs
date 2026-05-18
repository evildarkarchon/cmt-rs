//! Slint-free Scanner-tab controller and worker-payload reducer.
//!
//! The controller owns transient checkbox state, scan lifecycle state, result
//! selection, file-list visibility, and read-only action feedback. It performs
//! no filesystem, clipboard, desktop, settings, or Slint work; production UI
//! code should save scanner settings at scan start, schedule the returned worker
//! request off the event loop, and feed owned worker events back through this
//! reducer.

use std::collections::BTreeMap;

use crate::{
    domain::{
        autofix::{
            AutoFixButtonState, AutoFixCompletion, AutoFixOperationKey, AutoFixResultDetail,
            AutoFixSelectionIdentity, AutoFixStatus, AutoFixStatusKind,
        },
        scanner::{
            PROGRESS_REFRESHING_OVERVIEW_TEXT, SCAN_BUTTON_LABEL, SCANNING_BUTTON_LABEL,
            ScannerActionDescriptor, ScannerActionFeedback, ScannerActionKind, ScannerCategoryKind,
            ScannerCategoryProjection, ScannerDetailRecord, ScannerFileList, ScannerResult,
            ScannerResultGroup, ScannerScanSnapshot, scanner_category_projection,
            scanner_result_count_text,
        },
        settings::ScannerSettings,
    },
    services::autofix::AutoFixSupportCatalog,
    workers::{
        ScannerWorkerPayload, WorkerEvent, WorkerFailure, WorkerPayload, WorkerProgress,
        WorkerSpawnError, WorkerTask, WorkerTaskId, WorkerTaskKind, WorkerTaskStatus,
    },
};

/// Stable prefix for S07 Scanner scan worker task identifiers.
pub const SCANNER_SCAN_TASK_PREFIX: &str = "s07-scanner-scan:";
/// Stable prefix for S08 Scanner Auto-Fix worker task identifiers.
pub const SCANNER_AUTO_FIX_TASK_PREFIX: &str = "s08-scanner-autofix:";
/// Safe status shown when a Scanner worker cannot be scheduled.
pub const SCANNER_SCAN_START_FAILED_MESSAGE: &str = "Scanner scan could not be started.";
/// Safe status shown when a Scanner Auto-Fix worker cannot be scheduled.
pub const SCANNER_AUTO_FIX_START_FAILED_MESSAGE: &str = "Auto-Fix could not be started.";
/// Safe status shown when Scanner Auto-Fix is invoked without a selected row.
pub const SCANNER_AUTO_FIX_NO_SELECTION_MESSAGE: &str =
    "Select a scanner result before using Auto-Fix.";
/// Safe status shown when Scanner Auto-Fix is unavailable for the selected row.
pub const SCANNER_AUTO_FIX_UNAVAILABLE_MESSAGE: &str = "Auto-Fix is not available for this result.";
/// Safe status shown when Scanner Auto-Fix input no longer matches the visible scan.
pub const SCANNER_AUTO_FIX_STALE_MESSAGE: &str =
    "Auto-Fix could not run because the scan results changed. Scan again and retry.";
/// Safe status shown while a Scanner Auto-Fix operation is executing.
pub const SCANNER_AUTO_FIX_RUNNING_MESSAGE: &str = "Auto-Fix is running...";
/// Safe status shown when a Scanner Auto-Fix worker reports failure without text.
pub const SCANNER_AUTO_FIX_FAILED_MESSAGE: &str = "Auto-Fix could not complete this operation.";
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

/// Work request returned by accepted Scanner Auto-Fix intents and consumed by worker wiring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerAutoFixWorkerRequest {
    /// Scan id the selected result belonged to when the request was accepted.
    pub scan_id: ScannerScanId,
    /// Flat scanner result index selected by the UI.
    pub result_index: usize,
    /// Identity captured from the selected result at request time.
    pub selection_identity: AutoFixSelectionIdentity,
    /// Typed operation key resolved from retained scanner solution identity.
    pub operation_key: AutoFixOperationKey,
    /// Worker task metadata that must accompany lifecycle events.
    pub task: WorkerTask,
}

impl ScannerAutoFixWorkerRequest {
    /// Creates an owned Auto-Fix worker request for the selected Scanner result.
    pub fn new(
        scan_id: ScannerScanId,
        result_index: usize,
        selection_identity: AutoFixSelectionIdentity,
        operation_key: AutoFixOperationKey,
    ) -> Self {
        Self {
            scan_id,
            result_index,
            selection_identity,
            operation_key,
            task: scanner_auto_fix_task(scan_id, result_index, operation_key),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ScannerAutoFixResultKey {
    scan_id: ScannerScanId,
    result_index: usize,
    selection_identity: AutoFixSelectionIdentity,
}

impl ScannerAutoFixResultKey {
    fn new(
        scan_id: ScannerScanId,
        result_index: usize,
        selection_identity: AutoFixSelectionIdentity,
    ) -> Self {
        Self {
            scan_id,
            result_index,
            selection_identity,
        }
    }
}

/// Render-ready Auto-Fix state for the currently selected Scanner result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerAutoFixRenderState {
    /// Scan id the selected result belongs to.
    pub scan_id: ScannerScanId,
    /// Flat scanner result index selected by the UI.
    pub result_index: usize,
    /// Typed operation key resolved without display-string matching.
    pub operation_key: AutoFixOperationKey,
    /// Selected result identity used to reject stale/tampered events.
    pub selection_identity: AutoFixSelectionIdentity,
    /// Render-ready button state.
    pub button: AutoFixButtonState,
    /// Last safe status for this Auto-Fix lifecycle.
    pub status: AutoFixStatus,
    /// Inline `Auto-Fix Results` details shown after completion or failure.
    pub result_detail: Option<AutoFixResultDetail>,
    /// Whether this row should render as fixed.
    pub row_fixed: bool,
    /// Whether this row should render the checked state.
    pub row_checked: bool,
}

impl ScannerAutoFixRenderState {
    fn ready(
        scan_id: ScannerScanId,
        result_index: usize,
        operation_key: AutoFixOperationKey,
        selection_identity: AutoFixSelectionIdentity,
        safe_preview: impl Into<String>,
    ) -> Self {
        Self {
            scan_id,
            result_index,
            operation_key,
            selection_identity,
            button: AutoFixButtonState::from_status(AutoFixStatusKind::Ready),
            status: AutoFixStatus::new(AutoFixStatusKind::Ready, safe_preview),
            result_detail: None,
            row_fixed: false,
            row_checked: false,
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
    /// Auto-Fix state exposed only when the typed solution is supported.
    pub auto_fix: Option<ScannerAutoFixRenderState>,
}

impl ScannerSelectedDetail {
    fn from_result(
        result_index: usize,
        result: &ScannerResult,
        auto_fix: Option<ScannerAutoFixRenderState>,
    ) -> Self {
        Self {
            result_index,
            tree_label: result.tree_label.clone(),
            records: result.detail_records(true),
            copy_details_text: result.copy_details_text(true),
            actions: result.read_only_actions(),
            file_list: result.file_list.clone(),
            auto_fix,
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
    auto_fix_support_catalog: AutoFixSupportCatalog,
    auto_fix_states: BTreeMap<ScannerAutoFixResultKey, ScannerAutoFixRenderState>,
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
        Self::with_auto_fix_support_catalog(settings, AutoFixSupportCatalog::empty())
    }

    /// Creates an idle Scanner controller with an injected Auto-Fix support catalog.
    ///
    /// Production should use [`Self::new`], which preserves the reference app's empty
    /// Auto-Fix registry. Tests and future worker wiring can inject a fake catalog
    /// without passing operation closures or filesystem adapters into the controller.
    pub fn with_auto_fix_support_catalog(
        settings: ScannerSettings,
        auto_fix_support_catalog: AutoFixSupportCatalog,
    ) -> Self {
        let scan_enabled = any_scanner_category_enabled(&settings);
        Self {
            settings,
            auto_fix_support_catalog,
            auto_fix_states: BTreeMap::new(),
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

    /// Returns the injected closure-free Auto-Fix support catalog.
    pub fn auto_fix_support_catalog(&self) -> &AutoFixSupportCatalog {
        &self.auto_fix_support_catalog
    }

    /// Returns Auto-Fix state for the currently selected result, if visible.
    pub fn selected_auto_fix(&self) -> Option<&ScannerAutoFixRenderState> {
        self.selected_detail
            .as_ref()
            .and_then(|detail| detail.auto_fix.as_ref())
    }

    /// Returns tracked Auto-Fix state for a current flat result row.
    pub fn auto_fix_state_for_result(
        &self,
        result_index: usize,
    ) -> Option<&ScannerAutoFixRenderState> {
        let result = self.results.get(result_index)?;
        let scan_id = self.latest_scan_id?;
        let key = ScannerAutoFixResultKey::new(scan_id, result_index, result.selection_identity());
        self.auto_fix_states.get(&key)
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
        self.auto_fix_states.clear();
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
        self.auto_fix_states.clear();
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
        let Some(result) = self.results.get(result_index).cloned() else {
            self.clear_selection();
            tracing::debug!(
                event = "s07-scanner-selection-cleared",
                result_index,
                "Scanner selection cleared because the row was missing"
            );
            return ScannerTransitionResult::Applied;
        };

        let auto_fix = self.auto_fix_state_for_selection(result_index, &result);
        self.selected_detail = Some(ScannerSelectedDetail::from_result(
            result_index,
            &result,
            auto_fix,
        ));
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
            auto_fix_visible = self.selected_auto_fix().is_some(),
            "Scanner result selected"
        );
        ScannerTransitionResult::Applied
    }

    /// Requests Auto-Fix for the selected supported Scanner result.
    ///
    /// The controller performs only state transition and stale/tamper checks. The
    /// returned owned request is safe to move to a worker; operation execution stays
    /// outside this reducer.
    pub fn request_selected_auto_fix(&mut self) -> Option<ScannerAutoFixWorkerRequest> {
        tracing::info!(
            event = "s08-scanner-autofix-requested",
            latest_scan_id = ?self.latest_scan_id,
            selected_result_index = ?self.selected_detail.as_ref().map(|detail| detail.result_index),
            "Scanner Auto-Fix requested"
        );

        let Some(selected_auto_fix) = self.selected_auto_fix().cloned() else {
            let reason = if self.selected_detail.is_some() {
                "unsupported-or-missing-support"
            } else {
                "no-selection"
            };
            let message = if self.selected_detail.is_some() {
                SCANNER_AUTO_FIX_UNAVAILABLE_MESSAGE
            } else {
                SCANNER_AUTO_FIX_NO_SELECTION_MESSAGE
            };
            self.reject_auto_fix_request(reason, message, None, None, None, None);
            return None;
        };

        let scan_id = selected_auto_fix.scan_id;
        let result_index = selected_auto_fix.result_index;
        let operation_key = selected_auto_fix.operation_key;
        let selection_identity = selected_auto_fix.selection_identity.clone();

        if selected_auto_fix.button.kind == AutoFixStatusKind::Fixing {
            self.reject_auto_fix_request(
                "already-running",
                SCANNER_AUTO_FIX_RUNNING_MESSAGE,
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
                Some(selection_identity),
            );
            return None;
        }

        if self.latest_scan_id != Some(scan_id) {
            self.reject_auto_fix_request(
                "scan-mismatch",
                SCANNER_AUTO_FIX_STALE_MESSAGE,
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
                Some(selection_identity),
            );
            return None;
        }

        let Some(result) = self.results.get(result_index) else {
            self.reject_auto_fix_request(
                "result-missing",
                SCANNER_AUTO_FIX_STALE_MESSAGE,
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
                Some(selection_identity),
            );
            return None;
        };

        let current_identity = result.selection_identity();
        if current_identity != selection_identity {
            self.reject_auto_fix_request(
                "identity-mismatch",
                SCANNER_AUTO_FIX_STALE_MESSAGE,
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
                Some(selection_identity),
            );
            return None;
        }

        if result.auto_fix_operation_key() != Some(operation_key) {
            self.reject_auto_fix_request(
                "operation-mismatch",
                SCANNER_AUTO_FIX_UNAVAILABLE_MESSAGE,
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
                Some(current_identity),
            );
            return None;
        }

        if self
            .auto_fix_support_catalog
            .support_for_key(operation_key)
            .is_none()
        {
            self.reject_auto_fix_request(
                "missing-support",
                SCANNER_AUTO_FIX_UNAVAILABLE_MESSAGE,
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
                Some(current_identity),
            );
            return None;
        }

        let key = ScannerAutoFixResultKey::new(scan_id, result_index, current_identity.clone());
        let state = self
            .auto_fix_states
            .entry(key.clone())
            .or_insert_with(|| selected_auto_fix.clone());
        state.button = AutoFixButtonState::from_status(AutoFixStatusKind::Fixing);
        state.status =
            AutoFixStatus::new(AutoFixStatusKind::Fixing, SCANNER_AUTO_FIX_RUNNING_MESSAGE);
        state.result_detail = None;
        state.row_fixed = false;
        state.row_checked = false;
        self.status_text = SCANNER_AUTO_FIX_RUNNING_MESSAGE.to_owned();
        self.sync_selected_auto_fix_state(&key);

        let request = ScannerAutoFixWorkerRequest::new(
            scan_id,
            result_index,
            current_identity,
            operation_key,
        );
        tracing::info!(
            event = "s08-scanner-autofix-scheduled",
            scan_id,
            result_index,
            operation_key = %operation_key.as_id(),
            task_id = %request.task.id,
            "Scanner Auto-Fix worker request prepared"
        );
        tracing::debug!(
            event = "s08-scanner-autofix-fixing",
            scan_id,
            result_index,
            operation_key = %operation_key.as_id(),
            "Scanner Auto-Fix state moved to Fixing"
        );
        Some(request)
    }

    /// Applies a completed Auto-Fix payload when it still matches the selected result.
    pub fn auto_fix_completed(&mut self, completion: AutoFixCompletion) -> ScannerTransitionResult {
        let Some(scan_id) = completion.scan_id else {
            return self.auto_fix_stale_ignored(
                "missing-scan-id",
                None,
                completion.result_index,
                Some(completion.operation_key),
            );
        };
        let Some(result_index) = completion.result_index else {
            return self.auto_fix_stale_ignored(
                "missing-result-index",
                Some(scan_id),
                None,
                Some(completion.operation_key),
            );
        };
        let operation_key = completion.operation_key;

        let Some(selected_auto_fix) = self.selected_auto_fix().cloned() else {
            return self.auto_fix_stale_ignored(
                "selection-missing",
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
            );
        };
        if self.latest_scan_id != Some(scan_id)
            || selected_auto_fix.scan_id != scan_id
            || selected_auto_fix.result_index != result_index
            || selected_auto_fix.operation_key != operation_key
            || selected_auto_fix.selection_identity != completion.selection_identity
        {
            return self.auto_fix_stale_ignored(
                "selection-mismatch",
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
            );
        }

        let Some(result) = self.results.get(result_index) else {
            return self.auto_fix_stale_ignored(
                "result-missing",
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
            );
        };
        let current_identity = result.selection_identity();
        if current_identity != completion.selection_identity
            || result.auto_fix_operation_key() != Some(operation_key)
            || self
                .auto_fix_support_catalog
                .support_for_key(operation_key)
                .is_none()
        {
            return self.auto_fix_stale_ignored(
                "result-mismatch",
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
            );
        }

        let key = ScannerAutoFixResultKey::new(scan_id, result_index, current_identity);
        let fixed = completion.status.kind == AutoFixStatusKind::Fixed;
        let kind = if fixed {
            AutoFixStatusKind::Fixed
        } else {
            AutoFixStatusKind::Failed
        };
        let mut status = completion.status.clone();
        status.kind = kind;
        let state = ScannerAutoFixRenderState {
            scan_id,
            result_index,
            operation_key,
            selection_identity: completion.selection_identity,
            button: AutoFixButtonState::from_status(kind),
            status: status.clone(),
            result_detail: Some(completion.detail.clone()),
            row_fixed: fixed,
            row_checked: fixed,
        };
        self.auto_fix_states.insert(key.clone(), state);
        self.status_text = status.safe_message.clone();
        self.sync_selected_auto_fix_state(&key);

        if fixed {
            tracing::info!(
                event = "s08-scanner-autofix-completed",
                scan_id,
                result_index,
                operation_key = %operation_key.as_id(),
                safe_message = %status.safe_message,
                "Scanner Auto-Fix completed"
            );
        } else {
            tracing::warn!(
                event = "s08-scanner-autofix-failed",
                scan_id,
                result_index,
                operation_key = %operation_key.as_id(),
                safe_message = %status.safe_message,
                diagnostic = status.diagnostic.as_deref().unwrap_or(""),
                "Scanner Auto-Fix completed with failure"
            );
        }
        ScannerTransitionResult::Applied
    }

    /// Maps a worker failure into Fix Failed feedback for a matching Auto-Fix task.
    pub fn auto_fix_worker_failed(
        &mut self,
        scan_id: ScannerScanId,
        result_index: usize,
        operation_key: AutoFixOperationKey,
        failure: WorkerFailure,
    ) -> ScannerTransitionResult {
        let Some(selected_auto_fix) = self.selected_auto_fix().cloned() else {
            return self.auto_fix_stale_ignored(
                "selection-missing",
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
            );
        };
        if self.latest_scan_id != Some(scan_id)
            || selected_auto_fix.scan_id != scan_id
            || selected_auto_fix.result_index != result_index
            || selected_auto_fix.operation_key != operation_key
        {
            return self.auto_fix_stale_ignored(
                "selection-mismatch",
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
            );
        }
        let Some(result) = self.results.get(result_index) else {
            return self.auto_fix_stale_ignored(
                "result-missing",
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
            );
        };
        let current_identity = result.selection_identity();
        if current_identity != selected_auto_fix.selection_identity
            || result.auto_fix_operation_key() != Some(operation_key)
        {
            return self.auto_fix_stale_ignored(
                "result-mismatch",
                Some(scan_id),
                Some(result_index),
                Some(operation_key),
            );
        }

        let safe_message = if failure.safe_message().is_empty() {
            SCANNER_AUTO_FIX_FAILED_MESSAGE.to_owned()
        } else {
            failure.safe_message().to_owned()
        };
        let status = match failure.diagnostic.clone() {
            Some(diagnostic) => AutoFixStatus::new(AutoFixStatusKind::Failed, safe_message.clone())
                .with_diagnostic(diagnostic),
            None => AutoFixStatus::new(AutoFixStatusKind::Failed, safe_message.clone()),
        };
        let detail = match failure.diagnostic.clone() {
            Some(diagnostic) => {
                AutoFixResultDetail::new(safe_message.clone(), safe_message.clone())
                    .with_diagnostic(diagnostic)
            }
            None => AutoFixResultDetail::new(safe_message.clone(), safe_message.clone()),
        };
        let key = ScannerAutoFixResultKey::new(scan_id, result_index, current_identity);
        let state = ScannerAutoFixRenderState {
            scan_id,
            result_index,
            operation_key,
            selection_identity: selected_auto_fix.selection_identity,
            button: AutoFixButtonState::from_status(AutoFixStatusKind::Failed),
            status: status.clone(),
            result_detail: Some(detail),
            row_fixed: false,
            row_checked: false,
        };
        self.auto_fix_states.insert(key.clone(), state);
        self.status_text = safe_message;
        self.sync_selected_auto_fix_state(&key);
        tracing::warn!(
            event = "s08-scanner-autofix-failed",
            scan_id,
            result_index,
            operation_key = %operation_key.as_id(),
            safe_message = %status.safe_message,
            diagnostic = status.diagnostic.as_deref().unwrap_or(""),
            "Scanner Auto-Fix worker failed"
        );
        ScannerTransitionResult::Applied
    }

    /// Maps a worker spawn failure into safe Auto-Fix failure feedback.
    pub fn auto_fix_spawn_failed(
        &mut self,
        request: &ScannerAutoFixWorkerRequest,
        error: WorkerSpawnError,
    ) -> ScannerTransitionResult {
        tracing::error!(
            event = "s08-scanner-autofix-worker-spawn-failed",
            scan_id = request.scan_id,
            result_index = request.result_index,
            operation_key = %request.operation_key.as_id(),
            task_id = %request.task.id,
            diagnostic = %error,
            "Scanner Auto-Fix worker could not be scheduled"
        );
        self.auto_fix_worker_failed(
            request.scan_id,
            request.result_index,
            request.operation_key,
            WorkerFailure::new(SCANNER_AUTO_FIX_START_FAILED_MESSAGE)
                .with_diagnostic(error.to_string()),
        )
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
            WorkerPayload::Error(failure)
                if task.kind == WorkerTaskKind::Patch && status == WorkerTaskStatus::Failed =>
            {
                match scanner_auto_fix_task_parts(&task.id) {
                    Some((scan_id, result_index, operation_key)) => {
                        self.auto_fix_worker_failed(scan_id, result_index, operation_key, failure)
                    }
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
            ScannerWorkerPayload::AutoFixCompleted { completion }
                if task.kind == WorkerTaskKind::Patch && status == WorkerTaskStatus::Completed =>
            {
                match scanner_auto_fix_task_parts(&task.id) {
                    Some((scan_id, result_index, operation_key))
                        if completion.scan_id == Some(scan_id)
                            && completion.result_index == Some(result_index)
                            && completion.operation_key == operation_key =>
                    {
                        self.auto_fix_completed(*completion)
                    }
                    Some((scan_id, result_index, operation_key)) => self.auto_fix_stale_ignored(
                        "task-payload-mismatch",
                        Some(scan_id),
                        Some(result_index),
                        Some(operation_key),
                    ),
                    None => ScannerTransitionResult::Ignored,
                }
            }
            _ => ScannerTransitionResult::Ignored,
        }
    }

    fn auto_fix_state_for_selection(
        &mut self,
        result_index: usize,
        result: &ScannerResult,
    ) -> Option<ScannerAutoFixRenderState> {
        let scan_id = self.latest_scan_id?;
        let operation_support = self
            .auto_fix_support_catalog
            .support_for_result(result)?
            .clone();
        let selection_identity = result.selection_identity();
        let key = ScannerAutoFixResultKey::new(scan_id, result_index, selection_identity.clone());
        if let Some(state) = self.auto_fix_states.get(&key) {
            return Some(state.clone());
        }

        let state = ScannerAutoFixRenderState::ready(
            scan_id,
            result_index,
            operation_support.operation_key,
            selection_identity,
            operation_support.safe_preview,
        );
        self.auto_fix_states.insert(key, state.clone());
        Some(state)
    }

    fn reject_auto_fix_request(
        &mut self,
        reason: &'static str,
        safe_message: &'static str,
        scan_id: Option<ScannerScanId>,
        result_index: Option<usize>,
        operation_key: Option<AutoFixOperationKey>,
        selection_identity: Option<AutoFixSelectionIdentity>,
    ) {
        self.status_text = safe_message.to_owned();
        let operation_id = operation_key
            .map(AutoFixOperationKey::as_id)
            .unwrap_or("unknown");
        tracing::warn!(
            event = "s08-scanner-autofix-rejected",
            scan_id = ?scan_id,
            result_index = ?result_index,
            operation_key = %operation_id,
            reason,
            safe_message,
            identity = selection_identity.as_ref().map(AutoFixSelectionIdentity::as_str).unwrap_or(""),
            "Scanner Auto-Fix request rejected"
        );
    }

    fn sync_selected_auto_fix_state(&mut self, key: &ScannerAutoFixResultKey) {
        let Some(detail) = self.selected_detail.as_mut() else {
            return;
        };
        if detail.result_index != key.result_index {
            return;
        }
        if let Some(state) = self.auto_fix_states.get(key).cloned() {
            detail.auto_fix = Some(state);
        }
    }

    fn auto_fix_stale_ignored(
        &self,
        reason: &'static str,
        scan_id: Option<ScannerScanId>,
        result_index: Option<usize>,
        operation_key: Option<AutoFixOperationKey>,
    ) -> ScannerTransitionResult {
        let operation_id = operation_key
            .map(AutoFixOperationKey::as_id)
            .unwrap_or("unknown");
        tracing::debug!(
            event = "s08-scanner-autofix-stale-ignored",
            scan_id = ?scan_id,
            result_index = ?result_index,
            operation_key = %operation_id,
            reason,
            latest_scan_id = ?self.latest_scan_id,
            selected_result_index = ?self.selected_detail.as_ref().map(|detail| detail.result_index),
            "Ignoring stale Scanner Auto-Fix event"
        );
        ScannerTransitionResult::StaleIgnored
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

/// Builds worker metadata for a blocking Scanner Auto-Fix operation.
pub fn scanner_auto_fix_task(
    scan_id: ScannerScanId,
    result_index: usize,
    operation_key: AutoFixOperationKey,
) -> WorkerTask {
    WorkerTask::new(
        format!(
            "{SCANNER_AUTO_FIX_TASK_PREFIX}{scan_id}:{result_index}:{}",
            operation_key.as_id()
        ),
        WorkerTaskKind::Patch,
    )
    .with_label("Scanner Auto-Fix")
}

/// Converts a completed Auto-Fix result into the Scanner worker payload shape.
pub fn scanner_auto_fix_completed_payload(completion: AutoFixCompletion) -> WorkerPayload {
    WorkerPayload::Scanner(ScannerWorkerPayload::auto_fix_completed(completion))
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

/// Parses S08 Scanner Auto-Fix task identity from a worker task id.
pub fn scanner_auto_fix_task_parts(
    task_id: &WorkerTaskId,
) -> Option<(ScannerScanId, usize, AutoFixOperationKey)> {
    let value = task_id
        .as_str()
        .strip_prefix(SCANNER_AUTO_FIX_TASK_PREFIX)?;
    let mut parts = value.split(':');
    let scan_id = parts.next()?.parse::<ScannerScanId>().ok()?;
    let result_index = parts.next()?.parse::<usize>().ok()?;
    let operation_key = AutoFixOperationKey::from_id(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some((scan_id, result_index, operation_key))
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
    use crate::{
        domain::{
            autofix::{
                AutoFixCompletion, AutoFixResultDetail, AutoFixRevalidationPlan, AutoFixStatus,
            },
            scanner::{
                ACTION_COPY_DETAILS_LABEL, ScannerExtraData, ScannerFileList, ScannerFileListEntry,
                ScannerProblemType, ScannerSolutionKind, group_scanner_results,
            },
        },
        services::autofix::{
            AutoFixOperationContext, AutoFixOperationFailure, AutoFixOperationRunner,
            AutoFixOperationSuccess, AutoFixOperationSupport, AutoFixRegistry,
        },
    };

    struct NoopAutoFixRunner;

    impl AutoFixOperationRunner for NoopAutoFixRunner {
        fn execute(
            &self,
            _context: &AutoFixOperationContext<'_>,
        ) -> Result<AutoFixOperationSuccess, AutoFixOperationFailure> {
            Ok(AutoFixOperationSuccess::new(
                "Fixed fake scanner result.",
                "Fake Auto-Fix details.",
            ))
        }
    }

    fn fake_auto_fix_catalog(keys: &[AutoFixOperationKey]) -> AutoFixSupportCatalog {
        let mut registry = AutoFixRegistry::empty();
        for key in keys {
            registry.register(
                AutoFixOperationSupport::new(*key, "Fake Auto-Fix", "Fake Auto-Fix preview."),
                NoopAutoFixRunner,
            );
        }
        registry.support_catalog()
    }

    fn controller_with_fake_auto_fix(keys: &[AutoFixOperationKey]) -> ScannerController {
        ScannerController::with_auto_fix_support_catalog(
            ScannerSettings::default(),
            fake_auto_fix_catalog(keys),
        )
    }

    fn example_result(detail_path: &str) -> ScannerResult {
        ScannerResult::with_path(
            ScannerProblemType::JunkFile,
            format!("C:/Games/Fallout 4/Data/{detail_path}"),
            detail_path,
            "This is a junk file not used by the game or mod managers.",
            Some(ScannerSolutionKind::DeleteOrIgnoreFile.into_solution_text()),
        )
    }

    fn auto_fix_result(detail_path: &str, solution_kind: ScannerSolutionKind) -> ScannerResult {
        ScannerResult::with_path(
            ScannerProblemType::JunkFile,
            format!("C:/Games/Fallout 4/Data/{detail_path}"),
            detail_path,
            "This is a junk file not used by the game or mod managers.",
            None,
        )
        .with_solution_kind(solution_kind)
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

    #[test]
    fn scanner_controller_autofix_production_catalog_hides_button_and_rejects_tampered_request() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            request.scan_id,
            completed_snapshot(
                request.scan_id,
                vec![auto_fix_result(
                    "desktop.ini",
                    ScannerSolutionKind::DeleteOrIgnoreFile,
                )],
            ),
        );
        controller.select_result(0);

        assert!(controller.auto_fix_support_catalog().is_empty());
        assert!(controller.selected_auto_fix().is_none());
        assert!(controller.request_selected_auto_fix().is_none());
        assert_eq!(
            controller.status_text(),
            SCANNER_AUTO_FIX_UNAVAILABLE_MESSAGE
        );
    }

    #[test]
    fn scanner_controller_autofix_fake_catalog_accepts_request_and_sets_fixing_state() {
        let mut controller =
            controller_with_fake_auto_fix(&[AutoFixOperationKey::DeleteOrIgnoreFile]);
        let scan = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            scan.scan_id,
            completed_snapshot(
                scan.scan_id,
                vec![auto_fix_result(
                    "desktop.ini",
                    ScannerSolutionKind::DeleteOrIgnoreFile,
                )],
            ),
        );
        controller.select_result(0);

        let ready = controller
            .selected_auto_fix()
            .expect("supported typed solution should expose Auto-Fix");
        assert_eq!(ready.button.label, "Auto-Fix");
        assert!(ready.button.enabled);
        assert_eq!(ready.status.safe_message, "Fake Auto-Fix preview.");

        let fix = controller
            .request_selected_auto_fix()
            .expect("supported selection should return a worker request");

        assert_eq!(fix.scan_id, scan.scan_id);
        assert_eq!(fix.result_index, 0);
        assert_eq!(fix.operation_key, AutoFixOperationKey::DeleteOrIgnoreFile);
        assert_eq!(fix.task.kind, WorkerTaskKind::Patch);
        assert_eq!(
            scanner_auto_fix_task_parts(&fix.task.id),
            Some((scan.scan_id, 0, AutoFixOperationKey::DeleteOrIgnoreFile))
        );
        let fixing = controller
            .selected_auto_fix()
            .expect("Auto-Fix should remain visible while fixing");
        assert_eq!(fixing.button.label, "Fixing...");
        assert!(!fixing.button.enabled);
        assert_eq!(fixing.status.safe_message, SCANNER_AUTO_FIX_RUNNING_MESSAGE);
    }

    #[test]
    fn scanner_controller_autofix_success_completion_sets_fixed_detail_and_row_state() {
        let mut controller =
            controller_with_fake_auto_fix(&[AutoFixOperationKey::DeleteOrIgnoreFile]);
        let scan = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            scan.scan_id,
            completed_snapshot(
                scan.scan_id,
                vec![auto_fix_result(
                    "desktop.ini",
                    ScannerSolutionKind::DeleteOrIgnoreFile,
                )],
            ),
        );
        controller.select_result(0);
        let request = controller
            .request_selected_auto_fix()
            .expect("Auto-Fix should schedule");
        let completion = AutoFixCompletion {
            scan_id: Some(request.scan_id),
            result_index: Some(request.result_index),
            operation_key: request.operation_key,
            selection_identity: request.selection_identity.clone(),
            revalidation: AutoFixRevalidationPlan::required(request.selection_identity.clone())
                .with_observed_identity(request.selection_identity.clone()),
            status: AutoFixStatus::new(AutoFixStatusKind::Fixed, "Fixed fake scanner result."),
            detail: AutoFixResultDetail::new(
                "Fixed fake scanner result.",
                "Deleted the fake junk file.",
            ),
        };

        let result = controller.handle_worker_event(WorkerEvent::completed(
            request.task,
            scanner_auto_fix_completed_payload(completion),
        ));

        assert_eq!(result, ScannerTransitionResult::Applied);
        let fixed = controller
            .selected_auto_fix()
            .expect("Auto-Fix state should remain selected");
        assert_eq!(fixed.button.label, "Fixed!");
        assert!(fixed.button.enabled);
        assert!(fixed.row_fixed);
        assert!(fixed.row_checked);
        assert_eq!(fixed.status.safe_message, "Fixed fake scanner result.");
        assert_eq!(
            fixed
                .result_detail
                .as_ref()
                .map(|detail| detail.details.as_str()),
            Some("Deleted the fake junk file.")
        );
        let row_state = controller
            .auto_fix_state_for_result(0)
            .expect("row state should be tracked");
        assert!(row_state.row_fixed);
        assert!(row_state.row_checked);
    }

    #[test]
    fn scanner_controller_autofix_failure_completion_uses_safe_text_and_keeps_diagnostics_off_ui() {
        let mut controller =
            controller_with_fake_auto_fix(&[AutoFixOperationKey::DeleteOrIgnoreFile]);
        let scan = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            scan.scan_id,
            completed_snapshot(
                scan.scan_id,
                vec![auto_fix_result(
                    "desktop.ini",
                    ScannerSolutionKind::DeleteOrIgnoreFile,
                )],
            ),
        );
        controller.select_result(0);
        let request = controller
            .request_selected_auto_fix()
            .expect("Auto-Fix should schedule");
        let completion = AutoFixCompletion {
            scan_id: Some(request.scan_id),
            result_index: Some(request.result_index),
            operation_key: request.operation_key,
            selection_identity: request.selection_identity.clone(),
            revalidation: AutoFixRevalidationPlan::required(request.selection_identity.clone()),
            status: AutoFixStatus::new(
                AutoFixStatusKind::Failed,
                "Auto-Fix could not complete this operation.",
            )
            .with_diagnostic("raw adapter failure"),
            detail: AutoFixResultDetail::new(
                "Auto-Fix could not complete this operation.",
                "No files were changed.",
            )
            .with_diagnostic("raw adapter failure"),
        };

        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                request.task,
                scanner_auto_fix_completed_payload(completion),
            )),
            ScannerTransitionResult::Applied
        );

        let failed = controller
            .selected_auto_fix()
            .expect("failed Auto-Fix state should remain visible");
        assert_eq!(failed.button.label, "Fix Failed");
        assert!(failed.button.enabled);
        assert!(!failed.row_fixed);
        assert!(!failed.row_checked);
        assert_eq!(
            failed.status.safe_message,
            "Auto-Fix could not complete this operation."
        );
        assert!(!failed.status.safe_message.contains("raw adapter"));
        let detail = failed.result_detail.as_ref().expect("details should exist");
        assert_eq!(detail.details, "No files were changed.");
        assert!(!detail.details.contains("raw adapter"));
        assert_eq!(detail.diagnostic.as_deref(), Some("raw adapter failure"));
    }

    #[test]
    fn scanner_controller_autofix_selection_change_and_new_scan_make_completion_stale() {
        let mut controller =
            controller_with_fake_auto_fix(&[AutoFixOperationKey::DeleteOrIgnoreFile]);
        let first = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            first.scan_id,
            completed_snapshot(
                first.scan_id,
                vec![
                    auto_fix_result("desktop.ini", ScannerSolutionKind::DeleteOrIgnoreFile),
                    auto_fix_result("thumbs.db", ScannerSolutionKind::DeleteOrIgnoreFile),
                ],
            ),
        );
        controller.select_result(0);
        let request = controller
            .request_selected_auto_fix()
            .expect("Auto-Fix should schedule");
        controller.select_result(1);
        let stale_completion = AutoFixCompletion {
            scan_id: Some(request.scan_id),
            result_index: Some(request.result_index),
            operation_key: request.operation_key,
            selection_identity: request.selection_identity.clone(),
            revalidation: AutoFixRevalidationPlan::required(request.selection_identity.clone()),
            status: AutoFixStatus::new(AutoFixStatusKind::Fixed, "Fixed stale result."),
            detail: AutoFixResultDetail::new("Fixed stale result.", "Should be ignored."),
        };

        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                request.task.clone(),
                scanner_auto_fix_completed_payload(stale_completion),
            )),
            ScannerTransitionResult::StaleIgnored
        );
        assert!(
            !controller
                .auto_fix_state_for_result(0)
                .expect("row zero state should exist")
                .row_fixed
        );

        let second = controller.request_scan().expect("new scan should start");
        let stale_new_scan = AutoFixCompletion {
            scan_id: Some(first.scan_id),
            result_index: Some(0),
            operation_key: AutoFixOperationKey::DeleteOrIgnoreFile,
            selection_identity: request.selection_identity,
            revalidation: AutoFixRevalidationPlan::required(
                AutoFixSelectionIdentity::from_fingerprint("scanner-result:v1:stale"),
            ),
            status: AutoFixStatus::new(AutoFixStatusKind::Fixed, "Fixed old scan."),
            detail: AutoFixResultDetail::new("Fixed old scan.", "Should be ignored."),
        };
        assert_eq!(second.scan_id, 2);
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                request.task,
                scanner_auto_fix_completed_payload(stale_new_scan),
            )),
            ScannerTransitionResult::StaleIgnored
        );
        assert_eq!(controller.active_scan_id(), Some(second.scan_id));
    }

    #[test]
    fn scanner_controller_autofix_worker_failure_maps_safe_message_without_diagnostics() {
        let mut controller =
            controller_with_fake_auto_fix(&[AutoFixOperationKey::DeleteOrIgnoreFile]);
        let scan = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            scan.scan_id,
            completed_snapshot(
                scan.scan_id,
                vec![auto_fix_result(
                    "desktop.ini",
                    ScannerSolutionKind::DeleteOrIgnoreFile,
                )],
            ),
        );
        controller.select_result(0);
        let request = controller
            .request_selected_auto_fix()
            .expect("Auto-Fix should schedule");

        let result = controller.handle_worker_event(WorkerEvent::failed(
            request.task,
            WorkerFailure::new("Auto-Fix could not be started.")
                .with_diagnostic("raw worker spawn failure"),
        ));

        assert_eq!(result, ScannerTransitionResult::Applied);
        let failed = controller
            .selected_auto_fix()
            .expect("worker failure should be visible as Auto-Fix state");
        assert_eq!(failed.button.label, "Fix Failed");
        assert_eq!(failed.status.safe_message, "Auto-Fix could not be started.");
        assert!(!failed.status.safe_message.contains("raw worker"));
        let detail = failed.result_detail.as_ref().expect("detail should exist");
        assert_eq!(detail.details, "Auto-Fix could not be started.");
        assert!(!detail.details.contains("raw worker"));
        assert_eq!(
            detail.diagnostic.as_deref(),
            Some("raw worker spawn failure")
        );
    }
}
