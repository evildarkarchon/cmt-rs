//! Application-facing shell contracts for the Rust/Slint port.
//!
//! The labels below are copied from the reference `Tab` enum in
//! `CMT/src/enums.py` and the creation order in `CMT/src/cm_checker.py`. They
//! intentionally remain static in Phase 1 so tests can lock the shell identity
//! without launching GUI automation or wiring real tab behavior.

/// Reference shell tab labels in their display order.
pub const SHELL_TAB_LABELS: [&str; 6] = ["Overview", "F4SE", "Scanner", "Tools", "Settings", "About"];

/// Returns the canonical shell tab labels in reference display order.
///
/// The labels match `CMT/src/enums.py` and the notebook construction order in
/// `CMT/src/cm_checker.py`. The function performs no GUI, filesystem, settings,
/// scanner, network, subprocess, or background work.
pub const fn shell_tab_labels() -> [&'static str; 6] {
    SHELL_TAB_LABELS
}

/// No-op application controller boundary reserved for future UI orchestration.
///
/// Phase 1 deliberately keeps this type inert: it owns no settings, platform
/// adapters, worker handles, or Slint component references, and constructing it
/// has no side effects.
#[derive(Debug, Default, Clone, Copy)]
pub struct ShellController;
