//! Filesystem-backed settings store for reference-compatible CMT settings IO.

use std::{fs, io, path::{Path, PathBuf}};

use crate::domain::settings::UpdateSource;

const SETTINGS_FILE_NAME: &str = "settings.json";
const DOWNLOAD_SOURCE_FILE_NAME: &str = "download-source.txt";

/// Resolves auxiliary settings assets such as `download-source.txt`.
///
/// The abstraction keeps packaged asset lookup separate from settings file IO so
/// tests can inject paths and later packaging can mirror CMT's asset directory
/// behavior without coupling assets to `settings.json` placement.
pub trait AssetResolver {
    /// Reads the configured download source text if the asset is available.
    ///
    /// Returning `Ok(None)` represents a missing asset and causes the settings
    /// store to use the reference-compatible Nexus fallback.
    fn read_download_source(&self) -> io::Result<Option<String>>;
}

/// Filesystem asset resolver rooted at a specific `download-source.txt` path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAssetResolver {
    download_source_path: PathBuf,
}

impl FileAssetResolver {
    /// Creates a resolver for an explicit `download-source.txt` path.
    pub fn new(download_source_path: PathBuf) -> Self {
        Self { download_source_path }
    }

    /// Creates the production resolver for `assets/download-source.txt`.
    ///
    /// This mirrors `CMT/src/utils.py::get_asset_path` for non-PyInstaller runs:
    /// assets are resolved under the current directory's `assets` folder, not
    /// beside `settings.json`.
    pub fn production() -> Self {
        Self::new(PathBuf::from("assets").join(DOWNLOAD_SOURCE_FILE_NAME))
    }
}

impl AssetResolver for FileAssetResolver {
    fn read_download_source(&self) -> io::Result<Option<String>> {
        match fs::read_to_string(&self.download_source_path) {
            Ok(source) => Ok(Some(source.trim().to_owned())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

/// Test and controller helper that returns a fixed asset value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticAssetResolver {
    download_source: Option<String>,
}

impl StaticAssetResolver {
    /// Creates a resolver that returns the provided download source text.
    pub fn new(download_source: Option<&str>) -> Self {
        Self {
            download_source: download_source.map(str::to_owned),
        }
    }
}

impl AssetResolver for StaticAssetResolver {
    fn read_download_source(&self) -> io::Result<Option<String>> {
        Ok(self.download_source.clone())
    }
}

/// Settings and asset paths used by [`SettingsStore`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPaths<R = FileAssetResolver> {
    settings_path: PathBuf,
    asset_resolver: R,
}

impl SettingsPaths<FileAssetResolver> {
    /// Creates production paths using current-directory `settings.json`.
    pub fn production() -> Self {
        Self {
            settings_path: PathBuf::from(SETTINGS_FILE_NAME),
            asset_resolver: FileAssetResolver::production(),
        }
    }
}

impl<R: AssetResolver> SettingsPaths<R> {
    /// Creates injectable paths for tests or controller-owned settings files.
    pub fn injected(settings_path: PathBuf, asset_resolver: R) -> Self {
        Self {
            settings_path,
            asset_resolver,
        }
    }

    /// Returns the settings JSON path used by the store.
    pub fn settings_path(&self) -> PathBuf {
        self.settings_path.clone()
    }
}

/// Filesystem-backed load/save boundary for [`crate::domain::settings::AppSettings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsStore<R = FileAssetResolver> {
    paths: SettingsPaths<R>,
}

impl SettingsStore<FileAssetResolver> {
    /// Creates a store with production current-directory settings and asset paths.
    pub fn production() -> Self {
        Self::new(SettingsPaths::production())
    }
}

impl<R: AssetResolver> SettingsStore<R> {
    /// Creates a store from explicit settings paths.
    pub fn new(paths: SettingsPaths<R>) -> Self {
        Self { paths }
    }

    /// Creates a store from a settings path and asset resolver.
    pub fn with_asset_resolver(settings_path: PathBuf, asset_resolver: R) -> Self {
        Self::new(SettingsPaths::injected(settings_path, asset_resolver))
    }

    /// Returns the settings file path without touching the filesystem.
    pub fn settings_path(&self) -> &Path {
        &self.paths.settings_path
    }

