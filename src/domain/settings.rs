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

#[cfg(test)]
mod tests {
    use super::*;

    const REFERENCE_KEYS: [&str; 11] = [
        "log_level",
        "update_source",
        "scanner_OverviewIssues",
        "scanner_Errors",
        "scanner_WrongFormat",
        "scanner_LoosePrevis",
        "scanner_JunkFiles",
        "scanner_ProblemOverrides",
        "scanner_RaceSubgraphs",
        "downgrader_keep_backups",
        "downgrader_delete_deltas",
    ];

    #[test]
    fn settings_missing_file_defaults() {
        let settings = AppSettings::default();

        assert_eq!(settings.log_level, LogLevel::Info);
        assert_eq!(settings.update_source, UpdateSource::Nexus);
        assert!(settings.downgrader.keep_backups);
        assert!(settings.downgrader.delete_deltas);
    }

    #[test]
    fn settings_persist_reference_keys() {
        let json = AppSettings::default().to_json_value();
        let object = json.as_object().expect("settings should serialize as object");

        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        let mut expected_keys = REFERENCE_KEYS.to_vec();
        expected_keys.sort_unstable();
        assert_eq!(keys, expected_keys);

        assert_eq!(json["log_level"], "INFO");
        assert_eq!(LogLevel::Debug.as_wire_value(), "DEBUG");
        assert_eq!(LogLevel::Info.as_wire_value(), "INFO");
        assert_eq!(LogLevel::Error.as_wire_value(), "ERROR");
        assert_eq!(json["update_source"], "nexus");
        assert!(!object.contains_key("scanner_overview_issues"));
        assert!(!object.contains_key("scanner_wrong_format"));
    }

    #[test]
    fn scanner_settings_defaults_enabled() {
        let scanner = ScannerSettings::default();

        assert!(scanner.overview_issues);
        assert!(scanner.errors);
        assert!(scanner.wrong_format);
        assert!(scanner.loose_previs);
        assert!(scanner.junk_files);
        assert!(scanner.problem_overrides);
        assert!(scanner.race_subgraphs);
    }
}
