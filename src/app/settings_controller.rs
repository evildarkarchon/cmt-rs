use std::io;

use crate::{
    domain::settings::{AppSettings, LogLevel, UpdateSource},
    platform::settings_store::{AssetResolver, SettingsStore},
};

/// Coordinates Settings-tab UI selections with persisted application settings.
///
/// The controller keeps the last successfully persisted settings snapshot so a
/// failed immediate-save radio change can return the previous value for Slint to
/// display. It does not reconfigure runtime logging; log-level selections only
/// update the persisted `settings.json` value in this phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsController<R> {
    store: SettingsStore<R>,
    last_persisted: AppSettings,
}

impl<R: AssetResolver> SettingsController<R> {
    /// Loads settings through the provided store and initializes UI-facing state.
    ///
    /// Any repairs performed by the settings store are reflected in the initial
    /// snapshot. Reference-valid persisted values that are not exposed as Phase 2
    /// radio choices, such as `WARNING`, are preserved until the user selects a
    /// displayed value.
    pub fn load(store: SettingsStore<R>) -> io::Result<Self> {
        let loaded = store.load()?;

        Ok(Self::from_settings(store, loaded.settings))
    }

    /// Creates a controller around a known settings snapshot without loading from disk.
    ///
    /// This is used as a startup fallback when the production store cannot load or
    /// create `settings.json`; subsequent selections still attempt to save through
    /// the same store and revert to this snapshot if persistence continues to fail.
    pub fn from_settings(store: SettingsStore<R>, settings: AppSettings) -> Self {
        Self {
            store,
            last_persisted: settings,
        }
    }

    /// Returns the update-source value Slint should display for the current snapshot.
    pub fn visible_update_source(&self) -> &'static str {
        self.last_persisted.update_source.as_wire_value()
    }

    /// Returns the lowercase log-level value Slint should display for the current snapshot.
    pub fn visible_log_level(&self) -> &'static str {
        log_level_to_ui_value(self.last_persisted.log_level)
    }

    /// Persists an Update Channel radio selection and returns the UI value to show.
    ///
    /// Unknown callback strings are treated as tampered input and the previous
    /// persisted UI value is returned without saving. If saving a known value
    /// fails, the previous snapshot is preserved and returned for UI reversion.
    pub fn select_update_source(&mut self, selected: &str) -> &'static str {
        let Some(update_source) = UpdateSource::from_wire_value(selected) else {
            tracing::error!(
                selected,
                "Settings : Invalid update source selection; reverting"
            );
            return self.visible_update_source();
        };

        let mut candidate = self.last_persisted.clone();
        candidate.update_source = update_source;
        self.save_candidate(candidate, AppSettings::update_source_ui_value)
    }

    /// Persists a Log Level radio selection and returns the lowercase UI value to show.
    ///
    /// Slint emits lowercase values (`debug`, `info`, `error`), while the domain
    /// model persists uppercase wire values. Unsupported callback strings are
    /// repaired to `INFO` before saving so tampered UI input cannot persist an
    /// unrepresented radio state.
    pub fn select_log_level(&mut self, selected: &str) -> &'static str {
        let log_level = ui_value_to_log_level(selected).unwrap_or_else(|| {
            tracing::error!(
                selected,
                "Settings : Invalid log level selection; repairing to INFO"
            );
            LogLevel::Info
        });

        let mut candidate = self.last_persisted.clone();
        candidate.log_level = log_level;
        self.save_candidate(candidate, AppSettings::log_level_ui_value)
    }

    fn save_candidate(
        &mut self,
        candidate: AppSettings,
        visible_value: impl FnOnce(&AppSettings) -> &'static str,
    ) -> &'static str {
        let previous_update_source = self.visible_update_source();
        let previous_log_level = self.visible_log_level();

        match self.store.save(&candidate) {
            Ok(()) => {
                self.last_persisted = candidate;
                visible_value(&self.last_persisted)
            }
            Err(error) => {
                tracing::error!(
                    path = %self.store.settings_path().display(),
                    %error,
                    "Settings : Failed to save settings; reverting UI selection"
                );
                // Preserve the pre-save snapshot: callers reset the Slint property to this value.
                match visible_value(&candidate) {
                    "debug" | "info" | "error" => previous_log_level,
                    _ => previous_update_source,
                }
            }
        }
    }
}

impl AppSettings {
    fn update_source_ui_value(&self) -> &'static str {
        self.update_source.as_wire_value()
    }

    fn log_level_ui_value(&self) -> &'static str {
        log_level_to_ui_value(self.log_level)
    }
}

