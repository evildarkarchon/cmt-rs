//! Domain model boundary for future CMT behavior.
//!
//! This module is intentionally inert in Phase 1. Later port slices can add
//! typed settings, scan results, game metadata, archive information, and other
//! pure domain state here without putting that logic in Slint markup.

pub mod archive_patcher;
pub mod autofix;
pub mod discovery;
pub mod downgrader;
pub mod f4se;
pub mod mod_manager;
pub mod overview;
pub mod scanner;
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

        assert_type::<crate::domain::archive_patcher::ArchivePatcherArchiveFormat>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherCandidateRow>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherCandidateSnapshot>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherHeader>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherLatestManifest>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherLogLevel>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherLogRow>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherPlanAction>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherPreviewPlan>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherPreviewPlanCounts>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherPreviewPlanRow>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherProgress>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherRestoreManifestEntry>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherSummaryCounts>();
        assert_type::<crate::domain::archive_patcher::ArchivePatcherTarget>();
        let _ = crate::domain::archive_patcher::ARCHIVE_PATCHER_MODAL_TITLE;
        let _ = crate::domain::archive_patcher::TARGET_OLD_GEN_LABEL;
        let _ = crate::domain::archive_patcher::TARGET_NEXT_GEN_LABEL;
        let _ = crate::domain::archive_patcher::PATCHER_FILTER_OLD_GEN;
        let _ = crate::domain::archive_patcher::PATCHER_FILTER_NEXT_GEN;
        let _ = crate::domain::archive_patcher::ABOUT_ARCHIVES_TITLE;
        let _ = crate::domain::archive_patcher::ABOUT_ARCHIVES_BODY;
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
        assert_type::<crate::domain::autofix::AutoFixButtonState>();
        assert_type::<crate::domain::autofix::AutoFixCompletion>();
        assert_type::<crate::domain::autofix::AutoFixOperationKey>();
        assert_type::<crate::domain::autofix::AutoFixPlanPreview>();
        assert_type::<crate::domain::autofix::AutoFixRejection>();
        assert_type::<crate::domain::autofix::AutoFixRequest>();
        assert_type::<crate::domain::autofix::AutoFixResultDetail>();
        assert_type::<crate::domain::autofix::AutoFixSelectionIdentity>();
        assert_type::<crate::domain::autofix::AutoFixStatus>();
        let _ = crate::domain::autofix::AUTO_FIX_BUTTON_LABEL;
        let _ = crate::domain::autofix::AUTO_FIXING_BUTTON_LABEL;
        let _ = crate::domain::autofix::AUTO_FIX_FIXED_BUTTON_LABEL;
        let _ = crate::domain::autofix::AUTO_FIX_FAILED_BUTTON_LABEL;
        let _ = crate::domain::autofix::AUTO_FIX_RESULTS_TITLE;
        assert_type::<crate::domain::downgrader::DowngraderFileDefinition>();
        assert_type::<crate::domain::downgrader::DowngraderCrcMapping>();
        assert_type::<crate::domain::downgrader::DowngraderFileGroup>();
        assert_type::<crate::domain::downgrader::DowngraderInstallStatus>();
        assert_type::<crate::domain::downgrader::DowngraderTarget>();
        assert_type::<crate::domain::downgrader::DowngraderStatusRow>();
        assert_type::<crate::domain::downgrader::DowngraderOptionsSnapshot>();
        assert_type::<crate::domain::downgrader::DowngraderPlanAction>();
        assert_type::<crate::domain::downgrader::DowngraderPlanStepKind>();
        assert_type::<crate::domain::downgrader::DowngraderPlanStep>();
        assert_type::<crate::domain::downgrader::DowngraderPlanRow>();
        assert_type::<crate::domain::downgrader::DowngraderExecutionLogRow>();
        assert_type::<crate::domain::downgrader::DowngraderLogLevel>();
        assert_type::<crate::domain::downgrader::DowngraderProgress>();
        let _ = crate::domain::downgrader::DOWNGRADER_MODAL_TITLE;
        let _ = crate::domain::downgrader::PATCH_ALL_BUTTON_LABEL;
        let _ = crate::domain::downgrader::INITIAL_LOG_LINE;
        let _ = crate::domain::downgrader::ABOUT_DOWNGRADING_TITLE;
        let _ = crate::domain::downgrader::ABOUT_DOWNGRADING_BODY;
        let _ = crate::domain::downgrader::TOOLTIP_DOWNGRADER_BACKUPS;
        let _ = crate::domain::downgrader::TOOLTIP_DOWNGRADER_DELTAS;
        let _ = crate::domain::downgrader::DOWNGRADER_FILE_DEFINITIONS;
        assert_type::<crate::domain::scanner::ScannerResult>();
        assert_type::<crate::domain::scanner::ScannerProblemType>();
        assert_type::<crate::domain::scanner::ScannerSolutionKind>();
        assert_type::<crate::domain::scanner::ScannerActionDescriptor>();
        assert_type::<crate::domain::f4se::F4seGameTarget>();
        assert_type::<crate::domain::f4se::F4seDllFacts>();
        assert_type::<crate::domain::f4se::F4seCompatibilityCell>();
        assert_type::<crate::domain::f4se::F4seDllRow>();
        assert_type::<crate::domain::f4se::F4seScanSnapshot>();
        assert_type::<crate::domain::f4se::F4seScanStatus>();
        assert_type::<crate::domain::tools::ToolGroup>();
        assert_type::<crate::domain::tools::ToolEntry>();
        assert_type::<crate::domain::tools::ToolActionId>();
        assert_type::<crate::domain::tools::AboutLink>();
        assert_type::<crate::domain::tools::AboutActionId>();
        let _ = crate::domain::tools::TOOL_GROUPS;
        let _ = crate::domain::tools::ABOUT_LINKS;
    }
}
