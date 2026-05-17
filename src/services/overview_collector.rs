//! Adapter-backed Overview filesystem fact collector.
//!
//! The pure diagnostics service in [`crate::services::overview`] expects typed
//! binary, archive, module, and enablement facts. This module performs the
//! filesystem/process-adapter work needed to collect those facts without
//! mutating user files, launching discovered binaries, or depending on Slint.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use crc32fast::Hasher;
use tracing::{debug, info_span};

use crate::{
    domain::discovery::{
        ArchiveFormat, ArchiveRecord, ArchiveVersion, Fallout4IniFiles, Fallout4InstallType,
        Fallout4Installation, ModuleHeaderVersion, ModuleKind, ModuleRecord,
    },
    platform::{
        PlatformError, PlatformErrorKind,
        filesystem::{DirectoryEntry, FileType, Filesystem},
        process::{ProcessInspector, VersionMetadata},
    },
    services::overview::{
        OverviewAddressLibraryFact, OverviewBinaryFact, OverviewEnablementFacts,
        OverviewFilePresence,
    },
};

const BA2_HEADER_LEN: usize = 12;
const MODULE_HEADER_LEN: usize = 34;
const MODULE_LIGHT_FLAG: u32 = 0x0200;
const NG_STARTUP_BA2_CRC: &str = "A5808F5F";

const ARCHIVE_LIST_KEYS: [&str; 4] = [
    "sresourceindexfilelist",
    "sresourcestartuparchivelist",
    "sresourcearchivelist",
    "sresourcearchivelist2",
];

const GAME_MASTERS: [&str; 9] = [
    "fallout4.esm",
    "fallout4_vr.esm",
    "dlcrobot.esm",
    "dlcworkshop01.esm",
    "dlcworkshop02.esm",
    "dlcworkshop03.esm",
    "dlccoast.esm",
    "dlcnukaworld.esm",
    "dlcultrahighresolution.esm",
];

const MODULE_VERSION_095: [u8; 4] = [0x33, 0x33, 0x73, 0x3f];
const MODULE_VERSION_100: [u8; 4] = [0x00, 0x00, 0x80, 0x3f];

