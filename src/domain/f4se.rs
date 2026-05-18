//! Slint-free domain contract for the F4SE diagnostics tab.
//!
//! The reference Tkinter tab lives in `CMT/src/tabs/_f4se.py`. This module keeps
//! the same user-facing labels, icon strings, and row compatibility rules in
//! pure Rust so scanner workers and Slint UI code can depend on a stable typed
//! contract without re-reading Python source or touching filesystem/process/UI
//! APIs.

use crate::domain::discovery::{
    DATA_FOLDER_NOT_FOUND_MESSAGE, F4SE_PLUGINS_NOT_FOUND_MESSAGE, Fallout4InstallType,
};

/// Reference tab title used by the original notebook tab.
pub const F4SE_TAB_TITLE: &str = "F4SE";
/// Reference loading text shown while DLL scanning runs.
pub const F4SE_LOADING_TEXT: &str = "Scanning DLLs...";
/// Reference table columns in display order.
pub const F4SE_TABLE_COLUMNS: [&str; 5] = ["DLL", "OG", "NG", "AE", "Your Game"];
/// Reference heading shown next to the DLL table.
pub const F4SE_HEADING: &str = "F4SE DLLs";
/// Reference loading error used when the Fallout 4 `Data` folder is unavailable.
pub const F4SE_DATA_FOLDER_NOT_FOUND_MESSAGE: &str = DATA_FOLDER_NOT_FOUND_MESSAGE;
/// Reference loading error prefix used when `Data/F4SE/Plugins` is unavailable.
pub const F4SE_PLUGINS_FOLDER_NOT_FOUND_MESSAGE: &str = F4SE_PLUGINS_NOT_FOUND_MESSAGE;
/// Reference hint appended when the app was not launched through a mod manager.
pub const F4SE_MOD_MANAGER_HINT: &str = "Try launching via your mod manager.";
/// Reference information/legend text from `ABOUT_F4SE_DLLS`.
pub const F4SE_LEGEND_TEXT: &str = "This checks all DLLs in\nData/F4SE/Plugins/ for\nversion-specific code to\ndetermine OG/NG support.\n\n✔ Version is supported\n\n❌ Version not supported\n\n❓ Not an F4SE DLL.\nMay still be loaded by\nother DLLs.\n\n⚠ Consult mod page to\nverify version support if\nyou see this icon.\nSome DLLs' version support\ncannot be reliably\ndetermined.";

const DETAIL_NOT_F4SE: &str = "Not an F4SE DLL. May still be loaded by other DLLs.";
const DETAIL_AMBIGUOUS_NGAE: &str = "Some DLLs' version support cannot be reliably determined. Consult the mod page to verify version support.";
const DETAIL_UNKNOWN_GAME: &str = "Your current Fallout 4 version could not be classified; compatibility with Your Game is unknown.";
const DETAIL_INCOMPATIBLE: &str = "Version not supported for the current game.";
const DETAIL_COMPATIBLE: &str = "Version is supported for the current game.";

/// Returns the reference `Data/F4SE/Plugins` missing-folder message.
///
/// The Python tab appends the mod-manager hint only when no mod manager was
/// detected, because launching through a manager is how the virtual plugin folder
/// normally becomes visible.
pub fn f4se_missing_plugins_message(mod_manager_detected: bool) -> String {
    if mod_manager_detected {
        F4SE_PLUGINS_FOLDER_NOT_FOUND_MESSAGE.to_owned()
    } else {
        format!("{F4SE_PLUGINS_FOLDER_NOT_FOUND_MESSAGE}\n{F4SE_MOD_MANAGER_HINT}")
    }
}

/// Current game generation used to classify the `Your Game` F4SE column.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F4seGameTarget {
    /// Fallout 4 1.10.163-era runtime, including the reference `Down-Grade` state.
    OldGen,
    /// Fallout 4 Next-Gen runtime.
    NextGen,
    /// Fallout 4 Anniversary runtime.
    Anniversary,
    /// The current runtime could not be mapped to OG, NG, or AE.
    #[default]
    Unknown,
}

