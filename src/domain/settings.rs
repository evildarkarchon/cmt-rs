//! Typed settings model for reference-compatible CMT settings data.

/// Top-level application settings persisted in the reference `settings.json`.
///
/// This aggregate keeps domain settings typed while later platform code owns
/// filesystem paths and save/load side effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSettings {
    /// Minimum log severity persisted with the `log_level` JSON key.
    pub log_level: LogLevel,
    /// Update channel persisted with the `update_source` JSON key.
    pub update_source: UpdateSource,
    /// Scanner toggles persisted as reference-compatible `scanner_*` keys.
    pub scanner: ScannerSettings,
    /// Downgrader toggles persisted as reference-compatible `downgrader_*` keys.
    pub downgrader: DowngraderSettings,
}

/// Reference log level values exposed by the Settings tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Extra-verbose log output.
    Debug,
    /// Reference default log level.
    Info,
    /// Error-only log output.
    Error,
}

/// Update channel persisted for later update-check behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSource {
    /// Check both GitHub and Nexus Mods.
    Both,
    /// Check GitHub only.
    Github,
    /// Check Nexus Mods only.
    Nexus,
    /// Do not check for updates.
    None,
}

/// Scanner category toggles persisted by the Settings model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerSettings {
    /// Enables overview issue checks.
    pub overview_issues: bool,
    /// Enables generic error checks.
    pub errors: bool,
    /// Enables wrong file format checks.
    pub wrong_format: bool,
    /// Enables loose previs checks.
    pub loose_previs: bool,
    /// Enables junk file checks.
    pub junk_files: bool,
    /// Enables problematic override checks.
    pub problem_overrides: bool,
    /// Enables race subgraph checks.
    pub race_subgraphs: bool,
}

/// Downgrader workflow preferences persisted by the Settings model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderSettings {
    /// Keeps backups when later downgrader operations modify files.
    pub keep_backups: bool,
    /// Deletes delta files after later downgrader operations use them.
    pub delete_deltas: bool,
}
