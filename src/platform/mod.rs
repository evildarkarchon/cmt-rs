//! Platform adapter boundary for future operating-system integrations.
//!
//! This module exists so later slices can isolate filesystem, registry, process,
//! dialog, and URL-opening adapters from UI and domain code. Phase 1 keeps the
//! boundary as a no-op marker and performs no platform access.

pub mod settings_store;

/// No-op platform services marker reserved for future OS-facing adapters.
///
/// Constructing this marker does not read paths, query the registry, inspect the
/// environment, launch processes, or disclose filesystem state.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlatformServices;
