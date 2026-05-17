//! Worker orchestration boundary for long-running CMT tasks.
//!
//! Scan, patch, download, discovery, and subprocess orchestration should enter
//! background work through this module so slow operations stay off the Slint UI
//! thread and report owned typed events through handoff sinks.

use std::{
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
};

use tokio::task::JoinHandle;

pub mod events;
pub mod handoff;

pub use events::{
    CancellationToken, ExternalActionKind, ExternalActionOutcome, ExternalActionPayload,
    OverviewWorkerPayload, WorkerCancellation, WorkerEvent, WorkerFailure, WorkerMessage,
    WorkerPayload, WorkerProgress, WorkerTask, WorkerTaskId, WorkerTaskKind, WorkerTaskStatus,
};
pub use handoff::{
    RecordingEventSink, SlintEventLoopSink, WorkerEventSink, WorkerHandoffError,
    WorkerHandoffErrorKind, WorkerHandoffResult,
};

/// Result returned by blocking worker closures.
pub type BlockingWorkerResult = Result<WorkerTaskOutcome, WorkerFailure>;

/// Final result state returned by a blocking worker closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerTaskOutcome {
    /// Worker completed successfully and produced a typed payload.
    Completed(WorkerPayload),
    /// Worker observed cancellation and stopped cleanly.
    Cancelled(WorkerCancellation),
}

/// Explicit facade for scheduling blocking work off the UI thread.
///
/// The facade does not own a Tokio runtime. Callers must construct or enter a
/// runtime before scheduling work; otherwise [`WorkerSpawnError::NoActiveRuntime`]
/// is returned instead of panicking.
#[derive(Debug, Default, Clone, Copy)]
pub struct WorkerRuntime;

impl WorkerRuntime {
    /// Creates the worker facade without spawning any work.
    pub const fn new() -> Self {
        Self
    }

    /// Schedules blocking work on Tokio's blocking thread pool and emits typed lifecycle events.
    pub fn spawn_blocking_task<S, F>(
        &self,
        task: WorkerTask,
        sink: S,
        work: F,
    ) -> Result<WorkerTaskHandle<S>, WorkerSpawnError>
    where
        S: WorkerEventSink,
        F: FnOnce(WorkerTaskContext<S>) -> BlockingWorkerResult + Send + 'static,
    {
        let runtime_handle = tokio::runtime::Handle::try_current().map_err(|error| {
            tracing::error!(
                event = "worker-spawn-failed",
                task_id = %task.id,
                task_kind = task.kind.label(),
                reason = "no-active-tokio-runtime",
                diagnostic = %error,
                "Worker task could not be scheduled because no Tokio runtime is active"
            );
            WorkerSpawnError::NoActiveRuntime {
                task_id: task.id.clone(),
            }
        })?;

        tracing::info!(
            event = "worker-spawn-scheduled",
            task_id = %task.id,
            task_kind = task.kind.label(),
            "Blocking worker task scheduled"
        );

        let cancellation = CancellationToken::new();
        let worker_task = task.clone();
        let worker_sink = sink.clone();
        let worker_cancellation = cancellation.clone();
        let join = runtime_handle.spawn_blocking(move || {
            let context = WorkerTaskContext::new(
                worker_task.clone(),
                worker_sink.clone(),
                worker_cancellation,
            );
            emit_or_log(&worker_sink, WorkerEvent::running(worker_task.clone()));

            match catch_unwind(AssertUnwindSafe(|| work(context))) {
                Ok(Ok(WorkerTaskOutcome::Completed(payload))) => {
                    tracing::info!(
                        event = "worker-completed",
                        task_id = %worker_task.id,
                        task_kind = worker_task.kind.label(),
                        status = WorkerTaskStatus::Completed.label(),
                        "Blocking worker task completed"
                    );
                    emit_or_log(&worker_sink, WorkerEvent::completed(worker_task, payload));
                }
                Ok(Ok(WorkerTaskOutcome::Cancelled(cancellation))) => {
                    tracing::info!(
                        event = "worker-cancelled",
                        task_id = %worker_task.id,
                        task_kind = worker_task.kind.label(),
                        status = WorkerTaskStatus::Cancelled.label(),
                        "Blocking worker task ended after cancellation"
                    );
                    emit_or_log(
                        &worker_sink,
                        WorkerEvent::cancelled(worker_task, cancellation),
                    );
                }
                Ok(Err(failure)) => {
                    tracing::error!(
                        event = "worker-failed",
                        task_id = %worker_task.id,
                        task_kind = worker_task.kind.label(),
                        status = WorkerTaskStatus::Failed.label(),
                        safe_message = failure.safe_message(),
                        diagnostic = failure.diagnostic().unwrap_or(""),
                        "Blocking worker task failed"
                    );
                    emit_or_log(&worker_sink, WorkerEvent::failed(worker_task, failure));
                }
                Err(payload) => {
                    let diagnostic = panic_payload_message(payload.as_ref());
                    tracing::error!(
                        event = "worker-panicked",
                        task_id = %worker_task.id,
                        task_kind = worker_task.kind.label(),
                        status = WorkerTaskStatus::Failed.label(),
                        diagnostic = diagnostic.as_deref().unwrap_or("panic payload unavailable"),
                        "Blocking worker task panicked"
                    );
                    let failure = WorkerFailure::new("Worker task panicked.");
                    let failure = if let Some(diagnostic) = diagnostic {
                        failure.with_diagnostic(diagnostic)
                    } else {
                        failure
                    };
                    emit_or_log(&worker_sink, WorkerEvent::failed(worker_task, failure));
                }
            }
        });

        Ok(WorkerTaskHandle {
            task,
            cancellation,
            sink,
            join,
        })
    }
}