fn ui_value_to_log_level(value: &str) -> Option<LogLevel> {
    match value {
        "debug" => Some(LogLevel::Debug),
        "info" => Some(LogLevel::Info),
        "error" => Some(LogLevel::Error),
        _ => None,
    }
}

fn log_level_to_ui_value(log_level: LogLevel) -> &'static str {
    match log_level {
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Warning => "warning",
        LogLevel::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{
        app::settings_controller::SettingsController,
        platform::settings_store::{SettingsStore, StaticAssetResolver},
    };

    #[test]
    fn settings_controller_saves_update_source_immediately() {
        let (_root, settings_path) = isolated_settings_path("update-source-save");
        let store = SettingsStore::with_asset_resolver(
            settings_path.clone(),
            StaticAssetResolver::new(Some("nexus")),
        );
        let mut controller = SettingsController::load(store).expect("controller should load");

        let visible_value = controller.select_update_source("both");

        assert_eq!(visible_value, "both");
        let persisted = fs::read_to_string(settings_path).expect("settings should persist");
        let persisted_json = persisted_json(&persisted);
        assert_eq!(persisted_json["update_source"], "both");
    }

    #[test]
    fn settings_controller_saves_lowercase_log_level_as_uppercase_wire_value() {
        let (_root, settings_path) = isolated_settings_path("log-level-save");
        let store = SettingsStore::with_asset_resolver(
            settings_path.clone(),
            StaticAssetResolver::new(Some("nexus")),
        );
        let mut controller = SettingsController::load(store).expect("controller should load");

        let visible_value = controller.select_log_level("debug");

        assert_eq!(visible_value, "debug");
        let persisted = fs::read_to_string(settings_path).expect("settings should persist");
        let persisted_json = persisted_json(&persisted);
        assert_eq!(persisted_json["log_level"], "DEBUG");
    }

    #[test]
    fn settings_controller_preserves_loaded_warning_log_level_until_user_selection() {
        let (_root, settings_path) = isolated_settings_path("warning-preserve");
        fs::write(
            &settings_path,
            r#"{"log_level":"WARNING","update_source":"github"}"#,
        )
        .expect("test fixture should write settings");
        let store = SettingsStore::with_asset_resolver(
            settings_path.clone(),
            StaticAssetResolver::new(Some("nexus")),
        );

        let controller =
            SettingsController::load(store).expect("controller should repair settings");

        assert_eq!(controller.visible_log_level(), "warning");
        assert_eq!(controller.visible_update_source(), "github");
        let persisted = fs::read_to_string(settings_path).expect("settings should persist");
        let persisted_json = persisted_json(&persisted);
        assert_eq!(persisted_json["log_level"], "WARNING");
        assert_eq!(persisted_json["update_source"], "github");
    }

    #[test]
    fn settings_controller_reverts_to_last_persisted_value_on_save_failure() {
        let (_root, settings_path) = isolated_settings_path("save-failure-revert");
        let setup_store = SettingsStore::with_asset_resolver(
            settings_path.clone(),
            StaticAssetResolver::new(Some("nexus")),
        );
        let mut controller = SettingsController::load(setup_store).expect("controller should load");
        assert_eq!(controller.select_update_source("github"), "github");
        fs::remove_file(&settings_path).expect("settings file should be removable");
        fs::create_dir_all(&settings_path).expect("directory should block future saves");

        let reverted_value = controller.select_update_source("both");

        assert_eq!(reverted_value, "github");
        assert_eq!(controller.visible_update_source(), "github");
    }

    #[test]
    fn settings_controller_rejects_invalid_log_level_selection_to_info() {
        let (_root, settings_path) = isolated_settings_path("invalid-log-selection");
        let store = SettingsStore::with_asset_resolver(
            settings_path.clone(),
            StaticAssetResolver::new(Some("nexus")),
        );
        let mut controller = SettingsController::load(store).expect("controller should load");

        let visible_value = controller.select_log_level("WARNING");

        assert_eq!(visible_value, "info");
        let persisted = fs::read_to_string(settings_path).expect("settings should persist");
        let persisted_json = persisted_json(&persisted);
        assert_eq!(persisted_json["log_level"], "INFO");
    }

    fn isolated_settings_path(case_name: &str) -> (PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("cmt-rs-settings-controller-{case_name}-{unique}"));
        fs::create_dir_all(&root).expect("test temp directory should be created");
        let settings_path = root.join("settings.json");
        (root, settings_path)
    }

    fn persisted_json(source: &str) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_str::<serde_json::Value>(source)
            .expect("persisted settings should be JSON")
            .as_object()
            .expect("persisted settings should be a JSON object")
            .clone()
    }
}