impl F4seGameTarget {
    /// Maps the broader discovery install type into the F4SE target generations.
    ///
    /// `NextGenAnniversary` is intentionally mapped to [`F4seGameTarget::Unknown`]
    /// because the F4SE table needs one concrete current runtime for the `Your
    /// Game` column; treating a combined binary classification as either NG or AE
    /// would hide uncertainty.
    pub const fn from_install_type(install_type: Fallout4InstallType) -> Self {
        match install_type {
            Fallout4InstallType::OldGen | Fallout4InstallType::DownGrade => Self::OldGen,
            Fallout4InstallType::NextGen => Self::NextGen,
            Fallout4InstallType::Anniversary => Self::Anniversary,
            Fallout4InstallType::Obsolete
            | Fallout4InstallType::NextGenAnniversary
            | Fallout4InstallType::Unknown
            | Fallout4InstallType::NotFound => Self::Unknown,
        }
    }
}

/// Compatibility icon semantics for a rendered F4SE table cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F4seCompatibilityIcon {
    /// Empty cell used by the reference OG/NG/AE columns for unsupported versions.
    UnsupportedReferenceColumn,
    /// Explicit blank cell used for non-F4SE rows in the `Your Game` column.
    Blank,
    /// Reference black question mark ornament for non-F4SE or unknown DLL facts.
    Unknown,
    /// Reference heavy check mark for supported versions.
    Supported,
    /// Reference cross mark for confirmed current-game incompatibility.
    UnsupportedCurrentGame,
    /// Reference warning sign for ambiguous support or unknown current-game state.
    Warning,
}

impl F4seCompatibilityIcon {
    /// Returns the exact icon string used by the reference Tkinter tab.
    pub const fn as_reference_str(self) -> &'static str {
        match self {
            Self::UnsupportedReferenceColumn | Self::Blank => "",
            Self::Unknown => "❓",
            Self::Supported => "✔",
            Self::UnsupportedCurrentGame => "❌",
            Self::Warning => "⚠",
        }
    }
}

/// Semantic state behind a rendered F4SE compatibility cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F4seCompatibilityState {
    /// The relevant version is supported.
    Supported,
    /// The relevant reference column is known unsupported but intentionally blank.
    UnsupportedReferenceColumn,
    /// The current game is known unsupported and should show a cross mark.
    UnsupportedCurrentGame,
    /// The DLL is not known to be F4SE or could not be inspected.
    Unknown,
    /// The support state is ambiguous or the current game target is unknown.
    Warning,
    /// Intentionally empty non-applicable cell.
    Blank,
}

/// Rendered F4SE compatibility cell independent of Slint table models.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct F4seCompatibilityCell {
    /// Reference icon category for this cell.
    pub icon: F4seCompatibilityIcon,
    /// Semantic state represented by the icon.
    pub state: F4seCompatibilityState,
    /// Safe user-facing detail suitable for row-detail panes or tooltips.
    pub detail: &'static str,
}

impl F4seCompatibilityCell {
    fn supported(detail: &'static str) -> Self {
        Self {
            icon: F4seCompatibilityIcon::Supported,
            state: F4seCompatibilityState::Supported,
            detail,
        }
    }

    fn unsupported_reference(detail: &'static str) -> Self {
        Self {
            icon: F4seCompatibilityIcon::UnsupportedReferenceColumn,
            state: F4seCompatibilityState::UnsupportedReferenceColumn,
            detail,
        }
    }

    fn unsupported_current_game() -> Self {
        Self {
            icon: F4seCompatibilityIcon::UnsupportedCurrentGame,
            state: F4seCompatibilityState::UnsupportedCurrentGame,
            detail: DETAIL_INCOMPATIBLE,
        }
    }