/// Error returned when a worker task cannot be scheduled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerSpawnError {
    /// No Tokio runtime was active for the current thread.
    NoActiveRuntime {
        /// Task id that could not be scheduled.
        task_id: WorkerTaskId,
    },
}

impl fmt::Display for WorkerSpawnError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoActiveRuntime { task_id } => write!(
                formatter,
                "Worker task {task_id} could not be scheduled because no Tokio runtime is active."
            ),
        }
    }
}

impl std::error::Error for WorkerSpawnError {}

/// Context passed into blocking worker closures.
#[derive(Clone)]
pub struct WorkerTaskContext<S>
where
    S: WorkerEventSink,
{
    task: WorkerTask,
    sink: S,
    cancellation: CancellationToken,
}

impl<S> WorkerTaskContext<S>
where
    S: WorkerEventSink,
{
    fn new(task: WorkerTask, sink: S, cancellation: CancellationToken) -> Self {
        Self {
            task,
            sink,
            cancellation,
        }
    }

    /// Returns the metadata for the running worker task.
    pub fn task(&self) -> &WorkerTask {
        &self.task
    }

    /// Returns a clone of the task cancellation token.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    /// Returns true when cancellation has been requested.
    pub fn is_cancellation_requested(&self) -> bool {
        self.cancellation.is_cancellation_requested()
    }

    /// Emits a custom typed payload for this task.
    pub fn emit_payload(
        &self,
        status: WorkerTaskStatus,
        payload: WorkerPayload,
    ) -> WorkerHandoffResult {
        self.sink
            .emit(WorkerEvent::new(self.task.clone(), status, payload))
    }

    /// Emits progress with optional text and counts for this task.
    pub fn emit_progress(&self, progress: WorkerProgress) -> WorkerHandoffResult {
        self.sink
            .emit(WorkerEvent::progress(self.task.clone(), progress))
    }

    /// Emits a cancellation acknowledgement after the worker observes the token.
    pub fn acknowledge_cancellation(
        &self,
        cancellation: WorkerCancellation,
    ) -> WorkerHandoffResult {
        self.sink.emit(WorkerEvent::cancellation_acknowledged(
            self.task.clone(),
            cancellation,
        ))
    }
}

/// Handle returned for a scheduled blocking worker task.
pub struct WorkerTaskHandle<S>
where
    S: WorkerEventSink,
{
    task: WorkerTask,
    cancellation: CancellationToken,
    sink: S,
    join: JoinHandle<()>,
}

impl<S> WorkerTaskHandle<S>
where
    S: WorkerEventSink,
{
    /// Returns metadata for the scheduled task.
    pub fn task(&self) -> &WorkerTask {
        &self.task
    }

    /// Requests cancellation and emits a cancellation-requested event.
    pub fn request_cancellation(&self, cancellation: WorkerCancellation) -> WorkerHandoffResult {
        self.cancellation.request_cancellation();
        self.sink.emit(WorkerEvent::cancellation_requested(
            self.task.clone(),
            cancellation,
        ))
    }

    /// Returns a clone of the cancellation token.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    /// Awaits the underlying worker join handle.
    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.join.await
    }
}

