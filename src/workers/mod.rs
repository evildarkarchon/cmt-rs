//! Worker orchestration boundary for future long-running CMT tasks.
//!
//! Future phases can place scan, patch, download, and subprocess orchestration
//! behind this module so slow work stays off the Slint UI thread. Phase 1 keeps
//! it inert and does not create runtimes, channels, tasks, or UI-thread handoffs.

/// No-op worker runtime marker reserved for future background orchestration.
///
/// Constructing this marker starts no Tokio runtime, spawns no tasks, and owns no
/// live worker handles. This preserves SAFE-05 while documenting the seam for
/// later non-blocking work.
#[derive(Debug, Default, Clone, Copy)]
pub struct WorkerRuntime;