    fn unknown(detail: &'static str) -> Self {
        Self {
            icon: F4seCompatibilityIcon::Unknown,
            state: F4seCompatibilityState::Unknown,
            detail,
        }
    }

    fn warning(detail: &'static str) -> Self {
        Self {
            icon: F4seCompatibilityIcon::Warning,
            state: F4seCompatibilityState::Warning,
            detail,
        }
    }

    fn blank(detail: &'static str) -> Self {
        Self {
            icon: F4seCompatibilityIcon::Blank,
            state: F4seCompatibilityState::Blank,
            detail,
        }
    }
}

/// Reference row tag names used for F4SE table coloring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F4seRowTag {
    /// Reference `neutral` tag for non-F4SE rows.
    Neutral,
    /// Reference `good` tag for current-game compatible F4SE rows.
    Good,
    /// Reference `bad` tag for confirmed current-game incompatible F4SE rows.
    Bad,
    /// Reference `note` tag for ambiguous or warning rows.
    Note,
}

impl F4seRowTag {
    /// Returns the exact Tkinter tag name used by the reference tab.
    pub const fn as_reference_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Good => "good",
            Self::Bad => "bad",
            Self::Note => "note",
        }
    }
}

/// Severity bucket for UI filtering, detail panels, and future tracing summaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F4seRowSeverity {
    /// Non-F4SE or otherwise informational row.
    Neutral,
    /// Current game is compatible.
    Compatible,
    /// Current game is confirmed incompatible.
    Incompatible,
    /// Compatibility could not be determined confidently.
    Warning,
}

/// Raw DLL facts produced by a future F4SE scanner before UI rendering.
///
/// These fields mirror the reference `DLLInfo` keys while allowing inspection
/// failures to stay visible. `exports_version` corresponds to the Python
/// `SupportsNGAE` flag (`F4SEPlugin_Version` exists). `supports_ng` and
/// `supports_ae` are `None` when the version export exists but compatible
/// versions could not be recognized reliably.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct F4seDllFacts {
    /// Basename displayed in the `DLL` column.
    pub dll_name: String,
    /// Whether the scanner confirmed F4SE exports; `None` means inspection failed.
    pub is_f4se: Option<bool>,
    /// Whether `F4SEPlugin_Query` was found, matching OG support in the reference.
    pub exports_query: bool,
    /// Whether `F4SEPlugin_Version` was found, matching NGAE support in the reference.
    pub exports_version: bool,
    /// NG support inferred from `compatibleVersions`, or ambiguous if `None`.
    pub supports_ng: Option<bool>,
    /// AE support inferred from `compatibleVersions`, or ambiguous if `None`.
    pub supports_ae: Option<bool>,
    /// Safe inspection failure message; raw parse details should stay in logs.
    pub inspection_error: Option<String>,
}

impl F4seDllFacts {
    /// Creates facts for a confirmed non-F4SE DLL.
    pub fn non_f4se(dll_name: impl Into<String>) -> Self {
        Self {
            dll_name: dll_name.into(),
            is_f4se: Some(false),
            exports_query: false,
            exports_version: false,
            supports_ng: None,
            supports_ae: None,
            inspection_error: None,
        }
    }

    /// Creates facts for a confirmed F4SE DLL.
    ///
    /// `supports_ng` and `supports_ae` should be populated only from parsed
    /// `F4SEPlugin_Version.compatibleVersions` facts. Passing `None` while
    /// `exports_version` is true preserves the reference warning-icon behavior.
    pub fn f4se(
        dll_name: impl Into<String>,
        exports_query: bool,
        exports_version: bool,
        supports_ng: Option<bool>,
        supports_ae: Option<bool>,
    ) -> Self {
        Self {
            dll_name: dll_name.into(),
            is_f4se: Some(true),
            exports_query,
            exports_version,
            supports_ng,
            supports_ae,
            inspection_error: None,
        }
    }

