//! Application services that orchestrate pure domain contracts over platform adapters.
//!
//! Services in this module are allowed to coordinate filesystem, registry,
//! process, and background seams through traits, but they should keep Slint and
//! user prompts out of the domain-facing behavior so workflows remain testable.

pub mod autofix;
pub mod discovery;
pub mod downgrader;
pub mod f4se;
pub mod overview;
pub mod overview_collector;
pub mod scanner;
pub mod tools;
pub mod update;

/// No-op service-layer marker reserved for future orchestration state.
///
/// Constructing this marker performs no filesystem, registry, process, network,
/// scanner, settings, or UI work.
#[derive(Debug, Default, Clone, Copy)]
pub struct ServiceLayer;