const BASE_FILES: &[BaseFileDefinition] = &[
    BaseFileDefinition {
        relative_path: "Fallout4.exe",
        classifications: &[
            BaseFileClassification::version("1.10.120.0", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("1.10.130.0", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("1.10.138.0", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("1.10.162.0", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("1.10.163.0", Fallout4InstallType::OldGen),
            BaseFileClassification::version("1.10.980.0", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("1.10.984.0", Fallout4InstallType::NextGen),
            BaseFileClassification::version("1.11.137.0", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("1.11.159.0", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("1.11.169.0", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("1.11.191.0", Fallout4InstallType::Anniversary),
        ],
    },
    BaseFileDefinition {
        relative_path: "Fallout4Launcher.exe",
        classifications: &[
            BaseFileClassification::hash("02445570", Fallout4InstallType::OldGen),
            BaseFileClassification::hash("F6A06FF5", Fallout4InstallType::NextGen),
            BaseFileClassification::hash("0E696744", Fallout4InstallType::Obsolete),
            BaseFileClassification::hash("D15C6A49", Fallout4InstallType::Obsolete),
            BaseFileClassification::hash("8C52BE93", Fallout4InstallType::Obsolete),
            BaseFileClassification::hash("591009C9", Fallout4InstallType::Obsolete),
            BaseFileClassification::hash("720BB9C3", Fallout4InstallType::Anniversary),
        ],
    },
    BaseFileDefinition {
        relative_path: "steam_api64.dll",
        classifications: &[
            BaseFileClassification::version("2.89.45.4", Fallout4InstallType::OldGen),
            BaseFileClassification::version("7.40.51.27", Fallout4InstallType::NextGenAnniversary),
            BaseFileClassification::hash("BD3AA35F", Fallout4InstallType::OldGen),
        ],
    },
    BaseFileDefinition {
        relative_path: "f4se_loader.exe",
        classifications: &[
            BaseFileClassification::version("0.0.6.23", Fallout4InstallType::OldGen),
            BaseFileClassification::version("0.0.7.2", Fallout4InstallType::NextGen),
            BaseFileClassification::version("0.0.7.4", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("0.0.7.5", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("0.0.7.6", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("0.0.7.7", Fallout4InstallType::Anniversary),
        ],
    },
    BaseFileDefinition {
        relative_path: "f4se_steam_loader.dll",
        classifications: &[BaseFileClassification::version(
            "0.0.6.23",
            Fallout4InstallType::OldGen,
        )],
    },
    BaseFileDefinition {
        relative_path: "CreationKit.exe",
        classifications: &[
            BaseFileClassification::version("1.10.162.0", Fallout4InstallType::OldGen),
            BaseFileClassification::version("1.10.943.1", Fallout4InstallType::Obsolete),
            BaseFileClassification::version("1.10.982.3", Fallout4InstallType::NextGen),
            BaseFileClassification::version("1.11.137.0", Fallout4InstallType::Anniversary),
        ],
    },
    BaseFileDefinition {
        relative_path: "Tools\\Archive2\\Archive2.exe",
        classifications: &[
            BaseFileClassification::hash("4CDFC7B5", Fallout4InstallType::OldGen),
            BaseFileClassification::hash("71A5240B", Fallout4InstallType::NextGen),
            BaseFileClassification::hash("C867674F", Fallout4InstallType::Anniversary),
        ],
    },
];

#[derive(Debug, Clone, Copy)]
struct BaseFileDefinition {
    relative_path: &'static str,
    classifications: &'static [BaseFileClassification],
}

#[derive(Debug, Clone, Copy)]
struct BaseFileClassification {
    token: &'static str,
    source: BinaryClassificationSource,
    install_type: Fallout4InstallType,
}

impl BaseFileClassification {
    const fn version(token: &'static str, install_type: Fallout4InstallType) -> Self {
        Self {
            token,
            source: BinaryClassificationSource::Version,
            install_type,
        }
    }

    const fn hash(token: &'static str, install_type: Fallout4InstallType) -> Self {
        Self {
            token,
            source: BinaryClassificationSource::Hash,
            install_type,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryClassificationSource {
    Version,
    Hash,
}

/// Configured environment paths used by Overview filesystem collection.
///
/// The collector does not read process environment variables itself. Production
/// callers should resolve known folders before calling this service, and tests
/// can inject deterministic paths.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OverviewCollectionEnvironment {
    /// `%LOCALAPPDATA%` equivalent used to derive `Fallout4/plugins.txt`.
    pub local_appdata: Option<PathBuf>,
    /// Explicit plugins.txt path, if it was already resolved by a caller.
    pub plugins_txt: Option<PathBuf>,
}

impl OverviewCollectionEnvironment {
    /// Returns a new environment configuration with no resolved paths.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the `%LOCALAPPDATA%` equivalent used for `Fallout4/plugins.txt`.
    pub fn with_local_appdata(mut self, path: impl Into<PathBuf>) -> Self {
        self.local_appdata = Some(path.into());
        self
    }

    /// Adds an explicit `plugins.txt` path.
    pub fn with_plugins_txt(mut self, path: impl Into<PathBuf>) -> Self {
        self.plugins_txt = Some(path.into());
        self
    }

    /// Returns the configured plugin enablement path, if one is available.
    pub fn plugins_txt_path(&self) -> Option<PathBuf> {
        self.plugins_txt.clone().or_else(|| {
            self.local_appdata
                .as_ref()
                .map(|path| path.join("Fallout4").join("plugins.txt"))
        })
    }
}

/// Input request for [`OverviewCollector::collect`].
#[derive(Debug, Clone, Copy)]
pub struct OverviewCollectionRequest<'a> {
    /// Installation discovered by the discovery service.
    pub installation: &'a Fallout4Installation,
    /// Pre-resolved environment paths used for enablement files.
    pub environment: &'a OverviewCollectionEnvironment,
}

impl<'a> OverviewCollectionRequest<'a> {
    /// Creates a collection request for a discovered Fallout 4 installation.
    pub const fn new(
        installation: &'a Fallout4Installation,
        environment: &'a OverviewCollectionEnvironment,
    ) -> Self {
        Self {
            installation,
            environment,
        }
    }
}

/// Fully collected Overview facts ready for the pure diagnostics builder.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OverviewCollectedFacts {
    /// Classified executable/DLL/BIN facts in reference display order.
    pub binaries: Vec<OverviewBinaryFact>,
    /// Classified BA2 records in deterministic path order.
    pub archives: Vec<ArchiveRecord>,
    /// Classified ESM/ESL/ESP records in deterministic path order.
    pub modules: Vec<ModuleRecord>,
    /// Required-file and enablement-file state.
    pub enablement: OverviewEnablementFacts,
    /// Safe collection diagnostics for logs, UI refresh state, or tests.
    pub diagnostics: OverviewCollectionDiagnostics,
}

/// High-level collector phase used by safe diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OverviewCollectionPhase {
    /// Reference base executable/DLL/BIN classification.
    Binaries,
    /// Optional Data directory traversal.
    DataTraversal,
    /// Fallout4.ccc, plugins.txt, and INI enablement parsing.
    Enablement,
    /// Address Library bin existence check.
    AddressLibrary,
    /// BA2 header classification.
    Archives,
    /// ESM/ESL/ESP header classification.
    Modules,
}

impl OverviewCollectionPhase {
    /// Returns a stable lowercase phase label for logs and diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Binaries => "binaries",
            Self::DataTraversal => "data_traversal",
            Self::Enablement => "enablement",
            Self::AddressLibrary => "address_library",
            Self::Archives => "archives",
            Self::Modules => "modules",
        }
    }
}

/// Safe diagnostic category for a collector observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OverviewCollectionDiagnosticKind {
    /// An optional or required input was not present.
    Missing,
    /// An existing input could not be read.
    Unreadable,
    /// An input was present but malformed or unsupported for Fallout 4.
    Invalid,
    /// An adapter operation is not supported on the current platform.
    Unsupported,
    /// A phase intentionally skipped work because a prerequisite was absent.
    Skipped,
}

/// Per-phase item count emitted by the collector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewCollectionPhaseSummary {
    /// Collector phase that produced this count.
    pub phase: OverviewCollectionPhase,
    /// Number of items successfully considered by the phase.
    pub item_count: usize,
}

/// A safe per-path collection diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewCollectionErrorDetail {
    /// Phase that observed the issue.
    pub phase: OverviewCollectionPhase,
    /// Safe category for callers to branch on.
    pub kind: OverviewCollectionDiagnosticKind,
    /// Path involved in the issue, if any.
    pub path: Option<PathBuf>,
    /// User-safe diagnostic summary; raw OS errors are not exposed here.
    pub safe_message: String,
}

/// Safe aggregate diagnostics from a collector run.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OverviewCollectionDiagnostics {
    /// Number of binary facts emitted.
    pub binary_count: usize,
    /// Number of archive records emitted.
    pub archive_count: usize,
    /// Number of module records emitted.
    pub module_count: usize,
    /// Number of archive records marked enabled.
    pub enabled_archive_count: usize,
    /// Number of module records marked enabled.
    pub enabled_module_count: usize,
    /// Count of diagnostics categorized as missing inputs.
    pub missing_file_count: usize,
    /// Count of diagnostics categorized as unreadable inputs.
    pub unreadable_file_count: usize,
    /// Per-phase item counts.
    pub phases: Vec<OverviewCollectionPhaseSummary>,
    /// Safe per-path details for missing, unreadable, invalid, or skipped inputs.
    pub errors: Vec<OverviewCollectionErrorDetail>,
}

impl OverviewCollectionDiagnostics {
    fn record_phase(&mut self, phase: OverviewCollectionPhase, item_count: usize) {
        self.phases
            .push(OverviewCollectionPhaseSummary { phase, item_count });
    }

    fn record_error(
        &mut self,
        phase: OverviewCollectionPhase,
        kind: OverviewCollectionDiagnosticKind,
        path: Option<PathBuf>,
        safe_message: impl Into<String>,
    ) {
        match kind {
            OverviewCollectionDiagnosticKind::Missing => self.missing_file_count += 1,
            OverviewCollectionDiagnosticKind::Unreadable => self.unreadable_file_count += 1,
            OverviewCollectionDiagnosticKind::Invalid
            | OverviewCollectionDiagnosticKind::Unsupported
            | OverviewCollectionDiagnosticKind::Skipped => {}
        }

        self.errors.push(OverviewCollectionErrorDetail {
            phase,
            kind,
            path,
            safe_message: safe_message.into(),
        });
    }

    fn record_platform_error(
        &mut self,
        phase: OverviewCollectionPhase,
        path: impl Into<Option<PathBuf>>,
        error: &PlatformError,
    ) {
        let kind = diagnostic_kind_from_platform_error(error);
        self.record_error(phase, kind, path.into(), error.user_message().to_owned());
    }
}

/// Filesystem/process backed collector for Overview diagnostics facts.
#[derive(Debug)]
pub struct OverviewCollector<'a, F: Filesystem + ?Sized, P: ProcessInspector + ?Sized> {
    filesystem: &'a F,
    process: &'a P,
}

impl<'a, F: Filesystem + ?Sized, P: ProcessInspector + ?Sized> OverviewCollector<'a, F, P> {
    /// Creates a collector over injected platform adapters.
    pub const fn new(filesystem: &'a F, process: &'a P) -> Self {
        Self {
            filesystem,
            process,
        }
    }

    /// Collects typed facts for a single Overview refresh.
    ///
    /// This method is intentionally infallible: malformed, missing, and
    /// permission-denied inputs are converted into typed records or safe
    /// diagnostics rather than panics or modal warnings.
    pub fn collect(&self, request: OverviewCollectionRequest<'_>) -> OverviewCollectedFacts {
        let span = info_span!(
            "overview_collector.collect",
            game_path = %request.installation.game_path.display()
        );
        let _guard = span.enter();
        debug!("overview filesystem collection started");

        let mut diagnostics = OverviewCollectionDiagnostics::default();
        let mut binaries = self.collect_binaries(request.installation, &mut diagnostics);
        let detected_install_type = detected_install_type_from_binaries(&binaries);
        let data_index = self.collect_data_index(request.installation, &mut diagnostics);
        let enablement_inputs = self.collect_enablement(
            request.installation,
            request.environment,
            data_index.as_ref(),
            detected_install_type,
            &mut diagnostics,
        );
        let archives = self.collect_archives(
            data_index.as_ref(),
            &enablement_inputs.enabled_archives,
            &mut diagnostics,
        );
        let modules = self.collect_modules(
            data_index.as_ref(),
            &enablement_inputs.enabled_modules,
            &mut diagnostics,
        );

        diagnostics.binary_count = binaries.len();
        diagnostics.archive_count = archives.len();
        diagnostics.module_count = modules.len();
        diagnostics.enabled_archive_count = archives.iter().filter(|record| record.enabled).count();
        diagnostics.enabled_module_count = modules.iter().filter(|record| record.enabled).count();
        diagnostics.record_phase(OverviewCollectionPhase::Binaries, binaries.len());
        diagnostics.record_phase(OverviewCollectionPhase::Archives, archives.len());
        diagnostics.record_phase(OverviewCollectionPhase::Modules, modules.len());

        normalize_binary_order(&mut binaries);

        debug!(
            binary_count = diagnostics.binary_count,
            archive_count = diagnostics.archive_count,
            module_count = diagnostics.module_count,
            enabled_archive_count = diagnostics.enabled_archive_count,
            enabled_module_count = diagnostics.enabled_module_count,
            missing_file_count = diagnostics.missing_file_count,
            unreadable_file_count = diagnostics.unreadable_file_count,
            "overview filesystem collection completed"
        );

        OverviewCollectedFacts {
            binaries,
            archives,
            modules,
            enablement: enablement_inputs.facts,
            diagnostics,
        }
    }

    fn collect_binaries(
        &self,
        installation: &Fallout4Installation,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) -> Vec<OverviewBinaryFact> {
        let mut facts = Vec::with_capacity(BASE_FILES.len());
        let mut detected_game_type = Fallout4InstallType::Unknown;

        for definition in BASE_FILES {
            let path = join_relative_path(&installation.game_path, definition.relative_path);
            let mut fact = self.collect_binary(definition, &path, diagnostics);

            if path_basename_eq(definition.relative_path, "Fallout4.exe") {
                self.apply_downgrade_detection(installation, &mut fact, diagnostics);
                detected_game_type = fact.install_type;
            } else if fact.install_type == Fallout4InstallType::NextGenAnniversary
                && matches!(
                    detected_game_type,
                    Fallout4InstallType::NextGen | Fallout4InstallType::Anniversary
                )
            {
                fact.install_type = detected_game_type;
            }

            facts.push(fact);
        }

        facts
    }

    fn collect_binary(
        &self,
        definition: &BaseFileDefinition,
        path: &Path,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) -> OverviewBinaryFact {
        match self.filesystem.metadata(path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => {
                diagnostics.record_error(
                    OverviewCollectionPhase::Binaries,
                    OverviewCollectionDiagnosticKind::Invalid,
                    Some(path.to_path_buf()),
                    "Reference binary path is not a regular file.",
                );
                return OverviewBinaryFact::missing(definition.relative_path);
            }
            Err(error) if error.kind == PlatformErrorKind::NotFound => {
                return OverviewBinaryFact::missing(definition.relative_path)
                    .with_path(path.to_path_buf());
            }
            Err(error) => {
                diagnostics.record_platform_error(
                    OverviewCollectionPhase::Binaries,
                    Some(path.to_path_buf()),
                    &error,
                );
                return OverviewBinaryFact::new(
                    definition.relative_path,
                    Fallout4InstallType::Unknown,
                )
                .with_path(path.to_path_buf());
            }
        }

        let version = self.read_version_string(path, diagnostics);
        if let Some(version) = version.as_deref()
            && let Some(install_type) =
                lookup_install_type(definition, BinaryClassificationSource::Version, version)
        {
            return OverviewBinaryFact::new(definition.relative_path, install_type)
                .with_path(path.to_path_buf())
                .with_version_metadata(Some(version.to_owned()), None::<String>);
        }

        let hash = match self.filesystem.read_bytes(path) {
            Ok(bytes) => Some(crc32_upper(&bytes)),
            Err(error) => {
                diagnostics.record_platform_error(
                    OverviewCollectionPhase::Binaries,
                    Some(path.to_path_buf()),
                    &error,
                );
                None
            }
        };
        let install_type = hash
            .as_deref()
            .and_then(|hash| {
                lookup_install_type(definition, BinaryClassificationSource::Hash, hash)
            })
            .unwrap_or(Fallout4InstallType::Unknown);

        OverviewBinaryFact::new(definition.relative_path, install_type)
            .with_path(path.to_path_buf())
            .with_version_metadata(version, hash)
    }

    fn read_version_string(
        &self,
        path: &Path,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) -> Option<String> {
        match self.process.file_version(path) {
            Ok(Some(metadata)) => version_string(metadata),
            Ok(None) => None,
            Err(error) => {
                diagnostics.record_platform_error(
                    OverviewCollectionPhase::Binaries,
                    Some(path.to_path_buf()),
                    &error,
                );
                None
            }
        }
    }

    fn apply_downgrade_detection(
        &self,
        installation: &Fallout4Installation,
        fallout4_fact: &mut OverviewBinaryFact,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) {
        if fallout4_fact.install_type != Fallout4InstallType::OldGen {
            return;
        }

        let Some(data_path) = installation.data_path.as_ref() else {
            return;
        };
        let startup_path = data_path.join("Fallout4 - Startup.ba2");
        match self.filesystem.metadata(&startup_path) {
            Ok(metadata) if metadata.is_file() => match self.filesystem.read_bytes(&startup_path) {
                Ok(bytes) => {
                    let payload = bytes.get(BA2_HEADER_LEN..).unwrap_or(&[]);
                    if crc32_upper(payload) == NG_STARTUP_BA2_CRC {
                        fallout4_fact.install_type = Fallout4InstallType::DownGrade;
                    }
                }
                Err(error) => diagnostics.record_platform_error(
                    OverviewCollectionPhase::Binaries,
                    Some(startup_path),
                    &error,
                ),
            },
            Ok(_) => diagnostics.record_error(
                OverviewCollectionPhase::Binaries,
                OverviewCollectionDiagnosticKind::Invalid,
                Some(startup_path),
                "Fallout4 - Startup.ba2 is not a regular file.",
            ),
            Err(error) if error.kind == PlatformErrorKind::NotFound => diagnostics.record_error(
                OverviewCollectionPhase::Binaries,
                OverviewCollectionDiagnosticKind::Missing,
                Some(startup_path),
                "Fallout4 - Startup.ba2 was not found for downgrade detection.",
            ),
            Err(error) => diagnostics.record_platform_error(
                OverviewCollectionPhase::Binaries,
                Some(startup_path),
                &error,
            ),
        }
    }

    fn collect_data_index(
        &self,
        installation: &Fallout4Installation,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) -> Option<DataIndex> {
        let Some(data_path) = installation.data_path.as_ref() else {
            diagnostics.record_error(
                OverviewCollectionPhase::DataTraversal,
                OverviewCollectionDiagnosticKind::Skipped,
                None,
                "Data path was not supplied by discovery.",
            );
            diagnostics.record_phase(OverviewCollectionPhase::DataTraversal, 0);
            return None;
        };

        match self.filesystem.metadata(data_path) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                diagnostics.record_error(
                    OverviewCollectionPhase::DataTraversal,
                    OverviewCollectionDiagnosticKind::Invalid,
                    Some(data_path.clone()),
                    "Data path is not a directory.",
                );
                diagnostics.record_phase(OverviewCollectionPhase::DataTraversal, 0);
                return None;
            }
            Err(error) => {
                diagnostics.record_platform_error(
                    OverviewCollectionPhase::DataTraversal,
                    Some(data_path.clone()),
                    &error,
                );
                diagnostics.record_phase(OverviewCollectionPhase::DataTraversal, 0);
                return None;
            }
        }

        match self.filesystem.walk_dir(data_path) {
            Ok(entries) => {
                let index = DataIndex::new(data_path.clone(), entries);
                diagnostics.record_phase(OverviewCollectionPhase::DataTraversal, index.files.len());
                Some(index)
            }
            Err(error) => {
                diagnostics.record_platform_error(
                    OverviewCollectionPhase::DataTraversal,
                    Some(data_path.clone()),
                    &error,
                );
                diagnostics.record_phase(OverviewCollectionPhase::DataTraversal, 0);
                None
            }
        }
    }

    fn collect_enablement(
        &self,
        installation: &Fallout4Installation,
        environment: &OverviewCollectionEnvironment,
        data_index: Option<&DataIndex>,
        install_type: Fallout4InstallType,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) -> EnablementCollection {
        let mut collection = EnablementCollection::default();
        collection.facts.address_library =
            self.collect_address_library(installation, install_type, diagnostics);

        if let Some(index) = data_index {
            for master in GAME_MASTERS {
                if let Some(path) = index.resolve_relative(master) {
                    collection.enabled_modules.insert(path_key(&path));
                }
            }
        }

        let ccc_path = installation.game_path.join("Fallout4.ccc");
        self.collect_fallout4_ccc(&ccc_path, data_index, &mut collection, diagnostics);

        self.collect_plugins_txt(environment, data_index, &mut collection, diagnostics);
        self.collect_ini_enabled_archives(
            installation,
            data_index,
            install_type,
            &mut collection,
            diagnostics,
        );

        diagnostics.record_phase(
            OverviewCollectionPhase::Enablement,
            collection.enabled_modules.len() + collection.enabled_archives.len(),
        );

        collection
    }

    fn collect_address_library(
        &self,
        installation: &Fallout4Installation,
        _install_type: Fallout4InstallType,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) -> OverviewAddressLibraryFact {
        let Some(data_path) = installation.data_path.as_ref() else {
            diagnostics.record_phase(OverviewCollectionPhase::AddressLibrary, 0);
            return OverviewAddressLibraryFact::unknown();
        };

        let fallout4_path = join_relative_path(&installation.game_path, "Fallout4.exe");
        let Some(version) = self.read_version_string(&fallout4_path, diagnostics) else {
            diagnostics.record_phase(OverviewCollectionPhase::AddressLibrary, 0);
            return OverviewAddressLibraryFact::unknown();
        };
        let relative_path = PathBuf::from("F4SE")
            .join("Plugins")
            .join(format!("version-{}.bin", version.replace('.', "-")));
        let address_library_path = data_path.join(&relative_path);

        let fact = match self.filesystem.metadata(&address_library_path) {
            Ok(metadata) if metadata.is_file() => {
                OverviewAddressLibraryFact::installed(address_library_path, relative_path)
            }
            Ok(_) => {
                diagnostics.record_error(
                    OverviewCollectionPhase::AddressLibrary,
                    OverviewCollectionDiagnosticKind::Invalid,
                    Some(address_library_path.clone()),
                    "Address Library path is not a regular file.",
                );
                OverviewAddressLibraryFact::missing(address_library_path, relative_path)
            }
            Err(error) if error.kind == PlatformErrorKind::NotFound => {
                diagnostics.record_error(
                    OverviewCollectionPhase::AddressLibrary,
                    OverviewCollectionDiagnosticKind::Missing,
                    Some(address_library_path.clone()),
                    "Address Library bin was not found for the detected game version.",
                );
                OverviewAddressLibraryFact::missing(address_library_path, relative_path)
            }
            Err(error) => {
                diagnostics.record_platform_error(
                    OverviewCollectionPhase::AddressLibrary,
                    Some(address_library_path.clone()),
                    &error,
                );
                OverviewAddressLibraryFact::missing(address_library_path, relative_path)
            }
        };
        diagnostics.record_phase(OverviewCollectionPhase::AddressLibrary, 1);
        fact
    }

    fn collect_fallout4_ccc(
        &self,
        ccc_path: &Path,
        data_index: Option<&DataIndex>,
        collection: &mut EnablementCollection,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) {
        match self.filesystem.metadata(ccc_path) {
            Ok(metadata) if metadata.is_file() => match self.filesystem.read_to_string(ccc_path) {
                Ok(text) => {
                    collection.facts.fallout4_ccc = OverviewFilePresence::present(ccc_path);
                    if let Some(index) = data_index {
                        for line in text.lines().map(clean_enablement_line) {
                            if line.is_empty() {
                                continue;
                            }
                            if let Some(path) = index.resolve_relative(line)
                                && is_module_path(&path)
                            {
                                collection.enabled_modules.insert(path_key(&path));
                            }
                        }
                    }
                }
                Err(error) => {
                    diagnostics.record_platform_error(
                        OverviewCollectionPhase::Enablement,
                        Some(ccc_path.to_path_buf()),
                        &error,
                    );
                    collection.facts.fallout4_ccc = OverviewFilePresence::unreadable(ccc_path);
                }
            },
            Ok(_) => {
                diagnostics.record_error(
                    OverviewCollectionPhase::Enablement,
                    OverviewCollectionDiagnosticKind::Invalid,
                    Some(ccc_path.to_path_buf()),
                    "Fallout4.ccc is not a regular file.",
                );
                collection.facts.fallout4_ccc = OverviewFilePresence::unreadable(ccc_path);
            }
            Err(error) if error.kind == PlatformErrorKind::NotFound => {
                diagnostics.record_error(
                    OverviewCollectionPhase::Enablement,
                    OverviewCollectionDiagnosticKind::Missing,
                    Some(ccc_path.to_path_buf()),
                    "Fallout4.ccc was not found.",
                );
                collection.facts.fallout4_ccc = OverviewFilePresence::missing(ccc_path);
            }
            Err(error) => {
                diagnostics.record_platform_error(
                    OverviewCollectionPhase::Enablement,
                    Some(ccc_path.to_path_buf()),
                    &error,
                );
                collection.facts.fallout4_ccc = OverviewFilePresence::unreadable(ccc_path);
            }
        }
    }

    fn collect_plugins_txt(
        &self,
        environment: &OverviewCollectionEnvironment,
        data_index: Option<&DataIndex>,
        collection: &mut EnablementCollection,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) {
        let Some(plugins_path) = environment.plugins_txt_path() else {
            diagnostics.record_error(
                OverviewCollectionPhase::Enablement,
                OverviewCollectionDiagnosticKind::Skipped,
                None,
                "plugins.txt path was not supplied by the environment configuration.",
            );
            enable_all_modules_on_plugins_fallback(data_index, collection);
            return;
        };

        match self.filesystem.metadata(&plugins_path) {
            Ok(metadata) if metadata.is_file() => {
                match self.filesystem.read_to_string(&plugins_path) {
                    Ok(text) => {
                        collection.facts.plugins_txt = OverviewFilePresence::present(&plugins_path);
                        if let Some(index) = data_index {
                            for line in text.lines().map(clean_enablement_line) {
                                if let Some(plugin) = line.strip_prefix('*')
                                    && let Some(path) = index.resolve_relative(plugin.trim())
                                    && is_module_path(&path)
                                {
                                    collection.enabled_modules.insert(path_key(&path));
                                }
                            }
                        }
                    }
                    Err(error) => {
                        diagnostics.record_platform_error(
                            OverviewCollectionPhase::Enablement,
                            Some(plugins_path.clone()),
                            &error,
                        );
                        collection.facts.plugins_txt =
                            OverviewFilePresence::unreadable(&plugins_path);
                        enable_all_modules_on_plugins_fallback(data_index, collection);
                    }
                }
            }
            Ok(_) => {
                diagnostics.record_error(
                    OverviewCollectionPhase::Enablement,
                    OverviewCollectionDiagnosticKind::Invalid,
                    Some(plugins_path.clone()),
                    "plugins.txt is not a regular file.",
                );
                collection.facts.plugins_txt = OverviewFilePresence::unreadable(&plugins_path);
                enable_all_modules_on_plugins_fallback(data_index, collection);
            }
            Err(error) if error.kind == PlatformErrorKind::NotFound => {
                diagnostics.record_error(
                    OverviewCollectionPhase::Enablement,
                    OverviewCollectionDiagnosticKind::Missing,
                    Some(plugins_path.clone()),
                    "plugins.txt was not found.",
                );
                collection.facts.plugins_txt = OverviewFilePresence::missing(&plugins_path);
                enable_all_modules_on_plugins_fallback(data_index, collection);
            }
            Err(error) => {
                diagnostics.record_platform_error(
                    OverviewCollectionPhase::Enablement,
                    Some(plugins_path.clone()),
                    &error,
                );
                collection.facts.plugins_txt = OverviewFilePresence::unreadable(&plugins_path);
                enable_all_modules_on_plugins_fallback(data_index, collection);
            }
        }
    }

    fn collect_ini_enabled_archives(
        &self,
        installation: &Fallout4Installation,
        data_index: Option<&DataIndex>,
        install_type: Fallout4InstallType,
        collection: &mut EnablementCollection,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) {
        let Some(index) = data_index else {
            return;
        };

        for archive_name in ini_archive_names(&installation.ini_files) {
            match index.resolve_relative(&archive_name) {
                Some(path) if is_archive_path(&path) => {
                    collection.enabled_archives.insert(path_key(&path));
                }
                Some(_) => {}
                None => diagnostics.record_error(
                    OverviewCollectionPhase::Enablement,
                    OverviewCollectionDiagnosticKind::Missing,
                    Some(index.root.join(&archive_name)),
                    "INI archive list references a file that was not found in Data.",
                ),
            }
        }

        let suffixes = ba2_suffixes(&installation.ini_files);
        let enabled_modules = collection
            .enabled_modules
            .iter()
            .filter_map(|key| index.by_path_key.get(key))
            .cloned()
            .collect::<Vec<_>>();
        for module_path in enabled_modules {
            let Some(stem) = module_path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            for suffix in &suffixes {
                let archive_name = format!("{stem} - {suffix}.ba2");
                if let Some(path) = index.resolve_sibling(&module_path, &archive_name) {
                    collection.enabled_archives.insert(path_key(&path));
                }
            }
        }

        if ini_value(&installation.ini_files.prefs, "nvflex", "bnvflexenable") == Some("1") {
            self.add_hardcoded_archive(
                index,
                "Fallout4 - Nvflex.ba2",
                collection,
                diagnostics,
                "Nvidia Flex is enabled but Fallout4 - Nvflex.ba2 was not found.",
            );
        }

        if install_type == Fallout4InstallType::Anniversary {
            self.add_hardcoded_archive(
                index,
                "Fallout4 - TexturesPatch.ba2",
                collection,
                diagnostics,
                "Anniversary Edition requires Fallout4 - TexturesPatch.ba2, but it was not found.",
            );
        }
    }

    fn add_hardcoded_archive(
        &self,
        index: &DataIndex,
        relative_name: &str,
        collection: &mut EnablementCollection,
        diagnostics: &mut OverviewCollectionDiagnostics,
        missing_message: &'static str,
    ) {
        if let Some(path) = index.resolve_relative(relative_name) {
            collection.enabled_archives.insert(path_key(&path));
        } else {
            diagnostics.record_error(
                OverviewCollectionPhase::Enablement,
                OverviewCollectionDiagnosticKind::Missing,
                Some(index.root.join(relative_name)),
                missing_message,
            );
        }
    }

    fn collect_archives(
        &self,
        data_index: Option<&DataIndex>,
        enabled_archives: &BTreeSet<String>,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) -> Vec<ArchiveRecord> {
        let Some(index) = data_index else {
            return Vec::new();
        };

        index
            .files
            .iter()
            .filter(|path| is_archive_path(path))
            .map(|path| {
                self.collect_archive(
                    path,
                    enabled_archives.contains(&path_key(path)),
                    diagnostics,
                )
            })
            .collect()
    }

    fn collect_archive(
        &self,
        path: &Path,
        enabled: bool,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) -> ArchiveRecord {
        let header = match self.filesystem.read_prefix(path, BA2_HEADER_LEN) {
            Ok(header) => header,
            Err(error) => {
                diagnostics.record_platform_error(
                    OverviewCollectionPhase::Archives,
                    Some(path.to_path_buf()),
                    &error,
                );
                return unreadable_archive(path, enabled);
            }
        };

        if header.len() != BA2_HEADER_LEN {
            diagnostics.record_error(
                OverviewCollectionPhase::Archives,
                OverviewCollectionDiagnosticKind::Invalid,
                Some(path.to_path_buf()),
                "Archive header is shorter than the BA2 header length.",
            );
            return unreadable_archive(path, enabled);
        }
        if header.get(0..4) != Some(b"BTDX") {
            diagnostics.record_error(
                OverviewCollectionPhase::Archives,
                OverviewCollectionDiagnosticKind::Invalid,
                Some(path.to_path_buf()),
                "Archive magic is not BTDX.",
            );
            return unreadable_archive(path, enabled);
        }

        let version = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        let archive_version = match version {
            1 => ArchiveVersion::OldGen,
            7 => ArchiveVersion::NextGen7,
            8 => ArchiveVersion::NextGen8,
            other => {
                diagnostics.record_error(
                    OverviewCollectionPhase::Archives,
                    OverviewCollectionDiagnosticKind::Invalid,
                    Some(path.to_path_buf()),
                    format!("Archive version ({other}) is not valid for Fallout 4."),
                );
                ArchiveVersion::Unknown(other)
            }
        };
        let format = match &header[8..12] {
            b"GNRL" => ArchiveFormat::General,
            b"DX10" => ArchiveFormat::DirectX10,
            other => {
                let display = String::from_utf8_lossy(other).to_string();
                diagnostics.record_error(
                    OverviewCollectionPhase::Archives,
                    OverviewCollectionDiagnosticKind::Invalid,
                    Some(path.to_path_buf()),
                    format!("Archive format ({display}) is not valid for Fallout 4."),
                );
                ArchiveFormat::Unknown(display)
            }
        };

        ArchiveRecord {
            path: path.to_path_buf(),
            format,
            version: archive_version,
            enabled,
            readable: true,
        }
    }

    fn collect_modules(
        &self,
        data_index: Option<&DataIndex>,
        enabled_modules: &BTreeSet<String>,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) -> Vec<ModuleRecord> {
        let Some(index) = data_index else {
            return Vec::new();
        };

        index
            .files
            .iter()
            .filter(|path| is_module_path(path))
            .map(|path| {
                self.collect_module(path, enabled_modules.contains(&path_key(path)), diagnostics)
            })
            .collect()
    }

    fn collect_module(
        &self,
        path: &Path,
        enabled: bool,
        diagnostics: &mut OverviewCollectionDiagnostics,
    ) -> ModuleRecord {
        let header = match self.filesystem.read_prefix(path, MODULE_HEADER_LEN) {
            Ok(header) => header,
            Err(error) => {
                diagnostics.record_platform_error(
                    OverviewCollectionPhase::Modules,
                    Some(path.to_path_buf()),
                    &error,
                );
                return unreadable_module(path, enabled);
            }
        };

        if header.len() != MODULE_HEADER_LEN {
            diagnostics.record_error(
                OverviewCollectionPhase::Modules,
                OverviewCollectionDiagnosticKind::Invalid,
                Some(path.to_path_buf()),
                "Module header is shorter than the TES4 header probe length.",
            );
            return unreadable_module(path, enabled);
        }
        if header.get(0..4) != Some(b"TES4") {
            diagnostics.record_error(
                OverviewCollectionPhase::Modules,
                OverviewCollectionDiagnosticKind::Invalid,
                Some(path.to_path_buf()),
                "Module magic is not TES4.",
            );
            return unreadable_module(path, enabled);
        }
        if header.get(24..28) != Some(b"HEDR") {
            diagnostics.record_error(
                OverviewCollectionPhase::Modules,
                OverviewCollectionDiagnosticKind::Invalid,
                Some(path.to_path_buf()),
                "Module TES4 header does not contain an HEDR field at the expected offset.",
            );
            return unreadable_module(path, enabled);
        }

        let flags = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
        let kind = if flags & MODULE_LIGHT_FLAG != 0 || extension_eq(path, "esl") {
            ModuleKind::Light
        } else {
            ModuleKind::Full
        };
        let hedr_bytes = [header[30], header[31], header[32], header[33]];
        let header_version = match hedr_bytes {
            MODULE_VERSION_095 => ModuleHeaderVersion::Version095,
            MODULE_VERSION_100 => ModuleHeaderVersion::Version100,
            other => {
                let display = format_hedr_version(other);
                diagnostics.record_error(
                    OverviewCollectionPhase::Modules,
                    OverviewCollectionDiagnosticKind::Invalid,
                    Some(path.to_path_buf()),
                    format!("Module version ({display}) is not valid for Fallout 4."),
                );
                ModuleHeaderVersion::Unknown(display)
            }
        };

        ModuleRecord {
            path: path.to_path_buf(),
            kind,
            header_version,
            enabled,
            readable: true,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct EnablementCollection {
    facts: OverviewEnablementFacts,
    enabled_modules: BTreeSet<String>,
    enabled_archives: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DataIndex {
    root: PathBuf,
    files: Vec<PathBuf>,
    by_key: BTreeMap<String, PathBuf>,
    by_path_key: BTreeMap<String, PathBuf>,
}

impl DataIndex {
    fn new(root: PathBuf, entries: Vec<DirectoryEntry>) -> Self {
        let mut files = entries
            .into_iter()
            .filter(|entry| entry.file_type == FileType::File)
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        files.sort();

        let by_key = files
            .iter()
            .filter_map(|path| relative_key(&root, path).map(|key| (key, path.clone())))
            .collect::<BTreeMap<_, _>>();
        let by_path_key = files
            .iter()
            .map(|path| (path_key(path), path.clone()))
            .collect::<BTreeMap<_, _>>();

        Self {
            root,
            files,
            by_key,
            by_path_key,
        }
    }

    fn resolve_relative(&self, relative: &str) -> Option<PathBuf> {
        self.by_key.get(&normalize_relative_text(relative)).cloned()
    }

    fn resolve_sibling(&self, path: &Path, sibling_file_name: &str) -> Option<PathBuf> {
        let parent = path.parent()?;
        let relative_parent = parent.strip_prefix(&self.root).ok();
        let relative =
            match relative_parent {
                Some(parent) if !parent.as_os_str().is_empty() => normalize_relative_text(
                    &format!("{}/{}", path_to_slash_string(parent), sibling_file_name),
                ),
                _ => normalize_relative_text(sibling_file_name),
            };
        self.by_key.get(&relative).cloned()
    }
}

fn lookup_install_type(
    definition: &BaseFileDefinition,
    source: BinaryClassificationSource,
    token: &str,
) -> Option<Fallout4InstallType> {
    definition
        .classifications
        .iter()
        .find(|classification| {
            classification.source == source && classification.token.eq_ignore_ascii_case(token)
        })
        .map(|classification| classification.install_type)
}

fn version_string(metadata: VersionMetadata) -> Option<String> {
    metadata
        .raw
        .filter(|raw| !raw.trim().is_empty())
        .or_else(|| Some(metadata.semantic.to_string()).filter(|value| value != "0.0.0"))
}

fn detected_install_type_from_binaries(binaries: &[OverviewBinaryFact]) -> Fallout4InstallType {
    binaries
        .iter()
        .find(|fact| path_basename_eq(&fact.file_name, "Fallout4.exe"))
        .map(|fact| fact.install_type)
        .unwrap_or(Fallout4InstallType::Unknown)
}

fn normalize_binary_order(binaries: &mut [OverviewBinaryFact]) {
    let reference_order = BASE_FILES
        .iter()
        .enumerate()
        .map(|(index, definition)| (definition.relative_path, index))
        .collect::<BTreeMap<_, _>>();
    binaries.sort_by(|left, right| {
        let left_index = reference_order
            .get(left.file_name.as_str())
            .copied()
            .unwrap_or(usize::MAX);
        let right_index = reference_order
            .get(right.file_name.as_str())
            .copied()
            .unwrap_or(usize::MAX);
        left_index
            .cmp(&right_index)
            .then_with(|| left.file_name.cmp(&right.file_name))
    });
}

fn diagnostic_kind_from_platform_error(error: &PlatformError) -> OverviewCollectionDiagnosticKind {
    match error.kind {
        PlatformErrorKind::NotFound => OverviewCollectionDiagnosticKind::Missing,
        PlatformErrorKind::PermissionDenied => OverviewCollectionDiagnosticKind::Unreadable,
        PlatformErrorKind::InvalidInput | PlatformErrorKind::ParseError => {
            OverviewCollectionDiagnosticKind::Invalid
        }
        PlatformErrorKind::UnsupportedPlatform => OverviewCollectionDiagnosticKind::Unsupported,
        PlatformErrorKind::CommandFailed | PlatformErrorKind::Io => {
            OverviewCollectionDiagnosticKind::Unreadable
        }
    }
}

fn crc32_upper(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    format!("{:08X}", hasher.finalize())
}

fn clean_enablement_line(line: &str) -> &str {
    line.trim().trim_start_matches('\u{feff}')
}

fn enable_all_modules_on_plugins_fallback(
    data_index: Option<&DataIndex>,
    collection: &mut EnablementCollection,
) {
    let Some(index) = data_index else {
        return;
    };
    for path in index.files.iter().filter(|path| is_module_path(path)) {
        collection.enabled_modules.insert(path_key(path));
    }
}

fn ini_archive_names(ini_files: &Fallout4IniFiles) -> Vec<String> {
    let mut names = BTreeSet::new();
    for document in [&ini_files.fallout4, &ini_files.custom] {
        for key in ARCHIVE_LIST_KEYS {
            if let Some(value) = ini_value(document, "archive", key) {
                for item in value.split(',').map(|item| item.trim().trim_matches('"')) {
                    if !item.is_empty() {
                        names.insert(item.to_owned());
                    }
                }
            }
        }
    }
    names.into_iter().collect()
}

fn ba2_suffixes(ini_files: &Fallout4IniFiles) -> Vec<String> {
    let language = ini_value(&ini_files.custom, "general", "slanguage")
        .or_else(|| ini_value(&ini_files.fallout4, "general", "slanguage"))
        .unwrap_or("en")
        .trim()
        .to_ascii_lowercase();
    let mut suffixes = vec![
        "main".to_owned(),
        "textures".to_owned(),
        "voices_en".to_owned(),
    ];
    if !language.is_empty() && language != "en" {
        suffixes.push(format!("voices_{language}"));
    }
    suffixes
}

fn ini_value<'a>(
    document: &'a crate::domain::discovery::IniDocument,
    section: &str,
    key: &str,
) -> Option<&'a str> {
    document.get(&section.to_ascii_lowercase(), &key.to_ascii_lowercase())
}

fn unreadable_archive(path: &Path, enabled: bool) -> ArchiveRecord {
    ArchiveRecord {
        path: path.to_path_buf(),
        format: ArchiveFormat::Unknown(String::new()),
        version: ArchiveVersion::Unknown(0),
        enabled,
        readable: false,
    }
}

fn unreadable_module(path: &Path, enabled: bool) -> ModuleRecord {
    ModuleRecord {
        path: path.to_path_buf(),
        kind: ModuleKind::Full,
        header_version: ModuleHeaderVersion::Unknown(String::new()),
        enabled,
        readable: false,
    }
}

fn format_hedr_version(bytes: [u8; 4]) -> String {
    let value = f32::from_le_bytes(bytes);
    if !value.is_finite() {
        return format!("0x{}", bytes_to_hex(bytes));
    }
    if value.abs() >= 10.0 && (value.fract()).abs() < 0.005 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn bytes_to_hex(bytes: [u8; 4]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join("")
}

fn is_archive_path(path: &Path) -> bool {
    extension_eq(path, "ba2")
}

fn is_module_path(path: &Path) -> bool {
    ["esm", "esl", "esp"]
        .iter()
        .any(|extension| extension_eq(path, extension))
}

fn extension_eq(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn path_basename_eq(path: &str, expected: &str) -> bool {
    path.rsplit(['/', '\\'])
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn join_relative_path(base: &Path, relative_path: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for segment in relative_path
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
    {
        path.push(segment);
    }
    path
}

fn path_key(path: &Path) -> String {
    normalize_relative_text(&path_to_slash_string(path))
}

fn relative_key(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .map(|relative| normalize_relative_text(&path_to_slash_string(relative)))
}

fn normalize_relative_text(text: &str) -> String {
    text.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_ascii_lowercase()
}

fn path_to_slash_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod overview_collector_tests {
    use std::{cell::RefCell, collections::BTreeMap};

    use crate::{
        domain::discovery::SemanticVersion,
        platform::{
            PlatformOperation, PlatformResult,
            filesystem::{FileMetadata, FileType},
            process::{ProcessInfo, SystemMetadata},
        },
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FakeNode {
        File(Vec<u8>),
        Directory,
        UnreadableFile,
    }

    #[derive(Debug, Default)]
    struct FakeFilesystem {
        nodes: BTreeMap<PathBuf, FakeNode>,
        prefix_reads: RefCell<Vec<(PathBuf, usize)>>,
        full_reads: RefCell<Vec<PathBuf>>,
    }

    impl FakeFilesystem {
        fn with_dir(mut self, path: impl Into<PathBuf>) -> Self {
            self.nodes.insert(path.into(), FakeNode::Directory);
            self
        }

        fn with_file(mut self, path: impl Into<PathBuf>, bytes: impl Into<Vec<u8>>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes.insert(path, FakeNode::File(bytes.into()));
            self
        }

        fn with_text(self, path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
            self.with_file(path, text.into().into_bytes())
        }

        fn with_unreadable_file(mut self, path: impl Into<PathBuf>) -> Self {
            let path = path.into();
            self.ensure_parent_dirs(&path);
            self.nodes.insert(path, FakeNode::UnreadableFile);
            self
        }

        fn ensure_parent_dirs(&mut self, path: &Path) {
            let mut parents = Vec::new();
            let mut current = path.parent();
            while let Some(parent) = current {
                if parent.as_os_str().is_empty() {
                    break;
                }
                parents.push(parent.to_path_buf());
                current = parent.parent();
            }
            for parent in parents.into_iter().rev() {
                self.nodes.entry(parent).or_insert(FakeNode::Directory);
            }
        }

        fn node(&self, path: &Path, operation: PlatformOperation) -> PlatformResult<&FakeNode> {
            match self.nodes.get(path) {
                Some(node) => Ok(node),
                None => Err(PlatformError::new(
                    operation,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    format!("{} target was not found.", operation.label()),
                )),
            }
        }

        fn permission_denied(path: &Path, operation: PlatformOperation) -> PlatformError {
            PlatformError::new(
                operation,
                path.display().to_string(),
                PlatformErrorKind::PermissionDenied,
                format!(
                    "{} target could not be accessed because permission was denied.",
                    operation.label()
                ),
            )
        }
    }

    impl Filesystem for FakeFilesystem {
        fn metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
            match self.node(path, PlatformOperation::ReadMetadata)? {
                FakeNode::File(bytes) => Ok(FileMetadata {
                    file_type: FileType::File,
                    len: bytes.len() as u64,
                }),
                FakeNode::Directory => Ok(FileMetadata {
                    file_type: FileType::Directory,
                    len: 0,
                }),
                FakeNode::UnreadableFile => Ok(FileMetadata {
                    file_type: FileType::File,
                    len: 0,
                }),
            }
        }

        fn read_bytes(&self, path: &Path) -> PlatformResult<Vec<u8>> {
            self.full_reads.borrow_mut().push(path.to_path_buf());
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes) => Ok(bytes.clone()),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
                FakeNode::UnreadableFile => {
                    Err(Self::permission_denied(path, PlatformOperation::ReadFile))
                }
            }
        }

        fn read_prefix(&self, path: &Path, max_len: usize) -> PlatformResult<Vec<u8>> {
            self.prefix_reads
                .borrow_mut()
                .push((path.to_path_buf(), max_len));
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes) => Ok(bytes.iter().copied().take(max_len).collect()),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
                FakeNode::UnreadableFile => {
                    Err(Self::permission_denied(path, PlatformOperation::ReadFile))
                }
            }
        }

        fn read_to_string(&self, path: &Path) -> PlatformResult<String> {
            match self.node(path, PlatformOperation::ReadFile)? {
                FakeNode::File(bytes) => String::from_utf8(bytes.clone()).map_err(|error| {
                    PlatformError::parse_error(
                        PlatformOperation::ReadFile,
                        path.display().to_string(),
                        error.to_string(),
                    )
                }),
                FakeNode::Directory => Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::InvalidInput,
                    "File read target is invalid.",
                )),
                FakeNode::UnreadableFile => {
                    Err(Self::permission_denied(path, PlatformOperation::ReadFile))
                }
            }
        }

        fn read_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            self.node(path, PlatformOperation::ReadDirectory)?;
            Ok(self
                .nodes
                .iter()
                .filter(|(candidate, _)| candidate.parent() == Some(path))
                .map(|(candidate, node)| DirectoryEntry::new(candidate.clone(), node.file_type()))
                .collect())
        }

        fn walk_dir(&self, path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            self.node(path, PlatformOperation::WalkDirectory)?;
            Ok(self
                .nodes
                .iter()
                .filter(|(candidate, _)| candidate == &path || candidate.starts_with(path))
                .map(|(candidate, node)| DirectoryEntry::new(candidate.clone(), node.file_type()))
                .collect())
        }
    }

    impl FakeNode {
        fn file_type(&self) -> FileType {
            match self {
                Self::File(_) | Self::UnreadableFile => FileType::File,
                Self::Directory => FileType::Directory,
            }
        }
    }

    #[derive(Debug, Default)]
    struct FakeProcessInspector {
        versions: BTreeMap<PathBuf, PlatformResult<Option<VersionMetadata>>>,
    }

    impl FakeProcessInspector {
        fn with_raw_version(mut self, path: impl Into<PathBuf>, raw: &str) -> Self {
            self.versions
                .insert(path.into(), Ok(Some(raw_version_metadata(raw))));
            self
        }

        fn with_version_error(mut self, path: impl Into<PathBuf>) -> Self {
            let path = path.into();
            self.versions.insert(
                path.clone(),
                Err(PlatformError::new(
                    PlatformOperation::ReadVersionMetadata,
                    path.display().to_string(),
                    PlatformErrorKind::UnsupportedPlatform,
                    "Version metadata read is not supported on this platform.",
                )),
            );
            self
        }
    }

    impl ProcessInspector for FakeProcessInspector {
        fn list_processes(&self) -> PlatformResult<Vec<ProcessInfo>> {
            Ok(Vec::new())
        }

        fn file_version(&self, path: &Path) -> PlatformResult<Option<VersionMetadata>> {
            self.versions.get(path).cloned().unwrap_or(Ok(None))
        }

        fn system_metadata(&self) -> PlatformResult<SystemMetadata> {
            Ok(SystemMetadata::new(
                "Windows",
                Some("11 24H2"),
                "x86_64",
                Some("Fake CPU"),
                Some(16 * 1024 * 1024 * 1024),
                Some(8),
            ))
        }
    }

    fn raw_version_metadata(raw: &str) -> VersionMetadata {
        let mut parts = raw
            .split('.')
            .filter_map(|part| part.parse::<u64>().ok())
            .collect::<Vec<_>>();
        parts.resize(3, 0);
        VersionMetadata::new(
            SemanticVersion::new(parts[0], parts[1], parts[2]),
            Some(raw.to_owned()),
        )
    }

    fn installation() -> Fallout4Installation {
        Fallout4Installation::with_optional_paths(
            "C:/Games/Fallout 4",
            Some("C:/Games/Fallout 4/Data"),
            Some("C:/Games/Fallout 4/Data/F4SE/Plugins"),
        )
    }

    fn environment() -> OverviewCollectionEnvironment {
        OverviewCollectionEnvironment::new().with_local_appdata("C:/Users/Example/AppData/Local")
    }

    fn collect(
        fs: &FakeFilesystem,
        process: &FakeProcessInspector,
        installation: &Fallout4Installation,
        environment: &OverviewCollectionEnvironment,
    ) -> OverviewCollectedFacts {
        OverviewCollector::new(fs, process)
            .collect(OverviewCollectionRequest::new(installation, environment))
    }

    fn base_fs() -> FakeFilesystem {
        FakeFilesystem::default()
            .with_dir("C:/Games/Fallout 4")
            .with_dir("C:/Games/Fallout 4/Data")
            .with_dir("C:/Users/Example/AppData/Local/Fallout4")
            .with_file("C:/Games/Fallout 4/Fallout4.exe", b"fallout4".to_vec())
            .with_file(
                "C:/Games/Fallout 4/Fallout4Launcher.exe",
                b"unknown".to_vec(),
            )
            .with_file("C:/Games/Fallout 4/steam_api64.dll", b"steam".to_vec())
            .with_file("C:/Games/Fallout 4/f4se_loader.exe", b"f4se".to_vec())
            .with_file(
                "C:/Games/Fallout 4/f4se_steam_loader.dll",
                b"loader".to_vec(),
            )
            .with_file("C:/Games/Fallout 4/CreationKit.exe", b"ck".to_vec())
            .with_file(
                "C:/Games/Fallout 4/Tools/Archive2/Archive2.exe",
                b"archive2".to_vec(),
            )
            .with_text("C:/Games/Fallout 4/Fallout4.ccc", "")
            .with_text("C:/Users/Example/AppData/Local/Fallout4/plugins.txt", "")
    }

    fn ba2(version: u32, format: &[u8; 4]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BTDX");
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(format);
        bytes.extend_from_slice(b"payload");
        bytes
    }

    fn module(flags: u32, hedr: [u8; 4]) -> Vec<u8> {
        let mut bytes = vec![0u8; MODULE_HEADER_LEN];
        bytes[0..4].copy_from_slice(b"TES4");
        bytes[8..12].copy_from_slice(&flags.to_le_bytes());
        bytes[24..28].copy_from_slice(b"HEDR");
        bytes[30..34].copy_from_slice(&hedr);
        bytes
    }

    fn row<'a>(facts: &'a [OverviewBinaryFact], name: &str) -> &'a OverviewBinaryFact {
        facts
            .iter()
            .find(|fact| path_basename_eq(&fact.file_name, name))
            .expect("binary fact should exist")
    }

    #[test]
    fn overview_collector_classifies_binary_versions_crc_fallback_unknown_and_missing_base_files() {
        assert_eq!(crc32_upper(&[0x40, 0x5a, 0x64, 0x4c]), NG_STARTUP_BA2_CRC);

        let fs = base_fs()
            .with_file("C:/Games/Fallout 4/Data/Fallout4 - Startup.ba2", {
                let mut bytes = ba2(8, b"GNRL");
                bytes.truncate(BA2_HEADER_LEN);
                bytes.extend_from_slice(&[0x40, 0x5a, 0x64, 0x4c]);
                bytes
            })
            .with_file(
                "C:/Games/Fallout 4/Fallout4Launcher.exe",
                [0x0e, 0x27, 0x0c, 0xb3],
            );
        let process = FakeProcessInspector::default()
            .with_raw_version("C:/Games/Fallout 4/Fallout4.exe", "1.10.163.0")
            .with_raw_version("C:/Games/Fallout 4/steam_api64.dll", "7.40.51.27")
            .with_raw_version("C:/Games/Fallout 4/f4se_loader.exe", "9.9.9.9");

        let facts = collect(&fs, &process, &installation(), &environment());

        assert_eq!(
            row(&facts.binaries, "Fallout4.exe").install_type,
            Fallout4InstallType::DownGrade
        );
        assert_eq!(
            row(&facts.binaries, "Fallout4Launcher.exe").install_type,
            Fallout4InstallType::OldGen
        );
        assert_eq!(
            row(&facts.binaries, "steam_api64.dll").install_type,
            Fallout4InstallType::NextGenAnniversary
        );
        assert_eq!(
            row(&facts.binaries, "f4se_loader.exe").install_type,
            Fallout4InstallType::Unknown
        );
        assert_eq!(
            row(&facts.binaries, "CreationKit.exe").install_type,
            Fallout4InstallType::Unknown
        );
        assert_eq!(facts.diagnostics.binary_count, BASE_FILES.len());
        assert_eq!(
            row(&facts.binaries, "Fallout4Launcher.exe").hash.as_deref(),
            Some("02445570")
        );
    }

    #[test]
    fn overview_collector_reports_missing_base_data_and_address_library_without_panics() {
        let mut missing_data_installation = Fallout4Installation::new("C:/Games/Fallout 4");
        missing_data_installation.install_type = Fallout4InstallType::Unknown;
        let fs = FakeFilesystem::default()
            .with_dir("C:/Games/Fallout 4")
            .with_file("C:/Games/Fallout 4/Fallout4.exe", b"fallout4".to_vec());
        let process = FakeProcessInspector::default()
            .with_raw_version("C:/Games/Fallout 4/Fallout4.exe", "1.10.984.0");

        let facts = collect(&fs, &process, &missing_data_installation, &environment());

        assert_eq!(facts.archives, Vec::new());
        assert_eq!(facts.modules, Vec::new());
        assert!(matches!(
            facts.enablement.address_library,
            OverviewAddressLibraryFact {
                required: false,
                ..
            }
        ));
        assert!(facts.diagnostics.errors.iter().any(|error| {
            error.phase == OverviewCollectionPhase::DataTraversal
                && error.kind == OverviewCollectionDiagnosticKind::Skipped
        }));

        let fs = base_fs();
        let facts = collect(&fs, &process, &installation(), &environment());
        assert!(matches!(
            facts.enablement.address_library,
            OverviewAddressLibraryFact { required: true, .. }
        ));
        assert!(facts.diagnostics.errors.iter().any(|error| {
            error.phase == OverviewCollectionPhase::AddressLibrary
                && error.kind == OverviewCollectionDiagnosticKind::Missing
        }));
    }

    #[test]
    fn overview_collector_reads_bounded_ba2_headers_and_classifies_versions_formats_and_errors() {
        let mut install = installation();
        install
            .ini_files
            .fallout4
            .sections
            .entry("archive".to_owned())
            .or_default()
            .insert(
                "sresourcearchivelist".to_owned(),
                "Old.ba2, Seven.ba2, Eight.ba2, Unknown.ba2, Short.ba2, BadMagic.ba2, Locked.ba2"
                    .to_owned(),
            );
        let fs = base_fs()
            .with_file("C:/Games/Fallout 4/Data/Old.ba2", ba2(1, b"GNRL"))
            .with_file("C:/Games/Fallout 4/Data/Seven.ba2", ba2(7, b"DX10"))
            .with_file("C:/Games/Fallout 4/Data/Eight.ba2", ba2(8, b"GNRL"))
            .with_file("C:/Games/Fallout 4/Data/Unknown.ba2", ba2(42, b"GNRL"))
            .with_file("C:/Games/Fallout 4/Data/Short.ba2", b"BTDX".to_vec())
            .with_file("C:/Games/Fallout 4/Data/BadMagic.ba2", {
                let mut bytes = ba2(8, b"GNRL");
                bytes[0..4].copy_from_slice(b"NOPE");
                bytes
            })
            .with_unreadable_file("C:/Games/Fallout 4/Data/Locked.ba2");
        let process = FakeProcessInspector::default()
            .with_raw_version("C:/Games/Fallout 4/Fallout4.exe", "1.10.984.0");

        let facts = collect(&fs, &process, &install, &environment());

        let versions = facts
            .archives
            .iter()
            .map(|record| {
                (
                    record
                        .path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    record.version,
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(versions["Old.ba2"], ArchiveVersion::OldGen);
        assert_eq!(versions["Seven.ba2"], ArchiveVersion::NextGen7);
        assert_eq!(versions["Eight.ba2"], ArchiveVersion::NextGen8);
        assert_eq!(versions["Unknown.ba2"], ArchiveVersion::Unknown(42));
        assert!(
            facts
                .archives
                .iter()
                .any(|record| record.path.ends_with("Locked.ba2")
                    && !record.readable
                    && record.enabled)
        );
        assert!(
            fs.prefix_reads
                .borrow()
                .iter()
                .filter(|(_, len)| *len == BA2_HEADER_LEN)
                .count()
                >= 7
        );
        assert!(
            !fs.full_reads
                .borrow()
                .iter()
                .any(|path| path.extension().is_some_and(|extension| extension == "ba2"))
        );
    }

    #[test]
    fn overview_collector_reads_bounded_module_headers_and_classifies_light_full_hedr_and_unreadable()
     {
        let fs = base_fs()
            .with_text(
                "C:/Users/Example/AppData/Local/Fallout4/plugins.txt",
                "*Full.esp\n*LightFlag.esp\n*LightExt.esl\n*OldHedr.esm\n*UnknownHedr.esp\n*Locked.esp\n*BadHedr.esp\n",
            )
            .with_file("C:/Games/Fallout 4/Data/Full.esp", module(0, MODULE_VERSION_100))
            .with_file(
                "C:/Games/Fallout 4/Data/LightFlag.esp",
                module(MODULE_LIGHT_FLAG, MODULE_VERSION_100),
            )
            .with_file("C:/Games/Fallout 4/Data/LightExt.esl", module(0, MODULE_VERSION_100))
            .with_file("C:/Games/Fallout 4/Data/OldHedr.esm", module(0, MODULE_VERSION_095))
            .with_file("C:/Games/Fallout 4/Data/UnknownHedr.esp", module(0, 0.94f32.to_le_bytes()))
            .with_file("C:/Games/Fallout 4/Data/BadHedr.esp", {
                let mut bytes = module(0, MODULE_VERSION_100);
                bytes[24..28].copy_from_slice(b"XXXX");
                bytes
            })
            .with_unreadable_file("C:/Games/Fallout 4/Data/Locked.esp");
        let process = FakeProcessInspector::default()
            .with_raw_version("C:/Games/Fallout 4/Fallout4.exe", "1.10.984.0");

        let facts = collect(&fs, &process, &installation(), &environment());

        let modules = facts
            .modules
            .iter()
            .map(|record| {
                (
                    record
                        .path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    record,
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(modules["Full.esp"].kind, ModuleKind::Full);
        assert_eq!(modules["LightFlag.esp"].kind, ModuleKind::Light);
        assert_eq!(modules["LightExt.esl"].kind, ModuleKind::Light);
        assert_eq!(
            modules["Full.esp"].header_version,
            ModuleHeaderVersion::Version100
        );
        assert_eq!(
            modules["OldHedr.esm"].header_version,
            ModuleHeaderVersion::Version095
        );
        assert_eq!(
            modules["UnknownHedr.esp"].header_version,
            ModuleHeaderVersion::Unknown("0.94".to_owned())
        );
        assert!(!modules["Locked.esp"].readable);
        assert!(!modules["BadHedr.esp"].readable);
        assert!(
            fs.prefix_reads
                .borrow()
                .iter()
                .filter(|(_, len)| *len == MODULE_HEADER_LEN)
                .count()
                >= 7
        );
    }

    #[test]
    fn overview_collector_parses_enablement_files_ini_archives_and_plugins_fallback() {
        let mut install = installation();
        install
            .ini_files
            .fallout4
            .sections
            .entry("archive".to_owned())
            .or_default()
            .insert(
                "sresourcearchivelist".to_owned(),
                " Fallout4 - Startup.ba2, MissingFromIni.ba2 ".to_owned(),
            );
        install
            .ini_files
            .custom
            .sections
            .entry("general".to_owned())
            .or_default()
            .insert("slanguage".to_owned(), "fr".to_owned());
        install
            .ini_files
            .prefs
            .sections
            .entry("nvflex".to_owned())
            .or_default()
            .insert("bnvflexenable".to_owned(), "1".to_owned());

        let fs = base_fs()
            .with_file(
                "C:/Games/Fallout 4/Data/Fallout4.esm",
                module(0, MODULE_VERSION_100),
            )
            .with_file(
                "C:/Games/Fallout 4/Data/PluginA.esp",
                module(0, MODULE_VERSION_100),
            )
            .with_file(
                "C:/Games/Fallout 4/Data/PluginB.esl",
                module(0, MODULE_VERSION_100),
            )
            .with_file(
                "C:/Games/Fallout 4/Data/PluginB - voices_fr.ba2",
                ba2(8, b"DX10"),
            )
            .with_file(
                "C:/Games/Fallout 4/Data/Fallout4 - Startup.ba2",
                ba2(8, b"GNRL"),
            )
            .with_file(
                "C:/Games/Fallout 4/Data/Fallout4 - Nvflex.ba2",
                ba2(1, b"GNRL"),
            );
        let process = FakeProcessInspector::default()
            .with_raw_version("C:/Games/Fallout 4/Fallout4.exe", "1.10.984.0");
        let missing_plugins_env = OverviewCollectionEnvironment::new()
            .with_plugins_txt("C:/Users/Example/AppData/Local/Fallout4/missing-plugins.txt");

        let facts = collect(&fs, &process, &install, &missing_plugins_env);

        assert!(matches!(
            facts.enablement.fallout4_ccc,
            OverviewFilePresence::Present(_)
        ));
        assert!(matches!(
            facts.enablement.plugins_txt,
            OverviewFilePresence::Missing(_)
        ));
        assert!(facts.modules.iter().all(|record| record.enabled));
        assert!(
            facts
                .archives
                .iter()
                .any(|record| record.path.ends_with("PluginB - voices_fr.ba2") && record.enabled)
        );
        assert!(
            facts
                .archives
                .iter()
                .any(|record| record.path.ends_with("Fallout4 - Nvflex.ba2") && record.enabled)
        );
        assert!(facts.diagnostics.errors.iter().any(|error| {
            error
                .path
                .as_ref()
                .is_some_and(|path| path.ends_with("MissingFromIni.ba2"))
                && error.kind == OverviewCollectionDiagnosticKind::Missing
        }));
    }

    #[test]
    fn overview_collector_marks_missing_ccc_and_non_utf8_plugins_without_modal_fallback_failure() {
        let fs = FakeFilesystem::default()
            .with_dir("C:/Games/Fallout 4")
            .with_dir("C:/Games/Fallout 4/Data")
            .with_dir("C:/Users/Example/AppData/Local/Fallout4")
            .with_file("C:/Games/Fallout 4/Fallout4.exe", b"fallout4".to_vec())
            .with_file(
                "C:/Games/Fallout 4/Data/AnyPlugin.esp",
                module(0, MODULE_VERSION_100),
            )
            .with_file(
                "C:/Users/Example/AppData/Local/Fallout4/plugins.txt",
                vec![0xff, 0xfe, 0xfd],
            );
        let process = FakeProcessInspector::default()
            .with_raw_version("C:/Games/Fallout 4/Fallout4.exe", "1.10.984.0");

        let facts = collect(&fs, &process, &installation(), &environment());

        assert!(matches!(
            facts.enablement.fallout4_ccc,
            OverviewFilePresence::Missing(_)
        ));
        assert!(matches!(
            facts.enablement.plugins_txt,
            OverviewFilePresence::Unreadable(_)
        ));
        assert_eq!(facts.modules.len(), 1);
        assert!(facts.modules[0].enabled);
        assert!(facts.diagnostics.errors.iter().any(|error| {
            error.phase == OverviewCollectionPhase::Enablement
                && error.kind == OverviewCollectionDiagnosticKind::Invalid
        }));
    }

    #[test]
    fn overview_collector_records_version_adapter_failures_and_keeps_crc_fallback_testable() {
        let fs = base_fs().with_file(
            "C:/Games/Fallout 4/Fallout4Launcher.exe",
            [0x0e, 0x27, 0x0c, 0xb3],
        );
        let process = FakeProcessInspector::default()
            .with_version_error("C:/Games/Fallout 4/Fallout4Launcher.exe");

        let facts = collect(&fs, &process, &installation(), &environment());

        assert_eq!(
            row(&facts.binaries, "Fallout4Launcher.exe").install_type,
            Fallout4InstallType::OldGen
        );
        assert!(facts.diagnostics.errors.iter().any(|error| {
            error.phase == OverviewCollectionPhase::Binaries
                && error.kind == OverviewCollectionDiagnosticKind::Unsupported
        }));
    }
}
