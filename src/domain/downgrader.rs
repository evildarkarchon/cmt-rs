//! Slint-free Downgrader workflow contract.
//!
//! The reference Tkinter modal lives in `CMT/src/downgrader.py`, with shared
//! copy in `CMT/src/globals.py` and status labels in `CMT/src/enums.py`. This
//! module freezes those labels, file definitions, CRC classifications, backup
//! names, patch names, and row payload shapes as pure Rust data. It deliberately
//! performs no filesystem, network, xdelta, settings, or Slint work.

use std::fmt;

/// Reference modal title passed to `ModalWindow`.
pub const DOWNGRADER_MODAL_TITLE: &str = "Downgrader";
/// Reference modal width in logical pixels.
pub const DOWNGRADER_MODAL_WIDTH: i32 = 600;
/// Reference modal height in logical pixels.
pub const DOWNGRADER_MODAL_HEIGHT: i32 = 334;

/// Reference labelframe text for the current game files.
pub const CURRENT_GAME_GROUP_LABEL: &str = "Current Game";
/// Reference labelframe text for the current Creation Kit files.
pub const CURRENT_CREATION_KIT_GROUP_LABEL: &str = "Current Creation Kit";
/// Reference labelframe text for the desired-version radios.
pub const DESIRED_VERSION_GROUP_LABEL: &str = "Desired Version";
/// Reference labelframe text for downgrade options.
pub const OPTIONS_GROUP_LABEL: &str = "Options";

/// Reference desired-version radio label for the old-gen target.
pub const TARGET_OLD_GEN_LABEL: &str = "Old-Gen";
/// Reference desired-version radio label for the next-gen target.
pub const TARGET_NEXT_GEN_LABEL: &str = "Next-Gen";
/// Reference keep-backups checkbox label.
pub const KEEP_BACKUPS_CHECKBOX_LABEL: &str = "Keep Backups";
/// Reference delete-deltas checkbox label.
pub const DELETE_PATCHES_CHECKBOX_LABEL: &str = "Delete Patches";
/// Reference patch button label, preserving the intentional newline and space.
pub const PATCH_ALL_BUTTON_LABEL: &str = "Patch\n All";
/// Reference about button label.
pub const ABOUT_BUTTON_LABEL: &str = "About";
/// Initial modal log line written without forwarding to the file logger.
pub const INITIAL_LOG_LINE: &str = "Patches will be downloaded and applied as-needed.";

/// Reference title for the downgrading About dialog.
pub const ABOUT_DOWNGRADING_TITLE: &str = "About Downgrading Fallout 4 & Creation Kit";
/// Reference body copy for the downgrading About dialog.
pub const ABOUT_DOWNGRADING_BODY: &str = "This downgrader makes use of delta patches which are downloaded as-needed from the CMT GitHub page.\nPatches range in size from 23KB to 63MB.\n\nBackups are created prior to patching, and will be used instead of patches if present.\nSimple Downgrader's backups will also be used.\nBackup naming:\nFallout4_downgradeBackup.exe\nFallout4_upgradeBackup.exe\n\nBoth Creation Kit and the game require steam_api64.dll to match their version, so they must be patched together (for now).";

/// Reference tooltip for the keep-backups checkbox.
pub const TOOLTIP_DOWNGRADER_BACKUPS: &str = "Backups are created prior to patching, and will be used instead of patches if present.\nSimple Downgrader's backups will also be used.\n\nBackups allow quicker switching between versions.\nUncheck this to delete backups after patching.";
/// Reference tooltip for the delete-patches checkbox.
pub const TOOLTIP_DOWNGRADER_DELTAS: &str = "Delta patches are downloaded as-needed and used to patch from one version to another.\nIf backups are present, they will be used instead.\n\nThese xdelta files are only needed during the patching process.\nCheck this to delete xdeltas after patching.";

/// Reference release asset base URL for xdelta patches.
pub const PATCH_URL_BASE: &str =
    "https://github.com/wxMichael/Collective-Modding-Toolkit/releases/download/delta-patches/";
/// Reference xdelta patch direction used when downgrading from next-gen to old-gen.
pub const PATCH_DIRECTION_NEXT_GEN_TO_OLD_GEN: &str = "NG-to-OG-";
/// Reference xdelta patch direction used when upgrading from old-gen to next-gen.
pub const PATCH_DIRECTION_OLD_GEN_TO_NEXT_GEN: &str = "OG-to-NG-";

const UPGRADE_BACKUP_MARKER: &str = "_upgradeBackup";
const DOWNGRADE_BACKUP_MARKER: &str = "_downgradeBackup";

/// Log row levels matching `CMT/src/enums.py::LogType` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DowngraderLogLevel {
    /// Reference informational log row.
    Info,
    /// Reference successful patch row.
    Good,
    /// Reference failed patch row.
    Bad,
}

impl DowngraderLogLevel {
    /// Returns the exact string value used by the reference `LogType` enum.
    pub const fn as_reference_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Good => "good",
            Self::Bad => "bad",
        }
    }
}

/// Install/status labels used by the Downgrader modal.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DowngraderInstallStatus {
    /// Reference `Obsolete` state.
    Obsolete,
    /// Reference `Old-Gen` state.
    OldGen,
    /// Reference `Next-Gen` state.
    NextGen,
    /// Reference `Anniversary` state.
    Anniversary,
    /// Reference `Next-Gen & Anniversary` state used by shared Steam API files.
    NextGenAnniversary,
    /// Reference status for an unrecognized CRC.
    #[default]
    Unknown,
    /// Reference status for a missing file.
    NotFound,
}