    /// Creates facts for a DLL that could not be inspected safely.
    ///
    /// The message is intended for user-visible diagnostics and should not expose
    /// raw unread bytes or platform-specific exception text.
    pub fn inspection_failed(dll_name: impl Into<String>, safe_message: impl Into<String>) -> Self {
        Self {
            dll_name: dll_name.into(),
            is_f4se: None,
            exports_query: false,
            exports_version: false,
            supports_ng: None,
            supports_ae: None,
            inspection_error: Some(safe_message.into()),
        }
    }
}

/// Rendered F4SE row ready for conversion into UI table models.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct F4seDllRow {
    /// Basename displayed in the `DLL` column.
    pub dll_name: String,
    /// Rendered OG support cell.
    pub og: F4seCompatibilityCell,
    /// Rendered NG support cell.
    pub ng: F4seCompatibilityCell,
    /// Rendered AE support cell.
    pub ae: F4seCompatibilityCell,
    /// Rendered current-game support cell.
    pub your_game: F4seCompatibilityCell,
    /// Reference row tag for color mapping.
    pub tag: F4seRowTag,
    /// Higher-level severity derived from the current-game cell.
    pub severity: F4seRowSeverity,
    /// Safe row detail messages for diagnostics panes or accessibility labels.
    pub details: Vec<String>,
}

/// Overall F4SE scan state used by controllers and UI status text.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F4seScanStatus {
    /// No F4SE scan has started yet.
    #[default]
    Idle,
    /// A worker is currently scanning DLLs.
    Loading,
    /// Scan completed and row data is available.
    Ready,
    /// Scan could not start or complete enough to show rows.
    Error,
}

/// Snapshot of the F4SE tab state independent of Slint models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct F4seScanSnapshot {
    /// Current scan status.
    pub status: F4seScanStatus,
    /// User-visible status or loading/error message.
    pub status_message: String,
    /// Rendered DLL rows, preserving the scanner-provided order.
    pub rows: Vec<F4seDllRow>,
}

impl F4seScanSnapshot {
    /// Creates an idle empty snapshot.
    pub fn idle() -> Self {
        Self {
            status: F4seScanStatus::Idle,
            status_message: String::new(),
            rows: Vec::new(),
        }
    }

    /// Creates the reference loading snapshot.
    pub fn loading() -> Self {
        Self {
            status: F4seScanStatus::Loading,
            status_message: F4SE_LOADING_TEXT.to_owned(),
            rows: Vec::new(),
        }
    }

    /// Creates a ready snapshot with rendered rows.
    pub fn ready(rows: Vec<F4seDllRow>) -> Self {
        Self {
            status: F4seScanStatus::Ready,
            status_message: String::new(),
            rows,
        }
    }

    /// Creates the reference Data-folder missing snapshot.
    pub fn missing_data_folder() -> Self {
        Self::error(F4SE_DATA_FOLDER_NOT_FOUND_MESSAGE)
    }

    /// Creates the reference F4SE plugins-folder missing snapshot.
    pub fn missing_plugins_folder(mod_manager_detected: bool) -> Self {
        Self::error(f4se_missing_plugins_message(mod_manager_detected))
    }

    /// Creates an error snapshot from a safe user-facing message.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: F4seScanStatus::Error,
            status_message: message.into(),
            rows: Vec::new(),
        }
    }
}

/// Renders scanner facts into F4SE table rows for a specific current-game target.
pub fn render_f4se_dll_rows(
    facts: &[F4seDllFacts],
    current_game: F4seGameTarget,
) -> Vec<F4seDllRow> {
    facts
        .iter()
        .map(|fact| render_f4se_dll_row(fact, current_game))
        .collect()
}

