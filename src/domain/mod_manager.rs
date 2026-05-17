//! Pure domain contracts for mod-manager discovery results.
//!
//! The reference app detects Mod Organizer 2 and Vortex from process ancestry,
//! then only reads MO2 configuration. These contracts capture that scope without
//! performing process inspection, filesystem reads, or INI parsing themselves.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Path, PathBuf},
};

use crate::domain::discovery::SemanticVersion;

/// Exact display names for supported mod managers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModManagerKind {
    /// Mod Organizer 2, displayed by the reference as `Mod Organizer`.
    ModOrganizer,
    /// Vortex, detected but not deeply parsed by the reference app.
    Vortex,
}

impl ModManagerKind {
    /// Returns the exact reference display name.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::ModOrganizer => "Mod Organizer",
            Self::Vortex => "Vortex",
        }
    }

    /// Returns the executable name used by the reference process detection.
    pub const fn executable_name(self) -> &'static str {
        match self {
            Self::ModOrganizer => "ModOrganizer.exe",
            Self::Vortex => "Vortex.exe",
        }
    }
}

impl fmt::Display for ModManagerKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}

/// A mod manager executable detected by platform adapters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DetectedModManager {
    /// Kind of manager detected.
    pub kind: ModManagerKind,
    /// Absolute or adapter-supplied executable path.
    pub executable_path: PathBuf,
    /// Three-part executable version, or `0.0.0` when unavailable.
    pub version: SemanticVersion,
}

impl DetectedModManager {
    /// Creates a detected manager from already-collected adapter data.
    pub fn new(
        kind: ModManagerKind,
        executable_path: impl Into<PathBuf>,
        version: SemanticVersion,
    ) -> Self {
        Self {
            kind,
            executable_path: executable_path.into(),
            version,
        }
    }

    /// Creates a Mod Organizer detection result.
    pub fn mod_organizer(executable_path: impl Into<PathBuf>, version: SemanticVersion) -> Self {
        Self::new(ModManagerKind::ModOrganizer, executable_path, version)
    }

    /// Creates a Vortex detection result with the reference `0.0.0` fallback.
    pub fn vortex(executable_path: impl Into<PathBuf>, version: Option<SemanticVersion>) -> Self {
        Self::new(
            ModManagerKind::Vortex,
            executable_path,
            version.unwrap_or_else(SemanticVersion::zero),
        )
    }

    /// Returns the exact reference display name for this manager.
    pub const fn display_name(&self) -> &'static str {
        self.kind.display_name()
    }

    /// Returns whether this manager has staging/config parsing in the reference scope.
    pub const fn has_staging_configuration_scope(&self) -> bool {
        matches!(self.kind, ModManagerKind::ModOrganizer)
    }
}

/// Context for Vortex detection.
///
/// The reference recognizes Vortex and displays a warning that it is not fully
/// supported. It does not parse staging folders or Vortex configuration, so this
/// type intentionally carries only the detected executable identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VortexContext {
    /// Detected Vortex executable identity.
    pub manager: DetectedModManager,
}

impl VortexContext {
    /// Creates Vortex context with a `0.0.0` fallback when version metadata is absent.
    pub fn new(executable_path: impl Into<PathBuf>, version: Option<SemanticVersion>) -> Self {
        Self {
            manager: DetectedModManager::vortex(executable_path, version),
        }
    }

    /// Returns false because Vortex staging/config parsing is outside the reference scope.
    pub const fn parses_staging_or_config(&self) -> bool {
        false
    }
}

/// Tool executable categories read from MO2 custom executables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModOrganizerTool {
    /// xEdit or FO4Edit executable.
    XEdit,
    /// BSArch executable inferred from xEdit folders or explicit entries.
    BSArch,
}

impl ModOrganizerTool {
    /// Returns the reference lowercase executable suffixes used for matching.
    pub const fn executable_suffixes(self) -> &'static [&'static str] {
        match self {
            Self::XEdit => &["xedit.exe", "fo4edit.exe"],
            Self::BSArch => &["bsarch.exe"],
        }
    }
}