impl DowngraderInstallStatus {
    /// Returns the exact user-facing label used by the reference `InstallType` enum.
    pub const fn as_reference_str(self) -> &'static str {
        match self {
            Self::Obsolete => "Obsolete",
            Self::OldGen => "Old-Gen",
            Self::NextGen => "Next-Gen",
            Self::Anniversary => "Anniversary",
            Self::NextGenAnniversary => "Next-Gen & Anniversary",
            Self::Unknown => "Unknown",
            Self::NotFound => "Not Found",
        }
    }

    /// Returns true when the reference CRC-by-type table treats this status as NG.
    ///
    /// `Next-Gen & Anniversary` CRCs are inserted into both NG and AE buckets by
    /// the Python reference because the same binary is shared by both versions.
    pub const fn counts_as_next_gen_crc(self) -> bool {
        matches!(self, Self::NextGen | Self::NextGenAnniversary)
    }

    /// Returns true when the reference CRC-by-type table treats this status as AE.
    ///
    /// `Next-Gen & Anniversary` CRCs are inserted into both NG and AE buckets by
    /// the Python reference because the same binary is shared by both versions.
    pub const fn counts_as_anniversary_crc(self) -> bool {
        matches!(self, Self::Anniversary | Self::NextGenAnniversary)
    }
}

impl fmt::Display for DowngraderInstallStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_reference_str())
    }
}

/// Desired target chosen by the Downgrader radio buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DowngraderTarget {
    /// Patch eligible files to the old-gen state.
    OldGen,
    /// Patch eligible files to the next-gen state.
    NextGen,
}

impl DowngraderTarget {
    /// Returns the reference radio label for this target.
    pub const fn as_reference_str(self) -> &'static str {
        match self {
            Self::OldGen => TARGET_OLD_GEN_LABEL,
            Self::NextGen => TARGET_NEXT_GEN_LABEL,
        }
    }

    /// Returns the install status considered already complete for this target.
    pub const fn desired_status(self) -> DowngraderInstallStatus {
        match self {
            Self::OldGen => DowngraderInstallStatus::OldGen,
            Self::NextGen => DowngraderInstallStatus::NextGen,
        }
    }

    /// Returns the reference xdelta patch direction prefix.
    pub const fn patch_direction(self) -> &'static str {
        match self {
            Self::OldGen => PATCH_DIRECTION_NEXT_GEN_TO_OLD_GEN,
            Self::NextGen => PATCH_DIRECTION_OLD_GEN_TO_NEXT_GEN,
        }
    }

    /// Builds the reference xdelta patch file name for a target and file path.
    pub fn patch_name_for(self, file_name_or_path: &str) -> String {
        format!(
            "{}{}.xdelta",
            self.patch_direction(),
            reference_file_name(file_name_or_path)
        )
    }

    /// Builds the reference GitHub release URL for a target and file path.
    pub fn patch_url_for(self, file_name_or_path: &str) -> String {
        format!(
            "{}{}",
            PATCH_URL_BASE,
            self.patch_name_for(file_name_or_path)
        )
    }

    /// Returns the backup filename that should contain the desired target version.
    ///
    /// This mirrors `backup_file_name_desired` from the Python reference.
    pub fn desired_backup_name_for(self, file_name_or_path: &str) -> String {
        match self {
            Self::OldGen => upgrade_backup_name(file_name_or_path),
            Self::NextGen => downgrade_backup_name(file_name_or_path),
        }
    }

    /// Returns the backup filename that should contain the current source version.
    ///
    /// This mirrors `backup_file_name_current` from the Python reference.
    pub fn current_backup_name_for(self, file_name_or_path: &str) -> String {
        match self {
            Self::OldGen => downgrade_backup_name(file_name_or_path),
            Self::NextGen => upgrade_backup_name(file_name_or_path),
        }
    }
}

impl fmt::Display for DowngraderTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_reference_str())
    }
}

/// Reference group for a downgrader-managed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DowngraderFileGroup {
    /// File belongs in the `Current Game` group.
    Game,
    /// File belongs in the `Current Creation Kit` group.
    CreationKit,
}

impl DowngraderFileGroup {
    /// Returns the exact labelframe label for this group.
    pub const fn as_reference_str(self) -> &'static str {
        match self {
            Self::Game => CURRENT_GAME_GROUP_LABEL,
            Self::CreationKit => CURRENT_CREATION_KIT_GROUP_LABEL,
        }
    }
}

/// Static CRC classification for one reference-managed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DowngraderCrcMapping {
    /// Uppercase eight-character CRC32 string from the Python reference.
    pub crc32: &'static str,
    /// Install status associated with the CRC.
    pub status: DowngraderInstallStatus,
}

impl DowngraderCrcMapping {
    const fn new(crc32: &'static str, status: DowngraderInstallStatus) -> Self {
        Self { crc32, status }
    }
}

/// Reference-managed file definition in modal display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DowngraderFileDefinition {
    /// Relative path from the Fallout 4 game directory, preserving reference `\\` separators.
    pub relative_path: &'static str,
    /// Basename displayed in the modal status labels.
    pub display_name: &'static str,
    /// Reference modal group for this file.
    pub group: DowngraderFileGroup,
    /// CRC-to-status mappings copied from `CMT/src/downgrader.py`.
    pub crc_mappings: &'static [DowngraderCrcMapping],
}

impl DowngraderFileDefinition {
    /// Looks up a CRC classification using the reference maps.
    pub fn status_for_crc(self, crc32: &str) -> Option<DowngraderInstallStatus> {
        self.crc_mappings
            .iter()
            .find(|mapping| mapping.crc32.eq_ignore_ascii_case(crc32))
            .map(|mapping| mapping.status)
    }

    /// Returns a status row for this definition.
    pub const fn status_row(self, status: DowngraderInstallStatus) -> DowngraderStatusRow {
        DowngraderStatusRow {
            relative_path: self.relative_path,
            display_name: self.display_name,
            group: self.group,
            status,
        }
    }
}