fn emit_or_log<S>(sink: &S, event: WorkerEvent)
where
    S: WorkerEventSink,
{
    let task_id = event.task.id.to_string();
    let task_kind = event.task.kind.label();
    let status = event.status.label();

    if let Err(error) = sink.emit(event) {
        tracing::warn!(
            event = "worker-event-handoff-failed",
            task_id = %task_id,
            task_kind,
            status,
            error = %error,
            diagnostic = error.diagnostic.as_deref().unwrap_or(""),
            "Worker event could not be delivered to its sink"
        );
    }
}

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> Option<String> {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        thread::ThreadId,
        time::Duration,
    };

    use super::*;
    use crate::workers::events::{WorkerMessage, WorkerTaskKind};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn blocking_facade_runs_work_off_calling_thread_and_records_events() {
        let runtime = WorkerRuntime::new();
        let sink = RecordingEventSink::new();
        let calling_thread = std::thread::current().id();
        let worker_thread: Arc<Mutex<Option<ThreadId>>> = Arc::new(Mutex::new(None));
        let worker_thread_for_task = Arc::clone(&worker_thread);
        let task = WorkerTask::new("scan-data", WorkerTaskKind::Scan);

        let handle = runtime
            .spawn_blocking_task(task, sink.clone(), move |context| {
                *worker_thread_for_task
                    .lock()
                    .expect("worker thread id should be writable") =
                    Some(std::thread::current().id());
                context
                    .emit_progress(
                        WorkerProgress::new()
                            .with_message("Scanning Data folder")
                            .with_counts(Some(1), Some(2)),
                    )
                    .expect("progress should be recorded");
                Ok(WorkerTaskOutcome::Completed(WorkerPayload::Scan(
                    WorkerMessage::new("Scan complete."),
                )))
            })
            .expect("active Tokio runtime should schedule blocking work");

        handle.join().await.expect("worker task should join");

        let actual_worker_thread = worker_thread
            .lock()
            .expect("worker thread id should be readable")
            .expect("worker thread id should be recorded");
        assert_ne!(actual_worker_thread, calling_thread);

        let events = sink.events().expect("recorded events should be readable");
        assert_eq!(
            events.first().map(|event| event.status),
            Some(WorkerTaskStatus::Running)
        );
        assert!(events.iter().any(|event| matches!(
            &event.payload,
            WorkerPayload::Progress(WorkerProgress {
                current: Some(1),
                total: Some(2),
                ..
            })
        )));
        assert_eq!(
            events.last().map(|event| event.status),
            Some(WorkerTaskStatus::Completed)
        );
        assert!(matches!(
            events.last().map(|event| &event.payload),
            Some(WorkerPayload::Scan(_))
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancellation_request_acknowledgement_and_final_cancelled_flow_through_facade() {
        let runtime = WorkerRuntime::new();
        let sink = RecordingEventSink::new();
        let task = WorkerTask::new("download-cancel", WorkerTaskKind::Download);

        let handle = runtime
            .spawn_blocking_task(task, sink.clone(), move |context| {
                while !context.is_cancellation_requested() {
                    std::thread::sleep(Duration::from_millis(1));
                }
                context
                    .acknowledge_cancellation(WorkerCancellation::with_message("Worker stopping."))
                    .expect("acknowledgement should be recorded");
                Ok(WorkerTaskOutcome::Cancelled(
                    WorkerCancellation::with_message("Download stopped."),
                ))
            })
            .expect("active Tokio runtime should schedule blocking work");

        handle
            .request_cancellation(WorkerCancellation::with_message("User requested stop."))
            .expect("cancellation request should be recorded");
        handle.join().await.expect("cancelled task should join");

        let statuses = sink
            .events()
            .expect("recorded events should be readable")
            .into_iter()
            .map(|event| event.status)
            .collect::<Vec<_>>();

        assert!(statuses.contains(&WorkerTaskStatus::CancellationRequested));
        assert!(statuses.contains(&WorkerTaskStatus::CancellationAcknowledged));
        assert!(statuses.contains(&WorkerTaskStatus::Cancelled));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn worker_failures_emit_safe_failure_events() {
        let runtime = WorkerRuntime::new();
        let sink = RecordingEventSink::new();
        let task = WorkerTask::new("patch-failure", WorkerTaskKind::Patch);

        let handle = runtime
            .spawn_blocking_task(task, sink.clone(), |_context| {
                Err(WorkerFailure::new("Patch failed.").with_diagnostic("raw diagnostic detail"))
            })
            .expect("active Tokio runtime should schedule blocking work");

        handle.join().await.expect("failed task should still join");

        let events = sink.events().expect("recorded events should be readable");
        let failed = events
            .iter()
            .find(|event| event.status == WorkerTaskStatus::Failed)
            .expect("failure event should be emitted");

        match &failed.payload {
            WorkerPayload::Error(failure) => {
                assert_eq!(failure.safe_message(), "Patch failed.");
                assert_eq!(failure.diagnostic(), Some("raw diagnostic detail"));
            }
            other => panic!("expected error payload, got {other:?}"),
        }
    }

    #[test]
    fn spawning_without_tokio_runtime_returns_typed_error() {
        let runtime = WorkerRuntime::new();
        let sink = RecordingEventSink::new();
        let task = WorkerTask::new("no-runtime", WorkerTaskKind::Generic);

        let error = match runtime.spawn_blocking_task(task, sink, |_context| {
            Ok(WorkerTaskOutcome::Completed(WorkerPayload::None))
        }) {
            Ok(_) => panic!("spawning without an active Tokio runtime should fail safely"),
            Err(error) => error,
        };

        assert_eq!(
            error,
            WorkerSpawnError::NoActiveRuntime {
                task_id: WorkerTaskId::new("no-runtime"),
            }
        );
    }
}
