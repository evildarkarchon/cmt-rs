pub mod app;
pub mod domain;
pub mod platform;
pub mod services;
pub mod workers;

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
};

use app::{
    about_controller::{AboutController, AboutState, AboutTransitionResult},
    archive_patcher_controller::{
        ARCHIVE_PATCHER_OVERVIEW_UNAVAILABLE_MESSAGE, ARCHIVE_PATCHER_PLAN_READY_MESSAGE,
        ArchivePatcherCandidateWorkerRequest, ArchivePatcherController, ArchivePatcherPatchAllRequest, ArchivePatcherPatchWorkerRequest,
        ArchivePatcherPlanWorkerRequest, ArchivePatcherRestoreWorkerRequest,
        ArchivePatcherTransitionResult, ArchivePatcherWorkerRequestKind,
        archive_patcher_candidates_loaded_payload, archive_patcher_log_row_payload,
        archive_patcher_patch_completed_payload, archive_patcher_plan_ready_payload,
        archive_patcher_progress_payload, archive_patcher_restore_completed_payload,
    },
    downgrader_controller::{
        DOWNGRADER_PLAN_READY_MESSAGE, DowngraderController, DowngraderPatchWorkerRequest,
        DowngraderPlanWorkerRequest, DowngraderRunWorkerRequest, DowngraderStatusWorkerRequest,
        DowngraderTransitionResult, DowngraderWorkerRequestKind, downgrader_log_row_payload,
        downgrader_plan_ready_payload, downgrader_progress_payload,
        downgrader_run_completed_payload, downgrader_status_loaded_payload,
    },
    f4se_controller::{
        F4seController, F4seScanWorkerRequest, F4seTransitionResult, f4se_scan_completed_payload,
    },
    overview_controller::{
        OverviewController, action_target_label, overview_desktop_action_payload,
        overview_desktop_task, unavailable_action_error,
    },
    scanner_controller::{
        SCANNER_ACTION_UNAVAILABLE_MESSAGE, ScannerAutoFixWorkerRequest, ScannerController,
        ScannerControllerPhase, ScannerScanWorkerRequest, ScannerTransitionResult,
        any_scanner_category_enabled, scanner_action_completed_payload,
        scanner_auto_fix_completed_payload, scanner_scan_completed_payload,
    },
    settings_controller::SettingsController,
    tools_controller::{
        TOOLS_DEFAULT_DISABLED_UTILITY_STATUS, ToolsController, ToolsState, ToolsTransitionResult,
    },
};
use domain::{
    archive_patcher::{
        ABOUT_ARCHIVES_BODY, ABOUT_ARCHIVES_TITLE, ArchivePatcherArchiveFormat,
        ArchivePatcherCandidateRow, ArchivePatcherExecutionResult, ArchivePatcherLogLevel,
        ArchivePatcherLogRow, ArchivePatcherPlanAction, ArchivePatcherPreviewPlanRow,
        ArchivePatcherProgress, ArchivePatcherTarget,
    },
    autofix::{
        AutoFixCompletion, AutoFixRejection, AutoFixRequest, AutoFixResultDetail,
        AutoFixRevalidationPlan, AutoFixStatus, AutoFixStatusKind,
    },
    discovery::{FALLOUT4_EXECUTABLE, Fallout4InstallType},
    downgrader::{
        ABOUT_DOWNGRADING_BODY, ABOUT_DOWNGRADING_TITLE, DowngraderExecutionLogRow,
        DowngraderFileGroup, DowngraderInstallStatus, DowngraderOptionsSnapshot,
        DowngraderStatusRow, DowngraderTarget,
    },
    f4se::{F4seDllRow, F4seGameTarget, F4seRowSeverity, F4seScanSnapshot, F4seScanStatus},
    mod_manager::ModManagerContext,
    overview::{
        ACTION_ARCHIVE_PATCHER_LABEL, ACTION_DOWNGRADE_MANAGER_LABEL, BinaryStatusRow,
        OverviewActionError, OverviewCountRow, OverviewDeferredAction, OverviewDeferredActionKind,
        OverviewDeferredActionTarget, OverviewProblem, OverviewRefreshState, OverviewSnapshot,
        OverviewTopStatusRow, StatusSeverity, UpdateBannerState, UpdateCheckFailure,
        UpdateProvider,
    },
    scanner::{
        DETAIL_LABEL_MOD, DETAIL_LABEL_PROBLEM, DETAIL_LABEL_SOLUTION, DETAIL_LABEL_SUMMARY,
        PROGRESS_REFRESHING_OVERVIEW_TEXT, ScannerActionDescriptor, ScannerActionFeedback,
        ScannerActionKind, ScannerActionTarget, ScannerCategoryKind, ScannerCategoryProjection,
        ScannerFileList, ScannerResult, ScannerResultGroup, ScannerScanSnapshot,
    },
    settings::{AppSettings, DowngraderSettings, UpdateSource},
    tools::{ABOUT_LINKS, AboutLinkId, ToolActionId},
};
use platform::{
    clipboard::{ClipboardActions, RealClipboardActions},
    desktop::{DesktopActions, RealDesktopActions},
    filesystem::{Filesystem, RealFilesystem},
    process::RealProcessInspector,
    registry::RealRegistry,
    settings_store::{AssetResolver, FileAssetResolver, SettingsStore},
};
use services::{
    archive_patcher::{
        ArchivePatcherCandidateRequest, ArchivePatcherExecutionError,
        ArchivePatcherExecutionRequest, ArchivePatcherPlanRequest, ArchivePatcherRestoreRequest,
        ArchivePatcherService,
    },
    autofix::{AutoFixService, AutoFixServiceResult},
    discovery::{DiscoveredModManager, DiscoveryRequest, DiscoveryService},
    downgrader::{
        DowngraderExecutionProgressEvent, DowngraderExecutionRequest, DowngraderPlanRequest,
        DowngraderService, DowngraderStatusRequest, ReqwestDeltaDownloader, VcdiffDeltaApplier,
    },
    f4se::{
        F4seScanDiagnosticKind, F4seScanDiagnostics, F4seScanRequest, F4seScanService,
        PeliteF4seDllInspector,
    },
    overview::{
        OverviewDesktopActionFeedback, OverviewDesktopActionOutcome, OverviewDiagnostics,
        OverviewDiagnosticsInput, OverviewUpdateCheckState,
    },
    overview_collector::{
        OverviewCollectedFacts, OverviewCollectionEnvironment, OverviewCollectionRequest,
        OverviewCollector,
    },
    scanner::{ScannerProgressEvent, ScannerScanOutput, ScannerScanRequest, ScannerScanService},
    tools::{
        AboutActionFeedback, AboutActionKind, ActionRejectionKind, ToolsActionFeedback,
        ToolsActionKind, ToolsActionService, about_action_for_id, tools_action_for_id,
    },
    update::{OverviewLinkService, RealUpdateCheckClient, UpdateCheckService},
};
use slint::{CloseRequestResponse, ComponentHandle, ModelRc, SharedString, VecModel};
use workers::{
    AboutActionWorkerPayload, ArchivePatcherWorkerPayload, ArchivePatcherWorkerStage,
    BlockingWorkerResult, SlintEventLoopSink, ToolsActionWorkerPayload, WorkerEvent,
    WorkerEventSink, WorkerFailure, WorkerPayload, WorkerRuntime, WorkerSpawnError, WorkerTask,
    WorkerTaskKind, WorkerTaskOutcome, WorkerTaskStatus,
};

slint::include_modules!();

const TOOLS_WORKER_TASK_PREFIX: &str = "s05-tools-action:";
const ABOUT_WORKER_TASK_PREFIX: &str = "s05-about-action:";
const SCANNER_ACTION_TASK_PREFIX: &str = "s07-scanner-action:";
const TOOLS_ACTION_START_ERROR: &str = "Tools action could not be started.";
const ABOUT_ACTION_START_ERROR: &str = "About action could not be started.";
const SCANNER_ACTION_START_ERROR: &str = "Scanner action could not be started.";

#[derive(Debug, Clone)]
struct PreparedScannerScan {
    request: ScannerScanWorkerRequest,
    settings: AppSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedScannerAutoFix {
    request: ScannerAutoFixWorkerRequest,
    snapshot: ScannerScanSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScannerActionExecution {
    scan_id: Option<u64>,
    descriptor: ScannerActionDescriptor,
    details_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AboutCallbackKind {
    Open,
    Copy,
}

impl AboutCallbackKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Copy => "copy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolsUiProjection {
    last_action_error: String,
    disabled_utility_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AboutUiProjection {
    last_action_error: String,
    nexus_copy_label: String,
    nexus_copy_enabled: bool,
    discord_copy_label: String,
    discord_copy_enabled: bool,
    github_copy_label: String,
    github_copy_enabled: bool,
}

struct F4seUiProjection {
    status_text: String,
    busy: bool,
    loading_or_error_text: String,
    unknown_game_detail: String,
    rows: Vec<F4seUiRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScannerCategoryToggleProjection {
    overview_issues: bool,
    errors: bool,
    wrong_file_formats: bool,
    loose_previs: bool,
    junk_files: bool,
    problem_overrides: bool,
    race_subgraphs: bool,
    read_only: bool,
}

struct ScannerUiProjection {
    categories: ScannerCategoryToggleProjection,
    scan_button_text: String,
    scan_button_enabled: bool,
    busy: bool,
    status_text: String,
    progress_text: String,
    progress_percent: f32,
    result_count_text: String,
    result_rows: Vec<ScannerResultUiRow>,
    show_mod_column: bool,
    detail_visible: bool,
    detail_mod: String,
    detail_problem: String,
    detail_summary: String,
    detail_solution: String,
    action_feedback: String,
    open_path_enabled: bool,
    open_url_enabled: bool,
    copy_url_enabled: bool,
    file_list_enabled: bool,
    file_list_visible: bool,
    file_list_title: String,
    file_list_description: String,
    file_list_first_column: String,
    file_list_second_column: String,
    file_list_rows: Vec<ScannerFileListUiRow>,
    auto_fix_button_visible: bool,
    auto_fix_button_label: String,
    auto_fix_button_enabled: bool,
    auto_fix_status_text: String,
    auto_fix_results_visible: bool,
    auto_fix_results_title: String,
    auto_fix_results_summary: String,
    auto_fix_results_details: String,
}

struct DowngraderUiProjection {
    current_game_status_rows: Vec<DowngraderStatusUiRow>,
    current_creation_kit_status_rows: Vec<DowngraderStatusUiRow>,
    selected_target: String,
    keep_backups: bool,
    delete_patches: bool,
    plan_rows: Vec<DowngraderPlanUiRow>,
    plan_visible: bool,
    confirmation_state: String,
    plan_confirmation_text: String,
    log_rows: Vec<DowngraderLogUiRow>,
    log_text: String,
    progress_percent: f32,
    progress_text: String,
    patch_enabled: bool,
    about_enabled: bool,
    controls_enabled: bool,
    close_blocked: bool,
}

struct ArchivePatcherUiProjection {
    selected_target: String,
    name_filter: String,
    candidate_rows: Vec<ArchivePatcherCandidateUiRow>,
    candidate_empty_text: String,
    plan_rows: Vec<ArchivePatcherPlanUiRow>,
    confirmation_visible: bool,
    confirmation_text: String,
    log_rows: Vec<ArchivePatcherLogUiRow>,
    log_text: String,
    progress_percent: f32,
    progress_text: String,
    status_text: String,
    patch_enabled: bool,
    restore_enabled: bool,
    about_enabled: bool,
    controls_enabled: bool,
    close_blocked: bool,
    about_dialog_visible: bool,
    about_title: String,
    about_body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DowngraderAboutProjection {
    title: String,
    body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DowngraderOpenSource {
    Overview,
    Tools,
}

impl DowngraderOpenSource {
    const fn label(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Tools => "tools",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchivePatcherOpenSource {
    Overview,
    Tools,
}

impl ArchivePatcherOpenSource {
    const fn label(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Tools => "tools",
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("cmt-rs-worker")
        .build()?;
    let runtime_handle = tokio_runtime.handle().clone();
    let _runtime_guard = tokio_runtime.enter();

    let app = MainWindow::new()?;
    let settings_controller = Rc::new(RefCell::new(load_settings_controller()));
    let shared_settings_snapshot = Arc::new(Mutex::new(
        settings_controller.borrow().current_settings().clone(),
    ));
    let overview_controller = Arc::new(Mutex::new(OverviewController::new()));
    let f4se_controller = Arc::new(Mutex::new(F4seController::new()));
    let scanner_controller = Arc::new(Mutex::new(ScannerController::new(
        settings_controller
            .borrow()
            .current_scanner_settings()
            .clone(),
    )));
    let tools_controller = Arc::new(Mutex::new(ToolsController::new()));
    let about_controller = Arc::new(Mutex::new(AboutController::new()));
    let downgrader_controller = Arc::new(Mutex::new(DowngraderController::new()));
    let archive_patcher_controller = Arc::new(Mutex::new(ArchivePatcherController::new()));
    let downgrader_window = DowngraderWindow::new()?;
    let archive_patcher_window = ArchivePatcherWindow::new()?;
    let worker_runtime = WorkerRuntime::new();

    app.set_update_source(settings_controller.borrow().visible_update_source().into());
    app.set_log_level(settings_controller.borrow().visible_log_level().into());
    apply_current_overview_snapshot(&app, &overview_controller);
    apply_current_f4se_snapshot(&app, &f4se_controller);
    apply_current_scanner_state(&app, &scanner_controller);
    apply_current_tools_state(&app, &tools_controller);
    apply_current_about_state(&app, &about_controller);
    apply_current_downgrader_state(&downgrader_window, &downgrader_controller);
    apply_current_archive_patcher_state(&archive_patcher_window, &archive_patcher_controller);

    let overview_sink = bind_overview_worker_sink(&app, Arc::clone(&overview_controller));
    let f4se_sink = bind_f4se_worker_sink(&app, Arc::clone(&f4se_controller));
    let scanner_sink = bind_scanner_worker_sink(&app, Arc::clone(&scanner_controller));
    let tools_sink = bind_tools_worker_sink(&app, Arc::clone(&tools_controller));
    let about_sink = bind_about_worker_sink(&app, Arc::clone(&about_controller));
    let archive_patcher_sink = bind_archive_patcher_worker_sink(
        &app,
        &archive_patcher_window,
        Arc::clone(&archive_patcher_controller),
        Arc::clone(&overview_controller),
        Arc::clone(&shared_settings_snapshot),
        worker_runtime,
        overview_sink.clone(),
        runtime_handle.clone(),
    );
    let downgrader_sink = bind_downgrader_worker_sink(
        &app,
        &downgrader_window,
        Arc::clone(&downgrader_controller),
        Arc::clone(&overview_controller),
        Arc::clone(&shared_settings_snapshot),
        worker_runtime,
        overview_sink.clone(),
        runtime_handle.clone(),
    );
    bind_settings_callbacks(
        &app,
        Rc::clone(&settings_controller),
        Arc::clone(&shared_settings_snapshot),
    );
    bind_downgrader_callbacks(
        &downgrader_window,
        Arc::clone(&downgrader_controller),
        Rc::clone(&settings_controller),
        Arc::clone(&shared_settings_snapshot),
        worker_runtime,
        downgrader_sink.clone(),
    );
    bind_archive_patcher_callbacks(
        &archive_patcher_window,
        Arc::clone(&archive_patcher_controller),
        worker_runtime,
        archive_patcher_sink.clone(),
    );
    bind_tools_callbacks(
        &app,
        &downgrader_window,
        &archive_patcher_window,
        Arc::clone(&tools_controller),
        Arc::clone(&downgrader_controller),
        Arc::clone(&archive_patcher_controller),
        Arc::clone(&overview_controller),
        Rc::clone(&settings_controller),
        worker_runtime,
        tools_sink.clone(),
        downgrader_sink.clone(),
        archive_patcher_sink.clone(),
    );
    bind_about_callbacks(
        &app,
        Arc::clone(&about_controller),
        worker_runtime,
        about_sink,
    );
    bind_f4se_callbacks(
        &app,
        Arc::clone(&f4se_controller),
        worker_runtime,
        f4se_sink,
    );
    bind_scanner_callbacks(
        &app,
        Arc::clone(&scanner_controller),
        Rc::clone(&settings_controller),
        worker_runtime,
        scanner_sink,
    );
    bind_overview_callbacks(
        &app,
        &downgrader_window,
        &archive_patcher_window,
        Arc::clone(&overview_controller),
        Arc::clone(&downgrader_controller),
        Arc::clone(&archive_patcher_controller),
        Rc::clone(&settings_controller),
        worker_runtime,
        overview_sink.clone(),
        downgrader_sink,
        archive_patcher_sink,
        runtime_handle.clone(),
    );

    request_overview_refresh(
        &app,
        &overview_controller,
        &settings_controller,
        worker_runtime,
        overview_sink,
        runtime_handle,
    );

    Ok(app.run()?)
}

fn load_settings_controller() -> SettingsController<FileAssetResolver> {
    SettingsController::load(SettingsStore::production()).unwrap_or_else(|error| {
        tracing::error!(%error, "Settings : Failed to load settings; using in-memory defaults");
        SettingsController::from_settings(SettingsStore::production(), AppSettings::default())
    })
}

fn bind_settings_callbacks(
    app: &MainWindow,
    controller: Rc<RefCell<SettingsController<FileAssetResolver>>>,
    shared_settings_snapshot: Arc<Mutex<AppSettings>>,
) {
    app.on_update_source_selected({
        let app = app.as_weak();
        let controller = Rc::clone(&controller);
        let shared_settings_snapshot = Arc::clone(&shared_settings_snapshot);

        move |selected| {
            let visible_value = controller
                .borrow_mut()
                .select_update_source(selected.as_str());
            remember_current_settings_snapshot(&controller, &shared_settings_snapshot);
            if let Some(app) = app.upgrade() {
                app.set_update_source(visible_value.into());
            }
        }
    });

    app.on_log_level_selected({
        let app = app.as_weak();
        let controller = Rc::clone(&controller);
        let shared_settings_snapshot = Arc::clone(&shared_settings_snapshot);

        move |selected| {
            let visible_value = controller.borrow_mut().select_log_level(selected.as_str());
            remember_current_settings_snapshot(&controller, &shared_settings_snapshot);
            if let Some(app) = app.upgrade() {
                app.set_log_level(visible_value.into());
            }
        }
    });
}

fn remember_current_settings_snapshot<R: AssetResolver>(
    controller: &Rc<RefCell<SettingsController<R>>>,
    shared_settings_snapshot: &Arc<Mutex<AppSettings>>,
) {
    let settings = controller.borrow().current_settings().clone();
    match shared_settings_snapshot.lock() {
        Ok(mut snapshot) => *snapshot = settings,
        Err(error) => tracing::error!(
            event = "settings-shared-snapshot-update-failed",
            diagnostic = %error,
            "Current settings snapshot could not be updated"
        ),
    }
}

fn bind_downgrader_worker_sink(
    app: &MainWindow,
    window: &DowngraderWindow,
    controller: Arc<Mutex<DowngraderController>>,
    overview_controller: Arc<Mutex<OverviewController>>,
    shared_settings_snapshot: Arc<Mutex<AppSettings>>,
    worker_runtime: WorkerRuntime,
    overview_sink: SlintEventLoopSink,
    runtime_handle: tokio::runtime::Handle,
) -> SlintEventLoopSink {
    let app = app.as_weak();
    let window = window.as_weak();
    SlintEventLoopSink::new(move |event| {
        let task_id = event.task.id.to_string();
        let task_kind = event.task.kind.label();
        let status = event.status.label();
        let was_run_completion = matches!(
            &event.payload,
            WorkerPayload::Downgrader(workers::DowngraderWorkerPayload::RunCompleted { .. })
        );
        let Some(window) = window.upgrade() else {
            tracing::warn!(
                event = "s09-downgrader-worker-event-dropped",
                task_id = %task_id,
                task_kind,
                status,
                "Downgrader worker event arrived after the Slint window was gone"
            );
            return;
        };

        let Some(result) = with_downgrader_controller_mut(&controller, |controller| {
            handle_downgrader_worker_event(controller, event)
        }) else {
            return;
        };

        match result {
            DowngraderTransitionResult::Applied => {
                tracing::debug!(
                    event = "s09-downgrader-worker-event-applied",
                    task_id = %task_id,
                    task_kind,
                    status,
                    "Downgrader worker event applied to render state"
                );
                apply_current_downgrader_state(&window, &controller);
                if was_run_completion {
                    let app_handle = app.upgrade();
                    let overview_settings =
                        overview_settings_for_downgrader_completion(&shared_settings_snapshot);
                    schedule_downgrader_completion_refresh(
                        app_handle.as_ref(),
                        &window,
                        &controller,
                        &overview_controller,
                        overview_settings,
                        worker_runtime,
                        overview_sink.clone(),
                        runtime_handle.clone(),
                    );
                }
            }
            DowngraderTransitionResult::StaleIgnored => tracing::debug!(
                event = "s09-downgrader-worker-event-stale-ignored",
                task_id = %task_id,
                task_kind,
                status,
                "Downgrader worker event was stale and ignored"
            ),
            DowngraderTransitionResult::Ignored => tracing::debug!(
                event = "s09-downgrader-worker-event-ignored",
                task_id = %task_id,
                task_kind,
                status,
                "Ignoring non-Downgrader worker event on Downgrader sink"
            ),
            DowngraderTransitionResult::Rejected | DowngraderTransitionResult::CloseBlocked => {
                tracing::debug!(
                    event = "s09-downgrader-worker-event-rejected",
                    task_id = %task_id,
                    task_kind,
                    status,
                    result = ?result,
                    "Downgrader worker event was rejected by current modal state"
                );
            }
        }
    })
}

fn bind_downgrader_callbacks(
    window: &DowngraderWindow,
    controller: Arc<Mutex<DowngraderController>>,
    settings_controller: Rc<RefCell<SettingsController<FileAssetResolver>>>,
    shared_settings_snapshot: Arc<Mutex<AppSettings>>,
    worker_runtime: WorkerRuntime,
    downgrader_sink: SlintEventLoopSink,
) {
    window.on_target_selected({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);

        move |target| {
            let Some(window) = window.upgrade() else {
                return;
            };
            with_downgrader_controller_mut(&controller, |controller| {
                controller.set_target_from_ui_value(target.as_str());
            });
            apply_current_downgrader_state(&window, &controller);
        }
    });

    window.on_option_toggled({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);

        move |option_id, enabled| {
            let Some(window) = window.upgrade() else {
                return;
            };
            with_downgrader_controller_mut(&controller, |controller| {
                controller.set_option_from_ui_value(option_id.as_str(), enabled);
            });
            apply_current_downgrader_state(&window, &controller);
        }
    });

    window.on_patch_requested({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);
        let settings_controller = Rc::clone(&settings_controller);
        let shared_settings_snapshot = Arc::clone(&shared_settings_snapshot);
        let downgrader_sink = downgrader_sink.clone();

        move || {
            if let Some(window) = window.upgrade() {
                request_downgrader_patch_or_run(
                    &window,
                    &controller,
                    &settings_controller,
                    &shared_settings_snapshot,
                    worker_runtime,
                    downgrader_sink.clone(),
                );
            }
        }
    });

    window.on_confirm_requested({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);
        let settings_controller = Rc::clone(&settings_controller);
        let shared_settings_snapshot = Arc::clone(&shared_settings_snapshot);
        let downgrader_sink = downgrader_sink.clone();

        move || {
            if let Some(window) = window.upgrade() {
                request_downgrader_patch_or_run(
                    &window,
                    &controller,
                    &settings_controller,
                    &shared_settings_snapshot,
                    worker_runtime,
                    downgrader_sink.clone(),
                );
            }
        }
    });

    window.on_about_requested({
        let window = window.as_weak();

        move || {
            if let Some(window) = window.upgrade() {
                show_downgrader_about_dialog(&window);
            }
        }
    });

    window.on_about_close_requested({
        let window = window.as_weak();

        move || {
            if let Some(window) = window.upgrade() {
                hide_downgrader_about_dialog(&window);
            }
        }
    });

    window.on_modal_close_requested({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);

        move || {
            if let Some(window) = window.upgrade() {
                close_downgrader_modal(&window, &controller);
            }
        }
    });

    window.window().on_close_requested({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);

        move || {
            let Some(window) = window.upgrade() else {
                return CloseRequestResponse::HideWindow;
            };
            match close_downgrader_modal(&window, &controller) {
                DowngraderTransitionResult::CloseBlocked => CloseRequestResponse::KeepWindowShown,
                _ => CloseRequestResponse::HideWindow,
            }
        }
    });
}

fn bind_archive_patcher_worker_sink(
    app: &MainWindow,
    window: &ArchivePatcherWindow,
    controller: Arc<Mutex<ArchivePatcherController>>,
    overview_controller: Arc<Mutex<OverviewController>>,
    shared_settings_snapshot: Arc<Mutex<AppSettings>>,
    worker_runtime: WorkerRuntime,
    overview_sink: SlintEventLoopSink,
    runtime_handle: tokio::runtime::Handle,
) -> SlintEventLoopSink {
    let app = app.as_weak();
    let window = window.as_weak();
    SlintEventLoopSink::new(move |event| {
        let task_id = event.task.id.to_string();
        let task_kind = event.task.kind.label();
        let status = event.status.label();
        let refresh_after_completion = archive_patcher_completion_refresh_needed(&event);
        let Some(window) = window.upgrade() else {
            tracing::warn!(
                event = "s10-archive-patcher-worker-event-dropped",
                task_id = %task_id,
                task_kind,
                status,
                "Archive Patcher worker event arrived after the modal was gone"
            );
            return;
        };

        let Some(result) = with_archive_patcher_controller_mut(&controller, |controller| {
            handle_archive_patcher_worker_event(controller, event)
        }) else {
            return;
        };

        match result {
            ArchivePatcherTransitionResult::Applied => {
                tracing::debug!(
                    event = "s10-archive-patcher-worker-event-applied",
                    task_id = %task_id,
                    task_kind,
                    status,
                    refresh_after_completion,
                    "Archive Patcher worker event applied"
                );
                apply_current_archive_patcher_state(&window, &controller);
                if refresh_after_completion {
                    let Some(app) = app.upgrade() else {
                        tracing::warn!(
                            event = "s10-archive-patcher-overview-refresh-dropped",
                            task_id = %task_id,
                            "Archive Patcher completion could not refresh Overview because the main window was gone"
                        );
                        return;
                    };
                    let settings = overview_settings_for_archive_patcher_completion(
                        &shared_settings_snapshot,
                    );
                    tracing::info!(
                        event = "s10-archive-patcher-overview-refresh-requested",
                        task_id = %task_id,
                        "Requesting Overview refresh after Archive Patcher completion"
                    );
                    request_overview_refresh_from_settings(
                        &app,
                        &overview_controller,
                        settings,
                        worker_runtime,
                        overview_sink.clone(),
                        runtime_handle.clone(),
                    );
                }
            }
            ArchivePatcherTransitionResult::StaleIgnored => tracing::debug!(
                event = "s10-archive-patcher-worker-event-stale-ignored",
                task_id = %task_id,
                task_kind,
                status,
                "Archive Patcher worker event was stale and ignored"
            ),
            ArchivePatcherTransitionResult::Ignored => tracing::debug!(
                event = "s10-archive-patcher-worker-event-ignored",
                task_id = %task_id,
                task_kind,
                status,
                "Ignoring non-Archive-Patcher worker event on Archive Patcher sink"
            ),
            ArchivePatcherTransitionResult::Rejected => tracing::debug!(
                event = "s10-archive-patcher-worker-event-rejected",
                task_id = %task_id,
                task_kind,
                status,
                "Archive Patcher worker event was rejected by current modal state"
            ),
            ArchivePatcherTransitionResult::CloseBlocked => tracing::debug!(
                event = "s10-archive-patcher-worker-event-close-blocked",
                task_id = %task_id,
                task_kind,
                status,
                "Archive Patcher worker event unexpectedly hit close-blocked transition"
            ),
        }
    })
}

fn bind_archive_patcher_callbacks(
    window: &ArchivePatcherWindow,
    controller: Arc<Mutex<ArchivePatcherController>>,
    worker_runtime: WorkerRuntime,
    archive_patcher_sink: SlintEventLoopSink,
) {
    window.on_target_selected({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);
        let archive_patcher_sink = archive_patcher_sink.clone();

        move |target| {
            let Some(window) = window.upgrade() else {
                return;
            };
            let request = with_archive_patcher_controller_mut(&controller, |controller| {
                controller.set_target_from_ui_value(target.as_str())
            })
            .flatten();
            apply_current_archive_patcher_state(&window, &controller);
            if let Some(request) = request {
                schedule_archive_patcher_candidate_request(
                    &window,
                    &controller,
                    worker_runtime,
                    archive_patcher_sink.clone(),
                    request,
                );
            }
        }
    });

    window.on_name_filter_edited({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);
        let archive_patcher_sink = archive_patcher_sink.clone();

        move |filter| {
            let Some(window) = window.upgrade() else {
                return;
            };
            let request = with_archive_patcher_controller_mut(&controller, |controller| {
                controller.set_name_filter(filter.to_string())
            })
            .flatten();
            apply_current_archive_patcher_state(&window, &controller);
            if let Some(request) = request {
                schedule_archive_patcher_candidate_request(
                    &window,
                    &controller,
                    worker_runtime,
                    archive_patcher_sink.clone(),
                    request,
                );
            }
        }
    });

    window.on_patch_requested({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);
        let archive_patcher_sink = archive_patcher_sink.clone();

        move || {
            let Some(window) = window.upgrade() else {
                return;
            };
            let request = with_archive_patcher_controller_mut(&controller, |controller| {
                controller.request_patch_all()
            })
            .flatten();
            apply_current_archive_patcher_state(&window, &controller);
            match request {
                Some(ArchivePatcherPatchAllRequest::PreviewPlan(request)) => {
                    schedule_archive_patcher_plan_request(
                        &window,
                        &controller,
                        worker_runtime,
                        archive_patcher_sink.clone(),
                        request,
                    );
                }
                Some(ArchivePatcherPatchAllRequest::ConfirmedPatch(request)) => {
                    schedule_archive_patcher_patch_request(
                        &window,
                        &controller,
                        worker_runtime,
                        archive_patcher_sink.clone(),
                        request,
                    );
                }
                None => {}
            }
        }
    });

    window.on_restore_last_run_requested({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);
        let archive_patcher_sink = archive_patcher_sink.clone();

        move || {
            let Some(window) = window.upgrade() else {
                return;
            };
            let request = with_archive_patcher_controller_mut(&controller, |controller| {
                controller.request_restore_last_run()
            })
            .flatten();
            apply_current_archive_patcher_state(&window, &controller);
            if let Some(request) = request {
                schedule_archive_patcher_restore_request(
                    &window,
                    &controller,
                    worker_runtime,
                    archive_patcher_sink.clone(),
                    request,
                );
            }
        }
    });

    window.on_about_requested({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);

        move || {
            if let Some(window) = window.upgrade() {
                with_archive_patcher_controller_mut(&controller, |controller| {
                    controller.open_about()
                });
                apply_current_archive_patcher_state(&window, &controller);
            }
        }
    });

    window.on_about_close_requested({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);

        move || {
            if let Some(window) = window.upgrade() {
                with_archive_patcher_controller_mut(&controller, |controller| {
                    controller.close_about()
                });
                apply_current_archive_patcher_state(&window, &controller);
            }
        }
    });

    window.on_modal_close_requested({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);

        move || {
            if let Some(window) = window.upgrade() {
                close_archive_patcher_modal(&window, &controller);
            }
        }
    });

    window.window().on_close_requested({
        let window = window.as_weak();
        let controller = Arc::clone(&controller);

        move || {
            let Some(window) = window.upgrade() else {
                return CloseRequestResponse::HideWindow;
            };
            match close_archive_patcher_modal(&window, &controller) {
                ArchivePatcherTransitionResult::CloseBlocked => {
                    CloseRequestResponse::KeepWindowShown
                }
                _ => CloseRequestResponse::HideWindow,
            }
        }
    });
}

fn bind_tools_worker_sink(
    app: &MainWindow,
    controller: Arc<Mutex<ToolsController>>,
) -> SlintEventLoopSink {
    let app = app.as_weak();
    SlintEventLoopSink::new(move |event| {
        let task_id = event.task.id.to_string();
        let task_kind = event.task.kind.label();
        let status = event.status.label();
        let Some(app) = app.upgrade() else {
            tracing::warn!(
                event = "s05-tools-worker-event-dropped",
                task_id = %task_id,
                task_kind,
                status,
                "Tools worker event arrived after the Slint window was gone"
            );
            return;
        };

        let Some(result) = with_tools_controller_mut(&controller, |controller| {
            handle_tools_worker_event(controller, event)
        }) else {
            return;
        };

        if result.is_applied() {
            tracing::debug!(
                event = "s05-tools-worker-event-applied",
                task_id = %task_id,
                task_kind,
                status,
                "Tools worker event applied to render state"
            );
            apply_current_tools_state(&app, &controller);
        } else {
            tracing::debug!(
                event = "s05-tools-worker-event-ignored",
                task_id = %task_id,
                task_kind,
                status,
                "Tools worker event ignored because it belongs to another surface"
            );
        }
    })
}

fn bind_about_worker_sink(
    app: &MainWindow,
    controller: Arc<Mutex<AboutController>>,
) -> SlintEventLoopSink {
    let app = app.as_weak();
    SlintEventLoopSink::new(move |event| {
        let task_id = event.task.id.to_string();
        let task_kind = event.task.kind.label();
        let status = event.status.label();
        let Some(app) = app.upgrade() else {
            tracing::warn!(
                event = "s05-about-worker-event-dropped",
                task_id = %task_id,
                task_kind,
                status,
                "About worker event arrived after the Slint window was gone"
            );
            return;
        };

        let Some(result) = with_about_controller_mut(&controller, |controller| {
            handle_about_worker_event(controller, event)
        }) else {
            return;
        };

        if result.is_applied() {
            tracing::debug!(
                event = "s05-about-worker-event-applied",
                task_id = %task_id,
                task_kind,
                status,
                "About worker event applied to render state"
            );
            apply_current_about_state(&app, &controller);
        } else {
            tracing::debug!(
                event = "s05-about-worker-event-ignored",
                task_id = %task_id,
                task_kind,
                status,
                "About worker event ignored because it belongs to another surface"
            );
        }
    })
}

fn bind_tools_callbacks(
    app: &MainWindow,
    downgrader_window: &DowngraderWindow,
    archive_patcher_window: &ArchivePatcherWindow,
    controller: Arc<Mutex<ToolsController>>,
    downgrader_controller: Arc<Mutex<DowngraderController>>,
    archive_patcher_controller: Arc<Mutex<ArchivePatcherController>>,
    overview_controller: Arc<Mutex<OverviewController>>,
    settings_controller: Rc<RefCell<SettingsController<FileAssetResolver>>>,
    worker_runtime: WorkerRuntime,
    tools_sink: SlintEventLoopSink,
    downgrader_sink: SlintEventLoopSink,
    archive_patcher_sink: SlintEventLoopSink,
) {
    app.on_tool_action_requested({
        let app = app.as_weak();
        let downgrader_window = downgrader_window.as_weak();
        let archive_patcher_window = archive_patcher_window.as_weak();
        let controller = Arc::clone(&controller);
        let downgrader_controller = Arc::clone(&downgrader_controller);
        let archive_patcher_controller = Arc::clone(&archive_patcher_controller);
        let overview_controller = Arc::clone(&overview_controller);
        let settings_controller = Rc::clone(&settings_controller);
        let downgrader_sink = downgrader_sink.clone();
        let archive_patcher_sink = archive_patcher_sink.clone();

        move |action_id| {
            let action_id = action_id.to_string();
            if action_id == ToolActionId::DowngradeManager.as_str() {
                if let Some(downgrader_window) = downgrader_window.upgrade() {
                    request_open_downgrader_modal(
                        &downgrader_window,
                        &downgrader_controller,
                        &settings_controller,
                        worker_runtime,
                        downgrader_sink.clone(),
                        DowngraderOpenSource::Tools,
                    );
                }
                return;
            }

            if action_id == ToolActionId::ArchivePatcher.as_str() {
                if let Some(archive_patcher_window) = archive_patcher_window.upgrade() {
                    request_open_archive_patcher_modal(
                        &archive_patcher_window,
                        &archive_patcher_controller,
                        &overview_controller,
                        worker_runtime,
                        archive_patcher_sink.clone(),
                        ArchivePatcherOpenSource::Tools,
                    );
                }
                return;
            }

            if let Some(app) = app.upgrade() {
                request_tools_action(
                    &app,
                    &controller,
                    worker_runtime,
                    tools_sink.clone(),
                    action_id,
                );
            }
        }
    });

    app.on_tools_open_downgrade_manager_requested({
        let downgrader_window = downgrader_window.as_weak();
        let downgrader_controller = Arc::clone(&downgrader_controller);
        let settings_controller = Rc::clone(&settings_controller);
        let downgrader_sink = downgrader_sink.clone();

        move || {
            if let Some(downgrader_window) = downgrader_window.upgrade() {
                request_open_downgrader_modal(
                    &downgrader_window,
                    &downgrader_controller,
                    &settings_controller,
                    worker_runtime,
                    downgrader_sink.clone(),
                    DowngraderOpenSource::Tools,
                );
            }
        }
    });
}