/// Renders a single DLL fact record using the reference F4SE table rules.
pub fn render_f4se_dll_row(facts: &F4seDllFacts, current_game: F4seGameTarget) -> F4seDllRow {
    if facts.inspection_error.is_some() || facts.is_f4se.is_none() {
        return render_inspection_failed_row(facts);
    }

    if facts.is_f4se == Some(false) {
        return render_non_f4se_row(facts);
    }

    let og = if facts.exports_query {
        F4seCompatibilityCell::supported("OG support comes from F4SEPlugin_Query.")
    } else {
        F4seCompatibilityCell::unsupported_reference("F4SEPlugin_Query was not found.")
    };
    let ng = render_ngae_reference_cell(facts.exports_version, facts.supports_ng, "NG");
    let ae = render_ngae_reference_cell(facts.exports_version, facts.supports_ae, "AE");
    let your_game = render_current_game_cell(facts, current_game);
    let (tag, severity) = row_style_from_current_cell(&your_game);

    let mut details = Vec::new();
    if your_game.icon == F4seCompatibilityIcon::Warning
        || ng.icon == F4seCompatibilityIcon::Warning
        || ae.icon == F4seCompatibilityIcon::Warning
    {
        details.push(DETAIL_AMBIGUOUS_NGAE.to_owned());
    }
    if current_game == F4seGameTarget::Unknown {
        details.push(DETAIL_UNKNOWN_GAME.to_owned());
    }
    match your_game.state {
        F4seCompatibilityState::Supported => details.push(DETAIL_COMPATIBLE.to_owned()),
        F4seCompatibilityState::UnsupportedCurrentGame => {
            details.push(DETAIL_INCOMPATIBLE.to_owned())
        }
        F4seCompatibilityState::UnsupportedReferenceColumn
        | F4seCompatibilityState::Unknown
        | F4seCompatibilityState::Warning
        | F4seCompatibilityState::Blank => {}
    }

    F4seDllRow {
        dll_name: facts.dll_name.clone(),
        og,
        ng,
        ae,
        your_game,
        tag,
        severity,
        details,
    }
}

fn render_non_f4se_row(facts: &F4seDllFacts) -> F4seDllRow {
    F4seDllRow {
        dll_name: facts.dll_name.clone(),
        og: F4seCompatibilityCell::unknown(DETAIL_NOT_F4SE),
        ng: F4seCompatibilityCell::unknown(DETAIL_NOT_F4SE),
        ae: F4seCompatibilityCell::unknown(DETAIL_NOT_F4SE),
        your_game: F4seCompatibilityCell::blank(DETAIL_NOT_F4SE),
        tag: F4seRowTag::Neutral,
        severity: F4seRowSeverity::Neutral,
        details: vec![DETAIL_NOT_F4SE.to_owned()],
    }
}

fn render_inspection_failed_row(facts: &F4seDllFacts) -> F4seDllRow {
    let detail = facts
        .inspection_error
        .clone()
        .unwrap_or_else(|| "Could not inspect DLL. Compatibility is unknown.".to_owned());
    F4seDllRow {
        dll_name: facts.dll_name.clone(),
        og: F4seCompatibilityCell::unknown("DLL inspection failed."),
        ng: F4seCompatibilityCell::unknown("DLL inspection failed."),
        ae: F4seCompatibilityCell::unknown("DLL inspection failed."),
        your_game: F4seCompatibilityCell::warning("DLL inspection failed."),
        tag: F4seRowTag::Note,
        severity: F4seRowSeverity::Warning,
        details: vec![detail],
    }
}

fn render_ngae_reference_cell(
    exports_version: bool,
    supports_runtime: Option<bool>,
    runtime_name: &'static str,
) -> F4seCompatibilityCell {
    if !exports_version {
        return F4seCompatibilityCell::unsupported_reference("F4SEPlugin_Version was not found.");
    }

    match supports_runtime {
        Some(true) => F4seCompatibilityCell::supported(match runtime_name {
            "NG" => "NG support was found in compatibleVersions.",
            "AE" => "AE support was found in compatibleVersions.",
            _ => "Support was found in compatibleVersions.",
        }),
        Some(false) => F4seCompatibilityCell::unsupported_reference(match runtime_name {
            "NG" => "NG support was not found in compatibleVersions.",
            "AE" => "AE support was not found in compatibleVersions.",
            _ => "Support was not found in compatibleVersions.",
        }),
        None => F4seCompatibilityCell::warning(DETAIL_AMBIGUOUS_NGAE),
    }
}

