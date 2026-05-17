//! Worker event handoff sinks.
//!
//! Background workers emit owned [`WorkerEvent`](crate::workers::events::WorkerEvent)
//! values through this module. Tests can use the recording sink, while UI code can
//! use the Slint event-loop sink so worker closures never mutate Slint models or
//! component handles directly.

use std::{
    fmt,
    sync::{Arc, Mutex},
};

use crate::workers::events::WorkerEvent;

/// Result type returned by worker handoff sinks.
pub type WorkerHandoffResult<T = ()> = Result<T, WorkerHandoffError>;

/// Typed handoff failure categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkerHandoffErrorKind {
    /// The in-memory recording sink could not be locked.
    RecordingUnavailable,
    /// Slint rejected the event-loop handoff, usually because the loop is gone.
    SlintEventLoopUnavailable,
}

impl WorkerHandoffErrorKind {
    /// Returns a stable label suitable for structured logs and tests.
    pub const fn label(self) -> &'static str {
        match self {
            Self::RecordingUnavailable => "recording-unavailable",
            Self::SlintEventLoopUnavailable => "slint-event-loop-unavailable",
        }
    }
}

/// Failure emitted when a worker event cannot be handed to a sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerHandoffError {
    /// Typed failure category.
    pub kind: WorkerHandoffErrorKind,
    /// Safe user-facing message.
    pub safe_message: String,
    /// Optional diagnostic detail for logs or tests.
    pub diagnostic: Option<String>,
}

impl WorkerHandoffError {
    /// Creates a handoff error with a safe message.
    pub fn new(kind: WorkerHandoffErrorKind, safe_message: impl Into<String>) -> Self {
        Self {
            kind,
            safe_message: safe_message.into(),
            diagnostic: None,
        }
    }

    /// Adds diagnostic detail without changing the safe message.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }

    fn recording_poisoned(diagnostic: impl Into<String>) -> Self {
        Self::new(
            WorkerHandoffErrorKind::RecordingUnavailable,
            "Worker event recording is unavailable.",
        )
        .with_diagnostic(diagnostic)
    }

    fn slint_event_loop(diagnostic: impl Into<String>) -> Self {
        Self::new(
            WorkerHandoffErrorKind::SlintEventLoopUnavailable,
            "Worker event could not be delivered to the UI event loop.",
        )
        .with_diagnostic(diagnostic)
    }
}

impl fmt::Display for WorkerHandoffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({})", self.safe_message, self.kind.label())
    }
}

impl std::error::Error for WorkerHandoffError {}

/// Sink trait used by worker code to emit owned events.
pub trait WorkerEventSink: Clone + Send + Sync + 'static {
    /// Emits one owned worker event.
    fn emit(&self, event: WorkerEvent) -> WorkerHandoffResult;
}

/// Test and diagnostics sink that stores all emitted events in memory.
#[derive(Debug, Clone, Default)]
pub struct RecordingEventSink {
    events: Arc<Mutex<Vec<WorkerEvent>>>,
}

impl RecordingEventSink {
    /// Creates an empty recording sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a cloned snapshot of recorded events.
    pub fn events(&self) -> WorkerHandoffResult<Vec<WorkerEvent>> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|error| WorkerHandoffError::recording_poisoned(error.to_string()))
    }

    /// Returns the number of recorded events.
    pub fn len(&self) -> WorkerHandoffResult<usize> {
        self.events
            .lock()
            .map(|events| events.len())
            .map_err(|error| WorkerHandoffError::recording_poisoned(error.to_string()))
    }

    /// Returns true when no events have been recorded.
    pub fn is_empty(&self) -> WorkerHandoffResult<bool> {
        self.len().map(|len| len == 0)
    }
}

impl WorkerEventSink for RecordingEventSink {
    fn emit(&self, event: WorkerEvent) -> WorkerHandoffResult {
        self.events
            .lock()
            .map(|mut events| events.push(event))
            .map_err(|error| WorkerHandoffError::recording_poisoned(error.to_string()))
    }
}

/// Slint event-loop sink for handing worker events to UI-owned state.
///
/// The handler runs on Slint's event loop. Worker code only sees this sink and
/// owned [`WorkerEvent`](crate::workers::events::WorkerEvent) values; it never
/// receives Slint component handles or model references.
#[derive(Clone)]
pub struct SlintEventLoopSink {
    handler: Arc<dyn Fn(WorkerEvent) + Send + Sync + 'static>,
}

impl SlintEventLoopSink {
    /// Creates a Slint event-loop sink from a UI-thread handler.
    pub fn new(handler: impl Fn(WorkerEvent) + Send + Sync + 'static) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }
}

impl fmt::Debug for SlintEventLoopSink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SlintEventLoopSink")
            .field("handler", &"<event-loop handler>")
            .finish()
    }
}

impl WorkerEventSink for SlintEventLoopSink {
    fn emit(&self, event: WorkerEvent) -> WorkerHandoffResult {
        let handler = Arc::clone(&self.handler);
        slint::invoke_from_event_loop(move || {
            handler(event);
        })
        .map_err(|error| WorkerHandoffError::slint_event_loop(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workers::events::{WorkerEvent, WorkerTask, WorkerTaskKind};

    fn assert_sink<T: WorkerEventSink>() {}

    #[test]
    fn recording_sink_stores_owned_events() {
        let sink = RecordingEventSink::new();
        let event = WorkerEvent::running(WorkerTask::new("discover", WorkerTaskKind::Discovery));

        sink.emit(event.clone())
            .expect("recording sink should accept owned event");

        assert_eq!(
            sink.events().expect("events should be readable"),
            vec![event]
        );
        assert_eq!(sink.len().expect("length should be readable"), 1);
        assert!(!sink.is_empty().expect("emptiness should be readable"));
    }

    #[test]
    fn slint_event_loop_sink_is_constructible_without_a_window() {
        assert_sink::<SlintEventLoopSink>();

        let _sink = SlintEventLoopSink::new(|_event| {
            // Construction is enough for this unit test: emitting requires a
            // running Slint event loop, while compile checking verifies the
            // event-loop handoff path.
        });
    }
}
