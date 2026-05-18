pub mod app;
pub mod domain;
pub mod platform;
pub mod services;
pub mod workers;

use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use app::{
    about_controller::{AboutController, AboutState, AboutTransitionResult},
    f4se_controller::{
        F4seController, F4seScanWorkerRequest, F4seTransitionResult, f4se_scan_completed_payload,
    },
    overview_controller::{
        OverviewController, action_target_label, overview_desktop_action_payload,
        overview_desktop_task, unavailable_action_error,
    },
    settings_controller::SettingsController,
    tools_controller::{
        TOOLS_DEFAULT_DISABLED_UTILITY_STATUS, ToolsController, ToolsState, ToolsTransitionResult,
    },
};
use domain::{
    discovery::{FALLOUT4_EXECUTABLE, Fallout4InstallType},
    f4se::{F4seDllRow, F4seGameTarget, F4seRowSeverity, F4seScanSnapshot, F4seScanStatus},
    overview::{
        ACTION_ARCHIVE_PATCHER_LABEL, ACTION_DOWNGRADE_MANAGER_LABEL, BinaryStatusRow,
        OverviewActionError, OverviewCountRow, OverviewDeferredAction, OverviewDeferredActionKind,
        OverviewDeferredActionTarget, OverviewProblem, OverviewRefreshState, OverviewSnapshot,
        OverviewTopStatusRow, StatusSeverity, UpdateBannerState, UpdateCheckFailure,
        UpdateProvider,
    },
    settings::{AppSettings, UpdateSource},
    tools::{ABOUT_LINKS, AboutLinkId},
};
use platform::{
    clipboard::RealClipboardActions,
    desktop::RealDesktopActions,
    filesystem::RealFilesystem,
    process::RealProcessInspector,
    registry::RealRegistry,
    settings_store::{FileAssetResolver, SettingsStore},
};
use services::{
    discovery::{DiscoveryRequest, DiscoveryService},
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
    tools::{
        AboutActionFeedback, AboutActionKind, ActionRejectionKind, ToolsActionFeedback,
        ToolsActionKind, ToolsActionService, about_action_for_id, tools_action_for_id,
    },
    update::{OverviewLinkService, RealUpdateCheckClient, UpdateCheckService},
};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use workers::{
    AboutActionWorkerPayload, BlockingWorkerResult, SlintEventLoopSink, ToolsActionWorkerPayload,
    WorkerEvent, WorkerEventSink, WorkerFailure, WorkerPayload, WorkerRuntime, WorkerSpawnError,
    WorkerTask, WorkerTaskKind, WorkerTaskOutcome,
};

slint::include_modules!();