/// Paths parsed from an MO2 configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModOrganizerDirectories {
    /// MO2 base directory used to expand `%BASE_DIR%` settings.
    pub base_directory: PathBuf,
    /// MO2 cache/webcache directory.
    pub cache_directory: PathBuf,
    /// MO2 downloads directory.
    pub download_directory: PathBuf,
    /// MO2 mods/staging directory.
    pub mod_directory: PathBuf,
    /// MO2 overwrite directory.
    pub overwrite_directory: PathBuf,
    /// MO2 profiles directory.
    pub profiles_directory: PathBuf,
}

impl ModOrganizerDirectories {
    /// Creates explicit MO2 directories from adapter-parsed values.
    pub fn new(
        base_directory: impl Into<PathBuf>,
        cache_directory: impl Into<PathBuf>,
        download_directory: impl Into<PathBuf>,
        mod_directory: impl Into<PathBuf>,
        overwrite_directory: impl Into<PathBuf>,
        profiles_directory: impl Into<PathBuf>,
    ) -> Self {
        Self {
            base_directory: base_directory.into(),
            cache_directory: cache_directory.into(),
            download_directory: download_directory.into(),
            mod_directory: mod_directory.into(),
            overwrite_directory: overwrite_directory.into(),
            profiles_directory: profiles_directory.into(),
        }
    }

    /// Creates the reference MO2 defaults relative to an already-known base directory.
    pub fn reference_defaults(base_directory: impl Into<PathBuf>) -> Self {
        let base_directory = base_directory.into();
        Self {
            cache_directory: base_directory.join("webcache"),
            download_directory: base_directory.join("downloads"),
            mod_directory: base_directory.join("mods"),
            overwrite_directory: base_directory.join("overwrite"),
            profiles_directory: base_directory.join("profiles"),
            base_directory,
        }
    }
}

/// Skip rules parsed from an MO2 configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModOrganizerSkipRules {
    /// Lowercase file suffixes to skip, defaulting to `.mohidden`.
    pub file_suffixes: Vec<String>,
    /// Lowercase directory names to skip.
    pub directories: BTreeSet<String>,
}

impl Default for ModOrganizerSkipRules {
    fn default() -> Self {
        Self {
            file_suffixes: vec![".mohidden".to_owned()],
            directories: BTreeSet::new(),
        }
    }
}

impl ModOrganizerSkipRules {
    /// Creates skip rules while normalizing suffixes and directory names to lowercase.
    pub fn new(
        file_suffixes: impl IntoIterator<Item = impl AsRef<str>>,
        directories: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        Self {
            file_suffixes: file_suffixes
                .into_iter()
                .map(|suffix| suffix.as_ref().to_lowercase())
                .collect(),
            directories: directories
                .into_iter()
                .map(|directory| directory.as_ref().to_lowercase())
                .collect(),
        }
    }
}

/// Parsed MO2 context needed by scanner and overview workflows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModOrganizerContext {
    /// Detected Mod Organizer executable identity.
    pub manager: DetectedModManager,
    /// Optional `ModOrganizer.ini` path used to create this context.
    pub ini_path: Option<PathBuf>,
    /// Optional `portable.txt` marker path when portable mode is detected.
    pub portable_txt_path: Option<PathBuf>,
    /// Whether MO2 portable mode was detected.
    pub portable: bool,
    /// Optional `gamePath` value from `[General]`.
    pub game_path: Option<PathBuf>,
    /// `selected_profile` value from `[General]`.
    pub selected_profile: String,
    /// Directory values parsed from `[Settings]`.
    pub directories: ModOrganizerDirectories,
    /// `profile_local_inis` value from `[Settings]`.
    pub profile_local_inis: bool,
    /// `profile_local_saves` value from `[Settings]`.
    pub profile_local_saves: bool,
    /// MO2 skip rules from `[Settings]`.
    pub skip_rules: ModOrganizerSkipRules,
}