fn bind_about_callbacks(
    app: &MainWindow,
    controller: Arc<Mutex<AboutController>>,
    worker_runtime: WorkerRuntime,
    about_sink: SlintEventLoopSink,
) {
    app.on_about_open_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);
        let about_sink = about_sink.clone();

        move |action_id| {
            if let Some(app) = app.upgrade() {
                request_about_action(
                    &app,
                    &controller,
                    worker_runtime,
                    about_sink.clone(),
                    action_id.to_string(),
                    AboutCallbackKind::Open,
                );
            }
        }
    });

    app.on_about_copy_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);
        let about_sink = about_sink.clone();

        move |action_id| {
            if let Some(app) = app.upgrade() {
                request_about_action(
                    &app,
                    &controller,
                    worker_runtime,
                    about_sink.clone(),
                    action_id.to_string(),
                    AboutCallbackKind::Copy,
                );
            }
        }
    });

    app.on_about_copy_label_reset_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);

        move |action_id| {
            let action_id = action_id.to_string();
            let Some(app) = app.upgrade() else {
                tracing::warn!(
                    event = "s05-about-copy-label-reset-dropped",
                    action_id,
                    "About copy-label reset arrived after the Slint window was gone"
                );
                return;
            };

            let Some(result) = with_about_controller_mut(&controller, |controller| {
                controller.reset_copy_label(&action_id)
            }) else {
                return;
            };

            if result.is_applied() {
                apply_current_about_state(&app, &controller);
            } else {
                tracing::debug!(
                    event = "s05-about-copy-label-reset-ignored-at-runtime",
                    action_id,
                    "About copy-label reset id was not applied"
                );
            }
        }
    });
}

fn bind_scanner_worker_sink(
    app: &MainWindow,
    controller: Arc<Mutex<ScannerController>>,
) -> SlintEventLoopSink {
    let app = app.as_weak();
    SlintEventLoopSink::new(move |event| {
        let task_id = event.task.id.to_string();
        let task_kind = event.task.kind.label();
        let status = event.status.label();
        let Some(app) = app.upgrade() else {
            tracing::warn!(
                event = "s07-scanner-worker-event-dropped",
                task_id = %task_id,
                task_kind,
                status,
                "Scanner worker event arrived after the Slint window was gone"
            );
            return;
        };

        let Some(result) = with_scanner_controller_mut(&controller, |controller| {
            handle_scanner_worker_event(controller, event)
        }) else {
            return;
        };

        match result {
            ScannerTransitionResult::Applied => {
                tracing::debug!(
                    event = "s07-scanner-worker-event-applied",
                    task_id = %task_id,
                    task_kind,
                    status,
                    "Scanner worker event applied to render state"
                );
                apply_current_scanner_state(&app, &controller);
            }
            ScannerTransitionResult::StaleIgnored => tracing::debug!(
                event = "s07-scanner-worker-event-stale-ignored",
                task_id = %task_id,
                task_kind,
                status,
                "Scanner worker event was stale and ignored"
            ),
            ScannerTransitionResult::Ignored => tracing::debug!(
                event = "s07-scanner-worker-event-ignored",
                task_id = %task_id,
                task_kind,
                status,
                "Ignoring non-Scanner worker event on Scanner sink"
            ),
            ScannerTransitionResult::Rejected => tracing::debug!(
                event = "s07-scanner-worker-event-rejected",
                task_id = %task_id,
                task_kind,
                status,
                "Scanner worker event was rejected by the current selection state"
            ),
        }
    })
}

fn bind_scanner_callbacks(
    app: &MainWindow,
    controller: Arc<Mutex<ScannerController>>,
    settings_controller: Rc<RefCell<SettingsController<FileAssetResolver>>>,
    worker_runtime: WorkerRuntime,
    scanner_sink: SlintEventLoopSink,
) {
    app.on_scanner_category_toggled({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);

        move |category_id, enabled| {
            let category_id = category_id.to_string();
            let Some(category) = scanner_category_from_ui_id(&category_id) else {
                tracing::warn!(
                    event = "s07-scanner-category-toggle-invalid",
                    category_id,
                    enabled,
                    "Scanner category callback id was invalid"
                );
                return;
            };

            let Some(result) = with_scanner_controller_mut(&controller, |controller| {
                controller.toggle_category(category, enabled)
            }) else {
                return;
            };

            if result.is_applied()
                && let Some(app) = app.upgrade()
            {
                apply_current_scanner_state(&app, &controller);
            }
        }
    });

    app.on_scanner_scan_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);
        let settings_controller = Rc::clone(&settings_controller);
        let scanner_sink = scanner_sink.clone();

        move || {
            if let Some(app) = app.upgrade() {
                request_scanner_scan(
                    &app,
                    &controller,
                    &settings_controller,
                    worker_runtime,
                    scanner_sink.clone(),
                );
            }
        }
    });

    app.on_scanner_result_selected({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);

        move |result_index| {
            let Some(result_index) = i32_to_usize(result_index) else {
                tracing::warn!(
                    event = "s07-scanner-result-selection-invalid",
                    result_index,
                    "Scanner result selection index was invalid"
                );
                return;
            };
            let Some(result) = with_scanner_controller_mut(&controller, |controller| {
                controller.select_result(result_index)
            }) else {
                return;
            };
            if result.is_applied()
                && let Some(app) = app.upgrade()
            {
                apply_current_scanner_state(&app, &controller);
            }
        }
    });

    app.on_scanner_auto_fix_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);
        let scanner_sink = scanner_sink.clone();

        move || {
            if let Some(app) = app.upgrade() {
                request_scanner_auto_fix(&app, &controller, worker_runtime, scanner_sink.clone());
            }
        }
    });

    app.on_scanner_copy_details_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);
        let scanner_sink = scanner_sink.clone();

        move || {
            if let Some(app) = app.upgrade() {
                request_scanner_action(
                    &app,
                    &controller,
                    worker_runtime,
                    scanner_sink.clone(),
                    ScannerActionKind::CopyDetails,
                );
            }
        }
    });

    app.on_scanner_file_list_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);

        move || {
            if let Some(app) = app.upgrade() {
                toggle_scanner_file_list(&app, &controller);
            }
        }
    });

    app.on_scanner_open_path_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);
        let scanner_sink = scanner_sink.clone();

        move || {
            if let Some(app) = app.upgrade() {
                request_scanner_action(
                    &app,
                    &controller,
                    worker_runtime,
                    scanner_sink.clone(),
                    ScannerActionKind::OpenLocation,
                );
            }
        }
    });

    app.on_scanner_open_url_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);
        let scanner_sink = scanner_sink.clone();

        move || {
            if let Some(app) = app.upgrade() {
                request_scanner_action(
                    &app,
                    &controller,
                    worker_runtime,
                    scanner_sink.clone(),
                    ScannerActionKind::OpenSolutionUrl,
                );
            }
        }
    });

    app.on_scanner_copy_url_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);
        let scanner_sink = scanner_sink.clone();

        move || {
            if let Some(app) = app.upgrade() {
                request_scanner_action(
                    &app,
                    &controller,
                    worker_runtime,
                    scanner_sink.clone(),
                    ScannerActionKind::CopySolutionUrl,
                );
            }
        }
    });
}

fn bind_overview_worker_sink(
    app: &MainWindow,
    controller: Arc<Mutex<OverviewController>>,
) -> SlintEventLoopSink {
    let app = app.as_weak();
    SlintEventLoopSink::new(move |event| {
        let Some(app) = app.upgrade() else {
            tracing::warn!(
                event = "overview-worker-event-dropped",
                "Overview worker event arrived after the Slint window was gone"
            );
            return;
        };

        let Some(applied) = with_overview_controller_mut(&controller, |controller| {
            let result = controller.handle_worker_event(event);
            result.is_applied()
        }) else {
            return;
        };

        if applied {
            apply_current_overview_snapshot(&app, &controller);
        }
    })
}

fn bind_overview_callbacks(
    app: &MainWindow,
    downgrader_window: &DowngraderWindow,
    archive_patcher_window: &ArchivePatcherWindow,
    overview_controller: Arc<Mutex<OverviewController>>,
    downgrader_controller: Arc<Mutex<DowngraderController>>,
    archive_patcher_controller: Arc<Mutex<ArchivePatcherController>>,
    settings_controller: Rc<RefCell<SettingsController<FileAssetResolver>>>,
    worker_runtime: WorkerRuntime,
    overview_sink: SlintEventLoopSink,
    downgrader_sink: SlintEventLoopSink,
    archive_patcher_sink: SlintEventLoopSink,
    runtime_handle: tokio::runtime::Handle,
) {
    app.on_overview_refresh_requested({
        let app = app.as_weak();
        let overview_controller = Arc::clone(&overview_controller);
        let settings_controller = Rc::clone(&settings_controller);
        let overview_sink = overview_sink.clone();
        let runtime_handle = runtime_handle.clone();

        move || {
            if let Some(app) = app.upgrade() {
                request_overview_refresh(
                    &app,
                    &overview_controller,
                    &settings_controller,
                    worker_runtime,
                    overview_sink.clone(),
                    runtime_handle.clone(),
                );
            }
        }
    });

    app.on_overview_open_game_path_requested({
        let app = app.as_weak();
        let overview_controller = Arc::clone(&overview_controller);
        let overview_sink = overview_sink.clone();

        move || {
            if let Some(app) = app.upgrade() {
                schedule_overview_action(
                    &app,
                    &overview_controller,
                    worker_runtime,
                    overview_sink.clone(),
                    OverviewDeferredActionKind::OpenGamePath,
                    "Game path is not available.",
                    OverviewController::game_path_action,
                );
            }
        }
    });

    app.on_overview_open_nexus_update_requested({
        let app = app.as_weak();
        let overview_controller = Arc::clone(&overview_controller);
        let overview_sink = overview_sink.clone();

        move || {
            if let Some(app) = app.upgrade() {
                schedule_overview_action(
                    &app,
                    &overview_controller,
                    worker_runtime,
                    overview_sink.clone(),
                    OverviewDeferredActionKind::OpenUpdateProvider(UpdateProvider::NexusMods),
                    "Nexus Mods update link is not available.",
                    |controller| controller.update_provider_action(UpdateProvider::NexusMods),
                );
            }
        }
    });

    app.on_overview_open_github_update_requested({
        let app = app.as_weak();
        let overview_controller = Arc::clone(&overview_controller);
        let overview_sink = overview_sink.clone();

        move || {
            if let Some(app) = app.upgrade() {
                schedule_overview_action(
                    &app,
                    &overview_controller,
                    worker_runtime,
                    overview_sink.clone(),
                    OverviewDeferredActionKind::OpenUpdateProvider(UpdateProvider::Github),
                    "GitHub update link is not available.",
                    |controller| controller.update_provider_action(UpdateProvider::Github),
                );
            }
        }
    });

    app.on_overview_open_downgrade_manager_requested({
        let downgrader_window = downgrader_window.as_weak();
        let downgrader_controller = Arc::clone(&downgrader_controller);
        let settings_controller = Rc::clone(&settings_controller);
        let downgrader_sink = downgrader_sink.clone();

        move || {
            if let Some(downgrader_window) = downgrader_window.upgrade() {
                request_open_downgrader_modal(
                    &downgrader_window,
                    &downgrader_controller,
                    &settings_controller,
                    worker_runtime,
                    downgrader_sink.clone(),
                    DowngraderOpenSource::Overview,
                );
            }
        }
    });

    app.on_overview_open_archive_patcher_requested({
        let archive_patcher_window = archive_patcher_window.as_weak();
        let archive_patcher_controller = Arc::clone(&archive_patcher_controller);
        let overview_controller = Arc::clone(&overview_controller);
        let archive_patcher_sink = archive_patcher_sink.clone();

        move || {
            if let Some(archive_patcher_window) = archive_patcher_window.upgrade() {
                request_open_archive_patcher_modal(
                    &archive_patcher_window,
                    &archive_patcher_controller,
                    &overview_controller,
                    worker_runtime,
                    archive_patcher_sink.clone(),
                    ArchivePatcherOpenSource::Overview,
                );
            }
        }
    });
}

fn bind_f4se_worker_sink(
    app: &MainWindow,
    controller: Arc<Mutex<F4seController>>,
) -> SlintEventLoopSink {
    let app = app.as_weak();
    SlintEventLoopSink::new(move |event| {
        let task_id = event.task.id.to_string();
        let task_kind = event.task.kind.label();
        let status = event.status.label();
        let Some(app) = app.upgrade() else {
            tracing::warn!(
                event = "s06-f4se-worker-event-dropped",
                task_id = %task_id,
                task_kind,
                status,
                "F4SE worker event arrived after the Slint window was gone"
            );
            return;
        };

        let Some(result) = with_f4se_controller_mut(&controller, |controller| {
            handle_f4se_worker_event(controller, event)
        }) else {
            return;
        };

        match result {
            F4seTransitionResult::Applied => apply_current_f4se_snapshot(&app, &controller),
            F4seTransitionResult::StaleIgnored => tracing::debug!(
                event = "s06-f4se-worker-stale-ignored",
                task_id = %task_id,
                task_kind,
                status,
                "F4SE worker event was stale and ignored"
            ),
            F4seTransitionResult::Ignored => tracing::debug!(
                event = "s06-f4se-worker-event-ignored",
                task_id = %task_id,
                task_kind,
                status,
                "Ignoring non-F4SE worker event on F4SE sink"
            ),
        }
    })
}

fn bind_f4se_callbacks(
    app: &MainWindow,
    controller: Arc<Mutex<F4seController>>,
    worker_runtime: WorkerRuntime,
    f4se_sink: SlintEventLoopSink,
) {
    app.on_f4se_tab_activated({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);
        let f4se_sink = f4se_sink.clone();

        move || {
            if let Some(app) = app.upgrade() {
                request_f4se_initial_scan(&app, &controller, worker_runtime, f4se_sink.clone());
            }
        }
    });
}

fn request_f4se_initial_scan(
    app: &MainWindow,
    controller: &Arc<Mutex<F4seController>>,
    worker_runtime: WorkerRuntime,
    f4se_sink: SlintEventLoopSink,
) {
    let Some(request) = take_f4se_initial_scan_request(controller) else {
        tracing::debug!(
            event = "s06-f4se-lazy-activation-ignored",
            "F4SE tab activation did not schedule a duplicate scan"
        );
        return;
    };

    let scan_id = request.scan_id;
    let task = request.task.clone();
    tracing::info!(
        event = "s06-f4se-scan-schedule",
        scan_id,
        task_id = %task.id,
        "Scheduling lazy F4SE DLL scan worker"
    );

    if let Err(error) = worker_runtime.spawn_blocking_task(task, f4se_sink, move |_context| {
        build_f4se_scan_payload(scan_id)
    }) {
        tracing::error!(
            event = "s06-f4se-scan-spawn-failed",
            scan_id,
            error = %error,
            "F4SE scan worker could not be scheduled"
        );
        with_f4se_controller_mut(controller, |controller| {
            controller.spawn_failed(scan_id, error);
        });
        apply_current_f4se_snapshot(app, controller);
        return;
    }

    with_f4se_controller_mut(controller, |controller| {
        controller.scan_started(scan_id);
    });
    apply_current_f4se_snapshot(app, controller);
}

fn take_f4se_initial_scan_request(
    controller: &Arc<Mutex<F4seController>>,
) -> Option<F4seScanWorkerRequest> {
    with_f4se_controller_mut(controller, F4seController::request_initial_scan).flatten()
}

fn build_f4se_scan_payload(scan_id: u64) -> BlockingWorkerResult {
    let snapshot = build_f4se_scan_snapshot(scan_id);
    Ok(WorkerTaskOutcome::Completed(f4se_scan_completed_payload(
        scan_id, snapshot,
    )))
}

fn build_f4se_scan_snapshot(scan_id: u64) -> F4seScanSnapshot {
    let span = tracing::info_span!("f4se_scan_worker", scan_id);
    let _guard = span.enter();
    tracing::info!(
        event = "s06-f4se-scan-started",
        scan_id,
        "F4SE scan worker started"
    );

    let filesystem = RealFilesystem::new();
    let registry = RealRegistry::new();
    let process = RealProcessInspector::new();
    let current_working_directory = current_working_directory();
    let local_appdata = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);

    let mut discovery_request = DiscoveryRequest::new(current_working_directory);
    discovery_request = discovery_request.with_current_process_id(std::process::id());
    if let Some(path) = local_appdata.clone() {
        discovery_request = discovery_request.with_local_appdata(path);
    }

    let discovery =
        DiscoveryService::new(&filesystem, &registry, &process).discover(&discovery_request);
    if let Err(error) = &discovery.game {
        tracing::warn!(
            event = "s06-f4se-discovery-failure",
            scan_id,
            safe_message = %error.user_message(),
            "F4SE discovery did not find a usable Fallout 4 installation"
        );
    }
    if let Err(error) = &discovery.mod_manager {
        tracing::warn!(
            event = "s06-f4se-manager-discovery-failure",
            scan_id,
            safe_message = %error.user_message(),
            "F4SE mod-manager discovery failed safely"
        );
    }

    let mut collection_environment = OverviewCollectionEnvironment::new();
    if let Some(path) = local_appdata {
        collection_environment = collection_environment.with_local_appdata(path);
    }

    let collected = match &discovery.game {
        Ok(installation) => {
            let collector = OverviewCollector::new(&filesystem, &process);
            collector.collect(OverviewCollectionRequest::new(
                installation,
                &collection_environment,
            ))
        }
        Err(_) => OverviewCollectedFacts::default(),
    };
    let current_game = f4se_current_game_from_overview_facts(&collected);
    tracing::info!(
        event = "s06-f4se-current-game-classified",
        scan_id,
        current_game = ?current_game,
        binaries = collected.binaries.len(),
        "F4SE current-game target classified from Overview Fallout4.exe facts"
    );

    let mod_manager_detected = matches!(&discovery.mod_manager, Ok(Some(_)));
    let inspector = PeliteF4seDllInspector::new();
    let scan_service = F4seScanService::new(&filesystem, &inspector);
    let report = scan_service.scan(F4seScanRequest::new(
        discovery.game.as_ref().ok(),
        current_game,
        mod_manager_detected,
    ));
    trace_f4se_scan_diagnostics(scan_id, current_game, &report.diagnostics);

    report.snapshot
}

fn f4se_current_game_from_overview_facts(collected: &OverviewCollectedFacts) -> F4seGameTarget {
    let install_type = collected
        .binaries
        .iter()
        .find(|fact| is_fallout4_executable_fact(&fact.file_name))
        .map(|fact| fact.install_type)
        .unwrap_or(Fallout4InstallType::Unknown);
    F4seGameTarget::from_install_type(install_type)
}

fn is_fallout4_executable_fact(file_name: &str) -> bool {
    file_name
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(file_name)
        .eq_ignore_ascii_case(FALLOUT4_EXECUTABLE)
}

fn trace_f4se_scan_diagnostics(
    scan_id: u64,
    current_game: F4seGameTarget,
    diagnostics: &F4seScanDiagnostics,
) {
    tracing::info!(
        event = "s06-f4se-scan-counts",
        scan_id,
        current_game = ?current_game,
        enumerated_entries = diagnostics.enumerated_entry_count,
        dll_candidates = diagnostics.dll_candidate_count,
        inspected = diagnostics.inspected_dll_count,
        f4se = diagnostics.f4se_dll_count,
        non_f4se = diagnostics.non_f4se_dll_count,
        skipped = diagnostics.skipped_entry_count,
        unreadable = diagnostics.unreadable_dll_count,
        malformed = diagnostics.malformed_dll_count,
        version_warnings = diagnostics.version_data_warning_count,
        "F4SE scan diagnostics collected"
    );

    for detail in &diagnostics.details {
        match detail.kind {
            F4seScanDiagnosticKind::MissingDataFolder
            | F4seScanDiagnosticKind::MissingPluginsFolder => tracing::warn!(
                event = "s06-f4se-scan-missing-folder",
                scan_id,
                kind = ?detail.kind,
                path = detail
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                safe_message = detail.safe_message.as_str(),
                "F4SE scan prerequisite folder is unavailable"
            ),
            F4seScanDiagnosticKind::DirectoryReadFailed => tracing::warn!(
                event = "s06-f4se-scan-directory-read-failed",
                scan_id,
                kind = ?detail.kind,
                path = detail
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                safe_message = detail.safe_message.as_str(),
                "F4SE scan directory enumeration failed safely"
            ),
            F4seScanDiagnosticKind::FileReadFailed
            | F4seScanDiagnosticKind::DllInspectionFailed
            | F4seScanDiagnosticKind::VersionDataUnreadable => tracing::warn!(
                event = "s06-f4se-scan-dll-inspection-issue",
                scan_id,
                kind = ?detail.kind,
                path = detail
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                safe_message = detail.safe_message.as_str(),
                "F4SE DLL inspection produced a visible safe diagnostic"
            ),
            F4seScanDiagnosticKind::SkippedEntry => tracing::debug!(
                event = "s06-f4se-scan-entry-skipped",
                scan_id,
                path = detail
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                safe_message = detail.safe_message.as_str(),
                "F4SE scan skipped a direct child outside DLL scope"
            ),
        }
    }
}

fn request_open_downgrader_modal<R: AssetResolver>(
    window: &DowngraderWindow,
    controller: &Arc<Mutex<DowngraderController>>,
    settings_controller: &Rc<RefCell<SettingsController<R>>>,
    worker_runtime: WorkerRuntime,
    downgrader_sink: SlintEventLoopSink,
    source: DowngraderOpenSource,
) {
    let settings_snapshot = settings_controller
        .borrow()
        .current_downgrader_settings()
        .clone();
    let Some(request) = with_downgrader_controller_mut(controller, |controller| {
        controller.open(settings_snapshot, None)
    })
    .flatten() else {
        apply_current_downgrader_state(window, controller);
        return;
    };

    tracing::info!(
        event = "s09-downgrader-open-schedule",
        source = source.label(),
        request_id = request.request_id,
        task_id = %request.task.id,
        "Scheduling Downgrader status worker after modal open"
    );
    if let Err(error) = window.show() {
        tracing::warn!(
            event = "s09-downgrader-show-failed",
            source = source.label(),
            diagnostic = %error,
            "Downgrader modal show failed"
        );
    }
    apply_current_downgrader_state(window, controller);
    schedule_downgrader_status_request(
        window,
        controller,
        worker_runtime,
        downgrader_sink,
        request,
    );
}

fn request_downgrader_patch_or_run<R: AssetResolver>(
    window: &DowngraderWindow,
    controller: &Arc<Mutex<DowngraderController>>,
    settings_controller: &Rc<RefCell<SettingsController<R>>>,
    shared_settings_snapshot: &Arc<Mutex<AppSettings>>,
    worker_runtime: WorkerRuntime,
    downgrader_sink: SlintEventLoopSink,
) {
    let Some(options) =
        with_downgrader_controller_mut(controller, |controller| controller.options())
    else {
        return;
    };
    let save_result = settings_controller
        .borrow_mut()
        .save_downgrader_settings_for_workflow(downgrader_settings_from_options(options));
    if !save_result.should_schedule_workflow() {
        tracing::warn!(
            event = "s09-downgrader-workflow-not-scheduled",
            reason = "settings-save-failed",
            "Downgrader work not scheduled because settings persistence failed"
        );
        with_downgrader_controller_mut(controller, |controller| {
            controller.set_keep_backups(save_result.visible_settings.keep_backups);
            controller.set_delete_deltas(save_result.visible_settings.delete_deltas);
        });
        apply_current_downgrader_state(window, controller);
        return;
    }
    remember_current_settings_snapshot(settings_controller, shared_settings_snapshot);

    let Some(request) =
        with_downgrader_controller_mut(controller, |controller| controller.request_patch_all())
            .flatten()
    else {
        apply_current_downgrader_state(window, controller);
        return;
    };
    apply_current_downgrader_state(window, controller);

    match request {
        DowngraderPatchWorkerRequest::PreviewPlan(request) => {
            schedule_downgrader_plan_request(
                window,
                controller,
                worker_runtime,
                downgrader_sink,
                request,
            );
        }
        DowngraderPatchWorkerRequest::ConfirmedRun(request) => {
            schedule_downgrader_run_request(
                window,
                controller,
                worker_runtime,
                downgrader_sink,
                request,
            );
        }
    }
}

fn schedule_downgrader_status_request(
    window: &DowngraderWindow,
    controller: &Arc<Mutex<DowngraderController>>,
    worker_runtime: WorkerRuntime,
    downgrader_sink: SlintEventLoopSink,
    request: DowngraderStatusWorkerRequest,
) {
    let request_id = request.request_id;
    let task = request.task.clone();
    let task_for_failure = task.clone();
    tracing::info!(
        event = "s09-downgrader-status-schedule",
        request_id,
        task_id = %task.id,
        "Scheduling Downgrader status worker"
    );
    if let Err(error) = worker_runtime.spawn_blocking_task(task, downgrader_sink, move |_context| {
        build_downgrader_status_payload(request)
    }) {
        tracing::error!(
            event = "s09-downgrader-status-spawn-failed",
            request_id,
            task_id = %task_for_failure.id,
            error = %error,
            "Downgrader status worker could not be scheduled"
        );
        with_downgrader_controller_mut(controller, |controller| {
            controller.spawn_failed(DowngraderWorkerRequestKind::Status, request_id, error);
        });
        apply_current_downgrader_state(window, controller);
    }
}

fn schedule_downgrader_plan_request(
    window: &DowngraderWindow,
    controller: &Arc<Mutex<DowngraderController>>,
    worker_runtime: WorkerRuntime,
    downgrader_sink: SlintEventLoopSink,
    request: DowngraderPlanWorkerRequest,
) {
    let request_id = request.request_id;
    let task = request.task.clone();
    let task_for_failure = task.clone();
    tracing::info!(
        event = "s09-downgrader-plan-schedule",
        request_id,
        task_id = %task.id,
        "Scheduling Downgrader plan worker"
    );
    if let Err(error) = worker_runtime.spawn_blocking_task(task, downgrader_sink, move |_context| {
        build_downgrader_plan_payload(request)
    }) {
        tracing::error!(
            event = "s09-downgrader-plan-spawn-failed",
            request_id,
            task_id = %task_for_failure.id,
            error = %error,
            "Downgrader plan worker could not be scheduled"
        );
        with_downgrader_controller_mut(controller, |controller| {
            controller.spawn_failed(DowngraderWorkerRequestKind::Plan, request_id, error);
        });
        apply_current_downgrader_state(window, controller);
    }
}

fn schedule_downgrader_run_request(
    window: &DowngraderWindow,
    controller: &Arc<Mutex<DowngraderController>>,
    worker_runtime: WorkerRuntime,
    downgrader_sink: SlintEventLoopSink,
    request: DowngraderRunWorkerRequest,
) {
    let request_id = request.request_id;
    let task = request.task.clone();
    let task_for_failure = task.clone();
    tracing::info!(
        event = "s09-downgrader-run-schedule",
        request_id,
        confirmed_plan_request_id = request.confirmed_plan_request_id,
        task_id = %task.id,
        "Scheduling Downgrader confirmed run worker"
    );
    if let Err(error) = worker_runtime.spawn_blocking_task(task, downgrader_sink, move |context| {
        build_downgrader_run_payload(context, request)
    }) {
        tracing::error!(
            event = "s09-downgrader-run-spawn-failed",
            request_id,
            task_id = %task_for_failure.id,
            error = %error,
            "Downgrader run worker could not be scheduled"
        );
        with_downgrader_controller_mut(controller, |controller| {
            controller.spawn_failed(DowngraderWorkerRequestKind::Run, request_id, error);
        });
        apply_current_downgrader_state(window, controller);
    }
}

fn overview_settings_for_downgrader_completion(
    shared_settings_snapshot: &Arc<Mutex<AppSettings>>,
) -> AppSettings {
    match shared_settings_snapshot.lock() {
        Ok(settings) => settings.clone(),
        Err(error) => {
            tracing::error!(
                event = "s09-downgrader-overview-settings-unavailable",
                diagnostic = %error,
                "Current settings snapshot unavailable; using safe defaults for Overview refresh"
            );
            AppSettings::default()
        }
    }
}

fn schedule_downgrader_completion_refresh(
    app: Option<&MainWindow>,
    window: &DowngraderWindow,
    controller: &Arc<Mutex<DowngraderController>>,
    overview_controller: &Arc<Mutex<OverviewController>>,
    overview_settings: AppSettings,
    worker_runtime: WorkerRuntime,
    overview_sink: SlintEventLoopSink,
    runtime_handle: tokio::runtime::Handle,
) {
    if let Some(status_request) = with_downgrader_controller_mut(controller, |controller| {
        controller.take_pending_status_refresh()
    })
    .flatten()
    {
        schedule_downgrader_status_request(
            window,
            controller,
            worker_runtime,
            bind_downgrader_worker_sink_for_status_only(window, Arc::clone(controller)),
            status_request,
        );
    }

    if let Some(app) = app {
        tracing::info!(
            event = "s09-downgrader-overview-refresh-requested",
            "Requesting Overview refresh after Downgrader completion"
        );
        request_overview_refresh_from_settings(
            app,
            overview_controller,
            overview_settings,
            worker_runtime,
            overview_sink,
            runtime_handle,
        );
    }
}

fn bind_downgrader_worker_sink_for_status_only(
    window: &DowngraderWindow,
    controller: Arc<Mutex<DowngraderController>>,
) -> SlintEventLoopSink {
    let window = window.as_weak();
    SlintEventLoopSink::new(move |event| {
        let Some(window) = window.upgrade() else {
            return;
        };
        let Some(result) = with_downgrader_controller_mut(&controller, |controller| {
            handle_downgrader_worker_event(controller, event)
        }) else {
            return;
        };
        if result.is_applied() {
            apply_current_downgrader_state(&window, &controller);
        }
    })
}

fn build_downgrader_status_payload(request: DowngraderStatusWorkerRequest) -> BlockingWorkerResult {
    let span = tracing::info_span!(
        "s09_downgrader_status_worker",
        request_id = request.request_id
    );
    let _guard = span.enter();
    let filesystem = RealFilesystem::new();
    let installation = request
        .installation
        .or_else(discover_fallout4_installation_for_downgrader);
    let service = DowngraderService::new(&filesystem);
    let snapshot = service
        .status_snapshot(DowngraderStatusRequest::new(
            request.request_id,
            installation.as_ref(),
        ))
        .map_err(downgrader_service_failure)?;
    Ok(WorkerTaskOutcome::Completed(
        downgrader_status_loaded_payload(request.request_id, snapshot),
    ))
}

fn build_downgrader_plan_payload(request: DowngraderPlanWorkerRequest) -> BlockingWorkerResult {
    let span = tracing::info_span!(
        "s09_downgrader_plan_worker",
        request_id = request.request_id
    );
    let _guard = span.enter();
    let filesystem = RealFilesystem::new();
    let installation = request
        .installation
        .or_else(discover_fallout4_installation_for_downgrader);
    let service = DowngraderService::new(&filesystem);
    let plan = service
        .preview_plan(DowngraderPlanRequest::new(
            request.request_id,
            installation.as_ref(),
            request.options,
        ))
        .map_err(downgrader_service_failure)?;
    Ok(WorkerTaskOutcome::Completed(downgrader_plan_ready_payload(
        request.request_id,
        plan,
    )))
}

fn build_downgrader_run_payload<S>(
    context: workers::WorkerTaskContext<S>,
    request: DowngraderRunWorkerRequest,
) -> BlockingWorkerResult
where
    S: WorkerEventSink,
{
    let span = tracing::info_span!("s09_downgrader_run_worker", request_id = request.request_id);
    let _guard = span.enter();
    let filesystem = RealFilesystem::new();
    let installation = request
        .installation
        .or_else(discover_fallout4_installation_for_downgrader);
    let downloader = ReqwestDeltaDownloader::new().map_err(|error| {
        WorkerFailure::new(error.user_message().to_owned()).with_diagnostic(
            error
                .diagnostic()
                .unwrap_or("downgrader downloader could not be created")
                .to_owned(),
        )
    })?;
    let applier = VcdiffDeltaApplier;
    let service = DowngraderService::new(&filesystem);
    let request_id = request.request_id;
    let result = service
        .execute_confirmed_with_events(
            DowngraderExecutionRequest::new(request_id, installation.as_ref(), request.options)
                .with_confirmed_plan_digest(request.confirmed_plan_digest.clone()),
            &downloader,
            &applier,
            |row| emit_downgrader_run_log_event(&context, request_id, row),
            |progress| emit_downgrader_run_progress_event(&context, request_id, progress),
        )
        .map_err(downgrader_service_failure)?;

    Ok(WorkerTaskOutcome::Completed(
        downgrader_run_completed_payload(request_id, result),
    ))
}

fn emit_downgrader_run_progress_event<S>(
    context: &workers::WorkerTaskContext<S>,
    request_id: u64,
    progress: &DowngraderExecutionProgressEvent,
) where
    S: WorkerEventSink,
{
    if let Err(error) = context.emit_payload(
        WorkerTaskStatus::Progress,
        downgrader_progress_payload(request_id, progress.progress),
    ) {
        tracing::warn!(
            event = "s09-downgrader-progress-handoff-failed",
            request_id,
            relative_path = progress.relative_path,
            patch_name = progress.patch_name.as_str(),
            diagnostic = %error,
            "Downgrader progress event could not be handed to UI"
        );
    }
}

fn emit_downgrader_run_log_event<S>(
    context: &workers::WorkerTaskContext<S>,
    request_id: u64,
    row: &DowngraderExecutionLogRow,
) where
    S: WorkerEventSink,
{
    if let Err(error) = context.emit_payload(
        WorkerTaskStatus::Progress,
        downgrader_log_row_payload(request_id, row.clone()),
    ) {
        tracing::warn!(
            event = "s09-downgrader-log-handoff-failed",
            request_id,
            diagnostic = %error,
            "Downgrader log row could not be handed to UI"
        );
    }
}

fn downgrader_service_failure(
    error: services::downgrader::DowngraderServiceError,
) -> WorkerFailure {
    WorkerFailure::new(error.user_message().to_owned()).with_diagnostic(error.to_string())
}

fn discover_fallout4_installation_for_downgrader() -> Option<domain::discovery::Fallout4Installation>
{
    let filesystem = RealFilesystem::new();
    let registry = RealRegistry::new();
    let process = RealProcessInspector::new();
    let mut discovery_request = DiscoveryRequest::new(current_working_directory())
        .with_current_process_id(std::process::id());
    if let Some(path) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        discovery_request = discovery_request.with_local_appdata(path);
    }
    match DiscoveryService::new(&filesystem, &registry, &process)
        .discover(&discovery_request)
        .game
    {
        Ok(installation) => Some(installation),
        Err(error) => {
            tracing::warn!(
                event = "s09-downgrader-discovery-failed",
                safe_message = error.user_message(),
                "Downgrader discovery did not find a usable Fallout 4 installation"
            );
            None
        }
    }
}

fn downgrader_settings_from_options(options: DowngraderOptionsSnapshot) -> DowngraderSettings {
    DowngraderSettings {
        keep_backups: options.keep_backups,
        delete_deltas: options.delete_deltas,
    }
}