/// Render-ready status row for the file status panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DowngraderStatusRow {
    /// Relative path from the Fallout 4 game directory.
    pub relative_path: &'static str,
    /// Basename displayed in the modal.
    pub display_name: &'static str,
    /// Reference group containing the row.
    pub group: DowngraderFileGroup,
    /// Current install status displayed beside the file name.
    pub status: DowngraderInstallStatus,
}

impl DowngraderStatusRow {
    /// Returns the user-facing status label for this row.
    pub const fn status_label(self) -> &'static str {
        self.status.as_reference_str()
    }
}

/// Snapshot of user-selectable downgrader options at run time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DowngraderOptionsSnapshot {
    /// Desired target selected by the radio buttons.
    pub target: DowngraderTarget,
    /// Whether existing and newly-created backups should be kept after patching.
    pub keep_backups: bool,
    /// Whether xdelta patch files should be deleted after patching.
    pub delete_deltas: bool,
}

impl DowngraderOptionsSnapshot {
    /// Creates a pure options snapshot from UI/settings state.
    pub const fn new(target: DowngraderTarget, keep_backups: bool, delete_deltas: bool) -> Self {
        Self {
            target,
            keep_backups,
            delete_deltas,
        }
    }
}

/// Reference-style plan action for one file before IO-specific checks run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DowngraderPlanAction {
    /// File can be skipped because it already matches the target version.
    SkipAlreadyDesired,
    /// File can be skipped because it was not found.
    SkipNotFound,
    /// File can be skipped because the outer reference workflow rejects the status.
    SkipUnsupportedVersion,
    /// File should advance to backup/CRC validation and possible delta patching.
    ValidateBackupOrPatch,
}

impl DowngraderPlanAction {
    /// Computes the same first-pass action categories as `Downgrader.patch_files`.
    pub const fn from_status(
        current_status: DowngraderInstallStatus,
        target: DowngraderTarget,
    ) -> Self {
        if matches!(current_status, DowngraderInstallStatus::NotFound) {
            Self::SkipNotFound
        } else if matches!(
            current_status,
            DowngraderInstallStatus::Anniversary | DowngraderInstallStatus::Obsolete
        ) {
            Self::SkipUnsupportedVersion
        } else if matches!(
            (current_status, target),
            (DowngraderInstallStatus::OldGen, DowngraderTarget::OldGen)
                | (DowngraderInstallStatus::NextGen, DowngraderTarget::NextGen)
        ) {
            Self::SkipAlreadyDesired
        } else {
            Self::ValidateBackupOrPatch
        }
    }

    /// Returns true when later service code must inspect backups/CRC/source files.
    pub const fn requires_worker(self) -> bool {
        matches!(self, Self::ValidateBackupOrPatch)
    }
}

/// Detailed preview step kind for one file in the inline Downgrader plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DowngraderPlanStepKind {
    /// File will be skipped because it already matches the selected target.
    SkipAlreadyDesired,
    /// File will be skipped because it is not present in the game root.
    SkipNotFound,
    /// File will be skipped because its CRC cannot be used as a patch source.
    SkipUnsupportedVersion,
    /// Existing backup of the current version can be reused as the patch source.
    ReuseCurrentBackup,
    /// Existing backup is not useful for the current file and should be deleted before running.
    DeleteInvalidCurrentBackup,
    /// Existing desired-version backup has the wrong CRC and should be deleted before running.
    DeleteInvalidDesiredBackup,
    /// Current file should be backed up before restore or patch work mutates it.
    CreateCurrentBackup,
    /// Desired-version backup should be restored instead of downloading a delta.
    RestoreDesiredBackup,
    /// Delta patch asset must be downloaded before patching can continue.
    DownloadDelta,
    /// Delta patch should be applied to the current-version backup.
    ApplyDeltaPatch,
    /// Current-version backup should be deleted after a successful restore or patch.
    DeleteCurrentBackup,
    /// Downloaded delta patch should be deleted after a successful patch.
    DeleteDeltaPatch,
    /// Planning failed safely before any mutation could be attempted.
    PlanFailure,
}

impl DowngraderPlanStepKind {
    /// Returns a stable lowercase diagnostic label for tracing and tests.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SkipAlreadyDesired => "skip_already_desired",
            Self::SkipNotFound => "skip_not_found",
            Self::SkipUnsupportedVersion => "skip_unsupported_version",
            Self::ReuseCurrentBackup => "reuse_current_backup",
            Self::DeleteInvalidCurrentBackup => "delete_invalid_current_backup",
            Self::DeleteInvalidDesiredBackup => "delete_invalid_desired_backup",
            Self::CreateCurrentBackup => "create_current_backup",
            Self::RestoreDesiredBackup => "restore_desired_backup",
            Self::DownloadDelta => "download_delta",
            Self::ApplyDeltaPatch => "apply_delta_patch",
            Self::DeleteCurrentBackup => "delete_current_backup",
            Self::DeleteDeltaPatch => "delete_delta_patch",
            Self::PlanFailure => "plan_failure",
        }
    }

    /// Returns true when this step represents work that mutates files during execution.
    pub const fn is_mutating_execution_step(self) -> bool {
        !matches!(
            self,
            Self::SkipAlreadyDesired
                | Self::SkipNotFound
                | Self::SkipUnsupportedVersion
                | Self::PlanFailure
        )
    }
}

/// Render-ready detail row for inline plan confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderPlanStep {
    /// Machine-readable step kind for controllers, tests, and tracing.
    pub kind: DowngraderPlanStepKind,
    /// Safe user-facing summary of what a later confirmed run would do.
    pub message: String,
}

