//! Typed settings model for reference-compatible CMT settings data.

use serde_json::{Value, json};

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

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            update_source: UpdateSource::Nexus,
            scanner: ScannerSettings::default(),
            downgrader: DowngraderSettings::default(),
        }
    }
}

impl AppSettings {
    /// Converts settings into the reference-compatible JSON object shape.
    ///
    /// Keys intentionally preserve the mixed-case scanner names used by
    /// `CMT/src/app_settings.py` so later file IO can resave repaired settings
    /// without introducing Rust-style snake_case key drift.
    pub fn to_json_value(&self) -> Value {
        json!({
            "log_level": self.log_level.as_wire_value(),
            "update_source": self.update_source.as_wire_value(),
            "scanner_OverviewIssues": self.scanner.overview_issues,
            "scanner_Errors": self.scanner.errors,
            "scanner_WrongFormat": self.scanner.wrong_format,
            "scanner_LoosePrevis": self.scanner.loose_previs,
            "scanner_JunkFiles": self.scanner.junk_files,
            "scanner_ProblemOverrides": self.scanner.problem_overrides,
            "scanner_RaceSubgraphs": self.scanner.race_subgraphs,
            "downgrader_keep_backups": self.downgrader.keep_backups,
            "downgrader_delete_deltas": self.downgrader.delete_deltas,
        })
    }
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

impl LogLevel {
    /// Returns the exact string persisted for this log level in `settings.json`.
    pub const fn as_wire_value(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Error => "ERROR",
        }
    }
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

impl UpdateSource {
    /// Returns the exact string persisted for this update source in `settings.json`.
    pub const fn as_wire_value(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Github => "github",
            Self::Nexus => "nexus",
            Self::None => "none",
        }
    }
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

impl Default for ScannerSettings {
    fn default() -> Self {
        Self {
            overview_issues: true,
            errors: true,
            wrong_format: true,
            loose_previs: true,
            junk_files: true,
            problem_overrides: true,
            race_subgraphs: true,
        }
    }
}

/// Downgrader workflow preferences persisted by the Settings model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderSettings {
    /// Keeps backups when later downgrader operations modify files.
    pub keep_backups: bool,
    /// Deletes delta files after later downgrader operations use them.
    pub delete_deltas: bool,
}

impl Default for DowngraderSettings {
    fn default() -> Self {
        Self {
            keep_backups: true,
            delete_deltas: true,
        }
    }
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

    #[test]
    fn settings_repair() {
        assert!(AppSettings::from_json_str("{ not json").is_err());
        assert!(AppSettings::from_json_str("[]").is_err());

        let repaired = AppSettings::from_json_str(
            r#"{
                "log_level": "DEBUG",
                "update_source": "bogus",
                "scanner_OverviewIssues": false,
                "scanner_Errors": "yes",
                "scanner_WrongFormat": false,
                "scanner_LoosePrevis": false,
                "scanner_JunkFiles": false,
                "scanner_ProblemOverrides": false,
                "scanner_RaceSubgraphs": false,
                "downgrader_keep_backups": false,
                "unknown_setting": true
            }"#,
        )
        .expect("syntactically valid JSON object should repair per key");

        assert_eq!(repaired.settings.log_level, LogLevel::Debug);
        assert_eq!(repaired.settings.update_source, UpdateSource::Nexus);
        assert!(!repaired.settings.scanner.overview_issues);
        assert!(repaired.settings.scanner.errors);
        assert!(!repaired.settings.scanner.wrong_format);
        assert!(!repaired.settings.scanner.loose_previs);
        assert!(!repaired.settings.scanner.junk_files);
        assert!(!repaired.settings.scanner.problem_overrides);
        assert!(!repaired.settings.scanner.race_subgraphs);
        assert!(!repaired.settings.downgrader.keep_backups);
        assert!(repaired.settings.downgrader.delete_deltas);

        assert!(repaired.diagnostics.iter().any(|diagnostic| {
            matches!(diagnostic, RepairDiagnostic::InvalidValue { key } if key == "update_source")
        }));
        assert!(repaired.diagnostics.iter().any(|diagnostic| {
            matches!(diagnostic, RepairDiagnostic::InvalidType { key } if key == "scanner_Errors")
        }));
        assert!(repaired.diagnostics.iter().any(|diagnostic| {
            matches!(diagnostic, RepairDiagnostic::MissingKey { key } if key == "downgrader_delete_deltas")
        }));
        assert!(repaired.diagnostics.iter().any(|diagnostic| {
            matches!(diagnostic, RepairDiagnostic::UnknownKey { key } if key == "unknown_setting")
        }));

        let repaired_json = repaired.settings.to_json_value();
        let repaired_object = repaired_json
            .as_object()
            .expect("repaired settings should serialize as object");
        assert!(!repaired_object.contains_key("unknown_setting"));

        let unsupported_warning = AppSettings::from_json_str(r#"{"log_level":"WARNING"}"#)
            .expect("unsupported log_level should repair to default");
        assert_eq!(unsupported_warning.settings.log_level, LogLevel::Info);
    }
}