fn project_downgrader_about_dialog() -> DowngraderAboutProjection {
    DowngraderAboutProjection {
        title: ABOUT_DOWNGRADING_TITLE.to_owned(),
        body: ABOUT_DOWNGRADING_BODY.to_owned(),
    }
}

fn show_downgrader_about_dialog(window: &DowngraderWindow) {
    let projection = project_downgrader_about_dialog();
    window.set_about_title(projection.title.as_str().into());
    window.set_about_body(projection.body.as_str().into());
    window.set_about_dialog_visible(true);
    tracing::info!(
        event = "s09-downgrader-about-shown",
        title = projection.title.as_str(),
        "Downgrader About dialog shown"
    );
}

fn hide_downgrader_about_dialog(window: &DowngraderWindow) {
    window.set_about_dialog_visible(false);
    tracing::debug!(
        event = "s09-downgrader-about-hidden",
        "Downgrader About dialog hidden"
    );
}

fn close_downgrader_modal(
    window: &DowngraderWindow,
    controller: &Arc<Mutex<DowngraderController>>,
) -> DowngraderTransitionResult {
    let result =
        with_downgrader_controller_mut(controller, |controller| controller.request_close())
            .unwrap_or(DowngraderTransitionResult::Ignored);
    apply_current_downgrader_state(window, controller);
    if result.is_applied() {
        hide_downgrader_about_dialog(window);
        if let Err(error) = window.hide() {
            tracing::warn!(
                event = "s09-downgrader-hide-failed",
                diagnostic = %error,
                "Downgrader modal hide failed"
            );
        }
    }
    result
}

fn request_open_archive_patcher_modal(
    window: &ArchivePatcherWindow,
    controller: &Arc<Mutex<ArchivePatcherController>>,
    overview_controller: &Arc<Mutex<OverviewController>>,
    worker_runtime: WorkerRuntime,
    archive_patcher_sink: SlintEventLoopSink,
    source: ArchivePatcherOpenSource,
) {
    let snapshot = with_overview_controller_mut(overview_controller, |controller| {
        controller.snapshot().clone()
    });
    let manifest_path = archive_patcher_manifest_path();
    let manifest_available = archive_patcher_manifest_available(&manifest_path);
    let request = match snapshot {
        Some(snapshot) if !snapshot.archive_records.is_empty() => {
            let archive_count = snapshot.archive_records.len();
            let data_root_available = snapshot.data_path.is_some();
            tracing::info!(
                event = "s10-archive-patcher-open-requested",
                source = source.label(),
                archive_count,
                data_root_available,
                manifest_available,
                manifest_path = %manifest_path.display(),
                "Opening Archive Patcher from current Overview archive records"
            );
            with_archive_patcher_controller_mut(controller, |controller| {
                controller.open(
                    snapshot.archive_records,
                    snapshot.data_path,
                    manifest_path,
                    manifest_available,
                )
            })
            .flatten()
        }
        Some(snapshot) => {
            tracing::warn!(
                event = "s10-archive-patcher-open-missing-overview-archives",
                source = source.label(),
                refresh_phase = ?snapshot.refresh.phase,
                "Archive Patcher opened without Overview archive records"
            );
            with_archive_patcher_controller_mut(controller, |controller| {
                controller.open_unavailable(ARCHIVE_PATCHER_OVERVIEW_UNAVAILABLE_MESSAGE)
            });
            None
        }
        None => {
            tracing::error!(
                event = "s10-archive-patcher-open-overview-unavailable",
                source = source.label(),
                "Archive Patcher could not read Overview state"
            );
            with_archive_patcher_controller_mut(controller, |controller| {
                controller.open_unavailable(ARCHIVE_PATCHER_OVERVIEW_UNAVAILABLE_MESSAGE)
            });
            None
        }
    };

    apply_current_archive_patcher_state(window, controller);
    if let Err(error) = window.show() {
        tracing::warn!(
            event = "s10-archive-patcher-show-failed",
            source = source.label(),
            diagnostic = %error,
            "Archive Patcher modal show failed"
        );
    }
    if let Some(request) = request {
        schedule_archive_patcher_candidate_request(
            window,
            controller,
            worker_runtime,
            archive_patcher_sink,
            request,
        );
    }
}

fn close_archive_patcher_modal(
    window: &ArchivePatcherWindow,
    controller: &Arc<Mutex<ArchivePatcherController>>,
) -> ArchivePatcherTransitionResult {
    let result = with_archive_patcher_controller_mut(controller, |controller| {
        controller.request_close()
    })
    .unwrap_or(ArchivePatcherTransitionResult::Ignored);
    apply_current_archive_patcher_state(window, controller);
    if result.is_applied()
        && let Err(error) = window.hide()
    {
        tracing::warn!(
            event = "s10-archive-patcher-hide-failed",
            diagnostic = %error,
            "Archive Patcher modal hide failed"
        );
    }
    result
}

fn schedule_archive_patcher_candidate_request(
    window: &ArchivePatcherWindow,
    controller: &Arc<Mutex<ArchivePatcherController>>,
    worker_runtime: WorkerRuntime,
    archive_patcher_sink: SlintEventLoopSink,
    request: ArchivePatcherCandidateWorkerRequest,
) {
    tracing::info!(
        event = "s10-archive-patcher-candidates-schedule",
        request_id = request.request_id,
        task_id = %request.task.id,
        archive_count = request.archives.len(),
        "Scheduling Archive Patcher candidate worker"
    );
    let task = request.task.clone();
    let request_id = request.request_id;
    if let Err(error) = worker_runtime.spawn_blocking_task(
        task,
        archive_patcher_sink,
        move |_context| build_archive_patcher_candidates_payload(request),
    ) {
        with_archive_patcher_controller_mut(controller, |controller| {
            controller.spawn_failed(ArchivePatcherWorkerRequestKind::Candidates, request_id, error)
        });
        apply_current_archive_patcher_state(window, controller);
    }
}

fn schedule_archive_patcher_plan_request(
    window: &ArchivePatcherWindow,
    controller: &Arc<Mutex<ArchivePatcherController>>,
    worker_runtime: WorkerRuntime,
    archive_patcher_sink: SlintEventLoopSink,
    request: ArchivePatcherPlanWorkerRequest,
) {
    tracing::info!(
        event = "s10-archive-patcher-plan-schedule",
        request_id = request.request_id,
        task_id = %request.task.id,
        archive_count = request.archives.len(),
        "Scheduling Archive Patcher plan worker"
    );
    let task = request.task.clone();
    let request_id = request.request_id;
    if let Err(error) = worker_runtime.spawn_blocking_task(
        task,
        archive_patcher_sink,
        move |_context| build_archive_patcher_plan_payload(request),
    ) {
        with_archive_patcher_controller_mut(controller, |controller| {
            controller.spawn_failed(ArchivePatcherWorkerRequestKind::Plan, request_id, error)
        });
        apply_current_archive_patcher_state(window, controller);
    }
}

fn schedule_archive_patcher_patch_request(
    window: &ArchivePatcherWindow,
    controller: &Arc<Mutex<ArchivePatcherController>>,
    worker_runtime: WorkerRuntime,
    archive_patcher_sink: SlintEventLoopSink,
    request: ArchivePatcherPatchWorkerRequest,
) {
    tracing::info!(
        event = "s10-archive-patcher-patch-schedule",
        request_id = request.request_id,
        task_id = %request.task.id,
        archive_count = request.archives.len(),
        manifest_path = %request.manifest_path.display(),
        "Scheduling Archive Patcher confirmed patch worker"
    );
    let task = request.task.clone();
    let request_id = request.request_id;
    if let Err(error) = worker_runtime.spawn_blocking_task(
        task,
        archive_patcher_sink,
        move |context| build_archive_patcher_patch_payload(context, request),
    ) {
        with_archive_patcher_controller_mut(controller, |controller| {
            controller.spawn_failed(ArchivePatcherWorkerRequestKind::Patch, request_id, error)
        });
        apply_current_archive_patcher_state(window, controller);
    }
}

fn schedule_archive_patcher_restore_request(
    window: &ArchivePatcherWindow,
    controller: &Arc<Mutex<ArchivePatcherController>>,
    worker_runtime: WorkerRuntime,
    archive_patcher_sink: SlintEventLoopSink,
    request: ArchivePatcherRestoreWorkerRequest,
) {
    tracing::info!(
        event = "s10-archive-patcher-restore-schedule",
        request_id = request.request_id,
        task_id = %request.task.id,
        manifest_path = %request.manifest_path.display(),
        "Scheduling Archive Patcher restore worker"
    );
    let task = request.task.clone();
    let request_id = request.request_id;
    if let Err(error) = worker_runtime.spawn_blocking_task(
        task,
        archive_patcher_sink,
        move |context| build_archive_patcher_restore_payload(context, request),
    ) {
        with_archive_patcher_controller_mut(controller, |controller| {
            controller.spawn_failed(ArchivePatcherWorkerRequestKind::Restore, request_id, error)
        });
        apply_current_archive_patcher_state(window, controller);
    }
}

fn build_archive_patcher_candidates_payload(
    request: ArchivePatcherCandidateWorkerRequest,
) -> BlockingWorkerResult {
    let span = tracing::info_span!(
        "s10_archive_patcher_candidates_worker",
        request_id = request.request_id,
        archive_count = request.archives.len(),
    );
    let _guard = span.enter();
    let filesystem = RealFilesystem::new();
    let service = ArchivePatcherService::new(&filesystem);
    let snapshot = service.candidate_snapshot(ArchivePatcherCandidateRequest::new(
        request.request_id,
        &request.archives,
        request.target,
        request.name_filter.as_deref(),
    ));
    Ok(WorkerTaskOutcome::Completed(
        archive_patcher_candidates_loaded_payload(request.request_id, snapshot),
    ))
}

fn build_archive_patcher_plan_payload(
    request: ArchivePatcherPlanWorkerRequest,
) -> BlockingWorkerResult {
    let span = tracing::info_span!(
        "s10_archive_patcher_plan_worker",
        request_id = request.request_id,
        archive_count = request.archives.len(),
        has_data_root = request.data_root.is_some(),
    );
    let _guard = span.enter();
    let filesystem = RealFilesystem::new();
    let service = ArchivePatcherService::new(&filesystem);
    let plan = service.preview_plan(ArchivePatcherPlanRequest::new(
        request.request_id,
        request.data_root.as_deref(),
        &request.archives,
        request.target,
        request.name_filter.as_deref(),
    ));
    Ok(WorkerTaskOutcome::Completed(
        archive_patcher_plan_ready_payload(request.request_id, plan),
    ))
}

fn build_archive_patcher_patch_payload<S>(
    context: workers::WorkerTaskContext<S>,
    request: ArchivePatcherPatchWorkerRequest,
) -> BlockingWorkerResult
where
    S: WorkerEventSink,
{
    emit_archive_patcher_progress(
        &context,
        request.request_id,
        ArchivePatcherWorkerStage::Patch,
        ArchivePatcherProgress::new("Revalidating Archive Patcher plan...", 5.0),
    );
    let filesystem = RealFilesystem::new();
    let service = ArchivePatcherService::new(&filesystem);
    let result = service
        .execute_confirmed(
            ArchivePatcherExecutionRequest::new(
                request.request_id,
                request.data_root.as_deref(),
                &request.archives,
                request.target,
                request.name_filter.as_deref(),
                &request.manifest_path,
            )
            .with_confirmed_plan_digest(&request.confirmed_plan_digest),
        )
        .map_err(archive_patcher_service_failure)?;
    emit_archive_patcher_execution_log_rows(
        &context,
        request.request_id,
        ArchivePatcherWorkerStage::Patch,
        &result,
    );
    emit_archive_patcher_progress(
        &context,
        request.request_id,
        ArchivePatcherWorkerStage::Patch,
        ArchivePatcherProgress::complete(
            result
                .log_rows
                .last()
                .map(|row| row.message.clone())
                .unwrap_or_else(|| result.counts.patching_complete_message()),
        ),
    );
    Ok(WorkerTaskOutcome::Completed(
        archive_patcher_patch_completed_payload(request.request_id, result),
    ))
}

fn build_archive_patcher_restore_payload<S>(
    context: workers::WorkerTaskContext<S>,
    request: ArchivePatcherRestoreWorkerRequest,
) -> BlockingWorkerResult
where
    S: WorkerEventSink,
{
    emit_archive_patcher_progress(
        &context,
        request.request_id,
        ArchivePatcherWorkerStage::Restore,
        ArchivePatcherProgress::new("Reading Archive Patcher restore manifest...", 5.0),
    );
    let filesystem = RealFilesystem::new();
    let service = ArchivePatcherService::new(&filesystem);
    let result = service
        .restore_last_run(ArchivePatcherRestoreRequest::new(
            request.request_id,
            request.data_root.as_deref(),
            &request.manifest_path,
        ))
        .map_err(archive_patcher_service_failure)?;
    emit_archive_patcher_execution_log_rows(
        &context,
        request.request_id,
        ArchivePatcherWorkerStage::Restore,
        &result,
    );
    emit_archive_patcher_progress(
        &context,
        request.request_id,
        ArchivePatcherWorkerStage::Restore,
        ArchivePatcherProgress::complete(
            result
                .log_rows
                .last()
                .map(|row| row.message.clone())
                .unwrap_or_else(|| result.counts.restore_complete_message()),
        ),
    );
    Ok(WorkerTaskOutcome::Completed(
        archive_patcher_restore_completed_payload(request.request_id, result),
    ))
}

fn emit_archive_patcher_execution_log_rows<S>(
    context: &workers::WorkerTaskContext<S>,
    request_id: u64,
    stage: ArchivePatcherWorkerStage,
    result: &ArchivePatcherExecutionResult,
) where
    S: WorkerEventSink,
{
    for row in &result.log_rows {
        if let Err(error) = context.emit_payload(
            WorkerTaskStatus::Progress,
            archive_patcher_log_row_payload(request_id, stage, row.clone()),
        ) {
            tracing::warn!(
                event = "s10-archive-patcher-log-handoff-failed",
                request_id,
                stage = stage.label(),
                diagnostic = %error,
                "Archive Patcher log row could not be handed to UI"
            );
        }
    }
}

fn emit_archive_patcher_progress<S>(
    context: &workers::WorkerTaskContext<S>,
    request_id: u64,
    stage: ArchivePatcherWorkerStage,
    progress: ArchivePatcherProgress,
) where
    S: WorkerEventSink,
{
    if let Err(error) = context.emit_payload(
        WorkerTaskStatus::Progress,
        archive_patcher_progress_payload(request_id, stage, progress),
    ) {
        tracing::warn!(
            event = "s10-archive-patcher-progress-handoff-failed",
            request_id,
            stage = stage.label(),
            diagnostic = %error,
            "Archive Patcher progress could not be handed to UI"
        );
    }
}

fn archive_patcher_service_failure(error: ArchivePatcherExecutionError) -> WorkerFailure {
    let failure = WorkerFailure::new(error.user_message().to_owned());
    if let Some(diagnostic) = error.diagnostic() {
        failure.with_diagnostic(diagnostic.to_owned())
    } else {
        failure
    }
}

fn archive_patcher_completion_refresh_needed(event: &WorkerEvent) -> bool {
    matches!(
        (&event.status, &event.payload),
        (
            WorkerTaskStatus::Completed,
            WorkerPayload::ArchivePatcher(
                ArchivePatcherWorkerPayload::PatchCompleted { .. }
                    | ArchivePatcherWorkerPayload::RestoreCompleted { .. },
            ),
        )
    )
}

fn overview_settings_for_archive_patcher_completion(
    shared_settings_snapshot: &Arc<Mutex<AppSettings>>,
) -> AppSettings {
    match shared_settings_snapshot.lock() {
        Ok(settings) => settings.clone(),
        Err(error) => {
            tracing::error!(
                event = "s10-archive-patcher-overview-settings-unavailable",
                diagnostic = %error,
                "Current settings snapshot unavailable; using safe defaults for Overview refresh"
            );
            AppSettings::default()
        }
    }
}

fn archive_patcher_manifest_path() -> PathBuf {
    if let Some(project_dirs) = directories::ProjectDirs::from(
        "community",
        "Collective Modding",
        "Collective Modding Toolkit",
    ) {
        let config_dir = project_dirs.config_dir();
        if let Err(error) = std::fs::create_dir_all(config_dir) {
            tracing::warn!(
                event = "s10-archive-patcher-manifest-dir-create-failed",
                manifest_dir = %config_dir.display(),
                diagnostic = %error,
                "Archive Patcher manifest directory could not be created; falling back to current directory"
            );
        } else {
            return config_dir.join("archive-patcher-latest.json");
        }
    }
    PathBuf::from("archive-patcher-latest.json")
}

fn archive_patcher_manifest_available(path: &Path) -> bool {
    match RealFilesystem::new().exists(path) {
        Ok(available) => available,
        Err(error) => {
            tracing::warn!(
                event = "s10-archive-patcher-manifest-check-failed",
                manifest_path = %path.display(),
                diagnostic = %error,
                "Archive Patcher latest manifest availability check failed safely"
            );
            false
        }
    }
}

fn handle_archive_patcher_worker_event(
    controller: &mut ArchivePatcherController,
    event: WorkerEvent,
) -> ArchivePatcherTransitionResult {
    controller.handle_worker_event(event)
}

fn handle_downgrader_worker_event(
    controller: &mut DowngraderController,
    event: WorkerEvent,
) -> DowngraderTransitionResult {
    controller.handle_worker_event(event)
}

fn request_tools_action(
    app: &MainWindow,
    controller: &Arc<Mutex<ToolsController>>,
    worker_runtime: WorkerRuntime,
    tools_sink: SlintEventLoopSink,
    action_id: String,
) {
    let action = match tools_action_for_id(&action_id) {
        Ok(action @ ToolsActionKind::ExternalLink(_)) => action,
        Ok(action @ ToolsActionKind::InternalUtility(tool_id)) => {
            let safe_message = match tool_id {
                ToolActionId::DowngradeManager => "Open the Downgrade Manager workflow.",
                ToolActionId::ArchivePatcher => "Open the Archive Patcher workflow.",
                _ => "Open the selected workflow.",
            };
            let feedback = ToolsActionFeedback::succeeded(action_id.as_str(), action, safe_message);
            apply_tools_feedback(app, controller, feedback);
            return;
        }
        Ok(action @ ToolsActionKind::DeferredUtility(_)) => {
            let feedback = ToolsActionFeedback::rejected(
                action_id.as_str(),
                Some(action),
                ActionRejectionKind::DisabledUtility,
                "Tools action is not available.",
                Some("deferred Tools action reached runtime scheduler".to_owned()),
            );
            apply_tools_feedback(app, controller, feedback);
            return;
        }
        Err(feedback) => {
            tracing::warn!(
                event = "s05-tools-action-preflight-rejected",
                action_id = feedback.action_id.as_str(),
                outcome = ?feedback.outcome,
                diagnostic = feedback.diagnostic.as_deref().unwrap_or(""),
                "Tools action failed closed before worker scheduling"
            );
            apply_tools_feedback(app, controller, feedback);
            return;
        }
    };

    tracing::info!(
        event = "s05-tools-action-schedule",
        action_id = action_id.as_str(),
        action = ?action,
        "Scheduling Tools action worker"
    );

    let task = tools_action_worker_task(&action_id);
    let worker_action_id = action_id.clone();
    if let Err(error) = worker_runtime.spawn_blocking_task(task, tools_sink, move |_context| {
        build_tools_action_payload(worker_action_id)
    }) {
        tracing::error!(
            event = "s05-tools-action-spawn-failed",
            action_id = action_id.as_str(),
            error = %error,
            "Tools action worker could not be scheduled"
        );
        apply_tools_feedback(
            app,
            controller,
            tools_spawn_failed_feedback(&action_id, Some(action), error),
        );
    }
}

fn request_about_action(
    app: &MainWindow,
    controller: &Arc<Mutex<AboutController>>,
    worker_runtime: WorkerRuntime,
    about_sink: SlintEventLoopSink,
    action_id: String,
    callback_kind: AboutCallbackKind,
) {
    let action = match about_action_for_id(&action_id) {
        Ok(action) if about_action_matches_callback(action, callback_kind) => action,
        Ok(action) => {
            let feedback = about_callback_mismatch_feedback(&action_id, action, callback_kind);
            tracing::warn!(
                event = "s05-about-action-callback-mismatch",
                action_id = action_id.as_str(),
                callback_kind = callback_kind.label(),
                action_kind = if action.is_copy() { "copy" } else { "open" },
                "About action failed closed because it arrived through the wrong callback"
            );
            apply_about_feedback(app, controller, feedback);
            return;
        }
        Err(feedback) => {
            tracing::warn!(
                event = "s05-about-action-preflight-rejected",
                action_id = feedback.action_id.as_str(),
                callback_kind = callback_kind.label(),
                outcome = ?feedback.outcome,
                diagnostic = feedback.diagnostic.as_deref().unwrap_or(""),
                "About action failed closed before worker scheduling"
            );
            apply_about_feedback(app, controller, feedback);
            return;
        }
    };

    tracing::info!(
        event = "s05-about-action-schedule",
        action_id = action_id.as_str(),
        callback_kind = callback_kind.label(),
        link_id = action.link_id().as_str(),
        "Scheduling About action worker"
    );

    let task = about_action_worker_task(&action_id);
    let worker_action_id = action_id.clone();
    if let Err(error) = worker_runtime.spawn_blocking_task(task, about_sink, move |_context| {
        build_about_action_payload(worker_action_id)
    }) {
        tracing::error!(
            event = "s05-about-action-spawn-failed",
            action_id = action_id.as_str(),
            callback_kind = callback_kind.label(),
            error = %error,
            "About action worker could not be scheduled"
        );
        apply_about_feedback(
            app,
            controller,
            about_spawn_failed_feedback(&action_id, Some(action), error),
        );
    }
}

fn build_tools_action_payload(action_id: String) -> BlockingWorkerResult {
    let service = ToolsActionService::new(RealDesktopActions::new(), RealClipboardActions::new());
    let feedback = service.execute_tools_action(&action_id);
    Ok(WorkerTaskOutcome::Completed(WorkerPayload::ToolsAction(
        ToolsActionWorkerPayload::action_completed(feedback),
    )))
}

fn build_about_action_payload(action_id: String) -> BlockingWorkerResult {
    let service = ToolsActionService::new(RealDesktopActions::new(), RealClipboardActions::new());
    let feedback = service.execute_about_action(&action_id);
    Ok(WorkerTaskOutcome::Completed(WorkerPayload::AboutAction(
        AboutActionWorkerPayload::action_completed(feedback),
    )))
}

fn tools_action_worker_task(action_id: &str) -> WorkerTask {
    WorkerTask::new(
        format!("{TOOLS_WORKER_TASK_PREFIX}{action_id}"),
        WorkerTaskKind::DesktopAction,
    )
    .with_label(format!("Tools action {action_id}"))
}

fn about_action_worker_task(action_id: &str) -> WorkerTask {
    WorkerTask::new(
        format!("{ABOUT_WORKER_TASK_PREFIX}{action_id}"),
        WorkerTaskKind::DesktopAction,
    )
    .with_label(format!("About action {action_id}"))
}

fn tools_spawn_failed_feedback(
    action_id: &str,
    action: Option<ToolsActionKind>,
    error: WorkerSpawnError,
) -> ToolsActionFeedback {
    ToolsActionFeedback::rejected(
        action_id,
        action,
        ActionRejectionKind::WorkerUnavailable,
        TOOLS_ACTION_START_ERROR,
        Some(error.to_string()),
    )
}

fn about_spawn_failed_feedback(
    action_id: &str,
    action: Option<AboutActionKind>,
    error: WorkerSpawnError,
) -> AboutActionFeedback {
    AboutActionFeedback::rejected(
        action_id,
        action,
        ActionRejectionKind::WorkerUnavailable,
        ABOUT_ACTION_START_ERROR,
        Some(error.to_string()),
    )
}

fn about_action_matches_callback(
    action: AboutActionKind,
    callback_kind: AboutCallbackKind,
) -> bool {
    matches!(
        (action, callback_kind),
        (AboutActionKind::Open { .. }, AboutCallbackKind::Open)
            | (AboutActionKind::Copy { .. }, AboutCallbackKind::Copy)
    )
}

fn about_callback_mismatch_feedback(
    action_id: &str,
    action: AboutActionKind,
    callback_kind: AboutCallbackKind,
) -> AboutActionFeedback {
    AboutActionFeedback::rejected(
        action_id,
        Some(action),
        ActionRejectionKind::InvalidInput,
        "About action is not available.",
        Some(format!(
            "About {} callback received a {} action id",
            callback_kind.label(),
            if action.is_copy() { "copy" } else { "open" }
        )),
    )
}

fn request_scanner_scan(
    app: &MainWindow,
    controller: &Arc<Mutex<ScannerController>>,
    settings_controller: &Rc<RefCell<SettingsController<FileAssetResolver>>>,
    worker_runtime: WorkerRuntime,
    scanner_sink: SlintEventLoopSink,
) {
    let Some(prepared) = prepare_scanner_scan_request(controller, settings_controller) else {
        apply_current_scanner_state(app, controller);
        return;
    };

    apply_current_scanner_state(app, controller);

    let scan_id = prepared.request.scan_id;
    let task = prepared.request.task.clone();
    let worker_task = task.clone();
    let settings = prepared.settings;
    tracing::info!(
        event = "s07-scanner-scan-schedule",
        scan_id,
        task_id = %task.id,
        "Scheduling Scanner scan worker"
    );

    if let Err(error) = worker_runtime.spawn_blocking_task(task, scanner_sink, move |context| {
        build_scanner_scan_payload(context, scan_id, settings)
    }) {
        tracing::error!(
            event = "s07-scanner-scan-spawn-failed",
            scan_id,
            task_id = %worker_task.id,
            error = %error,
            "Scanner scan worker could not be scheduled"
        );
        with_scanner_controller_mut(controller, |controller| {
            controller.spawn_failed(scan_id, error);
        });
        apply_current_scanner_state(app, controller);
        return;
    }

    with_scanner_controller_mut(controller, |controller| {
        controller.scan_started(scan_id);
    });
    apply_current_scanner_state(app, controller);
}

fn prepare_scanner_scan_request<R: AssetResolver>(
    controller: &Arc<Mutex<ScannerController>>,
    settings_controller: &Rc<RefCell<SettingsController<R>>>,
) -> Option<PreparedScannerScan> {
    let scanner_settings =
        with_scanner_controller_mut(controller, |controller| controller.settings().clone())?;

    if !any_scanner_category_enabled(&scanner_settings) {
        tracing::warn!(
            event = "s07-scanner-scan-not-scheduled",
            reason = "no-enabled-categories",
            "Scanner scan not scheduled because all categories are disabled"
        );
        with_scanner_controller_mut(controller, |controller| {
            controller.request_scan();
        });
        return None;
    }

    let save_result = settings_controller
        .borrow_mut()
        .save_scanner_settings_for_scan(scanner_settings);
    with_scanner_controller_mut(controller, |controller| {
        controller.replace_settings(save_result.visible_settings.clone());
    });

    if !save_result.should_schedule_scan() {
        tracing::warn!(
            event = "s07-scanner-scan-not-scheduled",
            reason = "settings-save-failed",
            "Scanner scan not scheduled because settings persistence failed"
        );
        return None;
    }

    let request =
        with_scanner_controller_mut(controller, ScannerController::request_scan).flatten()?;
    let settings = settings_controller.borrow().current_settings().clone();
    Some(PreparedScannerScan { request, settings })
}

fn build_scanner_scan_payload<S>(
    context: workers::WorkerTaskContext<S>,
    scan_id: u64,
    settings: AppSettings,
) -> BlockingWorkerResult
where
    S: WorkerEventSink,
{
    let span = tracing::info_span!("s07_scanner_scan_worker", scan_id);
    let _guard = span.enter();
    tracing::info!(
        event = "s07-scanner-worker-started",
        scan_id,
        task_id = %context.task().id,
        "Scanner scan worker started"
    );
    emit_scanner_progress_message(
        &context,
        scan_id,
        PROGRESS_REFRESHING_OVERVIEW_TEXT,
        Some(1),
        Some(100),
    );

    let filesystem = RealFilesystem::new();
    let registry = RealRegistry::new();
    let process = RealProcessInspector::new();
    let current_working_directory = current_working_directory();
    let local_appdata = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);

    let mut discovery_request = DiscoveryRequest::new(current_working_directory);
    discovery_request = discovery_request.with_current_process_id(std::process::id());
    if let Some(path) = local_appdata.clone() {
        discovery_request = discovery_request.with_local_appdata(path);
    }

    let discovery =
        DiscoveryService::new(&filesystem, &registry, &process).discover(&discovery_request);
    if let Err(error) = &discovery.game {
        tracing::warn!(
            event = "s07-scanner-discovery-failure",
            scan_id,
            safe_message = %error.user_message(),
            "Scanner discovery did not find a usable Fallout 4 installation"
        );
    }
    if let Err(error) = &discovery.mod_manager {
        tracing::warn!(
            event = "s07-scanner-manager-discovery-failure",
            scan_id,
            safe_message = %error.user_message(),
            "Scanner mod-manager discovery failed safely"
        );
    }

    let mut collection_environment = OverviewCollectionEnvironment::new();
    if let Some(path) = local_appdata {
        collection_environment = collection_environment.with_local_appdata(path);
    }

    let collected = match &discovery.game {
        Ok(installation) => {
            let collector = OverviewCollector::new(&filesystem, &process);
            collector.collect(OverviewCollectionRequest::new(
                installation,
                &collection_environment,
            ))
        }
        Err(_) => OverviewCollectedFacts::default(),
    };

    tracing::info!(
        event = "s07-scanner-overview-facts-collected",
        scan_id,
        binaries = collected.diagnostics.binary_count,
        archives = collected.diagnostics.archive_count,
        modules = collected.diagnostics.module_count,
        enabled_archives = collected.diagnostics.enabled_archive_count,
        enabled_modules = collected.diagnostics.enabled_module_count,
        missing_files = collected.diagnostics.missing_file_count,
        unreadable_files = collected.diagnostics.unreadable_file_count,
        "Scanner worker rebuilt Overview facts before scan"
    );

    let update_state = OverviewUpdateCheckState::NotChecked;
    let overview_snapshot = OverviewDiagnostics::build(OverviewDiagnosticsInput {
        discovery: &discovery,
        settings: &settings,
        binaries: &collected.binaries,
        archives: &collected.archives,
        modules: &collected.modules,
        enablement: &collected.enablement,
        update: &update_state,
        last_desktop_action: None,
    });

    tracing::info!(
        event = "s07-scanner-overview-refresh-phase",
        scan_id,
        overview_problem_count = overview_snapshot.problems.len(),
        "Scanner worker built Overview problem feed"
    );

    let manager_context =
        scanner_manager_context(discovery.mod_manager.as_ref().ok().and_then(Option::as_ref));
    let mut scan_request = ScannerScanRequest::new(scan_id, &settings.scanner)
        .with_overview_problems(&overview_snapshot.problems)
        .with_enabled_modules(&collected.modules)
        .with_enabled_archives(&collected.archives);
    if let Ok(installation) = &discovery.game {
        scan_request = scan_request.with_installation(installation);
    }
    if let Some(manager_context) = manager_context.as_ref() {
        scan_request = scan_request.with_mod_manager(manager_context);
    }

    let service = ScannerScanService::new(&filesystem);
    let output = service.scan_with_progress(scan_request, |progress| {
        trace_scanner_progress_event(progress);
        emit_scanner_progress_event(&context, progress);
    });
    trace_scanner_scan_output(&output);

    let snapshot = ScannerScanSnapshot::from_grouped(
        output.scan_id,
        output.results,
        output.groups,
        output.status.safe_message,
    );
    Ok(WorkerTaskOutcome::Completed(
        scanner_scan_completed_payload(scan_id, snapshot),
    ))
}

fn scanner_manager_context(manager: Option<&DiscoveredModManager>) -> Option<ModManagerContext> {
    match manager {
        Some(DiscoveredModManager::ModOrganizer(configuration)) => Some(
            ModManagerContext::ModOrganizer(Box::new(configuration.context.clone())),
        ),
        Some(DiscoveredModManager::Vortex(context)) => {
            Some(ModManagerContext::Vortex(context.clone()))
        }
        None => None,
    }
}

fn emit_scanner_progress_event<S>(
    context: &workers::WorkerTaskContext<S>,
    progress: &ScannerProgressEvent,
) where
    S: WorkerEventSink,
{
    emit_scanner_progress_message(
        context,
        progress.scan_id,
        &progress.safe_message,
        scanner_progress_current(progress),
        scanner_progress_total(progress),
    );
}

fn emit_scanner_progress_message<S>(
    context: &workers::WorkerTaskContext<S>,
    scan_id: u64,
    message: &str,
    current: Option<u64>,
    total: Option<u64>,
) where
    S: WorkerEventSink,
{
    let progress = workers::WorkerProgress::new()
        .with_message(message.to_owned())
        .with_counts(current, total);
    if let Err(error) = context.emit_progress(progress) {
        tracing::warn!(
            event = "s07-scanner-progress-handoff-failed",
            scan_id,
            task_id = %context.task().id,
            error = %error,
            diagnostic = error.diagnostic.as_deref().unwrap_or(""),
            "Scanner progress could not be handed to the UI"
        );
    }
}

fn scanner_progress_current(progress: &ScannerProgressEvent) -> Option<u64> {
    progress
        .folder_index
        .map(|index| index as u64)
        .or_else(|| scanner_percent_as_count(progress.percent))
}

fn scanner_progress_total(progress: &ScannerProgressEvent) -> Option<u64> {
    progress
        .folder_total
        .map(|total| total as u64)
        .or_else(|| scanner_percent_as_count(progress.percent).map(|_| 100))
}

fn scanner_percent_as_count(percent: f32) -> Option<u64> {
    if percent.is_finite() {
        Some(percent.clamp(0.0, 100.0).round() as u64)
    } else {
        None
    }
}

fn trace_scanner_progress_event(progress: &ScannerProgressEvent) {
    tracing::debug!(
        event = "s07-scanner-progress-emitted",
        scan_id = progress.scan_id,
        phase = progress.phase.as_str(),
        percent = progress.percent,
        folder = progress.folder.as_deref().unwrap_or(""),
        folder_index = ?progress.folder_index,
        folder_total = ?progress.folder_total,
        safe_message = progress.safe_message.as_str(),
        "Scanner progress emitted from worker"
    );
}

fn trace_scanner_scan_output(output: &ScannerScanOutput) {
    tracing::info!(
        event = "s07-scanner-scan-output",
        scan_id = output.scan_id,
        status = ?output.status.kind,
        result_count = output.results.len(),
        group_count = output.groups.len(),
        overview_problem_count = output.diagnostics.overview_problem_count,
        indexed_mod_count = output.diagnostics.indexed_mod_count,
        indexed_file_count = output.diagnostics.indexed_file_count,
        traversed_folders = output.diagnostics.traversed_folder_count,
        traversed_files = output.diagnostics.traversed_file_count,
        partial_read_failures = output.diagnostics.partial_read_failure_count,
        race_subgraph_records = output.diagnostics.race_subgraph_record_count,
        race_subgraph_modules = output.diagnostics.race_subgraph_module_count,
        "Scanner scan completed with structured counts"
    );

    for diagnostic in &output.diagnostics.errors {
        tracing::warn!(
            event = "s07-scanner-safe-diagnostic",
            scan_id = output.scan_id,
            phase = diagnostic.phase.as_str(),
            kind = ?diagnostic.kind,
            path = %diagnostic
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            safe_message = diagnostic.safe_message.as_str(),
            "Scanner scan captured a recoverable diagnostic"
        );
    }
}

fn toggle_scanner_file_list(app: &MainWindow, controller: &Arc<Mutex<ScannerController>>) {
    let Some(result) = with_scanner_controller_mut(controller, ScannerController::toggle_file_list)
    else {
        return;
    };

    if result.is_applied() || matches!(result, ScannerTransitionResult::Rejected) {
        apply_current_scanner_state(app, controller);
    }
}

