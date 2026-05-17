//! Pure domain contracts for Fallout 4 discovery state.
//!
//! These types deliberately do not touch the filesystem, registry, processes, or
//! Slint. Later platform adapters can populate them from real discovery work
//! while tests can construct them from inline paths and values.

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

/// Reference game identifier used by the original Python `GameInfo` object.
pub const FALLOUT4_GAME_ID: &str = "Fallout4";
/// Executable name used to recognize a Fallout 4 installation directory.
pub const FALLOUT4_EXECUTABLE: &str = "Fallout4.exe";
/// Reference-compatible message shown when no valid game path is available.
pub const FALLOUT4_NOT_FOUND_MESSAGE: &str = "A Fallout 4 installation could not be found.";
/// Reference-compatible loading error used by Overview, F4SE, and Scanner paths.
pub const DATA_FOLDER_NOT_FOUND_MESSAGE: &str = "Data folder not found";
/// Reference-compatible F4SE loading error prefix.
pub const F4SE_PLUGINS_NOT_FOUND_MESSAGE: &str = "Data/F4SE/Plugins folder not found";

/// Semantic version used for tools and discovered executables.
///
/// The reference app uses Python's `packaging.version.Version` for manager
/// versions. The current Rust domain contract only needs a stable three-part
/// semantic version plus a `0.0.0` fallback for unknown executable metadata.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticVersion {
    /// Major version component.
    pub major: u64,
    /// Minor version component.
    pub minor: u64,
    /// Patch version component.
    pub patch: u64,
}

impl SemanticVersion {
    /// Creates a semantic version from major, minor, and patch components.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns the reference-compatible fallback used when file metadata is unavailable.
    pub const fn zero() -> Self {
        Self::new(0, 0, 0)
    }
}

impl fmt::Display for SemanticVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Reference install-type labels from `CMT/src/enums.py`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fallout4InstallType {
    /// Reference `Obsolete` state.
    Obsolete,
    /// Reference `Old-Gen` state.
    OldGen,
    /// Reference `Down-Grade` state.
    DownGrade,
    /// Reference `Next-Gen` state.
    NextGen,
    /// Reference `Anniversary` state.
    Anniversary,
    /// Reference `Next-Gen & Anniversary` state.
    NextGenAnniversary,
    /// Reference default state before binary classification runs.
    #[default]
    Unknown,
    /// Reference state used when a base executable is absent.
    NotFound,
}

impl Fallout4InstallType {
    /// Returns the exact user-facing label used by the reference enum.
    pub const fn as_reference_str(self) -> &'static str {
        match self {
            Self::Obsolete => "Obsolete",
            Self::OldGen => "Old-Gen",
            Self::DownGrade => "Down-Grade",
            Self::NextGen => "Next-Gen",
            Self::Anniversary => "Anniversary",
            Self::NextGenAnniversary => "Next-Gen & Anniversary",
            Self::Unknown => "Unknown",
            Self::NotFound => "Not Found",
        }
    }
}

impl fmt::Display for Fallout4InstallType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_reference_str())
    }
}

/// Pure representation of a discovered Fallout 4 installation.
///
/// `game_path` is the only required path. `data_path` and `f4se_plugins_path`
/// remain optional because the reference app can have a valid game executable
/// while Data or Data/F4SE/Plugins are absent, producing recoverable tab-level
/// loading messages instead of invalidating the installation itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fallout4Installation {
    /// Directory containing `Fallout4.exe`.
    pub game_path: PathBuf,
    /// Optional `Data` directory if a platform adapter has confirmed one.
    pub data_path: Option<PathBuf>,
    /// Optional `Data/F4SE/Plugins` directory if confirmed.
    pub f4se_plugins_path: Option<PathBuf>,
    /// Binary classification state for the current install.
    pub install_type: Fallout4InstallType,
    /// Archive records observed by later scanner/overview workflows.
    pub archives: Vec<ArchiveRecord>,
    /// Plugin/module records observed by later scanner/overview workflows.
    pub modules: Vec<ModuleRecord>,
    /// Parsed Fallout 4 INI state from the user profile.
    pub ini_files: Fallout4IniFiles,
}

