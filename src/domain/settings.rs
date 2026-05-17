//! Typed settings model for reference-compatible CMT settings data.

use serde_json::{Map, Value, json};

const LOG_LEVEL_KEY: &str = "log_level";
const UPDATE_SOURCE_KEY: &str = "update_source";
const SCANNER_OVERVIEW_ISSUES_KEY: &str = "scanner_OverviewIssues";
const SCANNER_ERRORS_KEY: &str = "scanner_Errors";
const SCANNER_WRONG_FORMAT_KEY: &str = "scanner_WrongFormat";
const SCANNER_LOOSE_PREVIS_KEY: &str = "scanner_LoosePrevis";
const SCANNER_JUNK_FILES_KEY: &str = "scanner_JunkFiles";
const SCANNER_PROBLEM_OVERRIDES_KEY: &str = "scanner_ProblemOverrides";
const SCANNER_RACE_SUBGRAPHS_KEY: &str = "scanner_RaceSubgraphs";
const DOWNGRADER_KEEP_BACKUPS_KEY: &str = "downgrader_keep_backups";
const DOWNGRADER_DELETE_DELTAS_KEY: &str = "downgrader_delete_deltas";

const KNOWN_KEYS: [&str; 11] = [
    LOG_LEVEL_KEY,
    UPDATE_SOURCE_KEY,
    SCANNER_OVERVIEW_ISSUES_KEY,
    SCANNER_ERRORS_KEY,
    SCANNER_WRONG_FORMAT_KEY,
    SCANNER_LOOSE_PREVIS_KEY,
    SCANNER_JUNK_FILES_KEY,
    SCANNER_PROBLEM_OVERRIDES_KEY,
    SCANNER_RACE_SUBGRAPHS_KEY,
    DOWNGRADER_KEEP_BACKUPS_KEY,
    DOWNGRADER_DELETE_DELTAS_KEY,
];

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
    /// Parses and repairs a syntactically valid JSON settings object.
    ///
    /// Malformed JSON and non-object roots are rejected so the platform store can
    /// perform the reference-compatible defaults-only reset required by D-09 and
    /// D-11. Valid objects preserve each valid known key independently and report
    /// diagnostics for keys that need default repair or removal.
    pub fn from_json_str(source: &str) -> Result<SettingsRepairResult, SettingsParseError> {
        let value: Value = serde_json::from_str(source).map_err(SettingsParseError::MalformedJson)?;
        let object = value.as_object().ok_or(SettingsParseError::NonObjectRoot)?;
        Ok(Self::apply_json_object(object))
    }

    /// Applies reference-compatible per-key repair semantics to a JSON object.
    ///
    /// Unknown keys are ignored in memory and therefore omitted when the returned
    /// settings are serialized with [`AppSettings::to_json_value`].
    pub fn apply_json_object(object: &Map<String, Value>) -> SettingsRepairResult {
        let mut settings = Self::default();
        let mut diagnostics = Vec::new();

        for key in KNOWN_KEYS {
            if !object.contains_key(key) {
                diagnostics.push(RepairDiagnostic::MissingKey {
                    key: key.to_owned(),
                });
            }
        }

        for (key, value) in object {
            match key.as_str() {
                LOG_LEVEL_KEY => match value.as_str().and_then(LogLevel::from_wire_value) {
                    Some(log_level) => settings.log_level = log_level,
                    None if value.is_string() => diagnostics.push(RepairDiagnostic::InvalidValue {
                        key: key.clone(),
                    }),
                    None => diagnostics.push(RepairDiagnostic::InvalidType { key: key.clone() }),
                },
                UPDATE_SOURCE_KEY => match value.as_str().and_then(UpdateSource::from_wire_value) {
                    Some(update_source) => settings.update_source = update_source,
                    None if value.is_string() => diagnostics.push(RepairDiagnostic::InvalidValue {
                        key: key.clone(),
                    }),
                    None => diagnostics.push(RepairDiagnostic::InvalidType { key: key.clone() }),
                },
                SCANNER_OVERVIEW_ISSUES_KEY => apply_bool(
                    value,
                    key,
                    &mut settings.scanner.overview_issues,
                    &mut diagnostics,
                ),
                SCANNER_ERRORS_KEY => apply_bool(
                    value,
                    key,
                    &mut settings.scanner.errors,
                    &mut diagnostics,
                ),
                SCANNER_WRONG_FORMAT_KEY => apply_bool(
                    value,
                    key,
                    &mut settings.scanner.wrong_format,
                    &mut diagnostics,
                ),
                SCANNER_LOOSE_PREVIS_KEY => apply_bool(
                    value,
                    key,
                    &mut settings.scanner.loose_previs,
                    &mut diagnostics,
                ),
                SCANNER_JUNK_FILES_KEY => apply_bool(
                    value,
                    key,
                    &mut settings.scanner.junk_files,
                    &mut diagnostics,
                ),
                SCANNER_PROBLEM_OVERRIDES_KEY => apply_bool(
                    value,
                    key,
                    &mut settings.scanner.problem_overrides,
                    &mut diagnostics,
                ),
                SCANNER_RACE_SUBGRAPHS_KEY => apply_bool(
                    value,
                    key,
                    &mut settings.scanner.race_subgraphs,
                    &mut diagnostics,
                ),
                DOWNGRADER_KEEP_BACKUPS_KEY => apply_bool(
                    value,
                    key,
                    &mut settings.downgrader.keep_backups,
                    &mut diagnostics,
                ),
                DOWNGRADER_DELETE_DELTAS_KEY => apply_bool(
                    value,
                    key,
                    &mut settings.downgrader.delete_deltas,
                    &mut diagnostics,
                ),
                _ => diagnostics.push(RepairDiagnostic::UnknownKey { key: key.clone() }),
            }
        }

        SettingsRepairResult {
            settings,
            diagnostics,
        }
    }

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