fn request_scanner_auto_fix(
    app: &MainWindow,
    controller: &Arc<Mutex<ScannerController>>,
    worker_runtime: WorkerRuntime,
    scanner_sink: SlintEventLoopSink,
) {
    let Some(prepared) = prepare_scanner_auto_fix_request(controller) else {
        apply_current_scanner_state(app, controller);
        return;
    };

    apply_current_scanner_state(app, controller);

    let request = prepared.request;
    let snapshot = prepared.snapshot;
    let task = request.task.clone();
    let worker_task = task.clone();
    let request_for_worker = request.clone();
    let request_for_failure = request.clone();
    tracing::info!(
        event = "s08-scanner-autofix-schedule",
        scan_id = request.scan_id,
        result_index = request.result_index,
        operation_key = %request.operation_key.as_id(),
        task_id = %task.id,
        "Scheduling Scanner Auto-Fix worker"
    );

    if let Err(error) = worker_runtime.spawn_blocking_task(task, scanner_sink, move |_context| {
        build_scanner_auto_fix_payload(request_for_worker, snapshot)
    }) {
        tracing::error!(
            event = "s08-scanner-autofix-spawn-failed",
            scan_id = request_for_failure.scan_id,
            result_index = request_for_failure.result_index,
            operation_key = %request_for_failure.operation_key.as_id(),
            task_id = %worker_task.id,
            error = %error,
            "Scanner Auto-Fix worker could not be scheduled"
        );
        with_scanner_controller_mut(controller, |controller| {
            controller.auto_fix_spawn_failed(&request_for_failure, error);
        });
        apply_current_scanner_state(app, controller);
    }
}

fn prepare_scanner_auto_fix_request(
    controller: &Arc<Mutex<ScannerController>>,
) -> Option<PreparedScannerAutoFix> {
    with_scanner_controller_mut(controller, |controller| {
        let request = controller.request_selected_auto_fix()?;
        let snapshot = ScannerScanSnapshot::from_grouped(
            request.scan_id,
            controller.results().to_vec(),
            controller.groups().to_vec(),
            controller.status_text().to_owned(),
        );
        Some(PreparedScannerAutoFix { request, snapshot })
    })
    .flatten()
}

fn build_scanner_auto_fix_payload(
    request: ScannerAutoFixWorkerRequest,
    snapshot: ScannerScanSnapshot,
) -> BlockingWorkerResult {
    let filesystem = RealFilesystem::new();
    let service = AutoFixService::new(&filesystem);
    execute_scanner_auto_fix_with_service(request, snapshot, &service)
}

fn execute_scanner_auto_fix_with_service(
    request: ScannerAutoFixWorkerRequest,
    snapshot: ScannerScanSnapshot,
    service: &AutoFixService<'_>,
) -> BlockingWorkerResult {
    tracing::info!(
        event = "s08-scanner-autofix-worker-started",
        scan_id = request.scan_id,
        result_index = request.result_index,
        operation_key = %request.operation_key.as_id(),
        task_id = %request.task.id,
        "Scanner Auto-Fix worker started"
    );

    let target_path = snapshot
        .results
        .get(request.result_index)
        .and_then(|result| result.absolute_path.clone());
    let auto_fix_request = AutoFixRequest {
        scan_id: Some(request.scan_id),
        operation_key: request.operation_key,
        selection_identity: request.selection_identity.clone(),
        target_path,
        confirmation: None,
        revalidation: AutoFixRevalidationPlan::required(request.selection_identity.clone()),
    };

    let completion = match service.execute(&snapshot, request.result_index, auto_fix_request) {
        AutoFixServiceResult::Completed(completion) => completion,
        AutoFixServiceResult::Rejected(rejection) => {
            tracing::warn!(
                event = "s08-scanner-autofix-rejected",
                scan_id = ?rejection.scan_id,
                result_index = ?rejection.result_index,
                operation_key = %rejection
                    .operation_key
                    .map(|key| key.as_id())
                    .unwrap_or("unknown"),
                safe_message = %rejection.safe_message,
                diagnostic = rejection.diagnostic.as_deref().unwrap_or(""),
                "Scanner Auto-Fix service rejected the request"
            );
            auto_fix_completion_from_rejection(rejection, &request)
        }
    };

    if completion.status.kind == AutoFixStatusKind::Fixed {
        tracing::info!(
            event = "s08-scanner-autofix-completed",
            scan_id = ?completion.scan_id,
            result_index = ?completion.result_index,
            operation_key = %completion.operation_key.as_id(),
            safe_message = %completion.status.safe_message,
            "Scanner Auto-Fix worker completed"
        );
    } else {
        tracing::warn!(
            event = "s08-scanner-autofix-failed",
            scan_id = ?completion.scan_id,
            result_index = ?completion.result_index,
            operation_key = %completion.operation_key.as_id(),
            safe_message = %completion.status.safe_message,
            diagnostic = completion.status.diagnostic.as_deref().unwrap_or(""),
            "Scanner Auto-Fix worker completed with failure"
        );
    }

    Ok(WorkerTaskOutcome::Completed(
        scanner_auto_fix_completed_payload(completion),
    ))
}

fn auto_fix_completion_from_rejection(
    rejection: AutoFixRejection,
    request: &ScannerAutoFixWorkerRequest,
) -> AutoFixCompletion {
    let selection_identity = rejection
        .selection_identity
        .clone()
        .unwrap_or_else(|| request.selection_identity.clone());
    let mut revalidation = AutoFixRevalidationPlan::required(selection_identity.clone());
    if let Some(observed_identity) = rejection.observed_identity.clone() {
        revalidation = revalidation.with_observed_identity(observed_identity);
    }

    let status = AutoFixStatus::new(AutoFixStatusKind::Failed, rejection.safe_message.clone());
    let status = match rejection.diagnostic.clone() {
        Some(diagnostic) => status.with_diagnostic(diagnostic),
        None => status,
    };
    let detail = AutoFixResultDetail::new(
        rejection.safe_message.clone(),
        rejection.safe_message.clone(),
    );
    let detail = match rejection.diagnostic {
        Some(diagnostic) => detail.with_diagnostic(diagnostic),
        None => detail,
    };

    AutoFixCompletion {
        scan_id: rejection.scan_id.or(Some(request.scan_id)),
        result_index: rejection.result_index.or(Some(request.result_index)),
        operation_key: rejection.operation_key.unwrap_or(request.operation_key),
        selection_identity,
        revalidation,
        status,
        detail,
    }
}

fn request_scanner_action(
    app: &MainWindow,
    controller: &Arc<Mutex<ScannerController>>,
    worker_runtime: WorkerRuntime,
    scanner_sink: SlintEventLoopSink,
    action: ScannerActionKind,
) {
    let Some(execution) = prepare_scanner_action_execution(controller, action) else {
        apply_current_scanner_state(app, controller);
        return;
    };

    let task = scanner_action_task(execution.descriptor.kind, execution.scan_id);
    let worker_task = task.clone();
    let scan_id = execution.scan_id;
    tracing::info!(
        event = "s07-scanner-action-schedule",
        action = execution.descriptor.kind.as_id(),
        scan_id = ?scan_id,
        task_id = %task.id,
        "Scheduling Scanner read-only action worker"
    );

    if let Err(error) = worker_runtime.spawn_blocking_task(task, scanner_sink, move |_context| {
        execute_scanner_action_payload(execution)
    }) {
        tracing::error!(
            event = "s07-scanner-action-spawn-failed",
            action = action.as_id(),
            scan_id = ?scan_id,
            task_id = %worker_task.id,
            error = %error,
            "Scanner read-only action worker could not be scheduled"
        );
        let feedback = ScannerActionFeedback::failed(scan_id, action, SCANNER_ACTION_START_ERROR)
            .with_diagnostic(error.to_string());
        with_scanner_controller_mut(controller, |controller| {
            controller.action_completed(feedback);
        });
        apply_current_scanner_state(app, controller);
    }
}

fn prepare_scanner_action_execution(
    controller: &Arc<Mutex<ScannerController>>,
    action: ScannerActionKind,
) -> Option<ScannerActionExecution> {
    with_scanner_controller_mut(controller, |controller| {
        let scan_id = controller.latest_scan_id();
        let details_text = controller
            .selected_detail()
            .map(|detail| detail.copy_details_text.clone());
        controller
            .request_selected_action(action.as_id())
            .map(|descriptor| ScannerActionExecution {
                scan_id,
                descriptor,
                details_text,
            })
    })
    .flatten()
}

fn scanner_action_task(action: ScannerActionKind, scan_id: Option<u64>) -> WorkerTask {
    let scan_label = scan_id
        .map(|scan_id| scan_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    WorkerTask::new(
        format!(
            "{SCANNER_ACTION_TASK_PREFIX}{scan_label}:{}",
            action.as_id()
        ),
        WorkerTaskKind::DesktopAction,
    )
    .with_label(action.as_id())
}

fn scanner_action_from_task_id(
    task_id: &workers::WorkerTaskId,
) -> Option<(Option<u64>, ScannerActionKind)> {
    let rest = task_id.as_str().strip_prefix(SCANNER_ACTION_TASK_PREFIX)?;
    let (scan_id, action_id) = rest.split_once(':')?;
    let scan_id = if scan_id == "none" {
        None
    } else {
        Some(scan_id.parse::<u64>().ok()?)
    };
    Some((scan_id, ScannerActionKind::from_id(action_id)?))
}

fn execute_scanner_action_payload(execution: ScannerActionExecution) -> BlockingWorkerResult {
    let feedback = execute_scanner_action_with_adapters(
        execution,
        RealDesktopActions::new(),
        RealClipboardActions::new(),
    );
    Ok(WorkerTaskOutcome::Completed(
        scanner_action_completed_payload(feedback),
    ))
}

fn execute_scanner_action_with_adapters<D, C>(
    execution: ScannerActionExecution,
    desktop: D,
    clipboard: C,
) -> ScannerActionFeedback
where
    D: DesktopActions,
    C: ClipboardActions,
{
    let scan_id = execution.scan_id;
    let action = execution.descriptor.kind;
    match (action, execution.descriptor.target) {
        (ScannerActionKind::CopyDetails, ScannerActionTarget::DetailsText) => {
            let Some(details_text) = execution.details_text.filter(|text| !text.is_empty()) else {
                return ScannerActionFeedback::failed(
                    scan_id,
                    action,
                    "Scanner details are not available to copy.",
                );
            };
            scanner_feedback_from_clipboard_result(
                scan_id,
                action,
                clipboard.copy_text(&details_text),
            )
        }
        (ScannerActionKind::OpenLocation, ScannerActionTarget::Path(path)) => {
            scanner_feedback_from_desktop_result(scan_id, action, desktop.open_path(&path))
        }
        (ScannerActionKind::OpenSolutionUrl, ScannerActionTarget::Url(url)) => {
            scanner_feedback_from_desktop_result(scan_id, action, desktop.open_url(&url))
        }
        (ScannerActionKind::CopySolutionUrl, ScannerActionTarget::Url(url)) => {
            scanner_feedback_from_clipboard_result(scan_id, action, clipboard.copy_text(&url))
        }
        _ => ScannerActionFeedback::failed(scan_id, action, SCANNER_ACTION_UNAVAILABLE_MESSAGE),
    }
}

fn scanner_feedback_from_desktop_result(
    scan_id: Option<u64>,
    action: ScannerActionKind,
    result: platform::desktop::DesktopActionResult,
) -> ScannerActionFeedback {
    let feedback = if result.is_success() {
        ScannerActionFeedback::succeeded(scan_id, action, result.safe_message())
    } else {
        ScannerActionFeedback::failed(scan_id, action, result.safe_message())
    };
    if let Some(diagnostic) = result.diagnostic() {
        feedback.with_diagnostic(diagnostic.to_owned())
    } else {
        feedback
    }
}

fn scanner_feedback_from_clipboard_result(
    scan_id: Option<u64>,
    action: ScannerActionKind,
    result: platform::clipboard::ClipboardActionResult,
) -> ScannerActionFeedback {
    let feedback = if result.is_success() {
        ScannerActionFeedback::succeeded(scan_id, action, result.safe_message())
    } else {
        ScannerActionFeedback::failed(scan_id, action, result.safe_message())
    };
    if let Some(diagnostic) = result.diagnostic() {
        feedback.with_diagnostic(diagnostic.to_owned())
    } else {
        feedback
    }
}

fn scanner_category_from_ui_id(category_id: &str) -> Option<ScannerCategoryKind> {
    match category_id {
        "overview-issues" => Some(ScannerCategoryKind::OverviewIssues),
        "errors" => Some(ScannerCategoryKind::Errors),
        "wrong-file-formats" => Some(ScannerCategoryKind::WrongFormat),
        "loose-previs" => Some(ScannerCategoryKind::LoosePrevis),
        "junk-files" => Some(ScannerCategoryKind::JunkFiles),
        "problem-overrides" => Some(ScannerCategoryKind::ProblemOverrides),
        "race-subgraphs" => Some(ScannerCategoryKind::RaceSubgraphs),
        _ => None,
    }
}

fn i32_to_usize(value: i32) -> Option<usize> {
    if value < 0 {
        None
    } else {
        Some(value as usize)
    }
}

fn request_overview_refresh(
    app: &MainWindow,
    overview_controller: &Arc<Mutex<OverviewController>>,
    settings_controller: &Rc<RefCell<SettingsController<FileAssetResolver>>>,
    worker_runtime: WorkerRuntime,
    overview_sink: SlintEventLoopSink,
    runtime_handle: tokio::runtime::Handle,
) {
    let settings = settings_controller.borrow().current_settings().clone();
    request_overview_refresh_from_settings(
        app,
        overview_controller,
        settings,
        worker_runtime,
        overview_sink,
        runtime_handle,
    );
}

fn request_overview_refresh_from_settings(
    app: &MainWindow,
    overview_controller: &Arc<Mutex<OverviewController>>,
    settings: AppSettings,
    worker_runtime: WorkerRuntime,
    overview_sink: SlintEventLoopSink,
    runtime_handle: tokio::runtime::Handle,
) {
    let Some(request) = with_overview_controller_mut(overview_controller, |controller| {
        controller.request_refresh(settings.update_source)
    }) else {
        return;
    };
    apply_current_overview_snapshot(app, overview_controller);

    tracing::info!(
        event = "overview-refresh-schedule",
        refresh_id = request.refresh_id,
        update_source = request.update_source.as_wire_value(),
        "Scheduling Overview refresh worker"
    );

    if let Err(error) = worker_runtime.spawn_blocking_task(
        request.refresh_task(),
        overview_sink.clone(),
        move |_context| build_overview_refresh_payload(request.refresh_id, settings),
    ) {
        tracing::error!(
            event = "overview-refresh-spawn-failed",
            refresh_id = request.refresh_id,
            error = %error,
            "Overview refresh worker could not be scheduled"
        );
        with_overview_controller_mut(overview_controller, |controller| {
            controller.refresh_spawn_failed(request.refresh_id, error);
        });
        apply_current_overview_snapshot(app, overview_controller);
        return;
    }

    schedule_overview_update_check(request, overview_sink, runtime_handle);
}

fn build_overview_refresh_payload(refresh_id: u64, settings: AppSettings) -> BlockingWorkerResult {
    build_overview_snapshot(settings).map(|snapshot| {
        WorkerTaskOutcome::Completed(WorkerPayload::Overview(
            workers::OverviewWorkerPayload::refresh_completed(refresh_id, snapshot),
        ))
    })
}

fn build_overview_snapshot(settings: AppSettings) -> Result<OverviewSnapshot, WorkerFailure> {
    let span = tracing::info_span!(
        "overview_refresh_worker",
        update_source = settings.update_source.as_wire_value()
    );
    let _guard = span.enter();
    tracing::info!(
        event = "overview-refresh-started",
        "Overview refresh started"
    );

    let filesystem = RealFilesystem::new();
    let registry = RealRegistry::new();
    let process = RealProcessInspector::new();
    let current_working_directory = current_working_directory();
    let local_appdata = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);

    let mut discovery_request = DiscoveryRequest::new(current_working_directory);
    discovery_request = discovery_request.with_current_process_id(std::process::id());
    if let Some(path) = local_appdata.clone() {
        discovery_request = discovery_request.with_local_appdata(path);
    }

    let discovery =
        DiscoveryService::new(&filesystem, &registry, &process).discover(&discovery_request);
    if let Err(error) = &discovery.game {
        tracing::warn!(
            event = "overview-discovery-game-failed",
            safe_message = error.user_message(),
            "Overview discovery did not find a usable Fallout 4 installation"
        );
    }
    if let Err(error) = &discovery.mod_manager {
        tracing::warn!(
            event = "overview-discovery-manager-failed",
            safe_message = error.user_message(),
            "Overview mod-manager discovery failed safely"
        );
    }

    let mut collection_environment = OverviewCollectionEnvironment::new();
    if let Some(path) = local_appdata {
        collection_environment = collection_environment.with_local_appdata(path);
    }

    let collected = match &discovery.game {
        Ok(installation) => {
            let collector = OverviewCollector::new(&filesystem, &process);
            collector.collect(OverviewCollectionRequest::new(
                installation,
                &collection_environment,
            ))
        }
        Err(_) => OverviewCollectedFacts::default(),
    };

    tracing::info!(
        event = "overview-filesystem-collected",
        binaries = collected.diagnostics.binary_count,
        archives = collected.diagnostics.archive_count,
        modules = collected.diagnostics.module_count,
        enabled_archives = collected.diagnostics.enabled_archive_count,
        enabled_modules = collected.diagnostics.enabled_module_count,
        missing_files = collected.diagnostics.missing_file_count,
        unreadable_files = collected.diagnostics.unreadable_file_count,
        "Overview filesystem collection completed"
    );

    let update_state = if matches!(settings.update_source, UpdateSource::None) {
        OverviewUpdateCheckState::NotChecked
    } else {
        OverviewUpdateCheckState::Checking
    };
    let snapshot = OverviewDiagnostics::build(OverviewDiagnosticsInput {
        discovery: &discovery,
        settings: &settings,
        binaries: &collected.binaries,
        archives: &collected.archives,
        modules: &collected.modules,
        enablement: &collected.enablement,
        update: &update_state,
        last_desktop_action: None,
    });

    tracing::info!(
        event = "overview-refresh-finished",
        phase = ?snapshot.refresh.phase,
        problems = snapshot.problems.len(),
        "Overview refresh snapshot built"
    );
    Ok(snapshot)
}

fn current_working_directory() -> PathBuf {
    match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(
                event = "overview-current-dir-failed",
                error = %error,
                "Current working directory could not be read; using empty fallback"
            );
            PathBuf::new()
        }
    }
}

fn schedule_overview_update_check(
    request: app::overview_controller::OverviewRefreshRequest,
    overview_sink: SlintEventLoopSink,
    runtime_handle: tokio::runtime::Handle,
) {
    if !request.should_check_updates() {
        tracing::debug!(
            event = "overview-update-skipped",
            refresh_id = request.refresh_id,
            update_source = request.update_source.as_wire_value(),
            "Overview update check skipped by settings"
        );
        return;
    }

    let task = request.update_task();
    emit_worker_event_or_log(&overview_sink, WorkerEvent::running(task.clone()));

    runtime_handle.spawn(async move {
        let banner = match RealUpdateCheckClient::new() {
            Ok(client) => {
                let service = UpdateCheckService::new(client);
                service.check(request.update_source).await.banner_state()
            }
            Err(error) => {
                tracing::warn!(
                    event = "overview-update-client-build-failed",
                    refresh_id = request.refresh_id,
                    error = %error,
                    "Overview update client could not be created; failing silently"
                );
                update_failure_banner(request.update_source, "update client could not be created")
            }
        };

        let event = WorkerEvent::completed(
            task,
            WorkerPayload::Overview(workers::OverviewWorkerPayload::update_check_completed(
                request.refresh_id,
                banner,
            )),
        );
        emit_worker_event_or_log(&overview_sink, event);
    });
}

fn update_failure_banner(update_source: UpdateSource, summary: &str) -> UpdateBannerState {
    let failures = update_providers(update_source)
        .into_iter()
        .map(|provider| UpdateCheckFailure::new(provider, summary.to_owned()))
        .collect();
    UpdateBannerState::failed_silently(update_source, failures)
}

fn update_providers(update_source: UpdateSource) -> Vec<UpdateProvider> {
    match update_source {
        UpdateSource::Both => vec![UpdateProvider::NexusMods, UpdateProvider::Github],
        UpdateSource::Github => vec![UpdateProvider::Github],
        UpdateSource::Nexus => vec![UpdateProvider::NexusMods],
        UpdateSource::None => Vec::new(),
    }
}

fn schedule_overview_action(
    app: &MainWindow,
    overview_controller: &Arc<Mutex<OverviewController>>,
    worker_runtime: WorkerRuntime,
    overview_sink: SlintEventLoopSink,
    action_kind: OverviewDeferredActionKind,
    unavailable_message: &'static str,
    action_lookup: impl FnOnce(&OverviewController) -> Option<OverviewDeferredAction>,
) {
    let action =
        with_overview_controller_mut(overview_controller, |controller| action_lookup(controller))
            .flatten();

    let Some(action) = action else {
        apply_action_error(
            app,
            overview_controller,
            action_kind,
            unavailable_message.to_owned(),
        );
        return;
    };

    tracing::info!(
        event = "overview-desktop-action-schedule",
        action = ?action.kind,
        target = action_target_label(&action).as_str(),
        "Scheduling Overview desktop action"
    );

    let task = overview_desktop_task(action.kind);
    if let Err(error) = worker_runtime.spawn_blocking_task(task, overview_sink, move |_context| {
        execute_overview_action_payload(action)
    }) {
        tracing::error!(
            event = "overview-desktop-action-spawn-failed",
            action = ?action_kind,
            error = %error,
            "Overview desktop action worker could not be scheduled"
        );
        apply_action_error(
            app,
            overview_controller,
            action_kind,
            "Desktop action could not be started.".to_owned(),
        );
    }
}

fn execute_overview_action_payload(action: OverviewDeferredAction) -> BlockingWorkerResult {
    let action_kind = action.kind;
    let service = OverviewLinkService::new(RealDesktopActions::new());
    let feedback = service.execute(&action);
    let (action, error) = action_error_from_feedback(feedback);
    debug_assert_eq!(action, action_kind);
    Ok(WorkerTaskOutcome::Completed(
        overview_desktop_action_payload(action, error),
    ))
}

fn action_error_from_feedback(
    feedback: OverviewDesktopActionFeedback,
) -> (OverviewDeferredActionKind, Option<OverviewActionError>) {
    match feedback.outcome {
        OverviewDesktopActionOutcome::Succeeded => (feedback.action, None),
        OverviewDesktopActionOutcome::Failed { safe_message } => (
            feedback.action,
            Some(OverviewActionError::new(feedback.action, safe_message)),
        ),
    }
}

fn apply_action_error(
    app: &MainWindow,
    overview_controller: &Arc<Mutex<OverviewController>>,
    action: OverviewDeferredActionKind,
    message: String,
) {
    with_overview_controller_mut(overview_controller, |controller| {
        controller
            .desktop_action_completed(action, Some(unavailable_action_error(action, message)));
    });
    apply_current_overview_snapshot(app, overview_controller);
}

fn apply_tools_feedback(
    app: &MainWindow,
    controller: &Arc<Mutex<ToolsController>>,
    feedback: ToolsActionFeedback,
) {
    with_tools_controller_mut(controller, |controller| {
        controller.handle_feedback(feedback);
    });
    apply_current_tools_state(app, controller);
}

fn apply_about_feedback(
    app: &MainWindow,
    controller: &Arc<Mutex<AboutController>>,
    feedback: AboutActionFeedback,
) {
    with_about_controller_mut(controller, |controller| {
        controller.handle_feedback(feedback);
    });
    apply_current_about_state(app, controller);
}

fn handle_f4se_worker_event(
    controller: &mut F4seController,
    event: WorkerEvent,
) -> F4seTransitionResult {
    controller.handle_worker_event(event)
}

fn handle_scanner_worker_event(
    controller: &mut ScannerController,
    event: WorkerEvent,
) -> ScannerTransitionResult {
    if let Some(feedback) = scanner_worker_failure_feedback_from_event(&event) {
        return controller.action_completed(feedback);
    }

    controller.handle_worker_event(event)
}

fn handle_tools_worker_event(
    controller: &mut ToolsController,
    event: WorkerEvent,
) -> ToolsTransitionResult {
    if let Some(feedback) = tools_worker_failure_feedback_from_event(&event) {
        return controller.handle_feedback(feedback);
    }

    controller.handle_worker_event(event)
}

fn handle_about_worker_event(
    controller: &mut AboutController,
    event: WorkerEvent,
) -> AboutTransitionResult {
    if let Some(feedback) = about_worker_failure_feedback_from_event(&event) {
        return controller.handle_feedback(feedback);
    }

    controller.handle_worker_event(event)
}

fn scanner_worker_failure_feedback_from_event(
    event: &WorkerEvent,
) -> Option<ScannerActionFeedback> {
    let WorkerPayload::Error(failure) = &event.payload else {
        return None;
    };
    let (scan_id, action) = scanner_action_from_task_id(&event.task.id)?;

    Some(
        ScannerActionFeedback::failed(scan_id, action, failure.safe_message.clone())
            .with_diagnostic(
                failure
                    .diagnostic
                    .clone()
                    .unwrap_or_else(|| "scanner action worker failed".to_owned()),
            ),
    )
}

fn tools_worker_failure_feedback_from_event(event: &WorkerEvent) -> Option<ToolsActionFeedback> {
    let WorkerPayload::Error(failure) = &event.payload else {
        return None;
    };
    let action_id = tools_action_id_from_task(&event.task)?;

    Some(ToolsActionFeedback::rejected(
        action_id,
        tools_action_for_id(action_id).ok(),
        ActionRejectionKind::WorkerUnavailable,
        failure.safe_message.clone(),
        failure.diagnostic.clone(),
    ))
}

fn about_worker_failure_feedback_from_event(event: &WorkerEvent) -> Option<AboutActionFeedback> {
    let WorkerPayload::Error(failure) = &event.payload else {
        return None;
    };
    let action_id = about_action_id_from_task(&event.task)?;

    Some(AboutActionFeedback::rejected(
        action_id,
        about_action_for_id(action_id).ok(),
        ActionRejectionKind::WorkerUnavailable,
        failure.safe_message.clone(),
        failure.diagnostic.clone(),
    ))
}

fn tools_action_id_from_task(task: &WorkerTask) -> Option<&str> {
    task.id.as_str().strip_prefix(TOOLS_WORKER_TASK_PREFIX)
}

fn about_action_id_from_task(task: &WorkerTask) -> Option<&str> {
    task.id.as_str().strip_prefix(ABOUT_WORKER_TASK_PREFIX)
}

fn emit_worker_event_or_log<S>(sink: &S, event: WorkerEvent)
where
    S: WorkerEventSink,
{
    let task_id = event.task.id.to_string();
    let task_kind = event.task.kind.label();
    let status = event.status.label();
    if let Err(error) = sink.emit(event) {
        tracing::warn!(
            event = "overview-worker-handoff-failed",
            task_id = %task_id,
            task_kind,
            status,
            error = %error,
            diagnostic = error.diagnostic.as_deref().unwrap_or(""),
            "Overview worker event could not be handed to the UI"
        );
    }
}

fn with_overview_controller_mut<T>(
    controller: &Arc<Mutex<OverviewController>>,
    action: impl FnOnce(&mut OverviewController) -> T,
) -> Option<T> {
    match controller.lock() {
        Ok(mut controller) => Some(action(&mut controller)),
        Err(error) => {
            tracing::error!(
                event = "overview-controller-lock-poisoned",
                diagnostic = %error,
                "Overview controller state is unavailable"
            );
            None
        }
    }
}

fn with_f4se_controller_mut<T>(
    controller: &Arc<Mutex<F4seController>>,
    action: impl FnOnce(&mut F4seController) -> T,
) -> Option<T> {
    match controller.lock() {
        Ok(mut controller) => Some(action(&mut controller)),
        Err(error) => {
            tracing::error!(
                event = "s06-f4se-controller-lock-poisoned",
                diagnostic = %error,
                "F4SE controller state is unavailable"
            );
            None
        }
    }
}

fn with_scanner_controller_mut<T>(
    controller: &Arc<Mutex<ScannerController>>,
    action: impl FnOnce(&mut ScannerController) -> T,
) -> Option<T> {
    match controller.lock() {
        Ok(mut controller) => Some(action(&mut controller)),
        Err(error) => {
            tracing::error!(
                event = "s07-scanner-controller-lock-poisoned",
                diagnostic = %error,
                "Scanner controller state is unavailable"
            );
            None
        }
    }
}

fn with_tools_controller_mut<T>(
    controller: &Arc<Mutex<ToolsController>>,
    action: impl FnOnce(&mut ToolsController) -> T,
) -> Option<T> {
    match controller.lock() {
        Ok(mut controller) => Some(action(&mut controller)),
        Err(error) => {
            tracing::error!(
                event = "s05-tools-controller-lock-poisoned",
                diagnostic = %error,
                "Tools controller state is unavailable"
            );
            None
        }
    }
}

fn with_about_controller_mut<T>(
    controller: &Arc<Mutex<AboutController>>,
    action: impl FnOnce(&mut AboutController) -> T,
) -> Option<T> {
    match controller.lock() {
        Ok(mut controller) => Some(action(&mut controller)),
        Err(error) => {
            tracing::error!(
                event = "s05-about-controller-lock-poisoned",
                diagnostic = %error,
                "About controller state is unavailable"
            );
            None
        }
    }
}

fn with_downgrader_controller_mut<T>(
    controller: &Arc<Mutex<DowngraderController>>,
    action: impl FnOnce(&mut DowngraderController) -> T,
) -> Option<T> {
    match controller.lock() {
        Ok(mut controller) => Some(action(&mut controller)),
        Err(error) => {
            tracing::error!(
                event = "s09-downgrader-controller-lock-poisoned",
                diagnostic = %error,
                "Downgrader controller state is unavailable"
            );
            None
        }
    }
}

fn with_archive_patcher_controller_mut<T>(
    controller: &Arc<Mutex<ArchivePatcherController>>,
    action: impl FnOnce(&mut ArchivePatcherController) -> T,
) -> Option<T> {
    match controller.lock() {
        Ok(mut controller) => Some(action(&mut controller)),
        Err(error) => {
            tracing::error!(
                event = "s10-archive-patcher-controller-lock-poisoned",
                diagnostic = %error,
                "Archive Patcher controller state is unavailable"
            );
            None
        }
    }
}

fn apply_current_downgrader_state(
    window: &DowngraderWindow,
    controller: &Arc<Mutex<DowngraderController>>,
) {
    let Some(projection) = with_downgrader_controller_mut(controller, |controller| {
        project_downgrader_state(controller)
    }) else {
        return;
    };
    apply_downgrader_projection(window, projection);
}

fn project_downgrader_state(controller: &DowngraderController) -> DowngraderUiProjection {
    let status_rows = controller.status_rows();
    let options = controller.options();
    DowngraderUiProjection {
        current_game_status_rows: format_downgrader_status_rows(
            &status_rows,
            DowngraderFileGroup::Game,
        ),
        current_creation_kit_status_rows: format_downgrader_status_rows(
            &status_rows,
            DowngraderFileGroup::CreationKit,
        ),
        selected_target: downgrader_target_ui_value(options.target).to_owned(),
        keep_backups: options.keep_backups,
        delete_patches: options.delete_deltas,
        plan_rows: controller
            .plan()
            .map(|plan| plan.rows.iter().map(format_downgrader_plan_row).collect())
            .unwrap_or_default(),
        plan_visible: controller.plan().is_some(),
        confirmation_state: if matches!(
            controller.phase(),
            app::downgrader_controller::DowngraderControllerPhase::PlanReady
        ) {
            "needs-confirmation".to_owned()
        } else {
            "idle".to_owned()
        },
        plan_confirmation_text: DOWNGRADER_PLAN_READY_MESSAGE.to_owned(),
        log_rows: controller
            .log_rows()
            .iter()
            .map(format_downgrader_log_row)
            .collect(),
        log_text: controller.status_text().to_owned(),
        progress_percent: controller.progress().percent,
        progress_text: controller.status_text().to_owned(),
        patch_enabled: controller.patch_button_enabled(),
        about_enabled: !matches!(
            controller.phase(),
            app::downgrader_controller::DowngraderControllerPhase::Running
        ),
        controls_enabled: !matches!(
            controller.phase(),
            app::downgrader_controller::DowngraderControllerPhase::LoadingStatus
                | app::downgrader_controller::DowngraderControllerPhase::Planning
                | app::downgrader_controller::DowngraderControllerPhase::Running
        ),
        close_blocked: !controller.close_enabled(),
    }
}

fn apply_downgrader_projection(window: &DowngraderWindow, projection: DowngraderUiProjection) {
    window.set_current_game_status_rows(model_from_vec(projection.current_game_status_rows));
    window.set_current_creation_kit_status_rows(model_from_vec(
        projection.current_creation_kit_status_rows,
    ));
    window.set_selected_target(projection.selected_target.as_str().into());
    window.set_keep_backups(projection.keep_backups);
    window.set_delete_patches(projection.delete_patches);
    window.set_plan_rows(model_from_vec(projection.plan_rows));
    window.set_plan_visible(projection.plan_visible);
    window.set_confirmation_state(projection.confirmation_state.as_str().into());
    window.set_plan_confirmation_text(projection.plan_confirmation_text.as_str().into());
    window.set_log_rows(model_from_vec(projection.log_rows));
    window.set_log_text(projection.log_text.as_str().into());
    window.set_progress_percent(projection.progress_percent);
    window.set_progress_text(projection.progress_text.as_str().into());
    window.set_patch_enabled(projection.patch_enabled);
    window.set_about_enabled(projection.about_enabled);
    window.set_controls_enabled(projection.controls_enabled);
    window.set_close_blocked(projection.close_blocked);
}

fn apply_current_archive_patcher_state(
    window: &ArchivePatcherWindow,
    controller: &Arc<Mutex<ArchivePatcherController>>,
) {
    let Some(projection) = with_archive_patcher_controller_mut(controller, |controller| {
        project_archive_patcher_state(controller)
    }) else {
        return;
    };
    apply_archive_patcher_projection(window, projection);
}

fn project_archive_patcher_state(
    controller: &ArchivePatcherController,
) -> ArchivePatcherUiProjection {
    ArchivePatcherUiProjection {
        selected_target: archive_patcher_target_ui_value(controller.target()).to_owned(),
        name_filter: controller.name_filter().to_owned(),
        candidate_rows: controller
            .candidate_rows()
            .iter()
            .map(format_archive_patcher_candidate_row)
            .collect(),
        candidate_empty_text: archive_patcher_candidate_empty_text(controller).to_owned(),
        plan_rows: controller
            .preview_plan_rows()
            .iter()
            .map(format_archive_patcher_plan_row)
            .collect(),
        confirmation_visible: controller.plan().is_some(),
        confirmation_text: ARCHIVE_PATCHER_PLAN_READY_MESSAGE.to_owned(),
        log_rows: controller
            .log_rows()
            .iter()
            .map(format_archive_patcher_log_row)
            .collect(),
        log_text: archive_patcher_log_text(controller),
        progress_percent: controller.progress().percent,
        progress_text: archive_patcher_progress_text(controller),
        status_text: controller.status_text().to_owned(),
        patch_enabled: controller.patch_button_enabled(),
        restore_enabled: controller.restore_button_enabled(),
        about_enabled: !matches!(
            controller.phase(),
            app::archive_patcher_controller::ArchivePatcherControllerPhase::PatchRunning
                | app::archive_patcher_controller::ArchivePatcherControllerPhase::RestoreRunning
        ),
        controls_enabled: !matches!(
            controller.phase(),
            app::archive_patcher_controller::ArchivePatcherControllerPhase::LoadingCandidates
                | app::archive_patcher_controller::ArchivePatcherControllerPhase::Planning
                | app::archive_patcher_controller::ArchivePatcherControllerPhase::PatchRunning
                | app::archive_patcher_controller::ArchivePatcherControllerPhase::RestoreRunning
        ),
        close_blocked: !controller.close_enabled(),
        about_dialog_visible: controller.about_open(),
        about_title: ABOUT_ARCHIVES_TITLE.to_owned(),
        about_body: ABOUT_ARCHIVES_BODY.to_owned(),
    }
}