impl Fallout4Installation {
    /// Creates an installation with only the required game path.
    ///
    /// This constructor performs no existence checks and intentionally does not
    /// infer `Data` or `F4SE` paths, preserving the Phase 3 contract that those
    /// derived paths are optional facts supplied by discovery adapters.
    pub fn new(game_path: impl Into<PathBuf>) -> Self {
        Self {
            game_path: game_path.into(),
            data_path: None,
            f4se_plugins_path: None,
            install_type: Fallout4InstallType::Unknown,
            archives: Vec::new(),
            modules: Vec::new(),
            ini_files: Fallout4IniFiles::default(),
        }
    }

    /// Creates an installation with optional derived paths supplied by an adapter.
    pub fn with_optional_paths(
        game_path: impl Into<PathBuf>,
        data_path: Option<impl Into<PathBuf>>,
        f4se_plugins_path: Option<impl Into<PathBuf>>,
    ) -> Self {
        let mut installation = Self::new(game_path);
        installation.data_path = data_path.map(Into::into);
        installation.f4se_plugins_path = f4se_plugins_path.map(Into::into);
        installation
    }

    /// Returns the reference game identifier for this installation contract.
    pub const fn game_id(&self) -> &'static str {
        FALLOUT4_GAME_ID
    }

    /// Returns whether an adapter has confirmed a Data directory.
    pub const fn has_data_path(&self) -> bool {
        self.data_path.is_some()
    }

    /// Returns whether an adapter has confirmed a Data/F4SE/Plugins directory.
    pub const fn has_f4se_plugins_path(&self) -> bool {
        self.f4se_plugins_path.is_some()
    }
}

/// Archive file format magic recognized by the reference scanner.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArchiveFormat {
    /// General BA2 archive (`GNRL`).
    General,
    /// DirectX 10 texture BA2 archive (`DX10`).
    DirectX10,
    /// Any other archive format marker retained for diagnostics.
    Unknown(String),
}

impl ArchiveFormat {
    /// Returns the reference byte marker text when the format is known.
    pub const fn as_reference_magic(&self) -> Option<&'static str> {
        match self {
            Self::General => Some("GNRL"),
            Self::DirectX10 => Some("DX10"),
            Self::Unknown(_) => None,
        }
    }
}

/// BA2 version classes used by the archive overview and patcher workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchiveVersion {
    /// v1, required by Old-Gen Fallout 4 and accepted by all versions.
    OldGen,
    /// v7, initial Next-Gen archive version.
    NextGen7,
    /// v8, current Next-Gen archive version.
    NextGen8,
    /// Any other version byte retained for diagnostics.
    Unknown(u32),
}

impl ArchiveVersion {
    /// Converts the typed archive version into the numeric value in the BA2 header.
    pub const fn as_header_value(self) -> u32 {
        match self {
            Self::OldGen => 1,
            Self::NextGen7 => 7,
            Self::NextGen8 => 8,
            Self::Unknown(value) => value,
        }
    }
}

/// Single archive record carried by scanner and overview workflows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArchiveRecord {
    /// Archive path as discovered by a platform/scanner adapter.
    pub path: PathBuf,
    /// Parsed archive format marker.
    pub format: ArchiveFormat,
    /// Parsed archive version.
    pub version: ArchiveVersion,
    /// Whether later enablement parsing found this archive active.
    pub enabled: bool,
    /// Whether the scanner could read enough bytes to classify the archive.
    pub readable: bool,
}

impl ArchiveRecord {
    /// Creates a readable archive record with an explicit enabled state.
    pub fn new(
        path: impl Into<PathBuf>,
        format: ArchiveFormat,
        version: ArchiveVersion,
        enabled: bool,
    ) -> Self {
        Self {
            path: path.into(),
            format,
            version,
            enabled,
            readable: true,
        }
    }

    /// Creates an unreadable archive record while preserving its path.
    pub fn unreadable(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            format: ArchiveFormat::Unknown(String::new()),
            version: ArchiveVersion::Unknown(0),
            enabled: false,
            readable: false,
        }
    }
}

