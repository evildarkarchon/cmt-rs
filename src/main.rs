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
    overview_controller::{
        OverviewController, action_target_label, overview_desktop_action_payload,
        overview_desktop_task, unavailable_action_error,
    },
    settings_controller::SettingsController,
};
use domain::{
    overview::{
        ACTION_ARCHIVE_PATCHER_LABEL, ACTION_DOWNGRADE_MANAGER_LABEL, BinaryStatusRow,
        OverviewActionError, OverviewCountRow, OverviewDeferredAction, OverviewDeferredActionKind,
        OverviewDeferredActionTarget, OverviewProblem, OverviewRefreshState, OverviewSnapshot,
        OverviewTopStatusRow, StatusSeverity, UpdateBannerState, UpdateCheckFailure,
        UpdateProvider,
    },
    settings::{AppSettings, UpdateSource},
};
use platform::{
    desktop::RealDesktopActions,
    filesystem::RealFilesystem,
    process::RealProcessInspector,
    registry::RealRegistry,
    settings_store::{FileAssetResolver, SettingsStore},
};
use services::{
    discovery::{DiscoveryRequest, DiscoveryService},
    overview::{
        OverviewDesktopActionFeedback, OverviewDesktopActionOutcome, OverviewDiagnostics,
        OverviewDiagnosticsInput, OverviewUpdateCheckState,
    },
    overview_collector::{
        OverviewCollectedFacts, OverviewCollectionEnvironment, OverviewCollectionRequest,
        OverviewCollector,
    },
    update::{OverviewLinkService, RealUpdateCheckClient, UpdateCheckService},
};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use workers::{
    BlockingWorkerResult, SlintEventLoopSink, WorkerEvent, WorkerEventSink, WorkerFailure,
    WorkerPayload, WorkerRuntime, WorkerTaskOutcome,
};

slint::include_modules!();

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
    let worker_runtime = WorkerRuntime::new();

    app.set_update_source(settings_controller.borrow().visible_update_source().into());
    app.set_log_level(settings_controller.borrow().visible_log_level().into());
    apply_current_overview_snapshot(&app, &overview_controller);

    let overview_sink = bind_overview_worker_sink(&app, Arc::clone(&overview_controller));
    bind_settings_callbacks(&app, Rc::clone(&settings_controller));
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

    const MAIN_SLINT: &str = include_str!("../ui/main.slint");
    const SETTINGS_SLINT: &str = include_str!("../ui/settings_tab.slint");
    const OVERVIEW_SLINT: &str = include_str!("../ui/overview_tab.slint");
    const TAB_COMPONENTS: [(&str, &str, &str, &str); 6] = [
        (
            "ui/overview_tab.slint",
            "OverviewTab",
            "Overview",
            OVERVIEW_SLINT,
        ),
        (
            "ui/f4se_tab.slint",
            "F4seTab",
            "F4SE",
            include_str!("../ui/f4se_tab.slint"),
        ),
        (
            "ui/scanner_tab.slint",
            "ScannerTab",
            "Scanner",
            include_str!("../ui/scanner_tab.slint"),
        ),
        (
            "ui/tools_tab.slint",
            "ToolsTab",
            "Tools",
            include_str!("../ui/tools_tab.slint"),
        ),
        (
            "ui/settings_tab.slint",
            "SettingsTab",
            "Settings",
            SETTINGS_SLINT,
        ),
        (
            "ui/about_tab.slint",
            "AboutTab",
            "About",
            include_str!("../ui/about_tab.slint"),
        ),
    ];
    const INERT_TAB_COMPONENTS: [(&str, &str, &str, &str); 4] = [
        TAB_COMPONENTS[1],
        TAB_COMPONENTS[2],
        TAB_COMPONENTS[3],
        TAB_COMPONENTS[5],
    ];

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