fn apply_archive_patcher_projection(
    window: &ArchivePatcherWindow,
    projection: ArchivePatcherUiProjection,
) {
    window.set_selected_target(projection.selected_target.as_str().into());
    window.set_name_filter(projection.name_filter.as_str().into());
    window.set_candidate_rows(model_from_vec(projection.candidate_rows));
    window.set_candidate_empty_text(projection.candidate_empty_text.as_str().into());
    window.set_plan_rows(model_from_vec(projection.plan_rows));
    window.set_confirmation_visible(projection.confirmation_visible);
    window.set_confirmation_text(projection.confirmation_text.as_str().into());
    window.set_log_rows(model_from_vec(projection.log_rows));
    window.set_log_text(projection.log_text.as_str().into());
    window.set_progress_percent(projection.progress_percent);
    window.set_progress_text(projection.progress_text.as_str().into());
    window.set_status_text(projection.status_text.as_str().into());
    window.set_patch_enabled(projection.patch_enabled);
    window.set_restore_enabled(projection.restore_enabled);
    window.set_about_enabled(projection.about_enabled);
    window.set_controls_enabled(projection.controls_enabled);
    window.set_close_blocked(projection.close_blocked);
    window.set_about_dialog_visible(projection.about_dialog_visible);
    window.set_about_title(projection.about_title.as_str().into());
    window.set_about_body(projection.about_body.as_str().into());
}

fn format_archive_patcher_candidate_row(
    row: &ArchivePatcherCandidateRow,
) -> ArchivePatcherCandidateUiRow {
    ArchivePatcherCandidateUiRow {
        display_name: row.display_name.as_str().into(),
        path: row.path.display().to_string().into(),
        version: archive_version_label(row.overview_version).into(),
        format: archive_format_label(&row.overview_format).into(),
        detail: row.path.display().to_string().into(),
    }
}

fn format_archive_patcher_plan_row(row: &ArchivePatcherPreviewPlanRow) -> ArchivePatcherPlanUiRow {
    let action = match row.action {
        ArchivePatcherPlanAction::PatchVersionByte => {
            format!("Patch BA2 header version to v{}", row.target_version)
        }
        ArchivePatcherPlanAction::PlanFailure => row
            .failure
            .clone()
            .unwrap_or_else(|| "Archive cannot be patched safely.".to_owned()),
    };
    let detail = row
        .header
        .map(|header| format!("Current v{} {}", header.version, header.format))
        .unwrap_or_default();
    ArchivePatcherPlanUiRow {
        display_name: row.candidate.display_name.as_str().into(),
        action: action.as_str().into(),
        detail: detail.as_str().into(),
        severity: if row.can_write() { "neutral" } else { "error" }.into(),
    }
}

fn format_archive_patcher_log_row(row: &ArchivePatcherLogRow) -> ArchivePatcherLogUiRow {
    ArchivePatcherLogUiRow {
        level: archive_patcher_log_level_label(row.level).into(),
        message: row.message.as_str().into(),
    }
}

fn archive_patcher_target_ui_value(target: ArchivePatcherTarget) -> &'static str {
    match target {
        ArchivePatcherTarget::OldGen => "old_gen",
        ArchivePatcherTarget::NextGen => "next_gen",
    }
}

fn archive_patcher_log_level_label(level: ArchivePatcherLogLevel) -> &'static str {
    level.as_reference_str()
}

fn archive_patcher_log_text(controller: &ArchivePatcherController) -> String {
    let status = controller.status_text();
    if status.is_empty() {
        "Archive Patcher has not run yet.".to_owned()
    } else {
        status.to_owned()
    }
}

fn archive_patcher_progress_text(controller: &ArchivePatcherController) -> String {
    if !controller.progress().text.is_empty() {
        controller.progress().text.clone()
    } else if !controller.status_text().is_empty() {
        controller.status_text().to_owned()
    } else {
        "Ready".to_owned()
    }
}

fn archive_patcher_candidate_empty_text(controller: &ArchivePatcherController) -> &'static str {
    if controller.safe_error().is_some() {
        ARCHIVE_PATCHER_OVERVIEW_UNAVAILABLE_MESSAGE
    } else {
        "No archives match the selected version/filter."
    }
}

fn archive_format_label(format: &crate::domain::discovery::ArchiveFormat) -> String {
    match format.as_reference_magic() {
        Some(label) => label.to_owned(),
        None => "Unknown".to_owned(),
    }
}

fn archive_version_label(version: crate::domain::discovery::ArchiveVersion) -> String {
    match version {
        crate::domain::discovery::ArchiveVersion::OldGen => "v1".to_owned(),
        crate::domain::discovery::ArchiveVersion::NextGen7 => "v7".to_owned(),
        crate::domain::discovery::ArchiveVersion::NextGen8 => "v8".to_owned(),
        crate::domain::discovery::ArchiveVersion::Unknown(value) => format!("v{value}"),
    }
}

fn format_downgrader_status_rows(
    rows: &[DowngraderStatusRow],
    group: DowngraderFileGroup,
) -> Vec<DowngraderStatusUiRow> {
    rows.iter()
        .copied()
        .filter(|row| row.group == group)
        .map(|row| DowngraderStatusUiRow {
            display_name: row.display_name.into(),
            status: row.status_label().into(),
            severity: downgrader_status_severity(row.status).into(),
            detail: row.relative_path.into(),
        })
        .collect()
}

fn format_downgrader_plan_row(
    row: &services::downgrader::DowngraderPreviewPlanRow,
) -> DowngraderPlanUiRow {
    let action = row.failure.as_deref().unwrap_or_else(|| {
        row.steps
            .first()
            .map(|step| step.message.as_str())
            .unwrap_or("No action.")
    });
    let detail = if row.failure.is_some() {
        String::new()
    } else {
        row.steps
            .iter()
            .skip(1)
            .map(|step| step.message.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    };
    DowngraderPlanUiRow {
        display_name: row.plan.display_name.into(),
        action: action.into(),
        detail: detail.as_str().into(),
        severity: if row.failure.is_some() {
            "error"
        } else {
            "neutral"
        }
        .into(),
    }
}

fn format_downgrader_log_row(row: &DowngraderExecutionLogRow) -> DowngraderLogUiRow {
    DowngraderLogUiRow {
        level: row.level.as_reference_str().into(),
        message: row.message.as_str().into(),
    }
}

fn downgrader_target_ui_value(target: DowngraderTarget) -> &'static str {
    match target {
        DowngraderTarget::OldGen => "old_gen",
        DowngraderTarget::NextGen => "next_gen",
    }
}

fn downgrader_status_severity(status: DowngraderInstallStatus) -> &'static str {
    match status {
        DowngraderInstallStatus::OldGen => "old-gen",
        DowngraderInstallStatus::NextGen | DowngraderInstallStatus::NextGenAnniversary => {
            "next-gen"
        }
        DowngraderInstallStatus::Anniversary => "anniversary",
        DowngraderInstallStatus::NotFound => "warning",
        DowngraderInstallStatus::Obsolete | DowngraderInstallStatus::Unknown => "error",
    }
}

fn apply_current_tools_state(app: &MainWindow, controller: &Arc<Mutex<ToolsController>>) {
    let Some(projection) = with_tools_controller_mut(controller, |controller| {
        project_tools_state(controller.state())
    }) else {
        return;
    };
    apply_tools_projection(app, &projection);
}

fn apply_current_about_state(app: &MainWindow, controller: &Arc<Mutex<AboutController>>) {
    let Some(projection) = with_about_controller_mut(controller, |controller| {
        project_about_state(controller.state())
    }) else {
        return;
    };
    apply_about_projection(app, &projection);
}

fn apply_current_f4se_snapshot(app: &MainWindow, controller: &Arc<Mutex<F4seController>>) {
    let Some(projection) = with_f4se_controller_mut(controller, |controller| {
        project_f4se_snapshot(controller.snapshot())
    }) else {
        return;
    };
    apply_f4se_projection(app, projection);
}

fn apply_current_scanner_state(app: &MainWindow, controller: &Arc<Mutex<ScannerController>>) {
    let Some(projection) =
        with_scanner_controller_mut(controller, |controller| project_scanner_state(controller))
    else {
        return;
    };
    apply_scanner_projection(app, projection);
}

fn project_f4se_snapshot(snapshot: &F4seScanSnapshot) -> F4seUiProjection {
    F4seUiProjection {
        status_text: f4se_status_text(snapshot),
        busy: snapshot.status == F4seScanStatus::Loading,
        loading_or_error_text: f4se_loading_or_error_text(snapshot),
        unknown_game_detail: f4se_unknown_game_detail(&snapshot.rows),
        rows: format_f4se_rows(&snapshot.rows),
    }
}

fn f4se_status_text(snapshot: &F4seScanSnapshot) -> String {
    match snapshot.status {
        F4seScanStatus::Idle => "F4SE scan has not run yet.".to_owned(),
        F4seScanStatus::Loading => "Scanning DLLs...".to_owned(),
        F4seScanStatus::Ready => format!("F4SE scan complete. DLLs: {}", snapshot.rows.len()),
        F4seScanStatus::Error => "F4SE scan failed.".to_owned(),
    }
}

fn f4se_loading_or_error_text(snapshot: &F4seScanSnapshot) -> String {
    match snapshot.status {
        F4seScanStatus::Loading | F4seScanStatus::Error => snapshot.status_message.clone(),
        F4seScanStatus::Idle | F4seScanStatus::Ready => String::new(),
    }
}

fn f4se_unknown_game_detail(rows: &[F4seDllRow]) -> String {
    rows.iter()
        .flat_map(|row| row.details.iter())
        .find(|detail| detail.contains("could not be classified"))
        .cloned()
        .unwrap_or_default()
}

fn format_f4se_rows(rows: &[F4seDllRow]) -> Vec<F4seUiRow> {
    rows.iter().map(format_f4se_row).collect()
}

fn format_f4se_row(row: &F4seDllRow) -> F4seUiRow {
    F4seUiRow {
        dll: row.dll_name.as_str().into(),
        og: row.og.icon.as_reference_str().into(),
        ng: row.ng.icon.as_reference_str().into(),
        ae: row.ae.icon.as_reference_str().into(),
        your_game: row.your_game.icon.as_reference_str().into(),
        severity: f4se_severity_label(row.severity).into(),
        detail: row.details.join(" ").into(),
    }
}

fn f4se_severity_label(severity: F4seRowSeverity) -> &'static str {
    match severity {
        F4seRowSeverity::Neutral => "neutral",
        F4seRowSeverity::Compatible => "compatible",
        F4seRowSeverity::Incompatible => "incompatible",
        F4seRowSeverity::Warning => "warning",
    }
}

fn apply_f4se_projection(app: &MainWindow, projection: F4seUiProjection) {
    app.set_f4se_status_text(projection.status_text.as_str().into());
    app.set_f4se_busy(projection.busy);
    app.set_f4se_loading_or_error_text(projection.loading_or_error_text.as_str().into());
    app.set_f4se_unknown_game_detail(projection.unknown_game_detail.as_str().into());
    app.set_f4se_rows(model_from_vec(projection.rows));
}

fn project_scanner_state(controller: &ScannerController) -> ScannerUiProjection {
    let categories = scanner_category_toggles(controller.category_projection());
    let detail = project_scanner_detail(controller.selected_detail());
    let visible_file_list = controller.visible_file_list();

    ScannerUiProjection {
        categories,
        scan_button_text: controller.scan_button_text().to_owned(),
        scan_button_enabled: controller.scan_button_enabled(),
        busy: controller.phase() == ScannerControllerPhase::Scanning,
        status_text: controller.status_text().to_owned(),
        progress_text: controller.progress_text().to_owned(),
        progress_percent: controller.progress_percent(),
        result_count_text: controller.result_count_text().to_owned(),
        result_rows: format_scanner_result_rows(
            controller.groups(),
            controller.results(),
            controller,
        ),
        show_mod_column: controller
            .results()
            .iter()
            .any(|result| result.mod_attribution.is_some()),
        detail_visible: detail.visible,
        detail_mod: detail.mod_name,
        detail_problem: detail.problem,
        detail_summary: detail.summary,
        detail_solution: detail.solution,
        action_feedback: controller
            .last_action_feedback()
            .map(|feedback| feedback.safe_message().to_owned())
            .unwrap_or_default(),
        open_path_enabled: detail.open_path_enabled,
        open_url_enabled: detail.open_url_enabled,
        copy_url_enabled: detail.copy_url_enabled,
        file_list_enabled: detail.file_list_enabled,
        file_list_visible: controller.file_list_visible(),
        file_list_title: visible_file_list
            .map(|file_list| file_list.title.clone())
            .unwrap_or_else(|| "Files".to_owned()),
        file_list_description: visible_file_list
            .map(|file_list| file_list.description.clone())
            .unwrap_or_default(),
        file_list_first_column: visible_file_list
            .map(|file_list| file_list.columns[0].clone())
            .unwrap_or_else(|| "Value".to_owned()),
        file_list_second_column: visible_file_list
            .map(|file_list| file_list.columns[1].clone())
            .unwrap_or_else(|| "File".to_owned()),
        file_list_rows: visible_file_list
            .map(format_scanner_file_list_rows)
            .unwrap_or_default(),
        auto_fix_button_visible: detail.auto_fix_button_visible,
        auto_fix_button_label: detail.auto_fix_button_label,
        auto_fix_button_enabled: detail.auto_fix_button_enabled,
        auto_fix_status_text: detail.auto_fix_status_text,
        auto_fix_results_visible: detail.auto_fix_results_visible,
        auto_fix_results_title: detail.auto_fix_results_title,
        auto_fix_results_summary: detail.auto_fix_results_summary,
        auto_fix_results_details: detail.auto_fix_results_details,
    }
}

fn scanner_category_toggles(
    categories: Vec<ScannerCategoryProjection>,
) -> ScannerCategoryToggleProjection {
    let mut projection = ScannerCategoryToggleProjection {
        overview_issues: false,
        errors: false,
        wrong_file_formats: false,
        loose_previs: false,
        junk_files: false,
        problem_overrides: false,
        race_subgraphs: false,
        read_only: true,
    };

    for category in categories {
        projection.read_only &= category.read_only;
        match category.kind {
            ScannerCategoryKind::OverviewIssues => projection.overview_issues = category.enabled,
            ScannerCategoryKind::Errors => projection.errors = category.enabled,
            ScannerCategoryKind::WrongFormat => projection.wrong_file_formats = category.enabled,
            ScannerCategoryKind::LoosePrevis => projection.loose_previs = category.enabled,
            ScannerCategoryKind::JunkFiles => projection.junk_files = category.enabled,
            ScannerCategoryKind::ProblemOverrides => {
                projection.problem_overrides = category.enabled;
            }
            ScannerCategoryKind::RaceSubgraphs => projection.race_subgraphs = category.enabled,
        }
    }

    projection
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScannerDetailProjection {
    visible: bool,
    mod_name: String,
    problem: String,
    summary: String,
    solution: String,
    open_path_enabled: bool,
    open_url_enabled: bool,
    copy_url_enabled: bool,
    file_list_enabled: bool,
    auto_fix_button_visible: bool,
    auto_fix_button_label: String,
    auto_fix_button_enabled: bool,
    auto_fix_status_text: String,
    auto_fix_results_visible: bool,
    auto_fix_results_title: String,
    auto_fix_results_summary: String,
    auto_fix_results_details: String,
}

fn project_scanner_detail(
    detail: Option<&app::scanner_controller::ScannerSelectedDetail>,
) -> ScannerDetailProjection {
    let Some(detail) = detail else {
        return ScannerDetailProjection {
            visible: false,
            mod_name: String::new(),
            problem: String::new(),
            summary: String::new(),
            solution: String::new(),
            open_path_enabled: false,
            open_url_enabled: false,
            copy_url_enabled: false,
            file_list_enabled: false,
            auto_fix_button_visible: false,
            auto_fix_button_label: String::new(),
            auto_fix_button_enabled: false,
            auto_fix_status_text: String::new(),
            auto_fix_results_visible: false,
            auto_fix_results_title: String::new(),
            auto_fix_results_summary: String::new(),
            auto_fix_results_details: String::new(),
        };
    };

    let auto_fix = detail.auto_fix.as_ref();
    let auto_fix_result_detail = auto_fix.and_then(|state| state.result_detail.as_ref());

    ScannerDetailProjection {
        visible: true,
        mod_name: scanner_detail_value(&detail.records, DETAIL_LABEL_MOD),
        problem: scanner_detail_value(&detail.records, DETAIL_LABEL_PROBLEM),
        summary: scanner_detail_value(&detail.records, DETAIL_LABEL_SUMMARY),
        solution: scanner_detail_value(&detail.records, DETAIL_LABEL_SOLUTION),
        open_path_enabled: scanner_detail_has_enabled_action(
            detail,
            ScannerActionKind::OpenLocation,
        ),
        open_url_enabled: scanner_detail_has_enabled_action(
            detail,
            ScannerActionKind::OpenSolutionUrl,
        ),
        copy_url_enabled: scanner_detail_has_enabled_action(
            detail,
            ScannerActionKind::CopySolutionUrl,
        ),
        file_list_enabled: scanner_detail_has_enabled_action(
            detail,
            ScannerActionKind::ShowFileList,
        ),
        auto_fix_button_visible: auto_fix.is_some(),
        auto_fix_button_label: auto_fix
            .map(|state| state.button.label.to_owned())
            .unwrap_or_default(),
        auto_fix_button_enabled: auto_fix
            .map(|state| state.button.enabled)
            .unwrap_or_default(),
        auto_fix_status_text: auto_fix
            .map(|state| state.status.safe_message.clone())
            .unwrap_or_default(),
        auto_fix_results_visible: auto_fix_result_detail.is_some(),
        auto_fix_results_title: auto_fix_result_detail
            .map(|result_detail| result_detail.title.to_owned())
            .unwrap_or_default(),
        auto_fix_results_summary: auto_fix_result_detail
            .map(|result_detail| result_detail.safe_summary.clone())
            .unwrap_or_default(),
        auto_fix_results_details: auto_fix_result_detail
            .map(|result_detail| result_detail.details.clone())
            .unwrap_or_default(),
    }
}

fn scanner_detail_value(records: &[domain::scanner::ScannerDetailRecord], label: &str) -> String {
    records
        .iter()
        .find(|record| record.label == label)
        .map(|record| record.value.clone())
        .unwrap_or_default()
}

fn scanner_detail_has_enabled_action(
    detail: &app::scanner_controller::ScannerSelectedDetail,
    action: ScannerActionKind,
) -> bool {
    detail
        .actions
        .iter()
        .any(|descriptor| descriptor.kind == action && descriptor.enabled)
}

fn format_scanner_result_rows(
    groups: &[ScannerResultGroup],
    flat_results: &[ScannerResult],
    controller: &ScannerController,
) -> Vec<ScannerResultUiRow> {
    let mut rows = Vec::new();
    let mut used_flat_indices = vec![false; flat_results.len()];

    for group in groups {
        rows.push(ScannerResultUiRow {
            row_kind: "group".into(),
            result_index: -1,
            problem: group.label.as_str().into(),
            mod_name: SharedString::default(),
            has_mod: false,
            row_fixed: false,
            row_checked: false,
        });

        for result in &group.results {
            let result_index =
                scanner_flat_result_index(result, flat_results, &mut used_flat_indices);
            let row_state = controller.auto_fix_state_for_result(result_index);
            rows.push(ScannerResultUiRow {
                row_kind: "result".into(),
                result_index: usize_to_i32(result_index),
                problem: result.tree_label.as_str().into(),
                mod_name: result
                    .mod_attribution
                    .as_ref()
                    .map(|mod_attribution| mod_attribution.display_name())
                    .unwrap_or_default()
                    .into(),
                has_mod: result.mod_attribution.is_some(),
                row_fixed: row_state.map(|state| state.row_fixed).unwrap_or_default(),
                row_checked: row_state.map(|state| state.row_checked).unwrap_or_default(),
            });
        }
    }

    rows
}

fn scanner_flat_result_index(
    result: &ScannerResult,
    flat_results: &[ScannerResult],
    used_flat_indices: &mut [bool],
) -> usize {
    for (index, candidate) in flat_results.iter().enumerate() {
        if !used_flat_indices[index] && candidate == result {
            used_flat_indices[index] = true;
            return index;
        }
    }
    0
}

fn usize_to_i32(value: usize) -> i32 {
    if value > i32::MAX as usize {
        i32::MAX
    } else {
        value as i32
    }
}

fn format_scanner_file_list_rows(file_list: &ScannerFileList) -> Vec<ScannerFileListUiRow> {
    file_list
        .entries
        .iter()
        .map(|entry| ScannerFileListUiRow {
            value: entry.value.as_str().into(),
            path: entry.path.display().to_string().into(),
        })
        .collect()
}

fn apply_scanner_projection(app: &MainWindow, projection: ScannerUiProjection) {
    app.set_scanner_overview_issues_enabled(projection.categories.overview_issues);
    app.set_scanner_errors_enabled(projection.categories.errors);
    app.set_scanner_wrong_file_formats_enabled(projection.categories.wrong_file_formats);
    app.set_scanner_loose_previs_enabled(projection.categories.loose_previs);
    app.set_scanner_junk_files_enabled(projection.categories.junk_files);
    app.set_scanner_problem_overrides_enabled(projection.categories.problem_overrides);
    app.set_scanner_race_subgraphs_enabled(projection.categories.race_subgraphs);
    app.set_scanner_settings_read_only(projection.categories.read_only);
    app.set_scanner_scan_button_text(projection.scan_button_text.as_str().into());
    app.set_scanner_scan_button_enabled(projection.scan_button_enabled);
    app.set_scanner_busy(projection.busy);
    app.set_scanner_status_text(projection.status_text.as_str().into());
    app.set_scanner_progress_text(projection.progress_text.as_str().into());
    app.set_scanner_progress_percent(projection.progress_percent);
    app.set_scanner_result_count_text(projection.result_count_text.as_str().into());
    app.set_scanner_result_rows(model_from_vec(projection.result_rows));
    app.set_scanner_show_mod_column(projection.show_mod_column);
    app.set_scanner_detail_visible(projection.detail_visible);
    app.set_scanner_detail_mod(projection.detail_mod.as_str().into());
    app.set_scanner_detail_problem(projection.detail_problem.as_str().into());
    app.set_scanner_detail_summary(projection.detail_summary.as_str().into());
    app.set_scanner_detail_solution(projection.detail_solution.as_str().into());
    app.set_scanner_action_feedback(projection.action_feedback.as_str().into());
    app.set_scanner_open_path_enabled(projection.open_path_enabled);
    app.set_scanner_open_url_enabled(projection.open_url_enabled);
    app.set_scanner_copy_url_enabled(projection.copy_url_enabled);
    app.set_scanner_file_list_enabled(projection.file_list_enabled);
    app.set_scanner_file_list_visible(projection.file_list_visible);
    app.set_scanner_file_list_title(projection.file_list_title.as_str().into());
    app.set_scanner_file_list_description(projection.file_list_description.as_str().into());
    app.set_scanner_file_list_first_column(projection.file_list_first_column.as_str().into());
    app.set_scanner_file_list_second_column(projection.file_list_second_column.as_str().into());
    app.set_scanner_file_list_rows(model_from_vec(projection.file_list_rows));
    app.set_scanner_auto_fix_button_visible(projection.auto_fix_button_visible);
    app.set_scanner_auto_fix_button_label(projection.auto_fix_button_label.as_str().into());
    app.set_scanner_auto_fix_button_enabled(projection.auto_fix_button_enabled);
    app.set_scanner_auto_fix_status_text(projection.auto_fix_status_text.as_str().into());
    app.set_scanner_auto_fix_results_visible(projection.auto_fix_results_visible);
    app.set_scanner_auto_fix_results_title(projection.auto_fix_results_title.as_str().into());
    app.set_scanner_auto_fix_results_summary(projection.auto_fix_results_summary.as_str().into());
    app.set_scanner_auto_fix_results_details(projection.auto_fix_results_details.as_str().into());
}

fn project_tools_state(state: &ToolsState) -> ToolsUiProjection {
    ToolsUiProjection {
        last_action_error: state
            .last_safe_error
            .as_ref()
            .map(|error| error.summary.clone())
            .unwrap_or_default(),
        disabled_utility_status: state
            .disabled_utility_status
            .clone()
            .unwrap_or_else(|| TOOLS_DEFAULT_DISABLED_UTILITY_STATUS.to_owned()),
    }
}

fn project_about_state(state: &AboutState) -> AboutUiProjection {
    let nexus = about_copy_projection(state, AboutLinkId::Nexus);
    let discord = about_copy_projection(state, AboutLinkId::Discord);
    let github = about_copy_projection(state, AboutLinkId::Github);

    AboutUiProjection {
        last_action_error: state
            .last_safe_error
            .as_ref()
            .map(|error| error.summary.clone())
            .unwrap_or_default(),
        nexus_copy_label: nexus.0,
        nexus_copy_enabled: nexus.1,
        discord_copy_label: discord.0,
        discord_copy_enabled: discord.1,
        github_copy_label: github.0,
        github_copy_enabled: github.1,
    }
}

fn about_copy_projection(state: &AboutState, link_id: AboutLinkId) -> (String, bool) {
    if let Some(button) = state
        .copy_buttons
        .iter()
        .find(|button| button.link_id == link_id)
    {
        return (button.label.clone(), button.enabled);
    }

    let fallback_label = ABOUT_LINKS
        .iter()
        .find(|link| link.id == link_id)
        .map(|link| link.copy_button_label)
        .unwrap_or_default();
    (fallback_label.to_owned(), true)
}

fn apply_tools_projection(app: &MainWindow, projection: &ToolsUiProjection) {
    app.set_tools_last_action_error(projection.last_action_error.as_str().into());
    app.set_tools_disabled_utility_status(projection.disabled_utility_status.as_str().into());
}

fn apply_about_projection(app: &MainWindow, projection: &AboutUiProjection) {
    app.set_about_last_action_error(projection.last_action_error.as_str().into());
    app.set_about_nexus_copy_label(projection.nexus_copy_label.as_str().into());
    app.set_about_nexus_copy_enabled(projection.nexus_copy_enabled);
    app.set_about_discord_copy_label(projection.discord_copy_label.as_str().into());
    app.set_about_discord_copy_enabled(projection.discord_copy_enabled);
    app.set_about_github_copy_label(projection.github_copy_label.as_str().into());
    app.set_about_github_copy_enabled(projection.github_copy_enabled);
}

fn apply_current_overview_snapshot(app: &MainWindow, controller: &Arc<Mutex<OverviewController>>) {
    let Some(snapshot) =
        with_overview_controller_mut(controller, |controller| controller.snapshot().clone())
    else {
        return;
    };
    apply_overview_snapshot(app, &snapshot);
}

fn apply_overview_snapshot(app: &MainWindow, snapshot: &OverviewSnapshot) {
    app.set_overview_refresh_message(refresh_message(&snapshot.refresh).into());
    app.set_overview_refresh_busy(snapshot.refresh.is_busy());
    app.set_overview_top_rows(model_from_vec(format_top_rows(snapshot)));
    app.set_overview_binary_rows(model_from_vec(format_binary_rows(snapshot)));
    app.set_overview_archive_rows(model_from_vec(format_count_panel_rows(
        &snapshot.archives.rows,
    )));
    app.set_overview_module_rows(model_from_vec(format_count_panel_rows(
        &snapshot.modules.rows,
    )));
    app.set_overview_problems_summary(format_problem_summary(snapshot).into());
    app.set_overview_problem_rows(model_from_vec(format_problem_rows(snapshot)));
    app.set_overview_game_path_enabled(matches!(
        snapshot.top.game_path,
        domain::overview::OverviewGamePathStatus::Found(_)
    ));
    app.set_overview_downgrade_label(
        deferred_action_label(
            &snapshot.binaries.actions,
            OverviewDeferredActionKind::OpenDowngradeManager,
            ACTION_DOWNGRADE_MANAGER_LABEL,
        )
        .into(),
    );
    app.set_overview_downgrade_enabled(
        snapshot
            .binaries
            .actions
            .iter()
            .find(|action| action.kind == OverviewDeferredActionKind::OpenDowngradeManager)
            .is_some_and(|action| action.enabled),
    );
    app.set_overview_downgrade_status(
        overview_downgrade_action_status(&snapshot.binaries.actions).into(),
    );
    app.set_overview_archive_patcher_label(
        deferred_action_label(
            &snapshot.archives.actions,
            OverviewDeferredActionKind::OpenArchivePatcher,
            ACTION_ARCHIVE_PATCHER_LABEL,
        )
        .into(),
    );
    app.set_overview_archive_patcher_enabled(
        snapshot
            .archives
            .actions
            .iter()
            .find(|action| action.kind == OverviewDeferredActionKind::OpenArchivePatcher)
            .is_some_and(|action| action.enabled),
    );
    app.set_overview_archive_patcher_status(
        overview_archive_patcher_action_status(&snapshot.archives.actions).into(),
    );
    app.set_overview_last_action_error(
        snapshot
            .last_action_error
            .as_ref()
            .map(|error| error.summary.as_str())
            .unwrap_or("")
            .into(),
    );
    apply_update_banner(app, &snapshot.update_banner);
}

fn model_from_vec<T: Clone + 'static>(rows: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(rows))
}

fn refresh_message(refresh: &OverviewRefreshState) -> String {
    refresh
        .message
        .clone()
        .unwrap_or_else(|| match refresh.phase {
            domain::overview::OverviewRefreshPhase::Idle => {
                "Overview has not been refreshed.".to_owned()
            }
            domain::overview::OverviewRefreshPhase::Loading => "Refreshing Overview...".to_owned(),
            domain::overview::OverviewRefreshPhase::Ready => "Overview refreshed.".to_owned(),
            domain::overview::OverviewRefreshPhase::Partial => {
                "Overview refreshed with recoverable issues.".to_owned()
            }
            domain::overview::OverviewRefreshPhase::Error => "Overview refresh failed.".to_owned(),
        })
}

fn format_top_rows(snapshot: &OverviewSnapshot) -> Vec<OverviewUiRow> {
    snapshot
        .top
        .rows()
        .into_iter()
        .map(format_top_row)
        .collect()
}

fn format_top_row(row: OverviewTopStatusRow) -> OverviewUiRow {
    overview_ui_row(row.label, row.value, "", row.severity)
}

fn format_binary_rows(snapshot: &OverviewSnapshot) -> Vec<OverviewUiRow> {
    let mut rows = snapshot
        .binaries
        .rows
        .iter()
        .map(format_binary_row)
        .collect::<Vec<_>>();

    if rows.is_empty() {
        rows.push(overview_ui_row(
            "Binaries",
            "No binary facts collected yet.",
            "",
            StatusSeverity::Unknown,
        ));
    }

    rows.push(overview_ui_row(
        "Address Library",
        snapshot.binaries.address_library.display_text(),
        "",
        snapshot.binaries.address_library.severity(),
    ));
    rows
}

fn format_binary_row(row: &BinaryStatusRow) -> OverviewUiRow {
    overview_ui_row(
        row.label.as_str(),
        row.install_type.to_string(),
        binary_detail(row),
        row.severity,
    )
}

fn binary_detail(row: &BinaryStatusRow) -> String {
    let mut parts = Vec::new();
    if let Some(version) = row.version.as_deref().filter(|value| !value.is_empty()) {
        parts.push(format!("Version: {version}"));
    }
    if let Some(hash) = row.hash.as_deref().filter(|value| !value.is_empty()) {
        parts.push(format!("CRC32: {hash}"));
    }
    parts.join(" · ")
}

fn format_count_panel_rows(rows: &[OverviewCountRow]) -> Vec<OverviewUiRow> {
    if rows.is_empty() {
        return vec![overview_ui_row(
            "Counts",
            "No facts collected yet.",
            "",
            StatusSeverity::Unknown,
        )];
    }

    rows.iter()
        .map(|row| {
            overview_ui_row(
                row.label,
                row.value.to_string(),
                row.limit
                    .map(|limit| format!("Limit: {limit}"))
                    .unwrap_or_default(),
                row.severity,
            )
        })
        .collect()
}

fn format_problem_summary(snapshot: &OverviewSnapshot) -> String {
    format!("Problems: {}", snapshot.problems.len())
}

fn format_problem_rows(snapshot: &OverviewSnapshot) -> Vec<OverviewUiRow> {
    if snapshot.problems.is_empty() {
        return vec![overview_ui_row(
            "Problems",
            "0",
            "No problems detected.",
            StatusSeverity::Good,
        )];
    }

    let mut rows = snapshot
        .problems
        .iter()
        .take(5)
        .map(format_problem_row)
        .collect::<Vec<_>>();
    if snapshot.problems.len() > 5 {
        rows.push(overview_ui_row(
            "More",
            format!("+{} more", snapshot.problems.len() - 5),
            "Open Scanner details in a later port slice.",
            StatusSeverity::Info,
        ));
    }
    rows
}

fn format_problem_row(problem: &OverviewProblem) -> OverviewUiRow {
    overview_ui_row(
        problem.problem.label(),
        problem.display_path.as_str(),
        problem.summary.as_str(),
        problem.severity,
    )
}

fn overview_ui_row(
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    severity: StatusSeverity,
) -> OverviewUiRow {
    OverviewUiRow {
        label: label.into(),
        value: value.into(),
        detail: detail.into(),
        severity: severity_label(severity).into(),
    }
}

fn severity_label(severity: StatusSeverity) -> &'static str {
    match severity {
        StatusSeverity::Good => "good",
        StatusSeverity::Warning => "warning",
        StatusSeverity::Error => "error",
        StatusSeverity::Info => "info",
        StatusSeverity::Neutral => "neutral",
        StatusSeverity::Unknown => "unknown",
    }
}

fn deferred_action_label(
    actions: &[OverviewDeferredAction],
    kind: OverviewDeferredActionKind,
    fallback: &'static str,
) -> String {
    actions
        .iter()
        .find(|action| action.kind == kind)
        .map(|action| action.label.clone())
        .unwrap_or_else(|| fallback.to_owned())
}

fn overview_downgrade_action_status(actions: &[OverviewDeferredAction]) -> String {
    let Some(action) = actions
        .iter()
        .find(|action| action.kind == OverviewDeferredActionKind::OpenDowngradeManager)
    else {
        return "Not available for the current Overview state.".to_owned();
    };

    if action.enabled {
        "Open Downgrade Manager.".to_owned()
    } else {
        "Action is disabled for the current Overview state.".to_owned()
    }
}

fn overview_archive_patcher_action_status(actions: &[OverviewDeferredAction]) -> String {
    let Some(action) = actions
        .iter()
        .find(|action| action.kind == OverviewDeferredActionKind::OpenArchivePatcher)
    else {
        return "Not available for the current Overview state.".to_owned();
    };

    if action.enabled {
        "Open Archive Patcher.".to_owned()
    } else {
        "Action is disabled for the current Overview state.".to_owned()
    }
}

fn deferred_action_status(
    actions: &[OverviewDeferredAction],
    kind: OverviewDeferredActionKind,
    workflow_name: &'static str,
) -> String {
    let Some(action) = actions.iter().find(|action| action.kind == kind) else {
        return "Not available for the current Overview state.".to_owned();
    };

    match &action.target {
        OverviewDeferredActionTarget::Internal => {
            format!("Deferred until the {workflow_name} workflow is ported.")
        }
        OverviewDeferredActionTarget::Path(_) | OverviewDeferredActionTarget::Url(_) => {
            if action.enabled {
                "Ready.".to_owned()
            } else {
                "Action is disabled for the current Overview state.".to_owned()
            }
        }
    }
}

fn apply_update_banner(app: &MainWindow, update_banner: &UpdateBannerState) {
    app.set_overview_update_heading(update_banner.heading().unwrap_or("").into());

    let nexus_label = update_release_label(update_banner, UpdateProvider::NexusMods)
        .unwrap_or_else(|| "Nexus Mods".to_owned());
    let github_label = update_release_label(update_banner, UpdateProvider::Github)
        .unwrap_or_else(|| "GitHub".to_owned());
    app.set_overview_nexus_update_enabled(
        update_release_label(update_banner, UpdateProvider::NexusMods).is_some(),
    );
    app.set_overview_github_update_enabled(
        update_release_label(update_banner, UpdateProvider::Github).is_some(),
    );
    app.set_overview_nexus_update_label(nexus_label.into());
    app.set_overview_github_update_label(github_label.into());
}

