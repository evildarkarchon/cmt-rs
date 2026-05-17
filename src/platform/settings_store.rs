//! Filesystem-backed settings store for reference-compatible CMT settings IO.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::settings::UpdateSource;

    use super::*;

    #[test]
    fn settings_store_uses_current_directory_settings_json_by_default() {
        let paths = SettingsPaths::production();

        assert_eq!(paths.settings_path(), PathBuf::from("settings.json"));
    }

    #[test]
    fn injected_paths_and_asset_resolver_drive_update_source_defaults() {
        let temp_root = std::env::temp_dir().join("cmt-rs-settings-store-red");
        let settings_path = temp_root.join("isolated-settings.json");
        let asset_path = temp_root.join("download-source.txt");
        let paths = SettingsPaths::injected(settings_path.clone(), FileAssetResolver::new(asset_path));
        let store = SettingsStore::new(paths);

        assert_eq!(store.settings_path(), settings_path.as_path());
        assert_eq!(store.default_update_source(), UpdateSource::Nexus);
    }

    #[test]
    fn asset_resolver_rejects_invalid_download_source_with_nexus_fallback() {
        let resolver = StaticAssetResolver::new(Some("invalid-source"));
        let store = SettingsStore::with_asset_resolver(PathBuf::from("settings.json"), resolver);

        assert_eq!(store.default_update_source(), UpdateSource::Nexus);
    }
}
