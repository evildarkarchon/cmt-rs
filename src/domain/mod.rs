//! Domain model boundary for future CMT behavior.
//!
//! This module is intentionally inert in Phase 1. Later port slices can add
//! typed settings, scan results, game metadata, archive information, and other
//! pure domain state here without putting that logic in Slint markup.

pub mod discovery;
pub mod mod_manager;
pub mod overview;
pub mod settings;
pub mod tools;

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
        fn assert_type<T>() {}

        assert_type::<crate::domain::settings::AppSettings>();
        assert_type::<crate::domain::settings::LogLevel>();
        assert_type::<crate::domain::settings::UpdateSource>();
        assert_type::<crate::domain::settings::ScannerSettings>();
        assert_type::<crate::domain::settings::DowngraderSettings>();
        assert_type::<crate::domain::discovery::Fallout4Installation>();
        assert_type::<crate::domain::discovery::DiscoveryError>();
        assert_type::<crate::domain::discovery::SemanticVersion>();
        assert_type::<crate::domain::mod_manager::DetectedModManager>();
        assert_type::<crate::domain::mod_manager::ModOrganizerContext>();
        assert_type::<crate::domain::mod_manager::VortexContext>();
        assert_type::<crate::domain::mod_manager::Mo2ParseError>();
        assert_type::<crate::domain::overview::OverviewSnapshot>();
        assert_type::<crate::domain::overview::OverviewProblem>();
        assert_type::<crate::domain::overview::UpdateBannerState>();
        assert_type::<crate::domain::tools::ToolGroup>();
        assert_type::<crate::domain::tools::ToolEntry>();
        assert_type::<crate::domain::tools::ToolActionId>();
        assert_type::<crate::domain::tools::AboutLink>();
        assert_type::<crate::domain::tools::AboutActionId>();
        let _ = crate::domain::tools::TOOL_GROUPS;
        let _ = crate::domain::tools::ABOUT_LINKS;
    }
}