/// Module/header classes used for ESP/ESM/ESL overview state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleHeaderVersion {
    /// Reference-supported HEDR v0.95.
    Version095,
    /// Reference-supported HEDR v1.00.
    Version100,
    /// Any unsupported header value retained as text for diagnostics.
    Unknown(String),
}

impl ModuleHeaderVersion {
    /// Returns the reference display text for known module header versions.
    pub const fn as_reference_str(&self) -> Option<&'static str> {
        match self {
            Self::Version095 => Some("0.95"),
            Self::Version100 => Some("1.00"),
            Self::Unknown(_) => None,
        }
    }
}

/// Plugin/module kind inferred from flags and extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleKind {
    /// Full plugin/module.
    Full,
    /// Light plugin/module, matching the reference light flag grouping.
    Light,
}

/// Single plugin/module record carried by scanner and overview workflows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleRecord {
    /// Module path as discovered by a platform/scanner adapter.
    pub path: PathBuf,
    /// Full or light module classification.
    pub kind: ModuleKind,
    /// Parsed HEDR/header version.
    pub header_version: ModuleHeaderVersion,
    /// Whether plugin enablement parsing found this module active.
    pub enabled: bool,
    /// Whether the scanner could read enough bytes to classify the module.
    pub readable: bool,
}

impl ModuleRecord {
    /// Creates a readable module record with explicit enablement state.
    pub fn new(
        path: impl Into<PathBuf>,
        kind: ModuleKind,
        header_version: ModuleHeaderVersion,
        enabled: bool,
    ) -> Self {
        Self {
            path: path.into(),
            kind,
            header_version,
            enabled,
            readable: true,
        }
    }

    /// Creates an unreadable module record while preserving its path.
    pub fn unreadable(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: ModuleKind::Full,
            header_version: ModuleHeaderVersion::Unknown(String::new()),
            enabled: false,
            readable: false,
        }
    }
}

/// Parsed key/value contents for a single Fallout 4 INI file.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IniDocument {
    /// Optional source path used for diagnostics and later reload operations.
    pub source_path: Option<PathBuf>,
    /// Lowercase section names mapped to lowercase setting keys and raw values.
    pub sections: BTreeMap<String, BTreeMap<String, String>>,
}

impl IniDocument {
    /// Creates an empty INI document with an optional source path.
    pub fn new(source_path: Option<impl Into<PathBuf>>) -> Self {
        Self {
            source_path: source_path.map(Into::into),
            sections: BTreeMap::new(),
        }
    }

    /// Returns a setting value by already-normalized section and key names.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections
            .get(section)
            .and_then(|settings| settings.get(key))
            .map(String::as_str)
    }
}

/// Parsed Fallout 4 INI documents used by overview/scanner behavior.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Fallout4IniFiles {
    /// Parsed `Fallout4.ini` settings.
    pub fallout4: IniDocument,
    /// Parsed `Fallout4Prefs.ini` settings.
    pub prefs: IniDocument,
    /// Parsed `Fallout4Custom.ini` settings.
    pub custom: IniDocument,
}

/// Recoverable discovery failure with a safe user-facing message.
///
/// Raw OS details should be kept in `diagnostic` unless a reference message
/// intentionally includes a path, such as the invalid registry path and missing
/// environment-folder cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryError {
    /// Typed failure category.
    pub kind: DiscoveryErrorKind,
    /// Non-user-facing detail for logs or diagnostics.
    pub diagnostic: Option<String>,
}

impl DiscoveryError {
    /// Creates the generic reference-compatible Fallout 4 not-found error.
    pub fn fallout4_not_found(diagnostic: impl Into<Option<String>>) -> Self {
        Self {
            kind: DiscoveryErrorKind::Fallout4NotFound,
            diagnostic: diagnostic.into(),
        }
    }