impl ModOrganizerContext {
    /// Creates required MO2 context with reference defaults for optional flags and skip rules.
    pub fn new(
        manager: DetectedModManager,
        selected_profile: impl Into<String>,
        directories: ModOrganizerDirectories,
    ) -> Self {
        Self {
            manager,
            ini_path: None,
            portable_txt_path: None,
            portable: false,
            game_path: None,
            selected_profile: selected_profile.into(),
            directories,
            profile_local_inis: false,
            profile_local_saves: false,
            skip_rules: ModOrganizerSkipRules::default(),
        }
    }

    /// Adds the optional `gamePath` setting.
    pub fn with_game_path(mut self, game_path: impl Into<PathBuf>) -> Self {
        self.game_path = Some(game_path.into());
        self
    }

    /// Adds source INI and portable marker details.
    pub fn with_source_paths(
        mut self,
        ini_path: impl Into<Option<PathBuf>>,
        portable_txt_path: impl Into<Option<PathBuf>>,
        portable: bool,
    ) -> Self {
        self.ini_path = ini_path.into();
        self.portable_txt_path = portable_txt_path.into();
        self.portable = portable;
        self
    }

    /// Adds profile-local flag values parsed from MO2 settings.
    pub const fn with_profile_local_flags(mut self, local_inis: bool, local_saves: bool) -> Self {
        self.profile_local_inis = local_inis;
        self.profile_local_saves = local_saves;
        self
    }

    /// Adds normalized skip rules parsed from MO2 settings.
    pub fn with_skip_rules(mut self, skip_rules: ModOrganizerSkipRules) -> Self {
        self.skip_rules = skip_rules;
        self
    }

    /// Returns the MO2 mods/staging directory.
    pub fn mod_directory(&self) -> &Path {
        &self.directories.mod_directory
    }

    /// Returns the MO2 overwrite directory.
    pub fn overwrite_directory(&self) -> &Path {
        &self.directories.overwrite_directory
    }

    /// Returns the MO2 profiles directory.
    pub fn profiles_directory(&self) -> &Path {
        &self.directories.profiles_directory
    }
}

/// Complete parsed MO2 configuration result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mo2Configuration {
    /// Context required by game discovery and staged scanning.
    pub context: ModOrganizerContext,
    /// Custom executable paths grouped by supported tool category.
    pub executables: BTreeMap<ModOrganizerTool, BTreeSet<PathBuf>>,
}

impl Mo2Configuration {
    /// Creates an MO2 configuration with no custom executables.
    pub fn new(context: ModOrganizerContext) -> Self {
        Self {
            context,
            executables: BTreeMap::new(),
        }
    }
}

/// Result alias for MO2 configuration parsing once adapters are implemented.
pub type Mo2ConfigurationResult = Result<Mo2Configuration, Mo2ParseError>;

/// Supported mod-manager context variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModManagerContext {
    /// Parsed Mod Organizer context.
    ModOrganizer(Box<ModOrganizerContext>),
    /// Detected Vortex context.
    Vortex(VortexContext),
}

impl ModManagerContext {
    /// Returns the exact reference display name for this context.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ModOrganizer(context) => context.manager.display_name(),
            Self::Vortex(context) => context.manager.display_name(),
        }
    }
}

/// Typed MO2 parse/discovery failure with safe user-facing text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mo2ParseError {
    /// Typed MO2 failure category.
    pub kind: Mo2ParseErrorKind,
    /// Non-user-facing detail for logs or diagnostics.
    pub diagnostic: Option<String>,
}

impl Mo2ParseError {
    /// Creates the portable marker error from `GameInfo.find_path`.
    pub fn portable_marker_without_ini() -> Self {
        Self {
            kind: Mo2ParseErrorKind::PortableMarkerWithoutIni,
            diagnostic: None,
        }
    }

    /// Creates the MO2 INI unavailable error from `GameInfo.find_path`.
    pub fn missing_mod_organizer_ini() -> Self {
        Self {
            kind: Mo2ParseErrorKind::MissingModOrganizerIni,
            diagnostic: None,
        }
    }