fn update_release_label(
    update_banner: &UpdateBannerState,
    provider: UpdateProvider,
) -> Option<String> {
    match update_banner {
        UpdateBannerState::Available { releases, .. } => releases
            .iter()
            .find(|release| release.provider == provider)
            .map(|release| release.display_label()),
        UpdateBannerState::Disabled
        | UpdateBannerState::NotChecked { .. }
        | UpdateBannerState::Checking { .. }
        | UpdateBannerState::NoUpdate { .. }
        | UpdateBannerState::FailedSilently { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{
        app::{SHELL_TAB_LABELS, ShellController, shell_tab_labels},
        domain::DomainState,
        platform::PlatformServices,
        services::ServiceLayer,
        workers::{WorkerRuntime, WorkerTaskStatus},
    };
    use crate::domain::{
        archive_patcher::{
            ABOUT_ARCHIVES_BODY as ARCHIVE_PATCHER_ABOUT_BODY,
            ABOUT_ARCHIVES_TITLE as ARCHIVE_PATCHER_ABOUT_TITLE,
            ABOUT_BUTTON_LABEL as ARCHIVE_PATCHER_ABOUT_BUTTON_LABEL, ARCHIVE_PATCHER_MODAL_HEIGHT,
            ARCHIVE_PATCHER_MODAL_TITLE, ARCHIVE_PATCHER_MODAL_WIDTH,
            DESIRED_VERSION_GROUP_LABEL as ARCHIVE_PATCHER_DESIRED_VERSION_GROUP_LABEL,
            NAME_FILTER_LABEL, PATCH_ALL_BUTTON_LABEL as ARCHIVE_PATCHER_PATCH_ALL_BUTTON_LABEL,
            PATCHER_FILTER_NEXT_GEN, PATCHER_FILTER_OLD_GEN,
            TARGET_NEXT_GEN_LABEL as ARCHIVE_PATCHER_TARGET_NEXT_GEN_LABEL,
            TARGET_OLD_GEN_LABEL as ARCHIVE_PATCHER_TARGET_OLD_GEN_LABEL,
        },
        autofix::{
            AUTO_FIX_BUTTON_LABEL, AUTO_FIX_FAILED_BUTTON_LABEL, AUTO_FIX_FIXED_BUTTON_LABEL,
            AUTO_FIX_RESULTS_TITLE, AUTO_FIXING_BUTTON_LABEL, AutoFixOperationKey,
        },
        downgrader::{
            ABOUT_BUTTON_LABEL, ABOUT_DOWNGRADING_BODY, ABOUT_DOWNGRADING_TITLE,
            CURRENT_CREATION_KIT_GROUP_LABEL, CURRENT_GAME_GROUP_LABEL,
            DELETE_PATCHES_CHECKBOX_LABEL, DESIRED_VERSION_GROUP_LABEL, DOWNGRADER_MODAL_HEIGHT,
            DOWNGRADER_MODAL_TITLE, DOWNGRADER_MODAL_WIDTH, DowngraderLogLevel, DowngraderProgress,
            INITIAL_LOG_LINE, KEEP_BACKUPS_CHECKBOX_LABEL, OPTIONS_GROUP_LABEL,
            PATCH_ALL_BUTTON_LABEL, TARGET_NEXT_GEN_LABEL, TARGET_OLD_GEN_LABEL,
        },
        f4se::{
            F4SE_HEADING, F4SE_LEGEND_TEXT, F4SE_LOADING_TEXT, F4SE_TABLE_COLUMNS, F4seDllFacts,
            render_f4se_dll_row,
        },
        scanner::{
            ACTION_COPY_DETAILS_LABEL, ACTION_COPY_URL_LABEL, ACTION_FILE_LIST_LABEL,
            ACTION_OPEN_URL_LABEL, DETAIL_LABEL_MOD, DETAIL_LABEL_PROBLEM, DETAIL_LABEL_SOLUTION,
            DETAIL_LABEL_SUMMARY, SCAN_BUTTON_LABEL, SCANNER_CATEGORY_LABELS,
            SCANNING_BUTTON_LABEL, ScannerExtraData, ScannerFileList, ScannerFileListEntry,
            ScannerProblemType, ScannerResult, ScannerScanSnapshot, ScannerSolutionKind,
        },
        settings::LogLevel,
        tools::{
            ABOUT_COPY_INVITE_LABEL, ABOUT_COPY_LINK_LABEL, ABOUT_COPY_SUCCESS_LABEL,
            ABOUT_CREDIT_LABEL, ABOUT_LINKS, ABOUT_TITLE_LABEL, AboutActionId, AboutLinkId,
            IMAGE_RESOURCE_PATHS, TOOL_GROUPS, ToolActionId,
        },
    };
    use crate::services::autofix::{
        AutoFixOperationContext, AutoFixOperationFailure, AutoFixOperationRunner,
        AutoFixOperationSuccess, AutoFixOperationSupport, AutoFixRegistry,
    };

    const MAIN_SLINT: &str = include_str!("../ui/main.slint");
    const SETTINGS_SLINT: &str = include_str!("../ui/settings_tab.slint");
    const OVERVIEW_SLINT: &str = include_str!("../ui/overview_tab.slint");
    const F4SE_SLINT: &str = include_str!("../ui/f4se_tab.slint");
    const SCANNER_SLINT: &str = include_str!("../ui/scanner_tab.slint");
    const DOWNGRADER_SLINT: &str = include_str!("../ui/downgrader_window.slint");
    const ARCHIVE_PATCHER_SLINT: &str = include_str!("../ui/archive_patcher_window.slint");
    const TOOLS_SLINT: &str = include_str!("../ui/tools_tab.slint");
    const ABOUT_SLINT: &str = include_str!("../ui/about_tab.slint");
    const TAB_COMPONENTS: [(&str, &str, &str, &str); 6] = [
        (
            "ui/overview_tab.slint",
            "OverviewTab",
            "Overview",
            OVERVIEW_SLINT,
        ),
        ("ui/f4se_tab.slint", "F4seTab", "F4SE", F4SE_SLINT),
        (
            "ui/scanner_tab.slint",
            "ScannerTab",
            "Scanner",
            SCANNER_SLINT,
        ),
        ("ui/tools_tab.slint", "ToolsTab", "Tools", TOOLS_SLINT),
        (
            "ui/settings_tab.slint",
            "SettingsTab",
            "Settings",
            SETTINGS_SLINT,
        ),
        ("ui/about_tab.slint", "AboutTab", "About", ABOUT_SLINT),
    ];
    const INERT_TAB_COMPONENTS: [(&str, &str, &str, &str); 0] = [];

    fn slint_string_property_values(source: &str, property: &str) -> Vec<String> {
        let prefix = format!("{property}:");

        source
            .lines()
            .filter_map(|line| line.trim().strip_prefix(&prefix))
            .filter_map(|value| value.trim().trim_end_matches(';').strip_prefix('"'))
            .filter_map(|value| value.strip_suffix('"'))
            .map(String::from)
            .collect()
    }

    fn assert_source_contains_in_order(source: &str, expected: &[&str]) {
        let mut search_from = 0;

        for value in expected {
            let relative_index = source[search_from..].find(value).unwrap_or_else(|| {
                panic!("expected source to contain {value:?} after byte {search_from}")
            });
            search_from += relative_index + value.len();
        }
    }

    fn assert_source_contains_strings_in_order(source: &str, expected: &[String]) {
        let mut search_from = 0;

        for value in expected {
            let relative_index = source[search_from..].find(value).unwrap_or_else(|| {
                panic!("expected source to contain {value:?} after byte {search_from}")
            });
            search_from += relative_index + value.len();
        }
    }

    fn slint_string_literal(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('"', "\\\"")
    }

    fn slint_assignment(property: &str, value: &str) -> String {
        format!("{property}: \"{}\"", slint_string_literal(value))
    }

    fn slint_image_reference(resource_path: &str) -> String {
        format!("@image-url(\"../{resource_path}\")")
    }

    fn assert_no_direct_urls_or_reference_tree(source_name: &str, source: &str) {
        for marker in [
            "https://",
            "http://",
            "webbrowser",
            "@image-url(\"../CMT",
            "@image-url(\"CMT",
        ] {
            assert!(
                !source.contains(marker),
                "{source_name} should not embed direct URL/reference-tree marker {marker:?}"
            );
        }
    }

    #[test]
    fn shell_tab_labels_match_reference_order() {
        assert_eq!(
            shell_tab_labels(),
            ["Overview", "F4SE", "Scanner", "Tools", "Settings", "About"]
        );
    }

    #[test]
    fn shell_tab_labels_count_is_reference_count() {
        assert_eq!(SHELL_TAB_LABELS.len(), 6);
    }

    #[test]
    fn shell_contract_main_slint_title_and_tabs_match_rust_contract() {
        let titles = slint_string_property_values(MAIN_SLINT, "title");

        assert_eq!(
            titles.first().map(String::as_str),
            Some("Collective Modding Toolkit")
        );
        assert_eq!(
            titles
                .iter()
                .skip(1)
                .map(String::as_str)
                .collect::<Vec<_>>(),
            SHELL_TAB_LABELS.to_vec()
        );
    }

    #[test]
    fn shell_contract_tab_component_files_export_expected_components() {
        for (file, component, label, source) in TAB_COMPONENTS {
            assert!(
                source.contains(&format!("export component {component}")),
                "{file} should export {component}"
            );
            assert!(
                source.contains(label),
                "{file} should preserve tab label marker {label:?}"
            );
        }
    }

    #[test]
    fn shell_contract_inert_tab_components_are_static_placeholders() {
        let prohibited_markers = [
            "callback",
            "clicked",
            "changed",
            "=>",
            "Timer",
            "FileDialog",
            "fs::",
            "std::fs",
            "filesystem",
            "network",
            "http://",
            "https://",
            "process",
            "Command",
            "spawn",
        ];

        for (file, component, label, source) in INERT_TAB_COMPONENTS {
            assert_eq!(
                source.matches("export component ").count(),
                1,
                "{file} should export exactly one component"
            );
            assert!(
                source.contains(&format!("export component {component}")),
                "{file} should export {component}"
            );
            assert!(
                source.contains(&format!("text: \"{label}\";")),
                "{file} should keep the reference tab heading"
            );
            assert!(
                source.contains(&format!(
                    "text: \"{label} behavior is reserved for a later port phase.\";"
                )),
                "{file} should keep the inert scope note"
            );

            for marker in prohibited_markers {
                assert!(
                    !source.contains(marker),
                    "{file} should not contain behavior marker {marker:?}"
                );
            }
        }
    }

    #[test]
    fn s06_f4se_slint_contract_replaces_placeholder_with_reference_table_and_legend() {
        assert!(F4SE_SLINT.contains("export struct F4seUiRow"));
        for field in [
            "dll: string",
            "og: string",
            "ng: string",
            "ae: string",
            "your_game: string",
            "severity: string",
            "detail: string",
        ] {
            assert!(
                F4SE_SLINT.contains(field),
                "F4SE UI row should expose field {field:?}"
            );
        }
        assert!(F4SE_SLINT.contains("background: #202020;"));
        assert!(F4SE_SLINT.contains("in-out property <string> f4se-status-text"));
        assert!(F4SE_SLINT.contains("in-out property <bool> f4se-busy"));
        assert!(F4SE_SLINT.contains("in-out property <string> f4se-loading-or-error-text"));
        assert!(F4SE_SLINT.contains("in-out property <string> f4se-unknown-game-detail"));
        assert!(F4SE_SLINT.contains("in-out property <[F4seUiRow]> f4se-rows"));
        assert!(!F4SE_SLINT.contains("F4SE behavior is reserved for a later port phase."));
        assert!(!F4SE_SLINT.contains("Button {"));
        assert!(!F4SE_SLINT.contains("text: \"Refresh\""));

        assert_source_contains_in_order(
            F4SE_SLINT,
            &[
                "title: \"DLL Compatibility\"",
                "text: \"F4SE\"",
                "text: \"DLL\"",
                "text: \"OG\"",
                "text: \"NG\"",
                "text: \"AE\"",
                "text: \"Your Game\"",
                "for row in root.f4se-rows",
                "title: \"F4SE DLLs\"",
            ],
        );
        assert_eq!(F4SE_TABLE_COLUMNS, ["DLL", "OG", "NG", "AE", "Your Game"]);
        assert!(F4SE_SLINT.contains(&slint_assignment("title", F4SE_HEADING)));
        assert!(F4SE_SLINT.contains(&slint_assignment("text", F4SE_LEGEND_TEXT)));
        assert!(F4SE_SLINT.contains(F4SE_LOADING_TEXT));
    }

    #[test]
    fn s06_f4se_slint_contract_main_window_forwards_properties_and_lazy_activation() {
        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "import { F4seTab, F4seUiRow }",
                "in-out property <string> f4se-status-text",
                "in-out property <bool> f4se-busy",
                "in-out property <string> f4se-loading-or-error-text",
                "in-out property <string> f4se-unknown-game-detail",
                "in-out property <[F4seUiRow]> f4se-rows",
                "property <int> active-tab-index: 0",
                "property <bool> f4se-tab-activation-observed: false",
                "callback f4se-tab-activated()",
                "f4se-tab-lazy-activation := Timer",
                "running: root.active-tab-index == 1 && !root.f4se-tab-activation-observed",
                "root.f4se-tab-activation-observed = true",
                "root.f4se-tab-activated()",
                "TabWidget {",
                "current-index <=> root.active-tab-index",
            ],
        );
        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "title: \"Overview\"",
                "title: \"F4SE\"",
                "F4seTab {",
                "f4se-status-text <=> root.f4se-status-text",
                "f4se-busy <=> root.f4se-busy",
                "f4se-loading-or-error-text <=> root.f4se-loading-or-error-text",
                "f4se-unknown-game-detail <=> root.f4se-unknown-game-detail",
                "f4se-rows <=> root.f4se-rows",
                "title: \"Scanner\"",
                "title: \"Tools\"",
                "title: \"Settings\"",
                "title: \"About\"",
            ],
        );
        assert!(!MAIN_SLINT.contains("f4se-refresh"));
        assert_eq!(
            shell_tab_labels(),
            ["Overview", "F4SE", "Scanner", "Tools", "Settings", "About"]
        );
    }

    #[test]
    fn s06_f4se_runtime_wiring_projects_controller_snapshots_to_slint_rows() {
        let facts = F4seDllFacts::f4se("modern.dll", true, true, Some(true), Some(false));
        let row = render_f4se_dll_row(&facts, F4seGameTarget::NextGen);
        let snapshot = F4seScanSnapshot::ready(vec![row]);

        let projection = project_f4se_snapshot(&snapshot);

        assert_eq!(projection.status_text, "F4SE scan complete. DLLs: 1");
        assert!(!projection.busy);
        assert_eq!(projection.loading_or_error_text, "");
        assert_eq!(projection.unknown_game_detail, "");
        assert_eq!(projection.rows.len(), 1);
        assert_eq!(projection.rows[0].dll.as_str(), "modern.dll");
        assert_eq!(projection.rows[0].og.as_str(), "✔");
        assert_eq!(projection.rows[0].ng.as_str(), "✔");
        assert_eq!(projection.rows[0].ae.as_str(), "");
        assert_eq!(projection.rows[0].your_game.as_str(), "✔");
        assert_eq!(projection.rows[0].severity.as_str(), "compatible");
        assert!(
            projection.rows[0]
                .detail
                .as_str()
                .contains("Version is supported")
        );
    }

    #[test]
    fn s06_f4se_runtime_wiring_first_activation_schedules_once() {
        let controller = Arc::new(Mutex::new(F4seController::new()));

        let first = take_f4se_initial_scan_request(&controller)
            .expect("first F4SE tab activation should request a worker");
        let second = take_f4se_initial_scan_request(&controller);

        assert_eq!(first.scan_id, 1);
        assert_eq!(first.task.kind, WorkerTaskKind::Scan);
        assert_eq!(first.task.id.as_str(), "s06-f4se-scan:1");
        assert!(second.is_none());
    }

    #[test]
    fn s06_f4se_runtime_wiring_spawn_failure_maps_to_safe_projection() {
        let mut controller = F4seController::new();
        let request = controller
            .request_initial_scan()
            .expect("initial activation should produce work");

        let result = controller.spawn_failed(
            request.scan_id,
            WorkerSpawnError::NoActiveRuntime {
                task_id: request.task.id.clone(),
            },
        );
        let projection = project_f4se_snapshot(controller.snapshot());

        assert_eq!(result, F4seTransitionResult::Applied);
        assert_eq!(projection.status_text, "F4SE scan failed.");
        assert!(!projection.busy);
        assert_eq!(
            projection.loading_or_error_text,
            "F4SE scan could not be started."
        );
        assert!(!projection.loading_or_error_text.contains("Tokio"));
        assert!(projection.rows.is_empty());
    }

    #[test]
    fn s06_f4se_runtime_wiring_worker_completion_application_and_unrelated_ignore() {
        let mut controller = F4seController::new();
        let request = controller.request_scan();
        assert_eq!(
            controller.scan_started(request.scan_id),
            F4seTransitionResult::Applied
        );
        assert!(project_f4se_snapshot(controller.snapshot()).busy);

        let facts = F4seDllFacts::f4se("complete.dll", true, true, Some(true), Some(true));
        let snapshot = F4seScanSnapshot::ready(vec![render_f4se_dll_row(
            &facts,
            F4seGameTarget::Anniversary,
        )]);
        let event = WorkerEvent::completed(
            request.task.clone(),
            f4se_scan_completed_payload(request.scan_id, snapshot),
        );

        assert_eq!(
            handle_f4se_worker_event(&mut controller, event),
            F4seTransitionResult::Applied
        );
        let projection = project_f4se_snapshot(controller.snapshot());
        assert_eq!(projection.status_text, "F4SE scan complete. DLLs: 1");
        assert!(!projection.busy);
        assert_eq!(projection.rows[0].dll.as_str(), "complete.dll");
        assert_eq!(projection.rows[0].your_game.as_str(), "✔");

        let before = controller.snapshot().clone();
        let unrelated = WorkerEvent::completed(
            WorkerTask::new("other-worker", WorkerTaskKind::Generic),
            WorkerPayload::Generic(workers::WorkerMessage::new("Other complete.")),
        );
        assert_eq!(
            handle_f4se_worker_event(&mut controller, unrelated),
            F4seTransitionResult::Ignored
        );
        assert_eq!(controller.snapshot(), &before);
    }

    #[test]
    fn s06_f4se_runtime_wiring_error_empty_and_unknown_game_warning_states_are_visible() {
        let idle_projection = project_f4se_snapshot(&F4seScanSnapshot::idle());
        assert_eq!(idle_projection.status_text, "F4SE scan has not run yet.");
        assert!(idle_projection.rows.is_empty());

        let error_projection =
            project_f4se_snapshot(&F4seScanSnapshot::missing_plugins_folder(false));
        assert_eq!(error_projection.status_text, "F4SE scan failed.");
        assert!(
            error_projection
                .loading_or_error_text
                .contains("Data/F4SE/Plugins folder not found")
        );
        assert!(
            error_projection
                .loading_or_error_text
                .contains("Try launching via your mod manager.")
        );
        assert!(error_projection.rows.is_empty());

        let facts = F4seDllFacts::f4se("unknown-game.dll", false, true, Some(true), Some(true));
        let warning =
            F4seScanSnapshot::ready(vec![render_f4se_dll_row(&facts, F4seGameTarget::Unknown)]);
        let warning_projection = project_f4se_snapshot(&warning);
        assert_eq!(warning_projection.rows[0].dll.as_str(), "unknown-game.dll");
        assert_eq!(warning_projection.rows[0].your_game.as_str(), "⚠");
        assert_eq!(warning_projection.rows[0].severity.as_str(), "warning");
        assert!(
            warning_projection
                .unknown_game_detail
                .contains("could not be classified")
        );
    }

    #[test]
    fn s08_scanner_autofix_slint_contract_replaces_placeholder_with_gated_auto_fix_surface() {
        assert!(SCANNER_SLINT.contains("export struct ScannerResultUiRow"));
        assert!(SCANNER_SLINT.contains("export struct ScannerFileListUiRow"));
        assert!(SCANNER_SLINT.contains("background: #202020;"));
        assert!(!SCANNER_SLINT.contains("Scanner behavior is reserved for a later port phase."));

        assert_source_contains_in_order(
            SCANNER_SLINT,
            &[
                "title: \"Scan Settings\"",
                "text: \"Overview Issues\"",
                "text: \"Errors\"",
                "text: \"Wrong File Formats\"",
                "text: \"Loose Previs\"",
                "text: \"Junk Files\"",
                "text: \"Problem Overrides\"",
                "text: \"Race Subgraphs\"",
                "text: root.scanner-busy ? \"Scanning...\" : root.scanner-scan-button-text",
                "title: \"Progress\"",
                "text: \"Scanner\"",
                "text: root.scanner-result-count-text",
                "ScannerResultHeader {",
                "for row in root.scanner-result-rows",
                "title: \"Details\"",
                "label: \"Mod:\"",
                "label: \"Problem:\"",
                "label: \"Summary:\"",
                "label: \"Solution:\"",
                "text: \"Copy Details\"",
                "text: \"File List\"",
                "text: \"Open Path\"",
                "text: \"Open URL\"",
                "text: \"Copy URL\"",
            ],
        );

        assert_source_contains_in_order(
            SCANNER_SLINT,
            &[
                "component ScannerResultHeader",
                "text: \"Problem\"",
                "if root.show-mod-column: Text",
                "text: \"Mod\"",
            ],
        );

        let expected_category_assignments = SCANNER_CATEGORY_LABELS
            .iter()
            .map(|label| slint_assignment("text", label))
            .collect::<Vec<_>>();
        assert_source_contains_strings_in_order(SCANNER_SLINT, &expected_category_assignments);

        for field in [
            "row_kind: string",
            "result_index: int",
            "problem: string",
            "mod_name: string",
            "has_mod: bool",
            "row_fixed: bool",
            "row_checked: bool",
            "value: string",
            "path: string",
        ] {
            assert!(
                SCANNER_SLINT.contains(field),
                "Scanner Slint structs should expose field {field:?}"
            );
        }

        for callback in [
            "callback category-toggled(string, bool)",
            "callback scan-requested()",
            "callback result-selected(int)",
            "callback auto-fix-requested()",
            "callback copy-details-requested()",
            "callback file-list-requested()",
            "callback open-path-requested()",
            "callback open-url-requested()",
            "callback copy-url-requested()",
        ] {
            assert!(
                SCANNER_SLINT.contains(callback),
                "ScannerTab should expose callback {callback:?}"
            );
        }

        assert_source_contains_in_order(
            SCANNER_SLINT,
            &[
                "if root.scanner-auto-fix-button-visible: Button",
                "text: root.scanner-auto-fix-button-label",
                "enabled: root.scanner-auto-fix-button-enabled",
                "root.auto-fix-requested()",
                "if root.scanner-auto-fix-results-visible: GroupBox",
                "title: root.scanner-auto-fix-results-title",
                "text: root.scanner-auto-fix-results-summary",
                "text: root.scanner-auto-fix-results-details",
            ],
        );
        assert!(SCANNER_SLINT.contains("if root.entry.row_checked: Rectangle"));
        assert!(!SCANNER_SLINT.contains("if !root.scanner-auto-fix-button-visible"));

        assert!(SCANNER_SLINT.contains(SCAN_BUTTON_LABEL));
        assert!(SCANNER_SLINT.contains(SCANNING_BUTTON_LABEL));
        assert!(SCANNER_SLINT.contains(AUTO_FIX_BUTTON_LABEL));
        assert!(SCANNER_SLINT.contains(AUTO_FIX_RESULTS_TITLE));
        assert!(SCANNER_SLINT.contains(ACTION_COPY_DETAILS_LABEL));
        assert!(SCANNER_SLINT.contains(ACTION_FILE_LIST_LABEL));
        assert!(SCANNER_SLINT.contains(ACTION_OPEN_URL_LABEL));
        assert!(SCANNER_SLINT.contains(ACTION_COPY_URL_LABEL));
        assert!(SCANNER_SLINT.contains(DETAIL_LABEL_MOD));
        assert!(SCANNER_SLINT.contains(DETAIL_LABEL_PROBLEM));
        assert!(SCANNER_SLINT.contains(DETAIL_LABEL_SUMMARY));
        assert!(SCANNER_SLINT.contains(DETAIL_LABEL_SOLUTION));
    }

    #[test]
    fn s08_scanner_autofix_slint_contract_main_window_forwards_properties_models_and_callbacks() {
        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "import { ScannerTab, ScannerResultUiRow, ScannerFileListUiRow }",
                "in-out property <bool> scanner-overview-issues-enabled",
                "in-out property <bool> scanner-errors-enabled",
                "in-out property <bool> scanner-wrong-file-formats-enabled",
                "in-out property <bool> scanner-loose-previs-enabled",
                "in-out property <bool> scanner-junk-files-enabled",
                "in-out property <bool> scanner-problem-overrides-enabled",
                "in-out property <bool> scanner-race-subgraphs-enabled",
                "in-out property <bool> scanner-settings-read-only",
                "in-out property <string> scanner-scan-button-text",
                "in-out property <bool> scanner-scan-button-enabled",
                "in-out property <bool> scanner-busy",
                "in-out property <string> scanner-status-text",
                "in-out property <string> scanner-progress-text",
                "in-out property <float> scanner-progress-percent",
                "in-out property <string> scanner-result-count-text",
                "in-out property <[ScannerResultUiRow]> scanner-result-rows",
                "in-out property <bool> scanner-show-mod-column",
                "in-out property <bool> scanner-detail-visible",
                "in-out property <[ScannerFileListUiRow]> scanner-file-list-rows",
                "in-out property <bool> scanner-auto-fix-button-visible",
                "in-out property <string> scanner-auto-fix-button-label",
                "in-out property <bool> scanner-auto-fix-button-enabled",
                "in-out property <string> scanner-auto-fix-status-text",
                "in-out property <bool> scanner-auto-fix-results-visible",
                "in-out property <string> scanner-auto-fix-results-title",
                "in-out property <string> scanner-auto-fix-results-summary",
                "in-out property <string> scanner-auto-fix-results-details",
                "callback scanner-category-toggled(string, bool)",
                "callback scanner-scan-requested()",
                "callback scanner-result-selected(int)",
                "callback scanner-auto-fix-requested()",
                "callback scanner-copy-details-requested()",
                "callback scanner-file-list-requested()",
                "callback scanner-open-path-requested()",
                "callback scanner-open-url-requested()",
                "callback scanner-copy-url-requested()",
            ],
        );

        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "title: \"Scanner\"",
                "ScannerTab {",
                "scanner-overview-issues-enabled <=> root.scanner-overview-issues-enabled",
                "scanner-errors-enabled <=> root.scanner-errors-enabled",
                "scanner-wrong-file-formats-enabled <=> root.scanner-wrong-file-formats-enabled",
                "scanner-loose-previs-enabled <=> root.scanner-loose-previs-enabled",
                "scanner-junk-files-enabled <=> root.scanner-junk-files-enabled",
                "scanner-problem-overrides-enabled <=> root.scanner-problem-overrides-enabled",
                "scanner-race-subgraphs-enabled <=> root.scanner-race-subgraphs-enabled",
                "scanner-result-rows <=> root.scanner-result-rows",
                "scanner-file-list-rows <=> root.scanner-file-list-rows",
                "scanner-auto-fix-button-visible <=> root.scanner-auto-fix-button-visible",
                "scanner-auto-fix-button-label <=> root.scanner-auto-fix-button-label",
                "scanner-auto-fix-button-enabled <=> root.scanner-auto-fix-button-enabled",
                "scanner-auto-fix-status-text <=> root.scanner-auto-fix-status-text",
                "scanner-auto-fix-results-visible <=> root.scanner-auto-fix-results-visible",
                "scanner-auto-fix-results-title <=> root.scanner-auto-fix-results-title",
                "scanner-auto-fix-results-summary <=> root.scanner-auto-fix-results-summary",
                "scanner-auto-fix-results-details <=> root.scanner-auto-fix-results-details",
                "root.scanner-category-toggled(id, enabled)",
                "root.scanner-scan-requested()",
                "root.scanner-result-selected(index)",
                "root.scanner-auto-fix-requested()",
                "root.scanner-copy-details-requested()",
                "root.scanner-file-list-requested()",
                "root.scanner-open-path-requested()",
                "root.scanner-open-url-requested()",
                "root.scanner-copy-url-requested()",
            ],
        );
    }

    #[test]
    fn s07_scanner_slint_contract_runtime_projection_uses_controller_snapshot() {
        let mut controller = ScannerController::new(Default::default());
        let idle = project_scanner_state(&controller);

        assert_eq!(idle.scan_button_text, SCAN_BUTTON_LABEL);
        assert!(idle.scan_button_enabled);
        assert!(!idle.busy);
        assert!(idle.categories.read_only);
        assert!(idle.categories.overview_issues);
        assert!(idle.categories.errors);
        assert!(idle.categories.wrong_file_formats);
        assert!(idle.categories.loose_previs);
        assert!(idle.categories.junk_files);
        assert!(idle.categories.problem_overrides);
        assert!(idle.categories.race_subgraphs);
        assert_eq!(
            idle.result_count_text,
            "0 Results ~ Select an item for details"
        );
        assert!(idle.result_rows.is_empty());
        assert!(!idle.detail_visible);
        assert!(!idle.show_mod_column);
        assert!(!idle.auto_fix_button_visible);
        assert!(!idle.auto_fix_results_visible);

        let request = controller
            .request_scan()
            .expect("enabled scanner categories should allow scan requests");
        let result = ScannerResult::with_path(
            ScannerProblemType::UnexpectedFormat,
            PathBuf::from("/game/Data/Sound/example.mp3"),
            PathBuf::from("Sound/example.mp3"),
            "Format not in whitelist for sound.",
            Some("This file may need to be converted.".to_owned()),
        )
        .with_mod_attribution("Example Mod")
        .with_extra_data(vec![ScannerExtraData::url("https://example.invalid/cmt")])
        .with_file_list(ScannerFileList::generic(vec![ScannerFileListEntry::new(
            1,
            PathBuf::from("Sound/example.mp3"),
        )]));
        let snapshot = ScannerScanSnapshot::from_results(
            request.scan_id,
            vec![result],
            "Scanner scan complete.",
        );

        controller.scan_completed(request.scan_id, snapshot);
        controller.select_result(0);
        let selected = project_scanner_state(&controller);

        assert_eq!(selected.result_rows.len(), 2);
        assert_eq!(selected.result_rows[0].row_kind.as_str(), "group");
        assert_eq!(
            selected.result_rows[0].problem.as_str(),
            "Unexpected Format"
        );
        assert_eq!(selected.result_rows[1].row_kind.as_str(), "result");
        assert_eq!(selected.result_rows[1].result_index, 0);
        assert_eq!(selected.result_rows[1].problem.as_str(), "example.mp3");
        assert_eq!(selected.result_rows[1].mod_name.as_str(), "Example Mod");
        assert!(selected.result_rows[1].has_mod);
        assert!(selected.show_mod_column);
        assert!(selected.detail_visible);
        assert_eq!(selected.detail_mod, "Example Mod");
        assert_eq!(selected.detail_problem, "Sound/example.mp3");
        assert_eq!(
            selected.detail_summary,
            "Format not in whitelist for sound."
        );
        assert!(
            selected
                .detail_solution
                .contains("This file may need to be converted.")
        );
        assert!(
            selected
                .detail_solution
                .contains("https://example.invalid/cmt")
        );
        assert!(selected.open_path_enabled);
        assert!(selected.open_url_enabled);
        assert!(selected.copy_url_enabled);
        assert!(selected.file_list_enabled);
        assert!(!selected.file_list_visible);
        assert!(!selected.auto_fix_button_visible);
        assert!(!selected.auto_fix_button_enabled);
        assert_eq!(selected.auto_fix_button_label, "");
        assert_eq!(selected.auto_fix_status_text, "");
        assert!(!selected.auto_fix_results_visible);

        controller.toggle_file_list();
        let file_list = project_scanner_state(&controller);
        assert!(file_list.file_list_visible);
        assert_eq!(file_list.file_list_title, "Files");
        assert_eq!(file_list.file_list_first_column, "Value");
        assert_eq!(file_list.file_list_second_column, " File");
        assert_eq!(file_list.file_list_rows.len(), 1);
        assert_eq!(file_list.file_list_rows[0].value.as_str(), "1");
        assert!(
            file_list.file_list_rows[0]
                .path
                .as_str()
                .contains("example.mp3")
        );
    }

    #[test]
    fn s07_scanner_runtime_wiring_startup_projection_uses_persisted_settings() {
        let settings = crate::domain::settings::ScannerSettings {
            overview_issues: true,
            errors: false,
            wrong_format: false,
            loose_previs: true,
            junk_files: false,
            problem_overrides: true,
            race_subgraphs: false,
        };
        let controller = ScannerController::new(settings);

        let projection = project_scanner_state(&controller);

        assert!(projection.categories.overview_issues);
        assert!(!projection.categories.errors);
        assert!(!projection.categories.wrong_file_formats);
        assert!(projection.categories.loose_previs);
        assert!(!projection.categories.junk_files);
        assert!(projection.categories.problem_overrides);
        assert!(!projection.categories.race_subgraphs);
        assert!(projection.categories.read_only);
        assert_eq!(projection.scan_button_text, SCAN_BUTTON_LABEL);
        assert!(projection.scan_button_enabled);
        assert!(!projection.busy);
    }

    #[test]
    fn s07_scanner_runtime_wiring_scan_scheduling_persists_and_clears_old_state() {
        let app_settings = AppSettings {
            scanner: crate::domain::settings::ScannerSettings {
                wrong_format: false,
                race_subgraphs: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let (settings_controller, settings_path) =
            main_test_settings_controller("scanner-schedule", app_settings.clone());
        let mut controller = ScannerController::new(app_settings.scanner.clone());
        let old_request = controller
            .request_scan()
            .expect("first scan should create old state");
        controller.scan_completed(
            old_request.scan_id,
            ScannerScanSnapshot::from_results(
                old_request.scan_id,
                vec![scanner_runtime_result()],
                "Old scan complete.",
            ),
        );
        controller.select_result(0);
        let controller = Arc::new(Mutex::new(controller));

        let prepared = prepare_scanner_scan_request(&controller, &settings_controller)
            .expect("persisted scanner settings should schedule a scan");
        let projection = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");
        let persisted = std::fs::read_to_string(settings_path).expect("settings should persist");

        assert_eq!(prepared.request.scan_id, 2);
        assert_eq!(prepared.request.task.kind, WorkerTaskKind::Scan);
        assert_eq!(prepared.request.task.id.as_str(), "s07-scanner-scan:2");
        assert!(!prepared.request.settings_snapshot.wrong_format);
        assert!(!prepared.settings.scanner.wrong_format);
        assert!(projection.busy);
        assert!(!projection.scan_button_enabled);
        assert_eq!(projection.status_text, "Scanning...");
        assert_eq!(projection.progress_text, PROGRESS_REFRESHING_OVERVIEW_TEXT);
        assert!(projection.result_rows.is_empty());
        assert!(!projection.detail_visible);
        assert!(persisted.contains("\"scanner_WrongFormat\": false"));
    }

    #[test]
    fn s07_scanner_runtime_wiring_does_not_schedule_when_save_reverts_or_toggles_off() {
        let disabled = crate::domain::settings::ScannerSettings {
            overview_issues: false,
            errors: false,
            wrong_format: false,
            loose_previs: false,
            junk_files: false,
            problem_overrides: false,
            race_subgraphs: false,
        };
        let app_settings = AppSettings {
            scanner: disabled.clone(),
            ..Default::default()
        };
        let (settings_controller, _settings_path) =
            main_test_settings_controller("scanner-no-toggles", app_settings.clone());
        let controller = Arc::new(Mutex::new(ScannerController::new(disabled)));

        assert!(prepare_scanner_scan_request(&controller, &settings_controller).is_none());
        let projection = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");
        assert!(!projection.busy);
        assert!(!projection.scan_button_enabled);
        assert_eq!(projection.status_text, "No scanner categories are enabled.");

        let root = unique_main_test_root("scanner-save-fails");
        let blocked_settings_path = root.join("settings.json");
        std::fs::create_dir_all(&blocked_settings_path)
            .expect("directory should block settings save");
        let store = SettingsStore::with_asset_resolver(
            blocked_settings_path,
            crate::platform::settings_store::StaticAssetResolver::new(Some("nexus")),
        );
        let settings_controller = Rc::new(RefCell::new(SettingsController::from_settings(
            store,
            AppSettings::default(),
        )));
        let mut scanner_controller = ScannerController::new(Default::default());
        scanner_controller.toggle_category(ScannerCategoryKind::WrongFormat, false);
        let controller = Arc::new(Mutex::new(scanner_controller));

        assert!(prepare_scanner_scan_request(&controller, &settings_controller).is_none());
        let reverted = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");
        assert!(!reverted.busy);
        assert!(reverted.scan_button_enabled);
        assert!(
            reverted.categories.wrong_file_formats,
            "failed save should revert visible Scanner toggles to last persisted settings"
        );
    }

    #[test]
    fn s07_scanner_runtime_wiring_applies_progress_completion_stale_and_zero_results() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");

        assert_eq!(
            handle_scanner_worker_event(
                &mut controller,
                WorkerEvent::running(request.task.clone())
            ),
            ScannerTransitionResult::Applied
        );
        assert_eq!(
            handle_scanner_worker_event(
                &mut controller,
                WorkerEvent::progress(
                    request.task.clone(),
                    workers::WorkerProgress::new()
                        .with_message("Scanning... 1/3: Meshes")
                        .with_counts(Some(1), Some(3)),
                ),
            ),
            ScannerTransitionResult::Applied
        );
        let progressing = project_scanner_state(&controller);
        assert!(progressing.busy);
        assert_eq!(progressing.progress_text, "Scanning... 1/3: Meshes");
        assert!(progressing.progress_percent > 33.0 && progressing.progress_percent < 34.0);

        let empty_snapshot =
            ScannerScanSnapshot::empty(request.scan_id, "Scanner completed with 0 results.");
        assert_eq!(
            handle_scanner_worker_event(
                &mut controller,
                WorkerEvent::completed(
                    request.task.clone(),
                    scanner_scan_completed_payload(request.scan_id, empty_snapshot),
                ),
            ),
            ScannerTransitionResult::Applied
        );
        let completed = project_scanner_state(&controller);
        assert!(!completed.busy);
        assert!(completed.scan_button_enabled);
        assert_eq!(completed.progress_percent, 100.0);
        assert_eq!(
            completed.result_count_text,
            "0 Results ~ Select an item for details"
        );
        assert!(completed.result_rows.is_empty());
        assert!(!completed.detail_visible);

        let second = controller.request_scan().expect("second scan should start");
        assert_eq!(second.scan_id, 2);
        let stale = handle_scanner_worker_event(
            &mut controller,
            WorkerEvent::completed(
                request.task,
                scanner_scan_completed_payload(
                    request.scan_id,
                    ScannerScanSnapshot::empty(request.scan_id, "Old completion."),
                ),
            ),
        );
        assert_eq!(stale, ScannerTransitionResult::StaleIgnored);
        assert_eq!(controller.active_scan_id(), Some(second.scan_id));
    }

    #[test]
    fn s07_scanner_runtime_wiring_selected_detail_actions_and_failure_feedback_are_safe() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            request.scan_id,
            ScannerScanSnapshot::from_results(
                request.scan_id,
                vec![scanner_runtime_result()],
                "Scanner scan complete.",
            ),
        );
        controller.select_result(0);
        let controller = Arc::new(Mutex::new(controller));

        let copy_execution =
            prepare_scanner_action_execution(&controller, ScannerActionKind::CopyDetails)
                .expect("selected result should expose Copy Details");
        let copy_feedback = execute_scanner_action_with_adapters(
            copy_execution,
            RuntimeFakeDesktopActions::default(),
            RuntimeFakeClipboardActions::default(),
        );
        assert!(copy_feedback.succeeded);
        assert_eq!(copy_feedback.safe_message(), "Copied to clipboard.");

        let path_execution =
            prepare_scanner_action_execution(&controller, ScannerActionKind::OpenLocation)
                .expect("selected result should expose Open Location");
        let path_feedback = execute_scanner_action_with_adapters(
            path_execution,
            RuntimeFakeDesktopActions {
                fail_path: true,
                fail_url: false,
            },
            RuntimeFakeClipboardActions::default(),
        );
        assert!(!path_feedback.succeeded);
        assert_eq!(path_feedback.safe_message(), "Path open failed.");
        assert_eq!(
            path_feedback.diagnostic.as_deref(),
            Some("raw path failure")
        );
        with_scanner_controller_mut(&controller, |controller| {
            controller.action_completed(path_feedback);
        });
        let projection = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");
        assert_eq!(projection.action_feedback, "Path open failed.");
        assert!(!projection.action_feedback.contains("raw path"));
    }

    #[test]
    fn s07_scanner_runtime_wiring_action_worker_failure_event_maps_to_safe_feedback() {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            request.scan_id,
            ScannerScanSnapshot::empty(request.scan_id, "Scanner scan complete."),
        );
        let task = scanner_action_task(ScannerActionKind::OpenLocation, Some(request.scan_id));
        assert_eq!(
            scanner_action_from_task_id(&task.id),
            Some((Some(request.scan_id), ScannerActionKind::OpenLocation))
        );

        let result = handle_scanner_worker_event(
            &mut controller,
            WorkerEvent::failed(
                task,
                WorkerFailure::new("Path open failed.").with_diagnostic("raw worker failure"),
            ),
        );

        assert_eq!(result, ScannerTransitionResult::Applied);
        let feedback = controller
            .last_action_feedback()
            .expect("failed action feedback should be visible");
        assert_eq!(feedback.safe_message(), "Path open failed.");
        assert_eq!(feedback.diagnostic.as_deref(), Some("raw worker failure"));
        let projection = project_scanner_state(&controller);
        assert_eq!(projection.action_feedback, "Path open failed.");
        assert!(!projection.action_feedback.contains("raw worker"));
    }

    #[test]
    fn s08_scanner_autofix_runtime_wiring_empty_production_hidden_state_and_tampered_callback_rejected()
     {
        let mut controller = ScannerController::default();
        let request = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            request.scan_id,
            ScannerScanSnapshot::from_results(
                request.scan_id,
                vec![scanner_autofix_runtime_result("desktop.ini")],
                "Scanner scan complete.",
            ),
        );
        controller.select_result(0);
        let projection = project_scanner_state(&controller);

        assert!(controller.auto_fix_support_catalog().is_empty());
        assert!(!projection.auto_fix_button_visible);
        assert!(!projection.auto_fix_button_enabled);
        assert_eq!(projection.auto_fix_button_label, "");
        assert_eq!(projection.auto_fix_status_text, "");
        assert!(!projection.auto_fix_results_visible);
        assert!(!projection.result_rows[1].row_fixed);
        assert!(!projection.result_rows[1].row_checked);

        let controller = Arc::new(Mutex::new(controller));
        assert!(prepare_scanner_auto_fix_request(&controller).is_none());
        let rejected = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");

        assert_eq!(
            rejected.status_text,
            "Auto-Fix is not available for this result."
        );
        assert!(!rejected.auto_fix_button_visible);
        assert!(!rejected.auto_fix_results_visible);
    }

    #[test]
    fn s08_scanner_autofix_runtime_wiring_fake_projection_shows_button_and_rejects_repeated_fixing_click()
     {
        let controller = Arc::new(Mutex::new(scanner_autofix_runtime_controller()));
        let ready = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");

        assert!(ready.auto_fix_button_visible);
        assert!(ready.auto_fix_button_enabled);
        assert_eq!(ready.auto_fix_button_label, AUTO_FIX_BUTTON_LABEL);
        assert_eq!(ready.auto_fix_status_text, "Fake Auto-Fix preview.");
        assert!(!ready.auto_fix_results_visible);
        assert!(!ready.result_rows[1].row_fixed);
        assert!(!ready.result_rows[1].row_checked);

        let prepared = prepare_scanner_auto_fix_request(&controller)
            .expect("fake supported result should prepare Auto-Fix worker data");
        assert_eq!(prepared.request.task.kind, WorkerTaskKind::Patch);
        assert_eq!(
            prepared.request.operation_key,
            AutoFixOperationKey::DeleteOrIgnoreFile
        );

        let fixing = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");
        assert!(fixing.auto_fix_button_visible);
        assert!(!fixing.auto_fix_button_enabled);
        assert_eq!(fixing.auto_fix_button_label, AUTO_FIXING_BUTTON_LABEL);
        assert_eq!(fixing.auto_fix_status_text, "Auto-Fix is running...");

        assert!(prepare_scanner_auto_fix_request(&controller).is_none());
        let rejected_repeat = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");
        assert_eq!(
            rejected_repeat.auto_fix_button_label,
            AUTO_FIXING_BUTTON_LABEL
        );
        assert_eq!(rejected_repeat.status_text, "Auto-Fix is running...");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn s08_scanner_autofix_runtime_wiring_fake_worker_success_shows_inline_results() {
        let controller = Arc::new(Mutex::new(scanner_autofix_runtime_controller()));
        let prepared = prepare_scanner_auto_fix_request(&controller)
            .expect("fake supported result should prepare Auto-Fix worker data");
        let task = prepared.request.task.clone();
        let sink = workers::RecordingEventSink::new();
        let handle = WorkerRuntime::new()
            .spawn_blocking_task(task, sink.clone(), move |_context| {
                let filesystem = RealFilesystem::new();
                let service = AutoFixService::with_registry(
                    &filesystem,
                    scanner_autofix_runtime_registry(RuntimeAutoFixOutcome::Succeed),
                );
                execute_scanner_auto_fix_with_service(prepared.request, prepared.snapshot, &service)
            })
            .expect("active Tokio runtime should schedule fake Auto-Fix worker");

        handle
            .join()
            .await
            .expect("fake Auto-Fix worker should join");
        let events = sink.events().expect("recorded events should be readable");
        assert!(
            events
                .iter()
                .any(|event| event.status == WorkerTaskStatus::Running)
        );
        assert!(
            events
                .iter()
                .any(|event| event.status == WorkerTaskStatus::Completed)
        );

        with_scanner_controller_mut(&controller, |controller| {
            for event in events {
                handle_scanner_worker_event(controller, event);
            }
        });
        let projection = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");

        assert_eq!(
            projection.auto_fix_button_label,
            AUTO_FIX_FIXED_BUTTON_LABEL
        );
        assert!(projection.auto_fix_button_enabled);
        assert_eq!(
            projection.auto_fix_status_text,
            "Fixed fake scanner result."
        );
        assert!(projection.auto_fix_results_visible);
        assert_eq!(projection.auto_fix_results_title, AUTO_FIX_RESULTS_TITLE);
        assert_eq!(
            projection.auto_fix_results_summary,
            "Fixed fake scanner result."
        );
        assert!(
            projection
                .auto_fix_results_details
                .contains("Fixed delete-or-ignore-file at row 0.")
        );
        assert!(projection.result_rows[1].row_fixed);
        assert!(projection.result_rows[1].row_checked);
    }

    #[test]
    fn s08_scanner_autofix_runtime_wiring_fake_worker_failure_shows_safe_inline_results() {
        let controller = Arc::new(Mutex::new(scanner_autofix_runtime_controller()));
        let prepared = prepare_scanner_auto_fix_request(&controller)
            .expect("fake supported result should prepare Auto-Fix worker data");
        let filesystem = RealFilesystem::new();
        let service = AutoFixService::with_registry(
            &filesystem,
            scanner_autofix_runtime_registry(RuntimeAutoFixOutcome::Fail),
        );
        let outcome = execute_scanner_auto_fix_with_service(
            prepared.request.clone(),
            prepared.snapshot,
            &service,
        )
        .expect("fake service should return a controlled worker outcome");
        let WorkerTaskOutcome::Completed(payload) = outcome else {
            panic!("fake Auto-Fix should complete with a payload");
        };

        with_scanner_controller_mut(&controller, |controller| {
            assert_eq!(
                handle_scanner_worker_event(
                    controller,
                    WorkerEvent::completed(prepared.request.task, payload),
                ),
                ScannerTransitionResult::Applied
            );
        });
        let projection = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");

        assert_eq!(
            projection.auto_fix_button_label,
            AUTO_FIX_FAILED_BUTTON_LABEL
        );
        assert!(projection.auto_fix_button_enabled);
        assert_eq!(
            projection.auto_fix_status_text,
            "Auto-Fix could not complete this operation."
        );
        assert!(projection.auto_fix_results_visible);
        assert_eq!(
            projection.auto_fix_results_details,
            "The fake operation reported a controlled failure."
        );
        assert!(!projection.auto_fix_status_text.contains("raw fake"));
        assert!(!projection.auto_fix_results_details.contains("raw fake"));
        assert!(!projection.result_rows[1].row_fixed);
        assert!(!projection.result_rows[1].row_checked);
    }

    #[test]
    fn s08_scanner_autofix_runtime_wiring_worker_spawn_and_failure_feedback_are_safe() {
        let controller = Arc::new(Mutex::new(scanner_autofix_runtime_controller()));
        let prepared = prepare_scanner_auto_fix_request(&controller)
            .expect("fake supported result should prepare Auto-Fix worker data");
        let spawn_error = match WorkerRuntime::new().spawn_blocking_task(
            prepared.request.task.clone(),
            workers::RecordingEventSink::new(),
            |_context| Ok(WorkerTaskOutcome::Completed(WorkerPayload::None)),
        ) {
            Ok(_) => panic!("spawning without an active Tokio runtime should fail safely"),
            Err(error) => error,
        };
        with_scanner_controller_mut(&controller, |controller| {
            controller.auto_fix_spawn_failed(&prepared.request, spawn_error);
        });
        let spawn_failed = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");
        assert_eq!(
            spawn_failed.auto_fix_button_label,
            AUTO_FIX_FAILED_BUTTON_LABEL
        );
        assert_eq!(
            spawn_failed.auto_fix_status_text,
            "Auto-Fix could not be started."
        );
        assert!(!spawn_failed.auto_fix_status_text.contains("runtime"));
        assert!(spawn_failed.auto_fix_results_visible);

        let controller = Arc::new(Mutex::new(scanner_autofix_runtime_controller()));
        let prepared = prepare_scanner_auto_fix_request(&controller)
            .expect("fake supported result should prepare Auto-Fix worker data");
        with_scanner_controller_mut(&controller, |controller| {
            assert_eq!(
                handle_scanner_worker_event(
                    controller,
                    WorkerEvent::failed(
                        prepared.request.task,
                        WorkerFailure::new("Auto-Fix could not complete this operation.")
                            .with_diagnostic("raw worker failure"),
                    ),
                ),
                ScannerTransitionResult::Applied
            );
        });
        let worker_failed = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");
        assert_eq!(
            worker_failed.auto_fix_button_label,
            AUTO_FIX_FAILED_BUTTON_LABEL
        );
        assert_eq!(
            worker_failed.auto_fix_status_text,
            "Auto-Fix could not complete this operation."
        );
        assert!(worker_failed.auto_fix_results_visible);
        assert!(
            !worker_failed
                .auto_fix_results_details
                .contains("raw worker")
        );
    }

    #[test]
    fn s08_scanner_autofix_runtime_wiring_stale_completion_is_ignored() {
        let mut controller = ScannerController::with_auto_fix_support_catalog(
            Default::default(),
            scanner_autofix_runtime_registry(RuntimeAutoFixOutcome::Succeed).support_catalog(),
        );
        let request = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            request.scan_id,
            ScannerScanSnapshot::from_results(
                request.scan_id,
                vec![
                    scanner_autofix_runtime_result("desktop.ini"),
                    scanner_autofix_runtime_result("Thumbs.db"),
                ],
                "Scanner scan complete.",
            ),
        );
        controller.select_result(0);
        let controller = Arc::new(Mutex::new(controller));
        let prepared = prepare_scanner_auto_fix_request(&controller)
            .expect("fake supported result should prepare Auto-Fix worker data");
        with_scanner_controller_mut(&controller, |controller| {
            controller.select_result(1);
        });

        let filesystem = RealFilesystem::new();
        let service = AutoFixService::with_registry(
            &filesystem,
            scanner_autofix_runtime_registry(RuntimeAutoFixOutcome::Succeed),
        );
        let outcome = execute_scanner_auto_fix_with_service(
            prepared.request.clone(),
            prepared.snapshot,
            &service,
        )
        .expect("fake service should return a controlled worker outcome");
        let WorkerTaskOutcome::Completed(payload) = outcome else {
            panic!("fake Auto-Fix should complete with a payload");
        };

        let result = with_scanner_controller_mut(&controller, |controller| {
            handle_scanner_worker_event(
                controller,
                WorkerEvent::completed(prepared.request.task, payload),
            )
        })
        .expect("controller should be readable");
        let projection = with_scanner_controller_mut(&controller, |controller| {
            project_scanner_state(controller)
        })
        .expect("controller should be readable");

        assert_eq!(result, ScannerTransitionResult::StaleIgnored);
        assert_eq!(projection.auto_fix_button_label, AUTO_FIX_BUTTON_LABEL);
        assert!(!projection.result_rows[1].row_fixed);
        assert!(!projection.result_rows[1].row_checked);
        assert!(!projection.result_rows[2].row_fixed);
        assert!(!projection.result_rows[2].row_checked);
    }

    fn scanner_runtime_result() -> ScannerResult {
        ScannerResult::with_path(
            ScannerProblemType::UnexpectedFormat,
            PathBuf::from("C:/Games/Fallout 4/Data/Sound/example.mp3"),
            PathBuf::from("Sound/example.mp3"),
            "Format not in whitelist for sound.",
            Some("This file may need to be converted.".to_owned()),
        )
        .with_extra_data(vec![ScannerExtraData::url("https://example.invalid/cmt")])
        .with_file_list(ScannerFileList::generic(vec![ScannerFileListEntry::new(
            1,
            PathBuf::from("Sound/example.mp3"),
        )]))
    }

    #[derive(Debug, Clone, Copy)]
    enum RuntimeAutoFixOutcome {
        Succeed,
        Fail,
    }

    struct RuntimeAutoFixRunner {
        outcome: RuntimeAutoFixOutcome,
    }

    impl AutoFixOperationRunner for RuntimeAutoFixRunner {
        fn execute(
            &self,
            context: &AutoFixOperationContext<'_>,
        ) -> Result<AutoFixOperationSuccess, AutoFixOperationFailure> {
            match self.outcome {
                RuntimeAutoFixOutcome::Succeed => Ok(AutoFixOperationSuccess::new(
                    "Fixed fake scanner result.",
                    format!(
                        "Fixed {} at row {}.",
                        context.operation_key.as_id(),
                        context.result_index
                    ),
                )
                .with_diagnostic("raw fake success diagnostic")),
                RuntimeAutoFixOutcome::Fail => Err(AutoFixOperationFailure::new(
                    "Auto-Fix could not complete this operation.",
                    "The fake operation reported a controlled failure.",
                )
                .with_diagnostic("raw fake operation failure")),
            }
        }
    }

    fn scanner_autofix_runtime_registry(outcome: RuntimeAutoFixOutcome) -> AutoFixRegistry {
        let mut registry = AutoFixRegistry::empty();
        registry.register(
            AutoFixOperationSupport::new(
                AutoFixOperationKey::DeleteOrIgnoreFile,
                "Fake Auto-Fix",
                "Fake Auto-Fix preview.",
            ),
            RuntimeAutoFixRunner { outcome },
        );
        registry
    }

    fn scanner_autofix_runtime_controller() -> ScannerController {
        let registry = scanner_autofix_runtime_registry(RuntimeAutoFixOutcome::Succeed);
        let mut controller = ScannerController::with_auto_fix_support_catalog(
            Default::default(),
            registry.support_catalog(),
        );
        let request = controller.request_scan().expect("scan should start");
        controller.scan_completed(
            request.scan_id,
            ScannerScanSnapshot::from_results(
                request.scan_id,
                vec![scanner_autofix_runtime_result("desktop.ini")],
                "Scanner scan complete.",
            ),
        );
        controller.select_result(0);
        controller
    }

    fn scanner_autofix_runtime_result(name: &str) -> ScannerResult {
        ScannerResult::with_path(
            ScannerProblemType::JunkFile,
            PathBuf::from(format!("C:/Games/Fallout 4/Data/{name}")),
            PathBuf::from(name),
            "This is a junk file not used by the game or mod managers.",
            None,
        )
        .with_solution_kind(ScannerSolutionKind::DeleteOrIgnoreFile)
    }

    fn main_test_settings_controller(
        name: &str,
        settings: AppSettings,
    ) -> (
        Rc<RefCell<SettingsController<crate::platform::settings_store::StaticAssetResolver>>>,
        PathBuf,
    ) {
        let root = unique_main_test_root(name);
        std::fs::create_dir_all(&root).expect("settings test root should be created");
        let settings_path = root.join("settings.json");
        let store = SettingsStore::with_asset_resolver(
            settings_path.clone(),
            crate::platform::settings_store::StaticAssetResolver::new(Some("nexus")),
        );
        (
            Rc::new(RefCell::new(SettingsController::from_settings(
                store, settings,
            ))),
            settings_path,
        )
    }

    fn unique_main_test_root(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("test clock should be after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cmt-rs-main-{name}-{unique}"))
    }

    #[derive(Debug, Default, Clone, Copy)]
    struct RuntimeFakeDesktopActions {
        fail_path: bool,
        fail_url: bool,
    }

    impl DesktopActions for RuntimeFakeDesktopActions {
        fn open_url(&self, url: &str) -> crate::platform::desktop::DesktopActionResult {
            if self.fail_url {
                crate::platform::desktop::DesktopActionResult::failure(
                    crate::platform::PlatformError::command_failed(
                        crate::platform::PlatformOperation::OpenUrl,
                        url,
                        "raw url failure",
                    ),
                )
            } else {
                crate::platform::desktop::DesktopActionResult::success(
                    crate::platform::PlatformOperation::OpenUrl,
                    url,
                )
            }
        }

        fn open_path(
            &self,
            path: &std::path::Path,
        ) -> crate::platform::desktop::DesktopActionResult {
            if self.fail_path {
                crate::platform::desktop::DesktopActionResult::failure(
                    crate::platform::PlatformError::command_failed(
                        crate::platform::PlatformOperation::OpenPath,
                        path.display().to_string(),
                        "raw path failure",
                    ),
                )
            } else {
                crate::platform::desktop::DesktopActionResult::success(
                    crate::platform::PlatformOperation::OpenPath,
                    path.display().to_string(),
                )
            }
        }

        fn launch_tool(
            &self,
            executable: &std::path::Path,
            _args: &[String],
        ) -> crate::platform::desktop::DesktopActionResult {
            crate::platform::desktop::DesktopActionResult::success(
                crate::platform::PlatformOperation::LaunchTool,
                executable.display().to_string(),
            )
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    struct RuntimeFakeClipboardActions {
        fail_copy: bool,
    }

    impl ClipboardActions for RuntimeFakeClipboardActions {
        fn copy_text(&self, _text: &str) -> crate::platform::clipboard::ClipboardActionResult {
            if self.fail_copy {
                crate::platform::clipboard::ClipboardActionResult::failure(
                    crate::platform::PlatformError::command_failed(
                        crate::platform::PlatformOperation::CopyToClipboard,
                        "system clipboard",
                        "raw clipboard failure",
                    ),
                )
            } else {
                crate::platform::clipboard::ClipboardActionResult::success("system clipboard")
            }
        }
    }

    fn s09_runtime_status_snapshot(
        request_id: u64,
    ) -> services::downgrader::DowngraderStatusSnapshot {
        services::downgrader::DowngraderStatusSnapshot {
            request_id,
            game_root: PathBuf::from("Game"),
            rows: Vec::new(),
            default_target: DowngraderTarget::OldGen,
            unknown_game: false,
            unknown_creation_kit: false,
            diagnostics: Vec::new(),
        }
    }

    fn s09_runtime_plan(
        request_id: u64,
        options: DowngraderOptionsSnapshot,
    ) -> services::downgrader::DowngraderPreviewPlan {
        let status = s09_runtime_status_snapshot(request_id);
        services::downgrader::DowngraderPreviewPlan {
            request_id,
            game_root: status.game_root.clone(),
            options,
            status,
            rows: Vec::new(),
            counts: services::downgrader::DowngraderPreviewPlanCounts::default(),
            can_execute: true,
        }
    }

    fn s09_runtime_execution_result(
        request_id: u64,
    ) -> services::downgrader::DowngraderExecutionResult {
        services::downgrader::DowngraderExecutionResult {
            request_id,
            game_root: PathBuf::from("Game"),
            options: DowngraderOptionsSnapshot::new(DowngraderTarget::OldGen, true, false),
            rows: Vec::new(),
            log_rows: vec![DowngraderExecutionLogRow::new(
                DowngraderLogLevel::Good,
                "Runtime completion row.",
            )],
            progress_events: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn s09_runtime_ready_controller() -> DowngraderController {
        let mut controller = DowngraderController::new();
        let request = controller
            .open(
                DowngraderSettings {
                    keep_backups: true,
                    delete_deltas: false,
                },
                Some(crate::domain::discovery::Fallout4Installation::new(
                    PathBuf::from("Game"),
                )),
            )
            .expect("open should request status");
        assert_eq!(
            controller.status_loaded(
                request.request_id,
                s09_runtime_status_snapshot(request.request_id)
            ),
            DowngraderTransitionResult::Applied
        );
        controller
    }

    #[test]
    fn s09_downgrader_runtime_wiring_about_projection_uses_reference_copy() {
        let projection = project_downgrader_about_dialog();

        assert_eq!(projection.title, ABOUT_DOWNGRADING_TITLE);
        assert_eq!(projection.body, ABOUT_DOWNGRADING_BODY);
        assert!(
            projection
                .body
                .contains("Simple Downgrader's backups will also be used.")
        );
        assert!(DOWNGRADER_SLINT.contains("in-out property <bool> about-dialog-visible"));
        assert!(DOWNGRADER_SLINT.contains("in-out property <string> about-title"));
        assert!(DOWNGRADER_SLINT.contains("in-out property <string> about-body"));
        assert!(DOWNGRADER_SLINT.contains("callback about-close-requested()"));
    }

    #[test]
    fn s09_downgrader_runtime_wiring_open_entrypoints_and_archive_patcher_deferred() {
        assert_eq!(DowngraderOpenSource::Overview.label(), "overview");
        assert_eq!(DowngraderOpenSource::Tools.label(), "tools");
        assert!(MAIN_SLINT.contains("callback overview-open-downgrade-manager-requested()"));
        assert!(MAIN_SLINT.contains("callback tools-open-downgrade-manager-requested()"));
        assert!(OVERVIEW_SLINT.contains("root.open-downgrade-manager-requested()"));
        assert!(TOOLS_SLINT.contains("root.open-downgrade-manager-requested()"));
        assert!(OVERVIEW_SLINT.contains("overview-archive-patcher-enabled: false"));
        assert!(OVERVIEW_SLINT.contains("Deferred until the Archive Patcher workflow is ported."));
        assert!(TOOLS_SLINT.contains(ToolActionId::ArchivePatcher.as_str()));
        assert!(TOOLS_SLINT.contains("Deferred until S10 Archive Patcher workflow is ported."));
    }

    #[test]
    fn s09_downgrader_runtime_wiring_settings_save_failure_reverts_visible_options() {
        let mut settings = AppSettings::default();
        settings.downgrader = DowngraderSettings {
            keep_backups: true,
            delete_deltas: false,
        };
        let (settings_controller, root) =
            main_test_settings_controller("s09-downgrader-save-failure", settings.clone());
        std::fs::create_dir_all(root.join("settings.json"))
            .expect("settings path directory should force save failure");

        let result = settings_controller
            .borrow_mut()
            .save_downgrader_settings_for_workflow(DowngraderSettings {
                keep_backups: false,
                delete_deltas: true,
            });

        assert!(!result.saved);
        assert_eq!(result.visible_settings, settings.downgrader);
        let shared = Arc::new(Mutex::new(AppSettings::default()));
        remember_current_settings_snapshot(&settings_controller, &shared);
        assert_eq!(
            overview_settings_for_downgrader_completion(&shared).downgrader,
            settings.downgrader
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn s09_downgrader_runtime_wiring_live_feedback_stale_close_and_spawn_failure_project_safely() {
        let mut controller = s09_runtime_ready_controller();
        let plan_request = match controller.request_patch_all() {
            Some(DowngraderPatchWorkerRequest::PreviewPlan(request)) => request,
            other => panic!("expected plan request, got {other:?}"),
        };
        assert_eq!(
            controller.plan_ready(
                plan_request.request_id,
                s09_runtime_plan(plan_request.request_id, plan_request.options),
            ),
            DowngraderTransitionResult::Applied
        );
        let run_request = match controller.request_patch_all() {
            Some(DowngraderPatchWorkerRequest::ConfirmedRun(request)) => request,
            other => panic!("expected run request, got {other:?}"),
        };
        assert!(!run_request.confirmed_plan_digest.is_empty());

        assert_eq!(
            controller.request_close(),
            DowngraderTransitionResult::CloseBlocked
        );
        assert!(project_downgrader_state(&controller).close_blocked);
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::new(
                run_request.task.clone(),
                WorkerTaskStatus::Progress,
                downgrader_progress_payload(run_request.request_id, DowngraderProgress::new(37.5)),
            )),
            DowngraderTransitionResult::Applied
        );
        assert_eq!(
            controller.handle_worker_event(WorkerEvent::new(
                run_request.task.clone(),
                WorkerTaskStatus::Progress,
                downgrader_log_row_payload(
                    run_request.request_id,
                    DowngraderExecutionLogRow::new(DowngraderLogLevel::Info, "Downloading patch."),
                ),
            )),
            DowngraderTransitionResult::Applied
        );
        let projection = project_downgrader_state(&controller);
        assert_eq!(projection.progress_percent, 37.5);
        assert!(
            projection
                .log_rows
                .iter()
                .any(|row| row.message.as_str() == "Downloading patch.")
        );

        assert_eq!(
            controller.handle_worker_event(WorkerEvent::completed(
                app::downgrader_controller::downgrader_run_task(run_request.request_id + 100),
                downgrader_run_completed_payload(
                    run_request.request_id + 100,
                    s09_runtime_execution_result(run_request.request_id + 100),
                ),
            )),
            DowngraderTransitionResult::StaleIgnored
        );
        assert_eq!(
            controller.phase(),
            app::downgrader_controller::DowngraderControllerPhase::Running
        );

        assert_eq!(
            controller.spawn_failed(
                DowngraderWorkerRequestKind::Run,
                run_request.request_id,
                WorkerSpawnError::NoActiveRuntime {
                    task_id: run_request.task.id.clone(),
                },
            ),
            DowngraderTransitionResult::Applied
        );
        let projection = project_downgrader_state(&controller);
        assert!(!projection.patch_enabled);
        assert!(!projection.close_blocked);
        assert!(projection.log_text.contains("could not be started"));
    }

    #[test]
    fn s09_downgrader_runtime_wiring_completion_refresh_uses_current_settings_snapshot() {
        let mut settings = AppSettings::default();
        settings.update_source = UpdateSource::Github;
        settings.log_level = LogLevel::Warning;
        settings.downgrader = DowngraderSettings {
            keep_backups: false,
            delete_deltas: true,
        };
        let shared = Arc::new(Mutex::new(settings.clone()));

        let projected = overview_settings_for_downgrader_completion(&shared);

        assert_eq!(projected, settings);
        assert_ne!(projected, AppSettings::default());
    }

    #[test]
    fn s09_downgrader_slint_contract_modal_source_exposes_reference_shape_and_labels() {
        assert!(DOWNGRADER_SLINT.contains("export struct DowngraderStatusUiRow"));
        assert!(DOWNGRADER_SLINT.contains("export struct DowngraderPlanUiRow"));
        assert!(DOWNGRADER_SLINT.contains("export struct DowngraderLogUiRow"));
        assert!(DOWNGRADER_SLINT.contains("export component DowngraderWindow inherits Window"));
        assert!(MAIN_SLINT.contains(
            "import { DowngraderWindow, DowngraderStatusUiRow, DowngraderPlanUiRow, DowngraderLogUiRow }"
        ));

        assert!(DOWNGRADER_SLINT.contains(&slint_assignment("title", DOWNGRADER_MODAL_TITLE)));
        assert!(DOWNGRADER_SLINT.contains(&format!("width: {}px", DOWNGRADER_MODAL_WIDTH)));
        assert!(DOWNGRADER_SLINT.contains(&format!("height: {}px", DOWNGRADER_MODAL_HEIGHT)));

        for label in [
            CURRENT_GAME_GROUP_LABEL,
            CURRENT_CREATION_KIT_GROUP_LABEL,
            DESIRED_VERSION_GROUP_LABEL,
            OPTIONS_GROUP_LABEL,
            TARGET_OLD_GEN_LABEL,
            TARGET_NEXT_GEN_LABEL,
            KEEP_BACKUPS_CHECKBOX_LABEL,
            DELETE_PATCHES_CHECKBOX_LABEL,
            ABOUT_BUTTON_LABEL,
            INITIAL_LOG_LINE,
        ] {
            assert!(
                DOWNGRADER_SLINT.contains(&slint_assignment("text", label))
                    || DOWNGRADER_SLINT.contains(&slint_assignment("title", label))
                    || DOWNGRADER_SLINT.contains(label),
                "Downgrader Slint should contain reference label {label:?}"
            );
        }
        assert!(DOWNGRADER_SLINT.contains(&slint_assignment("text", PATCH_ALL_BUTTON_LABEL)));

        assert_source_contains_in_order(
            DOWNGRADER_SLINT,
            &[
                "title: \"Current Game\"",
                "Fallout4.exe:",
                "Fallout4Launcher.exe:",
                "steam_api64.dll:",
                "title: \"Current Creation Kit\"",
                "CreationKit.exe:",
                "Archive2.exe:",
                "Archive2Interop.dll:",
                "title: \"Desired Version\"",
                "text: \"Old-Gen\"",
                "target-id: \"old_gen\"",
                "text: \"Next-Gen\"",
                "target-id: \"next_gen\"",
                "title: \"Options\"",
                "text: \"Keep Backups\"",
                "option-id: \"keep_backups\"",
                "text: \"Delete Patches\"",
                "option-id: \"delete_patches\"",
                "text: \"Patch\\n All\"",
                "text: \"About\"",
            ],
        );
        assert!(!DOWNGRADER_SLINT.contains("Tools\\\\Archive2\\\\Archive2.exe:"));
        assert!(!DOWNGRADER_SLINT.contains("Tools\\\\Archive2\\\\Archive2Interop.dll:"));

        assert_source_contains_in_order(
            DOWNGRADER_SLINT,
            &[
                "in-out property <[DowngraderStatusUiRow]> current-game-status-rows",
                "in-out property <[DowngraderStatusUiRow]> current-creation-kit-status-rows",
                "in-out property <string> selected-target",
                "in-out property <bool> keep-backups",
                "in-out property <bool> delete-patches",
                "in-out property <[DowngraderPlanUiRow]> plan-rows",
                "in-out property <bool> plan-visible",
                "in-out property <string> confirmation-state",
                "in-out property <[DowngraderLogUiRow]> log-rows",
                "in-out property <string> log-text",
                "in-out property <float> progress-percent",
                "in-out property <string> progress-text",
                "in-out property <bool> patch-enabled",
                "in-out property <bool> about-enabled",
                "in-out property <bool> close-blocked",
                "callback target-selected(string)",
                "callback option-toggled(string, bool)",
                "callback patch-requested()",
                "callback confirm-requested()",
                "callback about-requested()",
                "callback modal-close-requested()",
            ],
        );
        assert!(
            DOWNGRADER_SLINT.contains("Review the plan, then click Patch All again to confirm.")
        );
        assert!(DOWNGRADER_SLINT.contains("root.confirm-requested()"));
        assert!(DOWNGRADER_SLINT.contains("root.patch-requested()"));
        assert!(DOWNGRADER_SLINT.contains("close/Escape"));
    }

    #[test]
    fn s10_archive_patcher_slint_contract_modal_source_exposes_reference_shape_and_labels() {
        assert!(ARCHIVE_PATCHER_SLINT.contains("export struct ArchivePatcherCandidateUiRow"));
        assert!(ARCHIVE_PATCHER_SLINT.contains("export struct ArchivePatcherPlanUiRow"));
        assert!(ARCHIVE_PATCHER_SLINT.contains("export struct ArchivePatcherLogUiRow"));
        assert!(
            ARCHIVE_PATCHER_SLINT.contains("export component ArchivePatcherWindow inherits Window")
        );
        assert!(MAIN_SLINT.contains(
            "import { ArchivePatcherWindow, ArchivePatcherCandidateUiRow, ArchivePatcherPlanUiRow, ArchivePatcherLogUiRow }"
        ));
        assert!(MAIN_SLINT.contains(
            "export { ArchivePatcherWindow, ArchivePatcherCandidateUiRow, ArchivePatcherPlanUiRow, ArchivePatcherLogUiRow }"
        ));

        assert!(
            ARCHIVE_PATCHER_SLINT.contains(&slint_assignment("title", ARCHIVE_PATCHER_MODAL_TITLE))
        );
        assert!(
            ARCHIVE_PATCHER_SLINT.contains(&format!("width: {}px", ARCHIVE_PATCHER_MODAL_WIDTH))
        );
        assert!(
            ARCHIVE_PATCHER_SLINT.contains(&format!("height: {}px", ARCHIVE_PATCHER_MODAL_HEIGHT))
        );

        for label in [
            ARCHIVE_PATCHER_DESIRED_VERSION_GROUP_LABEL,
            ARCHIVE_PATCHER_TARGET_OLD_GEN_LABEL,
            ARCHIVE_PATCHER_TARGET_NEXT_GEN_LABEL,
            NAME_FILTER_LABEL,
            ARCHIVE_PATCHER_PATCH_ALL_BUTTON_LABEL,
            "Restore Last Run",
            ARCHIVE_PATCHER_ABOUT_BUTTON_LABEL,
            ARCHIVE_PATCHER_ABOUT_TITLE,
            "Candidates",
            "Plan",
            "Log",
        ] {
            assert!(
                ARCHIVE_PATCHER_SLINT.contains(&slint_assignment("text", label))
                    || ARCHIVE_PATCHER_SLINT.contains(&slint_assignment("title", label))
                    || ARCHIVE_PATCHER_SLINT.contains(label),
                "Archive Patcher Slint should contain reference label {label:?}"
            );
        }
        assert!(ARCHIVE_PATCHER_SLINT.contains(&slint_string_literal(PATCHER_FILTER_NEXT_GEN)));
        assert!(ARCHIVE_PATCHER_SLINT.contains(&slint_string_literal(PATCHER_FILTER_OLD_GEN)));

        assert_source_contains_in_order(
            ARCHIVE_PATCHER_SLINT,
            &[
                "title: \"Desired Version\"",
                "text: \"v1 (OG)\"",
                "target-id: \"old_gen\"",
                "text: \"v8 (NG)\"",
                "target-id: \"next_gen\"",
                "Showing all v1\\n(Includes Base Game/DLC/CC)",
                "Showing all v7 & v8\\n(Includes Base Game/DLC/CC)",
                "text: \"Patch All\"",
                "text: \"Restore Last Run\"",
                "text: \"About\"",
                "text: \"Name Filter:\"",
                "title: \"Candidates\"",
                "title: \"Plan\"",
                "title: \"Log\"",
            ],
        );
    }

    #[test]
    fn s10_archive_patcher_slint_contract_declares_models_callbacks_and_fail_closed_defaults() {
        for field in [
            "display-name: string",
            "path: string",
            "version: string",
            "format: string",
            "detail: string",
            "action: string",
            "severity: string",
            "level: string",
            "message: string",
        ] {
            assert!(
                ARCHIVE_PATCHER_SLINT.contains(field),
                "Archive Patcher UI structs should expose field {field:?}"
            );
        }

        assert_source_contains_in_order(
            ARCHIVE_PATCHER_SLINT,
            &[
                "in-out property <string> selected-target: \"old_gen\"",
                "in-out property <string> name-filter",
                "in-out property <[ArchivePatcherCandidateUiRow]> candidate-rows",
                "in-out property <[ArchivePatcherPlanUiRow]> plan-rows",
                "in-out property <bool> confirmation-visible: false",
                "in-out property <[ArchivePatcherLogUiRow]> log-rows",
                "in-out property <float> progress-percent",
                "in-out property <string> progress-text",
                "in-out property <string> status-text",
                "in-out property <bool> patch-enabled: false",
                "in-out property <bool> restore-enabled: false",
                "in-out property <bool> about-enabled: true",
                "in-out property <bool> controls-enabled: true",
                "in-out property <bool> close-blocked: false",
                "in-out property <bool> about-dialog-visible: false",
                "in-out property <string> about-title: \"Bethesda Archive (BA2) Formats & Versions\"",
                "in-out property <string> about-body",
            ],
        );

        for callback in [
            "callback target-selected(string)",
            "callback name-filter-edited(string)",
            "callback patch-requested()",
            "callback restore-last-run-requested()",
            "callback about-requested()",
            "callback about-close-requested()",
            "callback modal-close-requested()",
        ] {
            assert!(
                ARCHIVE_PATCHER_SLINT.contains(callback),
                "Archive Patcher window should expose callback {callback:?}"
            );
        }

        assert!(ARCHIVE_PATCHER_SLINT.contains("checked: root.selected-target == \"old_gen\""));
        assert!(
            ARCHIVE_PATCHER_SLINT.contains("enabled: root.controls-enabled && root.patch-enabled")
        );
        assert!(
            ARCHIVE_PATCHER_SLINT
                .contains("enabled: root.controls-enabled && root.restore-enabled")
        );
        assert!(ARCHIVE_PATCHER_SLINT.contains("enabled: root.controls-enabled"));
        assert!(ARCHIVE_PATCHER_SLINT.contains("No archives match the selected version/filter."));
    }

    #[test]
    fn s10_archive_patcher_slint_contract_negative_state_surfaces_are_model_driven() {
        assert_source_contains_in_order(
            ARCHIVE_PATCHER_SLINT,
            &[
                "LineEdit {",
                "text <=> root.name-filter",
                "edited(value) =>",
                "root.name-filter-edited(value)",
                "if root.candidate-rows.length == 0: Text",
                "text: root.candidate-empty-text",
                "for row in root.candidate-rows: ArchivePatcherCandidateRowItem",
                "if root.confirmation-visible: GroupBox",
                "for row in root.plan-rows: ArchivePatcherPlanRowItem",
                "if root.log-rows.length == 0: ArchivePatcherLogRowItem",
                "for row in root.log-rows: ArchivePatcherLogRowItem",
                "if root.about-dialog-visible: Rectangle",
                "text: root.about-title",
                "text: root.about-body",
                "text: \"Close\"",
                "root.about-close-requested()",
            ],
        );

        assert!(
            ARCHIVE_PATCHER_SLINT
                .contains("root.close-blocked ? root.close-blocked-text : root.status-text")
        );
        assert!(ARCHIVE_PATCHER_SLINT.contains("close/Escape"));
        assert!(ARCHIVE_PATCHER_SLINT.contains("modal-close-requested()"));
        assert!(!ARCHIVE_PATCHER_SLINT.contains(ARCHIVE_PATCHER_ABOUT_BODY));

        for prohibited_marker in [
            "std::fs",
            "filesystem",
            "Command",
            "spawn",
            "read_prefix",
            "write_version",
        ] {
            assert!(
                !ARCHIVE_PATCHER_SLINT.contains(prohibited_marker),
                "Archive Patcher Slint should not contain runtime/filesystem marker {prohibited_marker:?}"
            );
        }
        assert_no_direct_urls_or_reference_tree(
            "ui/archive_patcher_window.slint",
            ARCHIVE_PATCHER_SLINT,
        );
    }

    #[test]
    fn s10_archive_patcher_slint_contract_main_window_exports_modal_and_entrypoint_surfaces() {
        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "import { ArchivePatcherWindow, ArchivePatcherCandidateUiRow, ArchivePatcherPlanUiRow, ArchivePatcherLogUiRow }",
                "export { ArchivePatcherWindow, ArchivePatcherCandidateUiRow, ArchivePatcherPlanUiRow, ArchivePatcherLogUiRow }",
                "callback overview-open-archive-patcher-requested()",
                "OverviewTab {",
                "root.overview-open-archive-patcher-requested()",
                "ToolsTab {",
                "root.tool-action-requested(action_id)",
            ],
        );
        assert_source_contains_in_order(
            TOOLS_SLINT,
            &[
                "label: \"Archive Patcher\"",
                "action-id: \"tools.archive_patcher\"",
                "root.tool-action-requested(action_id)",
            ],
        );
    }

    #[test]
    fn s09_downgrader_slint_contract_entrypoints_forward_downgrader_but_keep_archive_patcher_deferred()
     {
        assert!(!DOWNGRADER_SLINT.contains("target-id: \"anniversary\""));
        assert!(!DOWNGRADER_SLINT.contains("text: \"Anniversary\""));

        assert_source_contains_in_order(
            OVERVIEW_SLINT,
            &[
                "overview-downgrade-enabled: true",
                "overview-downgrade-status: \"Open Downgrade Manager.\"",
                "callback open-downgrade-manager-requested()",
                "action-label: root.overview-downgrade-label",
                "action-enabled: root.overview-downgrade-enabled",
                "root.open-downgrade-manager-requested()",
            ],
        );
        assert!(OVERVIEW_SLINT.contains("overview-archive-patcher-enabled: false"));
        assert!(OVERVIEW_SLINT.contains("Deferred until the Archive Patcher workflow is ported."));

        assert_source_contains_in_order(
            TOOLS_SLINT,
            &[
                "callback open-downgrade-manager-requested()",
                "label: \"Downgrade Manager\"",
                "action-id: \"tools.downgrade_manager\"",
                "Open the Downgrade Manager workflow.",
                "root.open-downgrade-manager-requested()",
                "label: \"Archive Patcher\"",
                "action-id: \"tools.archive_patcher\"",
                "button-enabled: false;",
                "Deferred until S10 Archive Patcher workflow is ported.",
            ],
        );
        assert_eq!(TOOLS_SLINT.matches("button-enabled: false;").count(), 1);

        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "callback overview-open-downgrade-manager-requested()",
                "callback tools-open-downgrade-manager-requested()",
                "OverviewTab {",
                "root.overview-open-downgrade-manager-requested()",
                "ToolsTab {",
                "root.tools-open-downgrade-manager-requested()",
            ],
        );
    }

    #[test]
    fn s05_slint_contract_tools_tab_replaces_placeholder_with_reference_groups() {
        assert!(TOOLS_SLINT.contains("export component ToolsTab"));
        assert!(TOOLS_SLINT.contains("background: #202020;"));
        assert!(TOOLS_SLINT.contains("in-out property <string> tools-last-action-error"));
        assert!(TOOLS_SLINT.contains("in-out property <string> tools-disabled-utility-status"));
        assert!(TOOLS_SLINT.contains("callback tool-action-requested(string)"));
        assert!(TOOLS_SLINT.contains("callback open-downgrade-manager-requested()"));
        assert!(TOOLS_SLINT.contains("SafeErrorBanner"));
        assert!(!TOOLS_SLINT.contains("Tools behavior is reserved for a later port phase."));

        let mut expected = Vec::new();
        for group in TOOL_GROUPS {
            expected.push(slint_assignment("title", group.label));
            for entry in group.entries {
                expected.push(slint_assignment("label", entry.label));
                expected.push(slint_assignment("action-id", entry.id.as_str()));
            }
        }
        assert_source_contains_strings_in_order(TOOLS_SLINT, &expected);

        assert_source_contains_in_order(
            TOOLS_SLINT,
            &[
                "title: \"Toolkit Utilities\"",
                "label: \"Downgrade Manager\"",
                "action-id: \"tools.downgrade_manager\"",
                "Open the Downgrade Manager workflow.",
                "root.open-downgrade-manager-requested()",
                "label: \"Archive Patcher\"",
                "action-id: \"tools.archive_patcher\"",
                "button-enabled: false;",
                "Deferred until S10 Archive Patcher workflow is ported.",
            ],
        );
        assert_eq!(TOOLS_SLINT.matches("button-enabled: false;").count(), 1);
        assert!(TOOLS_SLINT.contains("root.tool-action-requested(action_id)"));
        assert_no_direct_urls_or_reference_tree("ui/tools_tab.slint", TOOLS_SLINT);
    }

    #[test]
    fn s05_slint_contract_about_tab_replaces_placeholder_with_reference_assets_and_copy_state() {
        assert!(ABOUT_SLINT.contains("export component AboutTab"));
        assert!(ABOUT_SLINT.contains("background: #202020;"));
        assert!(ABOUT_SLINT.contains("in-out property <string> about-last-action-error"));
        assert!(ABOUT_SLINT.contains("callback about-open-requested(string)"));
        assert!(ABOUT_SLINT.contains("callback about-copy-requested(string)"));
        assert!(ABOUT_SLINT.contains("callback about-copy-label-reset-requested(string)"));
        assert!(ABOUT_SLINT.contains("SafeErrorBanner"));
        assert!(!ABOUT_SLINT.contains("About behavior is reserved for a later port phase."));

        assert!(ABOUT_SLINT.contains(&slint_assignment("text", ABOUT_TITLE_LABEL)));
        assert!(ABOUT_SLINT.contains(&slint_assignment("text", ABOUT_CREDIT_LABEL)));
        for resource_path in IMAGE_RESOURCE_PATHS {
            assert!(
                ABOUT_SLINT.contains(&slint_image_reference(resource_path)),
                "About tab should use Rust-owned image resource {resource_path}"
            );
        }

        assert!(ABOUT_SLINT.contains(&format!(
            "in-out property <string> about-nexus-copy-label: \"{}\"",
            ABOUT_COPY_LINK_LABEL
        )));
        assert!(ABOUT_SLINT.contains(&format!(
            "in-out property <string> about-discord-copy-label: \"{}\"",
            ABOUT_COPY_INVITE_LABEL
        )));
        assert!(ABOUT_SLINT.contains(&format!(
            "in-out property <string> about-github-copy-label: \"{}\"",
            ABOUT_COPY_LINK_LABEL
        )));
        assert!(ABOUT_SLINT.contains("in-out property <bool> about-nexus-copy-enabled: true"));
        assert!(ABOUT_SLINT.contains("in-out property <bool> about-discord-copy-enabled: true"));
        assert!(ABOUT_SLINT.contains("in-out property <bool> about-github-copy-enabled: true"));

        for link in ABOUT_LINKS {
            assert!(ABOUT_SLINT.contains(&slint_assignment(
                "open-action-id",
                link.open_action_id.as_str()
            )));
            assert!(ABOUT_SLINT.contains(&slint_assignment(
                "copy-action-id",
                link.copy_action_id.as_str()
            )));
            assert!(ABOUT_SLINT.contains(&slint_assignment("open-label", link.open_button_label)));
            assert!(ABOUT_SLINT.contains(&format!(
                "root.about-copy-label-reset-requested(\"{}\")",
                link.copy_action_id.as_str()
            )));
        }
        assert_eq!(ABOUT_SLINT.matches("Timer {").count(), 3);
        assert_eq!(ABOUT_SLINT.matches("interval: 3000ms;").count(), 3);
        assert_eq!(ABOUT_SLINT.matches(ABOUT_COPY_SUCCESS_LABEL).count(), 3);
        assert!(ABOUT_SLINT.contains("root.about-open-requested(action_id)"));
        assert!(ABOUT_SLINT.contains("root.about-copy-requested(action_id)"));
        assert_no_direct_urls_or_reference_tree("ui/about_tab.slint", ABOUT_SLINT);
    }

    #[test]
    fn s05_slint_contract_main_window_forwards_tools_and_about_properties_and_callbacks() {
        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "in-out property <string> tools-last-action-error",
                "in-out property <string> tools-disabled-utility-status",
                "in-out property <string> about-last-action-error",
                "in-out property <string> about-nexus-copy-label",
                "in-out property <bool> about-nexus-copy-enabled",
                "in-out property <string> about-discord-copy-label",
                "in-out property <bool> about-discord-copy-enabled",
                "in-out property <string> about-github-copy-label",
                "in-out property <bool> about-github-copy-enabled",
                "callback tool-action-requested(string)",
                "callback about-open-requested(string)",
                "callback about-copy-requested(string)",
                "callback about-copy-label-reset-requested(string)",
            ],
        );
        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "ToolsTab {",
                "tools-last-action-error <=> root.tools-last-action-error",
                "tools-disabled-utility-status <=> root.tools-disabled-utility-status",
                "root.tool-action-requested(action_id)",
                "AboutTab {",
                "about-last-action-error <=> root.about-last-action-error",
                "about-nexus-copy-label <=> root.about-nexus-copy-label",
                "about-nexus-copy-enabled <=> root.about-nexus-copy-enabled",
                "about-discord-copy-label <=> root.about-discord-copy-label",
                "about-discord-copy-enabled <=> root.about-discord-copy-enabled",
                "about-github-copy-label <=> root.about-github-copy-label",
                "about-github-copy-enabled <=> root.about-github-copy-enabled",
                "root.about-open-requested(action_id)",
                "root.about-copy-requested(action_id)",
                "root.about-copy-label-reset-requested(action_id)",
            ],
        );
        assert_no_direct_urls_or_reference_tree("ui/main.slint", MAIN_SLINT);
    }

    #[test]
    fn s05_runtime_wiring_tools_projection_uses_initial_status_and_safe_errors() {
        let mut controller = ToolsController::new();

        let initial = project_tools_state(controller.state());

        assert_eq!(initial.last_action_error, "");
        assert_eq!(
            initial.disabled_utility_status,
            TOOLS_DEFAULT_DISABLED_UTILITY_STATUS
        );

        controller.handle_feedback(ToolsActionFeedback::failed(
            ToolActionId::BethiniPie.as_str(),
            ToolsActionKind::ExternalLink(ToolActionId::BethiniPie),
            crate::platform::PlatformOperation::OpenUrl,
            crate::platform::PlatformErrorKind::UnsupportedPlatform,
            "URL open is not supported on this platform.",
            Some("raw OS diagnostic".to_owned()),
        ));

        let failed = project_tools_state(controller.state());

        assert_eq!(
            failed.last_action_error,
            "URL open is not supported on this platform."
        );
        assert_eq!(
            failed.disabled_utility_status,
            TOOLS_DEFAULT_DISABLED_UTILITY_STATUS
        );
        assert!(!failed.last_action_error.contains("raw OS"));
    }

    #[test]
    fn s05_runtime_wiring_about_projection_tracks_copy_success_and_reset() {
        let mut controller = AboutController::new();

        controller.handle_feedback(AboutActionFeedback::succeeded(
            AboutActionId::CopyDiscord.as_str(),
            AboutActionKind::Copy {
                link_id: AboutLinkId::Discord,
                action_id: AboutActionId::CopyDiscord,
            },
            "Copied to clipboard.",
        ));

        let copied = project_about_state(controller.state());

        assert_eq!(copied.nexus_copy_label, ABOUT_COPY_LINK_LABEL);
        assert!(copied.nexus_copy_enabled);
        assert_eq!(copied.discord_copy_label, ABOUT_COPY_SUCCESS_LABEL);
        assert!(!copied.discord_copy_enabled);
        assert_eq!(copied.github_copy_label, ABOUT_COPY_LINK_LABEL);
        assert!(copied.github_copy_enabled);

        assert_eq!(
            controller.reset_copy_label(AboutActionId::CopyDiscord.as_str()),
            AboutTransitionResult::Applied
        );
        let reset = project_about_state(controller.state());

        assert_eq!(reset.discord_copy_label, ABOUT_COPY_INVITE_LABEL);
        assert!(reset.discord_copy_enabled);
    }

    #[test]
    fn s05_runtime_wiring_callback_id_mapping_fails_closed_before_workers() {
        let tools_unknown = tools_action_for_id("tools.open_arbitrary_url")
            .expect_err("unknown Tools ids should fail closed");
        assert_eq!(
            tools_unknown.outcome,
            crate::services::tools::ActionOutcome::Rejected(ActionRejectionKind::UnknownAction)
        );
        assert_eq!(
            tools_unknown.safe_message(),
            "Tools action is not available."
        );

        let tools_internal = tools_action_for_id(ToolActionId::DowngradeManager.as_str())
            .expect("Downgrade Manager should route as an enabled internal utility");
        assert_eq!(
            tools_internal,
            ToolsActionKind::InternalUtility(ToolActionId::DowngradeManager)
        );

        let tools_deferred = tools_action_for_id(ToolActionId::ArchivePatcher.as_str())
            .expect_err("Archive Patcher utility should still fail closed");
        assert_eq!(
            tools_deferred.outcome,
            crate::services::tools::ActionOutcome::Rejected(ActionRejectionKind::DisabledUtility)
        );

        let open_action = about_action_for_id(AboutActionId::OpenGithub.as_str())
            .expect("known About open action should parse");
        assert!(about_action_matches_callback(
            open_action,
            AboutCallbackKind::Open
        ));
        assert!(!about_action_matches_callback(
            open_action,
            AboutCallbackKind::Copy
        ));

        let copy_action = about_action_for_id(AboutActionId::CopyGithub.as_str())
            .expect("known About copy action should parse");
        let mismatch = about_callback_mismatch_feedback(
            AboutActionId::CopyGithub.as_str(),
            copy_action,
            AboutCallbackKind::Open,
        );

        assert_eq!(
            mismatch.outcome,
            crate::services::tools::ActionOutcome::Rejected(ActionRejectionKind::InvalidInput)
        );
        assert_eq!(mismatch.safe_message(), "About action is not available.");
    }

    #[test]
    fn s05_runtime_wiring_spawn_failure_feedback_maps_to_safe_errors() {
        let tools_error = WorkerSpawnError::NoActiveRuntime {
            task_id: workers::WorkerTaskId::new("tools-no-runtime"),
        };
        let tools_feedback = tools_spawn_failed_feedback(
            ToolActionId::BethiniPie.as_str(),
            Some(ToolsActionKind::ExternalLink(ToolActionId::BethiniPie)),
            tools_error,
        );

        assert_eq!(
            tools_feedback.outcome,
            crate::services::tools::ActionOutcome::Rejected(ActionRejectionKind::WorkerUnavailable)
        );
        assert_eq!(tools_feedback.safe_message(), TOOLS_ACTION_START_ERROR);
        assert!(tools_feedback.diagnostic().is_some());

        let about_error = WorkerSpawnError::NoActiveRuntime {
            task_id: workers::WorkerTaskId::new("about-no-runtime"),
        };
        let about_feedback = about_spawn_failed_feedback(
            AboutActionId::CopyNexus.as_str(),
            about_action_for_id(AboutActionId::CopyNexus.as_str()).ok(),
            about_error,
        );

        assert_eq!(
            about_feedback.outcome,
            crate::services::tools::ActionOutcome::Rejected(ActionRejectionKind::WorkerUnavailable)
        );
        assert_eq!(about_feedback.safe_message(), ABOUT_ACTION_START_ERROR);
        assert!(about_feedback.diagnostic().is_some());
    }

    #[test]
    fn s05_runtime_wiring_worker_payloads_apply_and_unrelated_payloads_are_ignored() {
        let mut tools = ToolsController::new();
        let tools_event = WorkerEvent::completed(
            tools_action_worker_task(ToolActionId::BethiniPie.as_str()),
            WorkerPayload::ToolsAction(ToolsActionWorkerPayload::action_completed(
                ToolsActionFeedback::failed(
                    ToolActionId::BethiniPie.as_str(),
                    ToolsActionKind::ExternalLink(ToolActionId::BethiniPie),
                    crate::platform::PlatformOperation::OpenUrl,
                    crate::platform::PlatformErrorKind::UnsupportedPlatform,
                    "URL open is not supported on this platform.",
                    Some("raw desktop diagnostic".to_owned()),
                ),
            )),
        );

        assert_eq!(
            handle_tools_worker_event(&mut tools, tools_event),
            ToolsTransitionResult::Applied
        );
        assert_eq!(
            project_tools_state(tools.state()).last_action_error,
            "URL open is not supported on this platform."
        );

        let about_success = AboutActionFeedback::succeeded(
            AboutActionId::CopyGithub.as_str(),
            AboutActionKind::Copy {
                link_id: AboutLinkId::Github,
                action_id: AboutActionId::CopyGithub,
            },
            "Copied to clipboard.",
        );
        let unrelated_for_tools = WorkerEvent::completed(
            about_action_worker_task(AboutActionId::CopyGithub.as_str()),
            WorkerPayload::AboutAction(AboutActionWorkerPayload::action_completed(
                about_success.clone(),
            )),
        );

        assert_eq!(
            handle_tools_worker_event(&mut tools, unrelated_for_tools),
            ToolsTransitionResult::Ignored
        );

        let mut about = AboutController::new();
        let about_event = WorkerEvent::completed(
            about_action_worker_task(AboutActionId::CopyGithub.as_str()),
            WorkerPayload::AboutAction(AboutActionWorkerPayload::action_completed(about_success)),
        );

        assert_eq!(
            handle_about_worker_event(&mut about, about_event),
            AboutTransitionResult::Applied
        );
        let about_projection = project_about_state(about.state());
        assert_eq!(about_projection.github_copy_label, ABOUT_COPY_SUCCESS_LABEL);
        assert!(!about_projection.github_copy_enabled);

        let unrelated_for_about = WorkerEvent::completed(
            tools_action_worker_task(ToolActionId::BethiniPie.as_str()),
            WorkerPayload::ToolsAction(ToolsActionWorkerPayload::action_completed(
                ToolsActionFeedback::succeeded(
                    ToolActionId::BethiniPie.as_str(),
                    ToolsActionKind::ExternalLink(ToolActionId::BethiniPie),
                    "Opened URL.",
                ),
            )),
        );

        assert_eq!(
            handle_about_worker_event(&mut about, unrelated_for_about),
            AboutTransitionResult::Ignored
        );
    }

    #[test]
    fn s05_runtime_wiring_worker_failure_payloads_map_by_surface_task_prefix() {
        let mut tools = ToolsController::new();
        let tools_failure = WorkerEvent::failed(
            tools_action_worker_task(ToolActionId::BethiniPie.as_str()),
            WorkerFailure::new("Worker task panicked.").with_diagnostic("panic payload"),
        );

        assert_eq!(
            handle_tools_worker_event(&mut tools, tools_failure),
            ToolsTransitionResult::Applied
        );
        assert_eq!(
            project_tools_state(tools.state()).last_action_error,
            "Worker task panicked."
        );

        let mut about = AboutController::new();
        let about_failure = WorkerEvent::failed(
            about_action_worker_task(AboutActionId::OpenNexus.as_str()),
            WorkerFailure::new("Worker task panicked.").with_diagnostic("panic payload"),
        );

        assert_eq!(
            handle_about_worker_event(&mut about, about_failure),
            AboutTransitionResult::Applied
        );
        assert_eq!(
            project_about_state(about.state()).last_action_error,
            "Worker task panicked."
        );

        let unrelated_failure = WorkerEvent::failed(
            WorkerTask::new("other-surface", WorkerTaskKind::Generic),
            WorkerFailure::new("Generic worker failed."),
        );
        assert_eq!(
            handle_about_worker_event(&mut about, unrelated_failure),
            AboutTransitionResult::Ignored
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn s05_runtime_wiring_non_windows_real_desktop_actions_fail_safely() {
        use crate::platform::{PlatformErrorKind, desktop::DesktopActions};

        let result = RealDesktopActions::new().open_url("https://example.invalid/cmt");

        assert_eq!(
            result.failure_kind(),
            Some(PlatformErrorKind::UnsupportedPlatform)
        );
        assert_eq!(
            result.safe_message(),
            "URL open is not supported on this platform."
        );
    }

    #[test]
    fn overview_tab_exposes_refresh_action_status_panels_and_safe_error_state() {
        assert_source_contains_in_order(
            OVERVIEW_SLINT,
            &[
                "export struct OverviewUiRow",
                "in-out property <[OverviewUiRow]> overview-top-rows",
                "in-out property <[OverviewUiRow]> overview-binary-rows",
                "in-out property <[OverviewUiRow]> overview-archive-rows",
                "in-out property <[OverviewUiRow]> overview-module-rows",
                "in-out property <[OverviewUiRow]> overview-problem-rows",
                "callback refresh-requested()",
                "callback open-game-path-requested()",
                "callback open-nexus-update-requested()",
                "callback open-github-update-requested()",
                "callback open-downgrade-manager-requested()",
                "callback open-archive-patcher-requested()",
                "text: \"Refresh\"",
                "text: \"Open Game Path\"",
                "title: \"Status\"",
                "title: \"Binaries (EXE/DLL/BIN)\"",
                "title: \"Archives (BA2)\"",
                "title: \"Modules (ESM/ESL/ESP)\"",
                "title: \"Problems\"",
            ],
        );
        assert!(OVERVIEW_SLINT.contains("overview-last-action-error"));
        assert!(OVERVIEW_SLINT.contains("overview-refresh-busy"));
        assert!(OVERVIEW_SLINT.contains("overview-downgrade-enabled: true"));
        assert!(OVERVIEW_SLINT.contains("overview-archive-patcher-enabled: false"));
        assert!(OVERVIEW_SLINT.contains("Open Downgrade Manager."));
        assert!(OVERVIEW_SLINT.contains("Deferred until the Archive Patcher workflow is ported."));
        assert!(!OVERVIEW_SLINT.contains("Overview behavior is reserved for a later port phase."));
    }

    #[test]
    fn overview_projection_rows_lock_reference_order_and_deferred_status() {
        let snapshot = OverviewSnapshot::empty();

        let top_labels = format_top_rows(&snapshot)
            .into_iter()
            .map(|row| row.label.to_string())
            .collect::<Vec<_>>();
        assert_eq!(top_labels, domain::overview::TOP_STATUS_LABELS.to_vec());

        let archive_labels = format_count_panel_rows(&snapshot.archives.rows)
            .into_iter()
            .map(|row| row.label.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            archive_labels,
            domain::overview::ARCHIVE_COUNT_LABELS.to_vec()
        );

        let module_labels = format_count_panel_rows(&snapshot.modules.rows)
            .into_iter()
            .map(|row| row.label.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            module_labels,
            domain::overview::MODULE_COUNT_LABELS.to_vec()
        );

        let binary_rows = format_binary_rows(&snapshot);
        assert!(
            binary_rows
                .iter()
                .any(|row| row.label.as_str() == "Address Library")
        );
        assert_eq!(
            deferred_action_label(
                &snapshot.binaries.actions,
                OverviewDeferredActionKind::OpenDowngradeManager,
                ACTION_DOWNGRADE_MANAGER_LABEL,
            ),
            "Downgrade Manager..."
        );
        assert_eq!(
            deferred_action_label(
                &snapshot.archives.actions,
                OverviewDeferredActionKind::OpenArchivePatcher,
                ACTION_ARCHIVE_PATCHER_LABEL,
            ),
            "Archive Patcher..."
        );
        assert_eq!(
            deferred_action_status(
                &snapshot.binaries.actions,
                OverviewDeferredActionKind::OpenDowngradeManager,
                "Downgrade Manager",
            ),
            "Deferred until the Downgrade Manager workflow is ported."
        );
        assert_eq!(format_problem_summary(&snapshot), "Problems: 0");
        assert_eq!(
            format_problem_rows(&snapshot)[0].detail.as_str(),
            "No problems detected."
        );
    }

    #[test]
    fn settings_tab_labels_are_exact_and_in_display_order() {
        let group_titles = slint_string_property_values(SETTINGS_SLINT, "title");
        assert_eq!(
            group_titles.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["Update Channel", "Log Level"]
        );

        let option_labels = slint_string_property_values(SETTINGS_SLINT, "text");
        assert_eq!(
            option_labels.iter().map(String::as_str).collect::<Vec<_>>(),
            vec![
                "All: GitHub & Nexus Mods",
                "Early: GitHub",
                "Stable: Nexus Mods",
                "Never: Don't Check",
                "Debug",
                "Info",
                "Warning",
                "Error",
            ]
        );
        assert_eq!(SETTINGS_SLINT.matches("SettingsRadioOption {").count(), 8);
    }

    #[test]
    fn settings_tab_update_channel_labels() {
        assert_source_contains_in_order(
            SETTINGS_SLINT,
            &[
                "title: \"Update Channel\"",
                "text: \"All: GitHub & Nexus Mods\"",
                "root.update-source = \"both\"",
                "root.update-source-selected(\"both\")",
                "text: \"Early: GitHub\"",
                "root.update-source = \"github\"",
                "root.update-source-selected(\"github\")",
                "text: \"Stable: Nexus Mods\"",
                "root.update-source = \"nexus\"",
                "root.update-source-selected(\"nexus\")",
                "text: \"Never: Don't Check\"",
                "root.update-source = \"none\"",
                "root.update-source-selected(\"none\")",
            ],
        );

        assert!(SETTINGS_SLINT.contains("in-out property <string> update-source"));
        assert!(SETTINGS_SLINT.contains("callback update-source-selected(string)"));
    }

    #[test]
    fn settings_tab_log_level_labels() {
        assert_source_contains_in_order(
            SETTINGS_SLINT,
            &[
                "title: \"Log Level\"",
                "text: \"Debug\"",
                "root.log-level = \"debug\"",
                "root.log-level-selected(\"debug\")",
                "text: \"Info\"",
                "root.log-level = \"info\"",
                "root.log-level-selected(\"info\")",
                "text: \"Warning\"",
                "root.log-level = \"warning\"",
                "root.log-level-selected(\"warning\")",
                "text: \"Error\"",
                "root.log-level = \"error\"",
                "root.log-level-selected(\"error\")",
            ],
        );

        assert!(SETTINGS_SLINT.contains("in-out property <string> log-level"));
        assert!(SETTINGS_SLINT.contains("callback log-level-selected(string)"));
    }

    #[test]
    fn settings_tab_uses_dark_mode_palette() {
        assert!(SETTINGS_SLINT.contains("background: #202020;"));
        assert!(SETTINGS_SLINT.contains("color: #f3f3f3;"));
        assert!(!SETTINGS_SLINT.contains("background: #f3f3f3;"));
    }

    #[test]
    fn main_window_forwards_settings_and_overview_tab_api() {
        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "in-out property <string> update-source",
                "in-out property <string> log-level",
                "in-out property <string> overview-refresh-message",
                "in-out property <[OverviewUiRow]> overview-top-rows",
                "in-out property <[OverviewUiRow]> overview-binary-rows",
                "in-out property <[OverviewUiRow]> overview-archive-rows",
                "in-out property <[OverviewUiRow]> overview-module-rows",
                "in-out property <[OverviewUiRow]> overview-problem-rows",
                "callback update-source-selected(string)",
                "callback log-level-selected(string)",
                "callback overview-refresh-requested()",
                "callback overview-open-downgrade-manager-requested()",
                "callback overview-open-archive-patcher-requested()",
                "SettingsTab {",
                "update-source <=> root.update-source",
                "log-level <=> root.log-level",
                "root.update-source-selected(value)",
                "root.log-level-selected(value)",
            ],
        );
        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "OverviewTab {",
                "overview-refresh-message <=> root.overview-refresh-message",
                "overview-top-rows <=> root.overview-top-rows",
                "overview-binary-rows <=> root.overview-binary-rows",
                "overview-archive-rows <=> root.overview-archive-rows",
                "overview-module-rows <=> root.overview-module-rows",
                "overview-problem-rows <=> root.overview-problem-rows",
                "overview-downgrade-enabled <=> root.overview-downgrade-enabled",
                "overview-archive-patcher-enabled <=> root.overview-archive-patcher-enabled",
                "overview-last-action-error <=> root.overview-last-action-error",
                "root.overview-refresh-requested()",
                "root.overview-open-game-path-requested()",
                "root.overview-open-nexus-update-requested()",
                "root.overview-open-github-update-requested()",
                "root.overview-open-downgrade-manager-requested()",
                "root.overview-open-archive-patcher-requested()",
            ],
        );
    }

    #[test]
    fn shell_contract_boundary_markers_construct_as_no_ops() {
        let _controller = ShellController;
        let _domain = DomainState;
        let _platform = PlatformServices;
        let _services = ServiceLayer;
        let _workers = WorkerRuntime;
    }
}