    /// Resolves the default update source through `download-source.txt`.
    ///
    /// Missing, unreadable, or invalid content falls back to Nexus so a damaged
    /// packaged asset never prevents startup.
    pub fn default_update_source(&self) -> UpdateSource {
        self.paths
            .asset_resolver
            .read_download_source()
            .ok()
            .flatten()
            .as_deref()
            .and_then(UpdateSource::from_wire_value)
            .unwrap_or(UpdateSource::Nexus)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

    use crate::domain::settings::{AppSettings, LogLevel, UpdateSource};

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

    #[test]
    fn settings_missing_file_defaults() {
        let (_root, settings_path) = isolated_settings_path("missing-defaults");
        let store = SettingsStore::with_asset_resolver(
            settings_path.clone(),
            StaticAssetResolver::new(Some("github")),
        );

        let loaded = store.load().expect("missing settings should create defaults");

        assert_eq!(loaded.settings.update_source, UpdateSource::Github);
        assert!(settings_path.is_file());
        let persisted = fs::read_to_string(settings_path).expect("defaults should be persisted");
        assert_eq!(persisted_json(&persisted)["update_source"], "github");
    }

    #[test]
    fn settings_repair_malformed_json_resets_to_defaults() {
        let (_root, settings_path) = isolated_settings_path("malformed-reset");
        fs::write(&settings_path, "{ not json").expect("test fixture should write malformed JSON");
        let store = SettingsStore::with_asset_resolver(
            settings_path.clone(),
            StaticAssetResolver::new(Some("both")),
        );

        let loaded = store.load().expect("malformed JSON should reset to defaults");

        assert!(loaded.reset_to_defaults);
        assert_eq!(loaded.settings.update_source, UpdateSource::Both);
        let persisted = fs::read_to_string(settings_path).expect("defaults should replace malformed JSON");
        assert_eq!(persisted_json(&persisted)["update_source"], "both");
    }

    #[test]
    fn settings_repair_partial_json_preserves_valid_fields_and_removes_unknown_keys() {
        let (_root, settings_path) = isolated_settings_path("partial-repair");
        fs::write(
            &settings_path,
            r#"{
                "log_level": "WARNING",
                "update_source": "github",
                "scanner_OverviewIssues": false,
                "scanner_Errors": true,
                "unknown_setting": true
            }"#,
        )
        .expect("test fixture should write partial JSON");
        let store = SettingsStore::with_asset_resolver(
            settings_path.clone(),
            StaticAssetResolver::new(Some("nexus")),
        );

        let loaded = store.load().expect("partial JSON should repair and resave");

        assert_eq!(loaded.settings.log_level, LogLevel::Info);
        assert_eq!(loaded.settings.update_source, UpdateSource::Github);
        assert!(!loaded.settings.scanner.overview_issues);
        let persisted = fs::read_to_string(settings_path).expect("repaired settings should be persisted");
        let persisted_json = persisted_json(&persisted);
        assert_eq!(persisted_json["log_level"], "INFO");
        assert_eq!(persisted_json["update_source"], "github");
        assert!(persisted_json.get("unknown_setting").is_none());
    }

    #[test]
    fn settings_persist_reference_keys() {
        let (_root, settings_path) = isolated_settings_path("persist-reference-keys");
        let store = SettingsStore::with_asset_resolver(
            settings_path.clone(),
            StaticAssetResolver::new(Some("nexus")),
        );

        store.save(&AppSettings::default()).expect("default settings should save");

        let persisted = fs::read_to_string(settings_path).expect("settings should be readable");
        let object = persisted_json(&persisted);
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "downgrader_delete_deltas",
                "downgrader_keep_backups",
                "log_level",
                "scanner_Errors",
                "scanner_JunkFiles",
                "scanner_LoosePrevis",
                "scanner_OverviewIssues",
                "scanner_ProblemOverrides",
                "scanner_RaceSubgraphs",
                "scanner_WrongFormat",
                "update_source",
            ]
        );
        assert_eq!(LogLevel::Debug.as_wire_value(), "DEBUG");
        assert_eq!(LogLevel::Info.as_wire_value(), "INFO");
        assert_eq!(LogLevel::Error.as_wire_value(), "ERROR");
    }

    #[test]
    fn settings_save_failure_is_returned() {
        let (_root, settings_path) = isolated_settings_path("save-failure");
        fs::create_dir_all(&settings_path).expect("directory at settings path should block file write");
        let store = SettingsStore::with_asset_resolver(settings_path, StaticAssetResolver::new(None));

        let error = store
            .save(&AppSettings::default())
            .expect_err("save should return an observable filesystem error");

        assert_eq!(error.kind(), io::ErrorKind::IsADirectory);
    }

    fn isolated_settings_path(case_name: &str) -> (PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cmt-rs-{case_name}-{unique}"));
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