    /// Creates the unsupported game-name error whose reference text includes the INI path.
    pub fn unsupported_game_name(
        game_name: impl Into<String>,
        ini_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            kind: Mo2ParseErrorKind::UnsupportedGameName {
                game_name: game_name.into(),
                ini_path: ini_path.into(),
            },
            diagnostic: None,
        }
    }

    /// Creates the missing selected-profile error from `ModManagerInfo.read_mo2_ini`.
    pub fn missing_selected_profile() -> Self {
        Self {
            kind: Mo2ParseErrorKind::MissingSelectedProfile,
            diagnostic: None,
        }
    }

    /// Creates the missing staged-scanner settings error.
    pub fn missing_stage_settings(
        mod_directory: Option<impl Into<PathBuf>>,
        profiles_directory: Option<impl Into<PathBuf>>,
        selected_profile: Option<impl Into<String>>,
        overwrite_directory: Option<impl Into<PathBuf>>,
    ) -> Self {
        Self {
            kind: Mo2ParseErrorKind::MissingStageSettings {
                mod_directory: mod_directory.map(Into::into),
                profiles_directory: profiles_directory.map(Into::into),
                selected_profile: selected_profile.map(Into::into),
                overwrite_directory: overwrite_directory.map(Into::into),
            },
            diagnostic: None,
        }
    }

    /// Creates the missing profile modlist error whose reference text includes the path.
    pub fn missing_modlist(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: Mo2ParseErrorKind::MissingModlist { path: path.into() },
            diagnostic: None,
        }
    }

    /// Returns the manager kind these parse errors belong to.
    pub const fn manager_kind(&self) -> ModManagerKind {
        ModManagerKind::ModOrganizer
    }

    /// Returns the message safe to show to users.
    pub fn user_message(&self) -> String {
        match &self.kind {
            Mo2ParseErrorKind::PortableMarkerWithoutIni => {
                "portable.txt found but no ModOrganizer.ini found in MO2 install path".to_owned()
            }
            Mo2ParseErrorKind::MissingModOrganizerIni => {
                "Unable to find ModOrganizer.ini. Please report this along with your MO2 instance details.".to_owned()
            }
            Mo2ParseErrorKind::UnsupportedGameName {
                game_name,
                ini_path,
            } => format!(
                "Only Fallout 4 is supported.\ngameName is '{game_name}' in INI: \n{}",
                ini_path.display()
            ),
            Mo2ParseErrorKind::MissingSelectedProfile => {
                "Profile is not set in ModOrganizer.ini.".to_owned()
            }
            Mo2ParseErrorKind::MissingStageSettings { .. } => "Missing MO2 settings".to_owned(),
            Mo2ParseErrorKind::MissingModlist { path } => {
                format!("File doesn't exist: {}", path.display())
            }
        }
    }

    /// Returns diagnostics that may include raw paths and context omitted from user messages.
    pub fn diagnostic_message(&self) -> String {
        match &self.kind {
            Mo2ParseErrorKind::MissingStageSettings {
                mod_directory,
                profiles_directory,
                selected_profile,
                overwrite_directory,
            } => format!(
                "Missing MO2 settings\nmods: {}\nprofiles: {}\nprofile: {}\noverwrite: {}",
                display_optional_path(mod_directory.as_deref()),
                display_optional_path(profiles_directory.as_deref()),
                selected_profile.as_deref().unwrap_or("None"),
                display_optional_path(overwrite_directory.as_deref())
            ),
            _ => self.user_message(),
        }
    }

    /// Returns whether the failure should be handled without terminating orchestration.
    pub const fn is_recoverable(&self) -> bool {
        true
    }
}

impl fmt::Display for Mo2ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.user_message())
    }
}

impl std::error::Error for Mo2ParseError {}