impl DowngraderPlanStep {
    /// Creates a preview step without performing any filesystem mutation.
    pub fn new(kind: DowngraderPlanStepKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Pure row describing the planned treatment of one downgrader-managed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderPlanRow {
    /// Relative path from the Fallout 4 game directory.
    pub relative_path: &'static str,
    /// Basename displayed in log and plan rows.
    pub display_name: &'static str,
    /// Reference file group for status-panel reuse.
    pub group: DowngraderFileGroup,
    /// Current install status known before IO-specific backup checks.
    pub current_status: DowngraderInstallStatus,
    /// Desired target selected by the user.
    pub target: DowngraderTarget,
    /// First-pass reference action for this row.
    pub action: DowngraderPlanAction,
    /// Backup name expected to hold the desired target version.
    pub desired_backup_name: String,
    /// Backup name expected to hold the current source version.
    pub current_backup_name: String,
    /// Xdelta patch asset name that would be used if a download is needed.
    pub patch_name: String,
    /// Xdelta patch URL that would be downloaded if needed.
    pub patch_url: String,
}

impl DowngraderPlanRow {
    /// Builds a pure plan row from a static file definition, status, and options.
    pub fn from_definition(
        definition: DowngraderFileDefinition,
        current_status: DowngraderInstallStatus,
        options: DowngraderOptionsSnapshot,
    ) -> Self {
        let target = options.target;
        Self {
            relative_path: definition.relative_path,
            display_name: definition.display_name,
            group: definition.group,
            current_status,
            target,
            action: DowngraderPlanAction::from_status(current_status, target),
            desired_backup_name: target.desired_backup_name_for(definition.display_name),
            current_backup_name: target.current_backup_name_for(definition.display_name),
            patch_name: target.patch_name_for(definition.display_name),
            patch_url: target.patch_url_for(definition.display_name),
        }
    }

    /// Returns the reference-style log row for first-pass skip actions.
    pub fn skip_log_row(&self) -> Option<DowngraderExecutionLogRow> {
        match self.action {
            DowngraderPlanAction::SkipAlreadyDesired => Some(skipped_already_log_row(
                self.display_name,
                self.target.desired_status(),
            )),
            DowngraderPlanAction::SkipNotFound => {
                Some(skipped_not_found_log_row(self.display_name))
            }
            DowngraderPlanAction::SkipUnsupportedVersion => {
                Some(skipped_unsupported_log_row(self.display_name))
            }
            DowngraderPlanAction::ValidateBackupOrPatch => None,
        }
    }
}

/// User-visible execution log row independent of Slint models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowngraderExecutionLogRow {
    /// Reference log level controlling row color.
    pub level: DowngraderLogLevel,
    /// Reference-style user-visible message.
    pub message: String,
}

impl DowngraderExecutionLogRow {
    /// Creates a log row with a caller-provided message.
    pub fn new(level: DowngraderLogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
        }
    }

    /// Creates the initial reference informational row.
    pub fn initial() -> Self {
        Self::new(DowngraderLogLevel::Info, INITIAL_LOG_LINE)
    }
}

/// Progress-bar value used by download and patch workers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DowngraderProgress {
    /// Percentage in the reference progress bar's `0..=100` range.
    pub percent: f32,
}

impl DowngraderProgress {
    /// Creates a clamped progress value suitable for the reference progress bar.
    pub fn new(percent: f32) -> Self {
        Self {
            percent: percent.clamp(0.0, 100.0),
        }
    }

    /// Returns an idle progress value.
    pub const fn idle() -> Self {
        Self { percent: 0.0 }
    }

    /// Returns a completed progress value.
    pub const fn complete() -> Self {
        Self { percent: 100.0 }
    }
}