    /// Creates the invalid-registry-path error whose reference text includes the path.
    pub fn invalid_registry_path(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: DiscoveryErrorKind::InvalidRegistryPath { path: path.into() },
            diagnostic: None,
        }
    }

    /// Creates the environment-folder missing error whose reference text includes the path.
    pub fn environment_folder_missing(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: DiscoveryErrorKind::EnvironmentFolderMissing { path: path.into() },
            diagnostic: None,
        }
    }

    /// Creates the reference-compatible Data folder loading error.
    pub fn data_folder_not_found() -> Self {
        Self {
            kind: DiscoveryErrorKind::DataFolderNotFound,
            diagnostic: None,
        }
    }

    /// Creates the reference-compatible F4SE plugins loading error.
    pub fn f4se_plugins_folder_not_found(mod_manager_detected: bool) -> Self {
        Self {
            kind: DiscoveryErrorKind::F4sePluginsFolderNotFound {
                mod_manager_detected,
            },
            diagnostic: None,
        }
    }

    /// Returns the message safe to show to users.
    pub fn user_message(&self) -> String {
        match &self.kind {
            DiscoveryErrorKind::Fallout4NotFound => FALLOUT4_NOT_FOUND_MESSAGE.to_owned(),
            DiscoveryErrorKind::InvalidRegistryPath { path } => format!(
                "A Fallout 4 installation could not be found.\n\nThe path set in your registry is:\n{}\n\nIf this is not correct, please run the Fallout 4 Launcher to correct it.",
                display_path(path)
            ),
            DiscoveryErrorKind::EnvironmentFolderMissing { path } => {
                format!("Folder does not exist:\n{}", display_path(path))
            }
            DiscoveryErrorKind::DataFolderNotFound => DATA_FOLDER_NOT_FOUND_MESSAGE.to_owned(),
            DiscoveryErrorKind::F4sePluginsFolderNotFound {
                mod_manager_detected,
            } => {
                if *mod_manager_detected {
                    F4SE_PLUGINS_NOT_FOUND_MESSAGE.to_owned()
                } else {
                    format!("{F4SE_PLUGINS_NOT_FOUND_MESSAGE}\nTry launching via your mod manager.")
                }
            }
        }
    }

    /// Returns whether the failure should be handled without terminating discovery orchestration.
    pub const fn is_recoverable(&self) -> bool {
        true
    }

    /// Returns whether resolving this error requires a manual file-picker UI.
    ///
    /// Phase 3 adapters should report the error and let higher layers decide how
    /// to recover; the domain contract itself does not mandate the reference
    /// Tkinter file-picker prompt.
    pub const fn requires_manual_file_picker(&self) -> bool {
        false
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.user_message())
    }
}

impl std::error::Error for DiscoveryError {}