/// Manager-specific MO2 parse/discovery failure categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mo2ParseErrorKind {
    /// `portable.txt` existed but no adjacent `ModOrganizer.ini` existed.
    PortableMarkerWithoutIni,
    /// No usable `ModOrganizer.ini` could be found for the current instance.
    MissingModOrganizerIni,
    /// The INI specified a non-Fallout 4 game.
    UnsupportedGameName {
        /// Unsupported `gameName` value.
        game_name: String,
        /// INI path intentionally included by the reference error message.
        ini_path: PathBuf,
    },
    /// The INI did not contain `selected_profile`.
    MissingSelectedProfile,
    /// Required staged-scanning MO2 settings were absent.
    MissingStageSettings {
        /// Optional mods/staging directory.
        mod_directory: Option<PathBuf>,
        /// Optional profiles directory.
        profiles_directory: Option<PathBuf>,
        /// Optional selected profile.
        selected_profile: Option<String>,
        /// Optional overwrite directory.
        overwrite_directory: Option<PathBuf>,
    },
    /// The selected profile did not contain `modlist.txt`.
    MissingModlist {
        /// Missing modlist path intentionally included by the reference error message.
        path: PathBuf,
    },
}

fn display_optional_path(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "None".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mo2_manager() -> DetectedModManager {
        DetectedModManager::mod_organizer(
            "C:/Modding/MO2/ModOrganizer.exe",
            SemanticVersion::new(2, 5, 2),
        )
    }

    #[test]
    fn manager_kind_display_names_and_executables_match_reference() {
        assert_eq!(ModManagerKind::ModOrganizer.display_name(), "Mod Organizer");
        assert_eq!(
            ModManagerKind::ModOrganizer.executable_name(),
            "ModOrganizer.exe"
        );
        assert_eq!(ModManagerKind::Vortex.display_name(), "Vortex");
        assert_eq!(ModManagerKind::Vortex.executable_name(), "Vortex.exe");
    }

    #[test]
    fn mo2_context_carries_required_reference_fields() {
        let directories = ModOrganizerDirectories::new(
            "C:/Modding/MO2",
            "C:/Modding/MO2/webcache",
            "C:/Downloads",
            "C:/MO2Mods",
            "C:/MO2Overwrite",
            "C:/MO2Profiles",
        );
        let context = ModOrganizerContext::new(mo2_manager(), "Default", directories)
            .with_game_path("C:/Games/Fallout 4")
            .with_source_paths(
                Some(PathBuf::from("C:/Modding/MO2/ModOrganizer.ini")),
                Some(PathBuf::from("C:/Modding/MO2/portable.txt")),
                true,
            )
            .with_profile_local_flags(true, false)
            .with_skip_rules(ModOrganizerSkipRules::new(
                [".MOHIDDEN", ".Bak"],
                ["Cache", "Temp"],
            ));

        assert_eq!(context.manager.display_name(), "Mod Organizer");
        assert_eq!(
            context.game_path.as_deref(),
            Some(Path::new("C:/Games/Fallout 4"))
        );
        assert_eq!(context.selected_profile, "Default");
        assert_eq!(context.mod_directory(), Path::new("C:/MO2Mods"));
        assert_eq!(context.overwrite_directory(), Path::new("C:/MO2Overwrite"));
        assert_eq!(context.profiles_directory(), Path::new("C:/MO2Profiles"));
        assert!(context.profile_local_inis);
        assert!(!context.profile_local_saves);
        assert!(context.portable);
        assert_eq!(context.skip_rules.file_suffixes, vec![".mohidden", ".bak"]);
        assert!(context.skip_rules.directories.contains("cache"));
        assert!(context.skip_rules.directories.contains("temp"));
    }

    #[test]
    fn mo2_reference_defaults_match_python_defaults_without_filesystem_access() {
        let directories = ModOrganizerDirectories::reference_defaults("C:/Modding/MO2");
        let context = ModOrganizerContext::new(mo2_manager(), "Default", directories);

        assert_eq!(
            context.directories.cache_directory,
            PathBuf::from("C:/Modding/MO2/webcache")
        );
        assert_eq!(
            context.directories.download_directory,
            PathBuf::from("C:/Modding/MO2/downloads")
        );
        assert_eq!(context.mod_directory(), Path::new("C:/Modding/MO2/mods"));
        assert_eq!(
            context.overwrite_directory(),
            Path::new("C:/Modding/MO2/overwrite")
        );
        assert_eq!(
            context.profiles_directory(),
            Path::new("C:/Modding/MO2/profiles")
        );
        assert_eq!(context.skip_rules.file_suffixes, vec![".mohidden"]);
        assert!(context.skip_rules.directories.is_empty());
    }

    #[test]
    fn vortex_detection_scope_is_identity_only_with_zero_version_fallback() {
        let context =
            VortexContext::new("C:/Program Files/Black Tree Gaming/Vortex/Vortex.exe", None);

        assert_eq!(context.manager.kind, ModManagerKind::Vortex);
        assert_eq!(context.manager.display_name(), "Vortex");
        assert_eq!(
            context.manager.executable_path,
            PathBuf::from("C:/Program Files/Black Tree Gaming/Vortex/Vortex.exe")
        );
        assert_eq!(context.manager.version, SemanticVersion::zero());
        assert!(!context.manager.has_staging_configuration_scope());
        assert!(!context.parses_staging_or_config());
    }

    #[test]
    fn mo2_parse_errors_are_typed_and_reference_compatible() {
        let portable = Mo2ParseError::portable_marker_without_ini();
        assert_eq!(portable.manager_kind(), ModManagerKind::ModOrganizer);
        assert_eq!(portable.kind, Mo2ParseErrorKind::PortableMarkerWithoutIni);
        assert_eq!(
            portable.user_message(),
            "portable.txt found but no ModOrganizer.ini found in MO2 install path"
        );
        assert!(portable.is_recoverable());

        let missing_ini = Mo2ParseError::missing_mod_organizer_ini();
        assert_eq!(
            missing_ini.user_message(),
            "Unable to find ModOrganizer.ini. Please report this along with your MO2 instance details."
        );

        let missing_profile = Mo2ParseError::missing_selected_profile();
        assert_eq!(
            missing_profile.user_message(),
            "Profile is not set in ModOrganizer.ini."
        );
    }

    #[test]
    fn mo2_path_including_messages_match_reference_exceptions() {
        let unsupported = Mo2ParseError::unsupported_game_name(
            "Skyrim Special Edition",
            "C:/Modding/MO2/ModOrganizer.ini",
        );
        assert_eq!(
            unsupported.user_message(),
            "Only Fallout 4 is supported.\ngameName is 'Skyrim Special Edition' in INI: \nC:/Modding/MO2/ModOrganizer.ini"
        );

        let missing_modlist = Mo2ParseError::missing_modlist("C:/MO2Profiles/Default/modlist.txt");
        assert_eq!(
            missing_modlist.user_message(),
            "File doesn't exist: C:/MO2Profiles/Default/modlist.txt"
        );
    }

    #[test]
    fn mo2_stage_setting_paths_stay_in_diagnostics_not_user_message() {
        let error = Mo2ParseError::missing_stage_settings(
            Some("C:/MO2Mods"),
            None::<PathBuf>,
            Some("Default"),
            Some("C:/MO2Overwrite"),
        );

        assert_eq!(error.user_message(), "Missing MO2 settings");
        assert_eq!(
            error.diagnostic_message(),
            "Missing MO2 settings\nmods: C:/MO2Mods\nprofiles: None\nprofile: Default\noverwrite: C:/MO2Overwrite"
        );
        assert!(!error.user_message().contains("C:/"));
    }

    #[test]
    fn mo2_configuration_result_can_hold_context_and_executables() {
        let context = ModOrganizerContext::new(
            mo2_manager(),
            "Default",
            ModOrganizerDirectories::reference_defaults("C:/Modding/MO2"),
        );
        let mut configuration = Mo2Configuration::new(context);
        configuration
            .executables
            .entry(ModOrganizerTool::XEdit)
            .or_default()
            .insert(PathBuf::from("C:/Tools/FO4Edit.exe"));

        let result: Mo2ConfigurationResult = Ok(configuration);
        let configuration = match result {
            Ok(configuration) => configuration,
            Err(error) => panic!("inline MO2 configuration should be valid: {error}"),
        };
        assert!(
            configuration.executables[&ModOrganizerTool::XEdit]
                .contains(&PathBuf::from("C:/Tools/FO4Edit.exe"))
        );
    }
}