const FALLOUT4_EXE_CRCS: [DowngraderCrcMapping; 12] = [
    DowngraderCrcMapping::new("97DA3E03", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("2ED2A242", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("A0100017", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("9ABC94F0", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("C6053902", DowngraderInstallStatus::OldGen),
    DowngraderCrcMapping::new("B61675B1", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("C5965A2E", DowngraderInstallStatus::NextGen),
    DowngraderCrcMapping::new("0AEB19A7", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("1E90BE57", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("0481725D", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("0E176ABC", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("CF47788D", DowngraderInstallStatus::Anniversary),
];

const FALLOUT4_LAUNCHER_EXE_CRCS: [DowngraderCrcMapping; 7] = [
    DowngraderCrcMapping::new("02445570", DowngraderInstallStatus::OldGen),
    DowngraderCrcMapping::new("F6A06FF5", DowngraderInstallStatus::NextGen),
    DowngraderCrcMapping::new("0E696744", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("D15C6A49", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("8C52BE93", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("591009C9", DowngraderInstallStatus::Obsolete),
    DowngraderCrcMapping::new("720BB9C3", DowngraderInstallStatus::Anniversary),
];

const STEAM_API64_DLL_CRCS: [DowngraderCrcMapping; 2] = [
    DowngraderCrcMapping::new("BBD912FC", DowngraderInstallStatus::OldGen),
    DowngraderCrcMapping::new("E36E7B4D", DowngraderInstallStatus::NextGenAnniversary),
];

const CREATION_KIT_EXE_CRCS: [DowngraderCrcMapping; 3] = [
    DowngraderCrcMapping::new("0F5C065B", DowngraderInstallStatus::OldGen),
    DowngraderCrcMapping::new("481CCE95", DowngraderInstallStatus::NextGen),
    DowngraderCrcMapping::new("49E45284", DowngraderInstallStatus::Anniversary),
];

const ARCHIVE2_EXE_CRCS: [DowngraderCrcMapping; 3] = [
    DowngraderCrcMapping::new("4CDFC7B5", DowngraderInstallStatus::OldGen),
    DowngraderCrcMapping::new("71A5240B", DowngraderInstallStatus::NextGen),
    DowngraderCrcMapping::new("C867674F", DowngraderInstallStatus::Anniversary),
];

const ARCHIVE2_INTEROP_DLL_CRCS: [DowngraderCrcMapping; 3] = [
    DowngraderCrcMapping::new("850D36A9", DowngraderInstallStatus::OldGen),
    DowngraderCrcMapping::new("EFBE3622", DowngraderInstallStatus::NextGen),
    DowngraderCrcMapping::new("7B893B0D", DowngraderInstallStatus::Anniversary),
];

/// Six reference-managed files in the exact order rendered and patched.
pub const DOWNGRADER_FILE_DEFINITIONS: [DowngraderFileDefinition; 6] = [
    DowngraderFileDefinition {
        relative_path: "Fallout4.exe",
        display_name: "Fallout4.exe",
        group: DowngraderFileGroup::Game,
        crc_mappings: &FALLOUT4_EXE_CRCS,
    },
    DowngraderFileDefinition {
        relative_path: "Fallout4Launcher.exe",
        display_name: "Fallout4Launcher.exe",
        group: DowngraderFileGroup::Game,
        crc_mappings: &FALLOUT4_LAUNCHER_EXE_CRCS,
    },
    DowngraderFileDefinition {
        relative_path: "steam_api64.dll",
        display_name: "steam_api64.dll",
        group: DowngraderFileGroup::Game,
        crc_mappings: &STEAM_API64_DLL_CRCS,
    },
    DowngraderFileDefinition {
        relative_path: "CreationKit.exe",
        display_name: "CreationKit.exe",
        group: DowngraderFileGroup::CreationKit,
        crc_mappings: &CREATION_KIT_EXE_CRCS,
    },
    DowngraderFileDefinition {
        relative_path: "Tools\\Archive2\\Archive2.exe",
        display_name: "Archive2.exe",
        group: DowngraderFileGroup::CreationKit,
        crc_mappings: &ARCHIVE2_EXE_CRCS,
    },
    DowngraderFileDefinition {
        relative_path: "Tools\\Archive2\\Archive2Interop.dll",
        display_name: "Archive2Interop.dll",
        group: DowngraderFileGroup::CreationKit,
        crc_mappings: &ARCHIVE2_INTEROP_DLL_CRCS,
    },
];

/// Returns the three reference game-file definitions.
pub fn game_file_definitions() -> &'static [DowngraderFileDefinition] {
    &DOWNGRADER_FILE_DEFINITIONS[..3]
}

/// Returns the three reference Creation Kit file definitions.
pub fn creation_kit_file_definitions() -> &'static [DowngraderFileDefinition] {
    &DOWNGRADER_FILE_DEFINITIONS[3..]
}

/// Finds a static file definition by relative path or display basename.
pub fn find_file_definition(file_name_or_path: &str) -> Option<DowngraderFileDefinition> {
    let normalized_name = reference_file_name(file_name_or_path);
    DOWNGRADER_FILE_DEFINITIONS
        .iter()
        .copied()
        .find(|definition| {
            definition
                .relative_path
                .eq_ignore_ascii_case(file_name_or_path)
                || definition
                    .display_name
                    .eq_ignore_ascii_case(normalized_name)
        })
}

/// Returns all CRCs that belong to a reference install-status bucket.
///
/// For parity with `CRCs_by_type`, `Next-Gen & Anniversary` entries are included
/// in both the `NextGen` and `Anniversary` bucket queries.
pub fn crcs_for_status(status: DowngraderInstallStatus) -> Vec<&'static str> {
    DOWNGRADER_FILE_DEFINITIONS
        .iter()
        .flat_map(|definition| definition.crc_mappings.iter())
        .filter_map(|mapping| {
            let matches_status = match status {
                DowngraderInstallStatus::NextGen => mapping.status.counts_as_next_gen_crc(),
                DowngraderInstallStatus::Anniversary => mapping.status.counts_as_anniversary_crc(),
                _ => mapping.status == status,
            };
            matches_status.then_some(mapping.crc32)
        })
        .collect()
}

/// Returns the CRC source bucket accepted by the reference for a target patch.
pub fn accepted_source_crcs_for_target(target: DowngraderTarget) -> Vec<&'static str> {
    match target {
        DowngraderTarget::OldGen => crcs_for_status(DowngraderInstallStatus::NextGen),
        DowngraderTarget::NextGen => crcs_for_status(DowngraderInstallStatus::OldGen),
    }
}

/// Builds the reference `_upgradeBackup` filename for a path or basename.
pub fn upgrade_backup_name(file_name_or_path: &str) -> String {
    backup_name(file_name_or_path, UPGRADE_BACKUP_MARKER)
}

/// Builds the reference `_downgradeBackup` filename for a path or basename.
pub fn downgrade_backup_name(file_name_or_path: &str) -> String {
    backup_name(file_name_or_path, DOWNGRADE_BACKUP_MARKER)
}

/// Returns the basename using both Windows and Unix path separators.
pub fn reference_file_name(file_name_or_path: &str) -> &str {
    file_name_or_path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(file_name_or_path)
}

/// Builds the reference skip message for files already matching the target.
pub fn skipped_already_message(
    file_name_or_path: &str,
    desired: DowngraderInstallStatus,
) -> String {
    format!(
        "Skipped {}: Already {}.",
        reference_file_name(file_name_or_path),
        desired.as_reference_str()
    )
}

/// Builds the reference skip message for missing files.
pub fn skipped_not_found_message(file_name_or_path: &str) -> String {
    format!(
        "Skipped {}: Not Found.",
        reference_file_name(file_name_or_path)
    )
}

/// Builds the reference skip message for unsupported versions.
pub fn skipped_unsupported_message(file_name_or_path: &str) -> String {
    format!(
        "Skipped {}: Unsupported Version.",
        reference_file_name(file_name_or_path)
    )
}

/// Builds the reference success message for patched files.
pub fn patched_message(file_name_or_path: &str) -> String {
    format!("Patched {}", reference_file_name(file_name_or_path))
}

/// Builds the reference failure message for patch failures.
pub fn failed_patching_message(file_name_or_path: &str) -> String {
    format!("Failed patching {}", reference_file_name(file_name_or_path))
}

/// Builds a reference log row for an already-desired skip.
pub fn skipped_already_log_row(
    file_name_or_path: &str,
    desired: DowngraderInstallStatus,
) -> DowngraderExecutionLogRow {
    DowngraderExecutionLogRow::new(
        DowngraderLogLevel::Info,
        skipped_already_message(file_name_or_path, desired),
    )
}

/// Builds a reference log row for a missing-file skip.
pub fn skipped_not_found_log_row(file_name_or_path: &str) -> DowngraderExecutionLogRow {
    DowngraderExecutionLogRow::new(
        DowngraderLogLevel::Info,
        skipped_not_found_message(file_name_or_path),
    )
}

/// Builds a reference log row for an unsupported-version skip.
pub fn skipped_unsupported_log_row(file_name_or_path: &str) -> DowngraderExecutionLogRow {
    DowngraderExecutionLogRow::new(
        DowngraderLogLevel::Info,
        skipped_unsupported_message(file_name_or_path),
    )
}

/// Builds a reference success log row for a patched file.
pub fn patched_log_row(file_name_or_path: &str) -> DowngraderExecutionLogRow {
    DowngraderExecutionLogRow::new(DowngraderLogLevel::Good, patched_message(file_name_or_path))
}

/// Builds a reference failure log row for a patch failure.
pub fn failed_patching_log_row(file_name_or_path: &str) -> DowngraderExecutionLogRow {
    DowngraderExecutionLogRow::new(
        DowngraderLogLevel::Bad,
        failed_patching_message(file_name_or_path),
    )
}

fn backup_name(file_name_or_path: &str, marker: &str) -> String {
    let file_name = reference_file_name(file_name_or_path);
    match file_name.rsplit_once('.') {
        Some((stem, suffix)) if !stem.is_empty() => format!("{stem}{marker}.{suffix}"),
        _ => format!("{file_name}{marker}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downgrader_domain_reference_strings_match_python_modal() {
        assert_eq!(DOWNGRADER_MODAL_TITLE, "Downgrader");
        assert_eq!(DOWNGRADER_MODAL_WIDTH, 600);
        assert_eq!(DOWNGRADER_MODAL_HEIGHT, 334);
        assert_eq!(CURRENT_GAME_GROUP_LABEL, "Current Game");
        assert_eq!(CURRENT_CREATION_KIT_GROUP_LABEL, "Current Creation Kit");
        assert_eq!(DESIRED_VERSION_GROUP_LABEL, "Desired Version");
        assert_eq!(OPTIONS_GROUP_LABEL, "Options");
        assert_eq!(TARGET_OLD_GEN_LABEL, "Old-Gen");
        assert_eq!(TARGET_NEXT_GEN_LABEL, "Next-Gen");
        assert_eq!(KEEP_BACKUPS_CHECKBOX_LABEL, "Keep Backups");
        assert_eq!(DELETE_PATCHES_CHECKBOX_LABEL, "Delete Patches");
        assert_eq!(PATCH_ALL_BUTTON_LABEL, "Patch\n All");
        assert_eq!(ABOUT_BUTTON_LABEL, "About");
        assert_eq!(
            INITIAL_LOG_LINE,
            "Patches will be downloaded and applied as-needed."
        );
        assert_eq!(
            ABOUT_DOWNGRADING_TITLE,
            "About Downgrading Fallout 4 & Creation Kit"
        );
        assert!(ABOUT_DOWNGRADING_BODY.contains("Patches range in size from 23KB to 63MB."));
        assert!(ABOUT_DOWNGRADING_BODY.contains("Simple Downgrader's backups will also be used."));
        assert!(ABOUT_DOWNGRADING_BODY.contains(
            "Both Creation Kit and the game require steam_api64.dll to match their version"
        ));
        assert!(
            TOOLTIP_DOWNGRADER_BACKUPS.contains("Uncheck this to delete backups after patching.")
        );
        assert!(TOOLTIP_DOWNGRADER_DELTAS.contains("Check this to delete xdeltas after patching."));
    }

    #[test]
    fn downgrader_domain_file_definitions_preserve_reference_order_and_groups() {
        let paths: Vec<&str> = DOWNGRADER_FILE_DEFINITIONS
            .iter()
            .map(|definition| definition.relative_path)
            .collect();
        assert_eq!(
            paths,
            vec![
                "Fallout4.exe",
                "Fallout4Launcher.exe",
                "steam_api64.dll",
                "CreationKit.exe",
                "Tools\\Archive2\\Archive2.exe",
                "Tools\\Archive2\\Archive2Interop.dll",
            ]
        );

        assert_eq!(game_file_definitions().len(), 3);
        assert!(
            game_file_definitions()
                .iter()
                .all(|definition| definition.group == DowngraderFileGroup::Game)
        );
        assert_eq!(creation_kit_file_definitions().len(), 3);
        assert!(
            creation_kit_file_definitions()
                .iter()
                .all(|definition| definition.group == DowngraderFileGroup::CreationKit)
        );
        assert_eq!(DOWNGRADER_FILE_DEFINITIONS[4].display_name, "Archive2.exe");
        assert_eq!(
            DOWNGRADER_FILE_DEFINITIONS[5].display_name,
            "Archive2Interop.dll"
        );
    }

    #[test]
    fn downgrader_domain_crc_mappings_match_reference() {
        let fallout4 = find_file_definition("Fallout4.exe").expect("Fallout4.exe definition");
        assert_eq!(fallout4.crc_mappings.len(), 12);
        assert_eq!(
            fallout4.status_for_crc("97DA3E03"),
            Some(DowngraderInstallStatus::Obsolete)
        );
        assert_eq!(
            fallout4.status_for_crc("C6053902"),
            Some(DowngraderInstallStatus::OldGen)
        );
        assert_eq!(
            fallout4.status_for_crc("c5965a2e"),
            Some(DowngraderInstallStatus::NextGen)
        );
        assert_eq!(
            fallout4.status_for_crc("CF47788D"),
            Some(DowngraderInstallStatus::Anniversary)
        );
        assert_eq!(fallout4.status_for_crc("DEADBEEF"), None);

        let launcher = find_file_definition("Fallout4Launcher.exe").expect("launcher definition");
        assert_eq!(launcher.crc_mappings.len(), 7);
        assert_eq!(
            launcher.status_for_crc("02445570"),
            Some(DowngraderInstallStatus::OldGen)
        );
        assert_eq!(
            launcher.status_for_crc("F6A06FF5"),
            Some(DowngraderInstallStatus::NextGen)
        );
        assert_eq!(
            launcher.status_for_crc("720BB9C3"),
            Some(DowngraderInstallStatus::Anniversary)
        );

        let steam_api = find_file_definition("steam_api64.dll").expect("steam API definition");
        assert_eq!(steam_api.crc_mappings.len(), 2);
        assert_eq!(
            steam_api.status_for_crc("BBD912FC"),
            Some(DowngraderInstallStatus::OldGen)
        );
        assert_eq!(
            steam_api.status_for_crc("E36E7B4D"),
            Some(DowngraderInstallStatus::NextGenAnniversary)
        );

        let creation_kit = find_file_definition("CreationKit.exe").expect("CreationKit definition");
        assert_eq!(
            creation_kit.status_for_crc("0F5C065B"),
            Some(DowngraderInstallStatus::OldGen)
        );
        assert_eq!(
            creation_kit.status_for_crc("481CCE95"),
            Some(DowngraderInstallStatus::NextGen)
        );
        assert_eq!(
            creation_kit.status_for_crc("49E45284"),
            Some(DowngraderInstallStatus::Anniversary)
        );

        let archive2 = find_file_definition("Archive2.exe").expect("Archive2 definition");
        assert_eq!(
            archive2.status_for_crc("4CDFC7B5"),
            Some(DowngraderInstallStatus::OldGen)
        );
        assert_eq!(
            archive2.status_for_crc("71A5240B"),
            Some(DowngraderInstallStatus::NextGen)
        );
        assert_eq!(
            archive2.status_for_crc("C867674F"),
            Some(DowngraderInstallStatus::Anniversary)
        );

        let interop = find_file_definition("Tools\\Archive2\\Archive2Interop.dll")
            .expect("Archive2Interop definition");
        assert_eq!(
            interop.status_for_crc("850D36A9"),
            Some(DowngraderInstallStatus::OldGen)
        );
        assert_eq!(
            interop.status_for_crc("EFBE3622"),
            Some(DowngraderInstallStatus::NextGen)
        );
        assert_eq!(
            interop.status_for_crc("7B893B0D"),
            Some(DowngraderInstallStatus::Anniversary)
        );
    }

    #[test]
    fn downgrader_domain_reference_crc_buckets_include_ngae_for_ng_and_ae() {
        let next_gen_crcs = crcs_for_status(DowngraderInstallStatus::NextGen);
        assert!(next_gen_crcs.contains(&"C5965A2E"));
        assert!(next_gen_crcs.contains(&"E36E7B4D"));
        assert!(next_gen_crcs.contains(&"EFBE3622"));

        let anniversary_crcs = crcs_for_status(DowngraderInstallStatus::Anniversary);
        assert!(anniversary_crcs.contains(&"CF47788D"));
        assert!(anniversary_crcs.contains(&"E36E7B4D"));
        assert!(anniversary_crcs.contains(&"7B893B0D"));

        let old_gen_sources = accepted_source_crcs_for_target(DowngraderTarget::NextGen);
        assert!(old_gen_sources.contains(&"C6053902"));
        assert!(old_gen_sources.contains(&"BBD912FC"));

        let next_gen_sources = accepted_source_crcs_for_target(DowngraderTarget::OldGen);
        assert!(next_gen_sources.contains(&"C5965A2E"));
        assert!(next_gen_sources.contains(&"E36E7B4D"));
    }

    #[test]
    fn downgrader_domain_status_and_target_labels_match_reference() {
        assert_eq!(
            DowngraderInstallStatus::Obsolete.as_reference_str(),
            "Obsolete"
        );
        assert_eq!(
            DowngraderInstallStatus::OldGen.as_reference_str(),
            "Old-Gen"
        );
        assert_eq!(
            DowngraderInstallStatus::NextGen.as_reference_str(),
            "Next-Gen"
        );
        assert_eq!(
            DowngraderInstallStatus::Anniversary.as_reference_str(),
            "Anniversary"
        );
        assert_eq!(
            DowngraderInstallStatus::NextGenAnniversary.as_reference_str(),
            "Next-Gen & Anniversary"
        );
        assert_eq!(
            DowngraderInstallStatus::Unknown.as_reference_str(),
            "Unknown"
        );
        assert_eq!(
            DowngraderInstallStatus::NotFound.as_reference_str(),
            "Not Found"
        );
        assert_eq!(DowngraderTarget::OldGen.as_reference_str(), "Old-Gen");
        assert_eq!(DowngraderTarget::NextGen.as_reference_str(), "Next-Gen");
        assert_eq!(DowngraderFileGroup::Game.as_reference_str(), "Current Game");
        assert_eq!(
            DowngraderFileGroup::CreationKit.as_reference_str(),
            "Current Creation Kit"
        );
        assert_eq!(DowngraderLogLevel::Info.as_reference_str(), "info");
        assert_eq!(DowngraderLogLevel::Good.as_reference_str(), "good");
        assert_eq!(DowngraderLogLevel::Bad.as_reference_str(), "bad");
    }

    #[test]
    fn downgrader_domain_backup_and_patch_helpers_match_reference() {
        assert_eq!(
            upgrade_backup_name("Fallout4.exe"),
            "Fallout4_upgradeBackup.exe"
        );
        assert_eq!(
            downgrade_backup_name("Fallout4.exe"),
            "Fallout4_downgradeBackup.exe"
        );
        assert_eq!(
            upgrade_backup_name("Tools\\Archive2\\Archive2Interop.dll"),
            "Archive2Interop_upgradeBackup.dll"
        );
        assert_eq!(
            downgrade_backup_name("Tools/Archive2/Archive2Interop.dll"),
            "Archive2Interop_downgradeBackup.dll"
        );

        assert_eq!(
            DowngraderTarget::OldGen.desired_backup_name_for("Fallout4.exe"),
            "Fallout4_upgradeBackup.exe"
        );
        assert_eq!(
            DowngraderTarget::OldGen.current_backup_name_for("Fallout4.exe"),
            "Fallout4_downgradeBackup.exe"
        );
        assert_eq!(
            DowngraderTarget::NextGen.desired_backup_name_for("Fallout4.exe"),
            "Fallout4_downgradeBackup.exe"
        );
        assert_eq!(
            DowngraderTarget::NextGen.current_backup_name_for("Fallout4.exe"),
            "Fallout4_upgradeBackup.exe"
        );

        assert_eq!(
            DowngraderTarget::OldGen.patch_name_for("Fallout4.exe"),
            "NG-to-OG-Fallout4.exe.xdelta"
        );
        assert_eq!(
            DowngraderTarget::NextGen.patch_name_for("Tools\\Archive2\\Archive2.exe"),
            "OG-to-NG-Archive2.exe.xdelta"
        );
        assert_eq!(
            DowngraderTarget::OldGen.patch_url_for("steam_api64.dll"),
            "https://github.com/wxMichael/Collective-Modding-Toolkit/releases/download/delta-patches/NG-to-OG-steam_api64.dll.xdelta"
        );
    }

    #[test]
    fn downgrader_domain_plan_and_log_rows_use_reference_vocabulary() {
        let options = DowngraderOptionsSnapshot::new(DowngraderTarget::OldGen, true, true);
        let fallout4 = find_file_definition("Fallout4.exe").expect("Fallout4 definition");
        let row =
            DowngraderPlanRow::from_definition(fallout4, DowngraderInstallStatus::NextGen, options);
        assert_eq!(row.action, DowngraderPlanAction::ValidateBackupOrPatch);
        assert!(row.action.requires_worker());
        assert_eq!(row.desired_backup_name, "Fallout4_upgradeBackup.exe");
        assert_eq!(row.current_backup_name, "Fallout4_downgradeBackup.exe");
        assert_eq!(row.patch_name, "NG-to-OG-Fallout4.exe.xdelta");
        assert!(row.skip_log_row().is_none());

        let step = DowngraderPlanStep::new(
            DowngraderPlanStepKind::CreateCurrentBackup,
            "Create backup Fallout4_downgradeBackup.exe from Fallout4.exe.",
        );
        assert_eq!(step.kind.as_str(), "create_current_backup");
        assert!(step.kind.is_mutating_execution_step());
        assert_eq!(
            step.message,
            "Create backup Fallout4_downgradeBackup.exe from Fallout4.exe."
        );
        assert!(!DowngraderPlanStepKind::SkipNotFound.is_mutating_execution_step());

        let already =
            DowngraderPlanRow::from_definition(fallout4, DowngraderInstallStatus::OldGen, options);
        assert_eq!(already.action, DowngraderPlanAction::SkipAlreadyDesired);
        assert_eq!(
            already.skip_log_row(),
            Some(DowngraderExecutionLogRow::new(
                DowngraderLogLevel::Info,
                "Skipped Fallout4.exe: Already Old-Gen."
            ))
        );

        assert_eq!(
            DowngraderPlanAction::from_status(
                DowngraderInstallStatus::NotFound,
                DowngraderTarget::OldGen
            ),
            DowngraderPlanAction::SkipNotFound
        );
        assert_eq!(
            DowngraderPlanAction::from_status(
                DowngraderInstallStatus::Anniversary,
                DowngraderTarget::OldGen
            ),
            DowngraderPlanAction::SkipUnsupportedVersion
        );
        assert_eq!(
            DowngraderPlanAction::from_status(
                DowngraderInstallStatus::Unknown,
                DowngraderTarget::OldGen
            ),
            DowngraderPlanAction::ValidateBackupOrPatch
        );

        assert_eq!(
            DowngraderExecutionLogRow::initial(),
            DowngraderExecutionLogRow::new(DowngraderLogLevel::Info, INITIAL_LOG_LINE)
        );
        assert_eq!(
            skipped_not_found_log_row("Tools\\Archive2\\Archive2.exe"),
            DowngraderExecutionLogRow::new(
                DowngraderLogLevel::Info,
                "Skipped Archive2.exe: Not Found."
            )
        );
        assert_eq!(
            skipped_unsupported_log_row("steam_api64.dll"),
            DowngraderExecutionLogRow::new(
                DowngraderLogLevel::Info,
                "Skipped steam_api64.dll: Unsupported Version."
            )
        );
        assert_eq!(
            patched_log_row("Fallout4Launcher.exe"),
            DowngraderExecutionLogRow::new(
                DowngraderLogLevel::Good,
                "Patched Fallout4Launcher.exe"
            )
        );
        assert_eq!(
            failed_patching_log_row("CreationKit.exe"),
            DowngraderExecutionLogRow::new(
                DowngraderLogLevel::Bad,
                "Failed patching CreationKit.exe"
            )
        );
    }

    #[test]
    fn downgrader_domain_progress_values_are_clamped_to_reference_range() {
        assert_eq!(DowngraderProgress::idle().percent, 0.0);
        assert_eq!(DowngraderProgress::complete().percent, 100.0);
        assert_eq!(DowngraderProgress::new(-10.0).percent, 0.0);
        assert_eq!(DowngraderProgress::new(42.5).percent, 42.5);
        assert_eq!(DowngraderProgress::new(150.0).percent, 100.0);
    }
}
