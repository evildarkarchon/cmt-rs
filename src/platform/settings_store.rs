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
