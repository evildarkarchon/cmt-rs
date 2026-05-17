//! Domain model boundary for future CMT behavior.
//!
//! This module is intentionally inert in Phase 1. Later port slices can add
//! typed settings, scan results, game metadata, archive information, and other
//! pure domain state here without putting that logic in Slint markup.

pub mod settings;

/// No-op domain state marker reserved for future typed application data.
///
/// Constructing this marker performs no filesystem, registry, settings,
/// scanner, network, subprocess, or background work.
#[derive(Debug, Default, Clone, Copy)]
pub struct DomainState;

#[cfg(test)]
mod tests {
    #[test]
    fn settings_domain_types_are_publicly_importable() {
        fn assert_imports(
            _settings: crate::domain::settings::AppSettings,
            _log_level: crate::domain::settings::LogLevel,
            _update_source: crate::domain::settings::UpdateSource,
            _scanner: crate::domain::settings::ScannerSettings,
            _downgrader: crate::domain::settings::DowngraderSettings,
        ) {
        }

        let _ = assert_imports;
    }
}