const TOOLS_WORKER_TASK_PREFIX: &str = "s05-tools-action:";
const ABOUT_WORKER_TASK_PREFIX: &str = "s05-about-action:";
const TOOLS_ACTION_START_ERROR: &str = "Tools action could not be started.";
const ABOUT_ACTION_START_ERROR: &str = "About action could not be started.";

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("cmt-rs-worker")
        .build()?;
    let runtime_handle = tokio_runtime.handle().clone();
    let _runtime_guard = tokio_runtime.enter();

    let app = MainWindow::new()?;
    let settings_controller = Rc::new(RefCell::new(load_settings_controller()));
    let overview_controller = Arc::new(Mutex::new(OverviewController::new()));
    let f4se_controller = Arc::new(Mutex::new(F4seController::new()));
    let tools_controller = Arc::new(Mutex::new(ToolsController::new()));
    let about_controller = Arc::new(Mutex::new(AboutController::new()));
    let worker_runtime = WorkerRuntime::new();

    app.set_update_source(settings_controller.borrow().visible_update_source().into());
    app.set_log_level(settings_controller.borrow().visible_log_level().into());
    apply_current_overview_snapshot(&app, &overview_controller);
    apply_current_f4se_snapshot(&app, &f4se_controller);
    apply_current_tools_state(&app, &tools_controller);
    apply_current_about_state(&app, &about_controller);

    let overview_sink = bind_overview_worker_sink(&app, Arc::clone(&overview_controller));
    let f4se_sink = bind_f4se_worker_sink(&app, Arc::clone(&f4se_controller));
    let tools_sink = bind_tools_worker_sink(&app, Arc::clone(&tools_controller));
    let about_sink = bind_about_worker_sink(&app, Arc::clone(&about_controller));
    bind_settings_callbacks(&app, Rc::clone(&settings_controller));
    bind_tools_callbacks(
        &app,
        Arc::clone(&tools_controller),
        worker_runtime,
        tools_sink.clone(),
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
    bind_overview_callbacks(
        &app,
        Arc::clone(&overview_controller),
        Rc::clone(&settings_controller),
        worker_runtime,
        overview_sink.clone(),
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
) {
    app.on_update_source_selected({
        let app = app.as_weak();
        let controller = Rc::clone(&controller);

        move |selected| {
            let visible_value = controller
                .borrow_mut()
                .select_update_source(selected.as_str());
            if let Some(app) = app.upgrade() {
                app.set_update_source(visible_value.into());
            }
        }
    });

    app.on_log_level_selected({
        let app = app.as_weak();
        let controller = Rc::clone(&controller);

        move |selected| {
            let visible_value = controller.borrow_mut().select_log_level(selected.as_str());
            if let Some(app) = app.upgrade() {
                app.set_log_level(visible_value.into());
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
    controller: Arc<Mutex<ToolsController>>,
    worker_runtime: WorkerRuntime,
    tools_sink: SlintEventLoopSink,
) {
    app.on_tool_action_requested({
        let app = app.as_weak();
        let controller = Arc::clone(&controller);

        move |action_id| {
            if let Some(app) = app.upgrade() {
                request_tools_action(
                    &app,
                    &controller,
                    worker_runtime,
                    tools_sink.clone(),
                    action_id.to_string(),
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
    overview_controller: Arc<Mutex<OverviewController>>,
    settings_controller: Rc<RefCell<SettingsController<FileAssetResolver>>>,
    worker_runtime: WorkerRuntime,
    overview_sink: SlintEventLoopSink,
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
        let app = app.as_weak();
        let overview_controller = Arc::clone(&overview_controller);

        move || {
            if let Some(app) = app.upgrade() {
                apply_action_error(
                    &app,
                    &overview_controller,
                    OverviewDeferredActionKind::OpenDowngradeManager,
                    "Downgrade Manager is reserved for a later port phase.".to_owned(),
                );
            }
        }
    });

    app.on_overview_open_archive_patcher_requested({
        let app = app.as_weak();
        let overview_controller = Arc::clone(&overview_controller);

        move || {
            if let Some(app) = app.upgrade() {
                apply_action_error(
                    &app,
                    &overview_controller,
                    OverviewDeferredActionKind::OpenArchivePatcher,
                    "Archive Patcher is reserved for a later port phase.".to_owned(),
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

fn request_tools_action(
    app: &MainWindow,
    controller: &Arc<Mutex<ToolsController>>,
    worker_runtime: WorkerRuntime,
    tools_sink: SlintEventLoopSink,
    action_id: String,
) {
    let action = match tools_action_for_id(&action_id) {
        Ok(action @ ToolsActionKind::ExternalLink(_)) => action,
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

fn request_overview_refresh(
    app: &MainWindow,
    overview_controller: &Arc<Mutex<OverviewController>>,
    settings_controller: &Rc<RefCell<SettingsController<FileAssetResolver>>>,
    worker_runtime: WorkerRuntime,
    overview_sink: SlintEventLoopSink,
    runtime_handle: tokio::runtime::Handle,
) {
    let settings = settings_controller.borrow().current_settings().clone();
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
    app.set_overview_downgrade_enabled(false);
    app.set_overview_downgrade_status(
        deferred_action_status(
            &snapshot.binaries.actions,
            OverviewDeferredActionKind::OpenDowngradeManager,
            "Downgrade Manager",
        )
        .into(),
    );
    app.set_overview_archive_patcher_label(
        deferred_action_label(
            &snapshot.archives.actions,
            OverviewDeferredActionKind::OpenArchivePatcher,
            ACTION_ARCHIVE_PATCHER_LABEL,
        )
        .into(),
    );
    app.set_overview_archive_patcher_enabled(false);
    app.set_overview_archive_patcher_status(
        deferred_action_status(
            &snapshot.archives.actions,
            OverviewDeferredActionKind::OpenArchivePatcher,
            "Archive Patcher",
        )
        .into(),
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
        workers::WorkerRuntime,
    };
    use crate::domain::{
        f4se::{
            F4SE_HEADING, F4SE_LEGEND_TEXT, F4SE_LOADING_TEXT, F4SE_TABLE_COLUMNS, F4seDllFacts,
            render_f4se_dll_row,
        },
        tools::{
            ABOUT_COPY_INVITE_LABEL, ABOUT_COPY_LINK_LABEL, ABOUT_COPY_SUCCESS_LABEL,
            ABOUT_CREDIT_LABEL, ABOUT_LINKS, ABOUT_TITLE_LABEL, AboutActionId, AboutLinkId,
            IMAGE_RESOURCE_PATHS, TOOL_GROUPS, ToolActionId,
        },
    };

    const MAIN_SLINT: &str = include_str!("../ui/main.slint");
    const SETTINGS_SLINT: &str = include_str!("../ui/settings_tab.slint");
    const OVERVIEW_SLINT: &str = include_str!("../ui/overview_tab.slint");
    const F4SE_SLINT: &str = include_str!("../ui/f4se_tab.slint");
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
            include_str!("../ui/scanner_tab.slint"),
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
    const INERT_TAB_COMPONENTS: [(&str, &str, &str, &str); 1] = [TAB_COMPONENTS[2]];

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
    fn s05_slint_contract_tools_tab_replaces_placeholder_with_reference_groups() {
        assert!(TOOLS_SLINT.contains("export component ToolsTab"));
        assert!(TOOLS_SLINT.contains("background: #202020;"));
        assert!(TOOLS_SLINT.contains("in-out property <string> tools-last-action-error"));
        assert!(TOOLS_SLINT.contains("in-out property <string> tools-disabled-utility-status"));
        assert!(TOOLS_SLINT.contains("callback tool-action-requested(string)"));
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
                "button-enabled: false;",
                "Deferred until S09 Downgrade Manager workflow is ported.",
                "label: \"Archive Patcher\"",
                "action-id: \"tools.archive_patcher\"",
                "button-enabled: false;",
                "Deferred until S10 Archive Patcher workflow is ported.",
            ],
        );
        assert_eq!(TOOLS_SLINT.matches("button-enabled: false;").count(), 2);
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

        let tools_deferred = tools_action_for_id(ToolActionId::DowngradeManager.as_str())
            .expect_err("deferred Tools utilities should fail closed");
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
        assert!(OVERVIEW_SLINT.contains("overview-downgrade-enabled: false"));
        assert!(OVERVIEW_SLINT.contains("overview-archive-patcher-enabled: false"));
        assert!(
            OVERVIEW_SLINT.contains("Deferred until the Downgrade Manager workflow is ported.")
        );
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