fn render_current_game_cell(
    facts: &F4seDllFacts,
    current_game: F4seGameTarget,
) -> F4seCompatibilityCell {
    match current_game {
        F4seGameTarget::OldGen => {
            if facts.exports_query {
                F4seCompatibilityCell::supported(DETAIL_COMPATIBLE)
            } else {
                F4seCompatibilityCell::unsupported_current_game()
            }
        }
        F4seGameTarget::NextGen => render_current_ngae_cell(facts, facts.supports_ng),
        F4seGameTarget::Anniversary => render_current_ngae_cell(facts, facts.supports_ae),
        F4seGameTarget::Unknown => F4seCompatibilityCell::warning(DETAIL_UNKNOWN_GAME),
    }
}

fn render_current_ngae_cell(
    facts: &F4seDllFacts,
    supports_runtime: Option<bool>,
) -> F4seCompatibilityCell {
    if !facts.exports_version {
        return F4seCompatibilityCell::unsupported_current_game();
    }

    match supports_runtime {
        Some(true) => F4seCompatibilityCell::supported(DETAIL_COMPATIBLE),
        Some(false) => F4seCompatibilityCell::unsupported_current_game(),
        None => F4seCompatibilityCell::warning(DETAIL_AMBIGUOUS_NGAE),
    }
}