/// Typed discovery failure categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryErrorKind {
    /// No valid Fallout 4 installation path could be found.
    Fallout4NotFound,
    /// A registry value was present but did not identify a valid Fallout 4 directory.
    InvalidRegistryPath {
        /// Registry path text intentionally included by the reference user message.
        path: PathBuf,
    },
    /// A required Windows known folder did not exist.
    EnvironmentFolderMissing {
        /// Missing folder path intentionally included by the reference helper.
        path: PathBuf,
    },
    /// A tab/workflow required Data but discovery did not provide it.
    DataFolderNotFound,
    /// A tab/workflow required Data/F4SE/Plugins but discovery did not provide it.
    F4sePluginsFolderNotFound {
        /// Whether a mod manager was already detected, matching the reference hint behavior.
        mod_manager_detected: bool,
    },
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallout4_installation_does_not_require_data_or_f4se_paths() {
        let installation = Fallout4Installation::new("C:/Games/Fallout 4");

        assert_eq!(installation.game_id(), "Fallout4");
        assert_eq!(installation.game_path, PathBuf::from("C:/Games/Fallout 4"));
        assert_eq!(installation.data_path, None);
        assert_eq!(installation.f4se_plugins_path, None);
        assert!(!installation.has_data_path());
        assert!(!installation.has_f4se_plugins_path());
        assert_eq!(installation.install_type.as_reference_str(), "Unknown");
    }

    #[test]
    fn fallout4_installation_can_carry_optional_derived_paths() {
        let installation = Fallout4Installation::with_optional_paths(
            "C:/Games/Fallout 4",
            Some("C:/Games/Fallout 4/Data"),
            Some("C:/Games/Fallout 4/Data/F4SE/Plugins"),
        );

        assert_eq!(
            installation.data_path.as_deref(),
            Some(Path::new("C:/Games/Fallout 4/Data"))
        );
        assert_eq!(
            installation.f4se_plugins_path.as_deref(),
            Some(Path::new("C:/Games/Fallout 4/Data/F4SE/Plugins"))
        );
        assert!(installation.has_data_path());
        assert!(installation.has_f4se_plugins_path());
    }

    #[test]
    fn archive_module_and_ini_records_are_pure_domain_state() {
        let mut installation = Fallout4Installation::new("C:/Games/Fallout 4");
        installation.archives.push(ArchiveRecord::new(
            "C:/Games/Fallout 4/Data/Fallout4 - Textures.ba2",
            ArchiveFormat::DirectX10,
            ArchiveVersion::NextGen8,
            true,
        ));
        installation.modules.push(ModuleRecord::new(
            "C:/Games/Fallout 4/Data/Example.esl",
            ModuleKind::Light,
            ModuleHeaderVersion::Version100,
            true,
        ));
        installation
            .ini_files
            .fallout4
            .sections
            .entry("general".to_owned())
            .or_default()
            .insert("slanguage".to_owned(), "en".to_owned());

        assert_eq!(
            installation.archives[0].format.as_reference_magic(),
            Some("DX10")
        );
        assert_eq!(installation.archives[0].version.as_header_value(), 8);
        assert_eq!(
            installation.modules[0].header_version.as_reference_str(),
            Some("1.00")
        );
        assert_eq!(
            installation.ini_files.fallout4.get("general", "slanguage"),
            Some("en")
        );
    }

    #[test]
    fn install_type_and_semantic_version_reference_display_values_are_stable() {
        assert_eq!(Fallout4InstallType::OldGen.to_string(), "Old-Gen");
        assert_eq!(Fallout4InstallType::DownGrade.to_string(), "Down-Grade");
        assert_eq!(
            Fallout4InstallType::NextGenAnniversary.to_string(),
            "Next-Gen & Anniversary"
        );
        assert_eq!(Fallout4InstallType::NotFound.to_string(), "Not Found");
        assert_eq!(SemanticVersion::new(2, 5, 2).to_string(), "2.5.2");
        assert_eq!(SemanticVersion::zero().to_string(), "0.0.0");
    }

    #[test]
    fn discovery_not_found_error_is_recoverable_and_does_not_require_file_picker() {
        let error = DiscoveryError::fallout4_not_found(Some(
            "registry read failed: access denied to HKLM path".to_owned(),
        ));

        assert_eq!(error.kind, DiscoveryErrorKind::Fallout4NotFound);
        assert_eq!(
            error.user_message(),
            "A Fallout 4 installation could not be found."
        );
        assert!(error.is_recoverable());
        assert!(!error.requires_manual_file_picker());
        assert_eq!(
            error.diagnostic.as_deref(),
            Some("registry read failed: access denied to HKLM path")
        );
        assert!(!error.user_message().contains("HKLM"));
    }

    #[test]
    fn discovery_messages_keep_reference_path_exceptions() {
        let registry_error = DiscoveryError::invalid_registry_path("D:/Moved/Fallout 4");
        assert_eq!(
            registry_error.user_message(),
            "A Fallout 4 installation could not be found.\n\nThe path set in your registry is:\nD:/Moved/Fallout 4\n\nIf this is not correct, please run the Fallout 4 Launcher to correct it."
        );

        let known_folder_error =
            DiscoveryError::environment_folder_missing("C:/Users/Example/Documents");
        assert_eq!(
            known_folder_error.user_message(),
            "Folder does not exist:\nC:/Users/Example/Documents"
        );
    }

    #[test]
    fn tab_loading_error_messages_match_reference_text() {
        assert_eq!(
            DiscoveryError::data_folder_not_found().user_message(),
            "Data folder not found"
        );
        assert_eq!(
            DiscoveryError::f4se_plugins_folder_not_found(true).user_message(),
            "Data/F4SE/Plugins folder not found"
        );
        assert_eq!(
            DiscoveryError::f4se_plugins_folder_not_found(false).user_message(),
            "Data/F4SE/Plugins folder not found\nTry launching via your mod manager."
        );
    }
}
