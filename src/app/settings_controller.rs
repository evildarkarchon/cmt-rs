#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

    use crate::{
        app::settings_controller::SettingsController,
        domain::settings::{LogLevel, UpdateSource},
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
    fn settings_controller_repairs_unsupported_loaded_log_level_to_info_ui_value() {
        let (_root, settings_path) = isolated_settings_path("warning-repair");
        fs::write(
            &settings_path,
            r#"{"log_level":"WARNING","update_source":"github"}"#,
        )
        .expect("test fixture should write settings");
        let store = SettingsStore::with_asset_resolver(
            settings_path,
            StaticAssetResolver::new(Some("nexus")),
        );

        let controller = SettingsController::load(store).expect("controller should repair settings");

        assert_eq!(controller.visible_log_level(), "info");
        assert_eq!(controller.visible_update_source(), "github");
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
        let root = std::env::temp_dir().join(format!("cmt-rs-settings-controller-{case_name}-{unique}"));
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