fn row_style_from_current_cell(cell: &F4seCompatibilityCell) -> (F4seRowTag, F4seRowSeverity) {
    match cell.state {
        F4seCompatibilityState::Supported => (F4seRowTag::Good, F4seRowSeverity::Compatible),
        F4seCompatibilityState::UnsupportedCurrentGame => {
            (F4seRowTag::Bad, F4seRowSeverity::Incompatible)
        }
        F4seCompatibilityState::Warning => (F4seRowTag::Note, F4seRowSeverity::Warning),
        F4seCompatibilityState::UnsupportedReferenceColumn
        | F4seCompatibilityState::Unknown
        | F4seCompatibilityState::Blank => (F4seRowTag::Neutral, F4seRowSeverity::Neutral),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::discovery::Fallout4InstallType;

    #[test]
    fn f4se_domain_reference_strings_are_locked() {
        assert_eq!(F4SE_TAB_TITLE, "F4SE");
        assert_eq!(F4SE_LOADING_TEXT, "Scanning DLLs...");
        assert_eq!(F4SE_TABLE_COLUMNS, ["DLL", "OG", "NG", "AE", "Your Game"]);
        assert_eq!(F4SE_HEADING, "F4SE DLLs");
        assert_eq!(F4SE_DATA_FOLDER_NOT_FOUND_MESSAGE, "Data folder not found");
        assert_eq!(
            F4SE_PLUGINS_FOLDER_NOT_FOUND_MESSAGE,
            "Data/F4SE/Plugins folder not found"
        );
        assert_eq!(F4SE_MOD_MANAGER_HINT, "Try launching via your mod manager.");
        assert_eq!(
            f4se_missing_plugins_message(true),
            "Data/F4SE/Plugins folder not found"
        );
        assert_eq!(
            f4se_missing_plugins_message(false),
            "Data/F4SE/Plugins folder not found\nTry launching via your mod manager."
        );
        assert_eq!(
            F4SE_LEGEND_TEXT,
            "This checks all DLLs in\nData/F4SE/Plugins/ for\nversion-specific code to\ndetermine OG/NG support.\n\n✔ Version is supported\n\n❌ Version not supported\n\n❓ Not an F4SE DLL.\nMay still be loaded by\nother DLLs.\n\n⚠ Consult mod page to\nverify version support if\nyou see this icon.\nSome DLLs' version support\ncannot be reliably\ndetermined."
        );
    }

    #[test]
    fn f4se_domain_icon_mapping_matches_reference() {
        assert_eq!(F4seCompatibilityIcon::Unknown.as_reference_str(), "❓");
        assert_eq!(F4seCompatibilityIcon::Supported.as_reference_str(), "✔");
        assert_eq!(
            F4seCompatibilityIcon::UnsupportedReferenceColumn.as_reference_str(),
            ""
        );
        assert_eq!(
            F4seCompatibilityIcon::UnsupportedCurrentGame.as_reference_str(),
            "❌"
        );
        assert_eq!(F4seCompatibilityIcon::Warning.as_reference_str(), "⚠");
        assert_eq!(F4seRowTag::Neutral.as_reference_str(), "neutral");
        assert_eq!(F4seRowTag::Good.as_reference_str(), "good");
        assert_eq!(F4seRowTag::Bad.as_reference_str(), "bad");
        assert_eq!(F4seRowTag::Note.as_reference_str(), "note");
    }

    #[test]
    fn f4se_domain_non_f4se_rendering_matches_reference() {
        let facts = F4seDllFacts::non_f4se("helper.dll");
        let row = render_f4se_dll_row(&facts, F4seGameTarget::NextGen);

        assert_eq!(row.dll_name, "helper.dll");
        assert_eq!(row.og.icon, F4seCompatibilityIcon::Unknown);
        assert_eq!(row.ng.icon, F4seCompatibilityIcon::Unknown);
        assert_eq!(row.ae.icon, F4seCompatibilityIcon::Unknown);
        assert_eq!(row.your_game.icon, F4seCompatibilityIcon::Blank);
        assert_eq!(row.tag, F4seRowTag::Neutral);
        assert_eq!(row.severity, F4seRowSeverity::Neutral);
        assert!(
            row.details
                .iter()
                .any(|detail| detail.contains("Not an F4SE DLL"))
        );
    }

    #[test]
    fn f4se_domain_f4se_without_version_export_is_confirmed_unsupported_for_ngae() {
        let facts = F4seDllFacts::f4se("legacy.dll", true, false, None, None);
        let row = render_f4se_dll_row(&facts, F4seGameTarget::NextGen);

        assert_eq!(row.og.icon, F4seCompatibilityIcon::Supported);
        assert_eq!(
            row.ng.icon,
            F4seCompatibilityIcon::UnsupportedReferenceColumn
        );
        assert_eq!(
            row.ae.icon,
            F4seCompatibilityIcon::UnsupportedReferenceColumn
        );
        assert_eq!(
            row.your_game.icon,
            F4seCompatibilityIcon::UnsupportedCurrentGame
        );
        assert_eq!(row.tag, F4seRowTag::Bad);
        assert_eq!(row.severity, F4seRowSeverity::Incompatible);
    }

    #[test]
    fn f4se_domain_ambiguous_ngae_rendering_uses_warning_icons() {
        let facts = F4seDllFacts::f4se("ambiguous.dll", true, true, None, None);
        let row = render_f4se_dll_row(&facts, F4seGameTarget::Anniversary);

        assert_eq!(row.og.icon, F4seCompatibilityIcon::Supported);
        assert_eq!(row.ng.icon, F4seCompatibilityIcon::Warning);
        assert_eq!(row.ae.icon, F4seCompatibilityIcon::Warning);
        assert_eq!(row.your_game.icon, F4seCompatibilityIcon::Warning);
        assert_eq!(row.tag, F4seRowTag::Note);
        assert_eq!(row.severity, F4seRowSeverity::Warning);
        assert!(
            row.details
                .iter()
                .any(|detail| detail.contains("cannot be reliably determined"))
        );
    }

    #[test]
    fn f4se_domain_current_game_mapping_is_target_specific() {
        let facts = F4seDllFacts::f4se("modern.dll", true, true, Some(true), Some(false));

        assert_eq!(
            render_f4se_dll_row(&facts, F4seGameTarget::OldGen)
                .your_game
                .icon,
            F4seCompatibilityIcon::Supported
        );
        assert_eq!(
            render_f4se_dll_row(&facts, F4seGameTarget::NextGen)
                .your_game
                .icon,
            F4seCompatibilityIcon::Supported
        );
        assert_eq!(
            render_f4se_dll_row(&facts, F4seGameTarget::Anniversary)
                .your_game
                .icon,
            F4seCompatibilityIcon::UnsupportedCurrentGame
        );
        assert_eq!(
            F4seGameTarget::from_install_type(Fallout4InstallType::DownGrade),
            F4seGameTarget::OldGen
        );
        assert_eq!(
            F4seGameTarget::from_install_type(Fallout4InstallType::NextGen),
            F4seGameTarget::NextGen
        );
        assert_eq!(
            F4seGameTarget::from_install_type(Fallout4InstallType::Anniversary),
            F4seGameTarget::Anniversary
        );
        assert_eq!(
            F4seGameTarget::from_install_type(Fallout4InstallType::NextGenAnniversary),
            F4seGameTarget::Unknown
        );
    }

    #[test]
    fn f4se_domain_unknown_game_target_warns_without_hiding_facts() {
        let facts = F4seDllFacts::f4se("known.dll", false, true, Some(true), Some(true));
        let row = render_f4se_dll_row(&facts, F4seGameTarget::Unknown);

        assert_eq!(
            row.og.icon,
            F4seCompatibilityIcon::UnsupportedReferenceColumn
        );
        assert_eq!(row.ng.icon, F4seCompatibilityIcon::Supported);
        assert_eq!(row.ae.icon, F4seCompatibilityIcon::Supported);
        assert_eq!(row.your_game.icon, F4seCompatibilityIcon::Warning);
        assert_eq!(row.tag, F4seRowTag::Note);
        assert_ne!(row.severity, F4seRowSeverity::Incompatible);
        assert!(
            row.details
                .iter()
                .any(|detail| detail.contains("could not be classified"))
        );
    }

    #[test]
    fn f4se_domain_scan_snapshot_models_loading_and_safe_errors() {
        assert_eq!(F4seScanSnapshot::loading().status, F4seScanStatus::Loading);
        assert_eq!(
            F4seScanSnapshot::loading().status_message,
            F4SE_LOADING_TEXT
        );

        let missing_data = F4seScanSnapshot::missing_data_folder();
        assert_eq!(missing_data.status, F4seScanStatus::Error);
        assert_eq!(
            missing_data.status_message,
            F4SE_DATA_FOLDER_NOT_FOUND_MESSAGE
        );

        let missing_plugins = F4seScanSnapshot::missing_plugins_folder(false);
        assert_eq!(missing_plugins.status, F4seScanStatus::Error);
        assert_eq!(
            missing_plugins.status_message,
            "Data/F4SE/Plugins folder not found\nTry launching via your mod manager."
        );
    }

    #[test]
    fn f4se_domain_inspection_failures_render_safe_unknown_cells() {
        let facts = F4seDllFacts::inspection_failed(
            "broken.dll",
            "Could not inspect DLL. Check file permissions or whether the file is malformed.",
        );
        let row = render_f4se_dll_row(&facts, F4seGameTarget::Anniversary);

        assert_eq!(row.og.icon, F4seCompatibilityIcon::Unknown);
        assert_eq!(row.ng.icon, F4seCompatibilityIcon::Unknown);
        assert_eq!(row.ae.icon, F4seCompatibilityIcon::Unknown);
        assert_eq!(row.your_game.icon, F4seCompatibilityIcon::Warning);
        assert_eq!(row.tag, F4seRowTag::Note);
        assert!(
            row.details
                .iter()
                .any(|detail| detail.contains("Could not inspect DLL"))
        );
    }
}