fn apply_bool(
    value: &Value,
    key: &str,
    target: &mut bool,
    diagnostics: &mut Vec<RepairDiagnostic>,
) {
    if let Some(value) = value.as_bool() {
        *target = value;
    } else {
        diagnostics.push(RepairDiagnostic::InvalidType {
            key: key.to_owned(),
        });
    }
}

/// Parsed settings plus diagnostics describing any quiet repairs made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsRepairResult {
    /// Typed settings after valid values were applied and invalid values defaulted.
    pub settings: AppSettings,
    /// Diagnostics for test assertions and later logging; values are deliberately omitted.
    pub diagnostics: Vec<RepairDiagnostic>,
}

/// Non-sensitive diagnostic for a single settings repair action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairDiagnostic {
    /// A known reference key was absent and retained its default value.
    MissingKey { key: String },
    /// A known enum/string key had an unsupported string value.
    InvalidValue { key: String },
    /// A known key had the wrong JSON type.
    InvalidType { key: String },
    /// An unknown key was ignored and will be omitted on resave.
    UnknownKey { key: String },
}

/// Parse errors that require the platform store to reset settings to defaults.
#[derive(Debug)]
pub enum SettingsParseError {
    /// JSON parsing failed before a trustworthy object could be inspected.
    MalformedJson(serde_json::Error),
    /// The parsed JSON root was not an object.
    NonObjectRoot,
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

    /// Converts a persisted log-level string into the typed value when supported.
    pub fn from_wire_value(value: &str) -> Option<Self> {
        match value {
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "ERROR" => Some(Self::Error),
            _ => None,
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

    /// Converts a persisted update-source string into the typed value when supported.
    pub fn from_wire_value(value: &str) -> Option<Self> {
        match value {
            "both" => Some(Self::Both),
            "github" => Some(Self::Github),
            "nexus" => Some(Self::Nexus),
            "none" => Some(Self::None),
            _ => None,
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
