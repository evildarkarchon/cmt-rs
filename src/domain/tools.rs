//! Pure Tools/About-tab reference contracts.
//!
//! The Python reference builds these tabs directly from Tk callbacks and image
//! paths. This module freezes the user-facing labels, URLs, copy feedback, and
//! deferred utility metadata as inert Rust data so Slint/UI code can bind to a
//! single source of truth without depending on `CMT/` at runtime.

use std::path::Path;

/// Reference application title from `CMT/src/globals.py`.
pub const APP_TITLE: &str = "Collective Modding Toolkit";
/// Reference application version from `CMT/src/globals.py`.
pub const APP_VERSION: &str = "0.6.1";
/// About-tab title text after the reference `rsplit(maxsplit=1)` line break.
pub const ABOUT_TITLE_LABEL: &str = "Collective Modding\nToolkit";
/// About-tab credit text from `CMT/src/tabs/_about.py`.
pub const ABOUT_CREDIT_LABEL: &str =
    "v0.6.1\n\nCreated by wxMichael for the\nCollective Modding Community\n#cm-toolkit on Discord";

/// Reference Nexus Mods project URL.
pub const NEXUS_LINK: &str = "https://www.nexusmods.com/fallout4/mods/87907";
/// Reference Collective Modding Discord invite URL.
pub const DISCORD_INVITE: &str = "https://discord.gg/tktyEyYHZH";
/// Reference GitHub repository URL.
pub const GITHUB_LINK: &str = "https://github.com/wxMichael/Collective-Modding-Toolkit";

/// Tooltip/hint text used for Nexus-hosted links.
pub const URL_HINT_NEXUS_MODS: &str = "View on Nexus Mods";
/// Tooltip/hint text used for GitHub-hosted links.
pub const URL_HINT_GITHUB: &str = "View on GitHub";
/// Tooltip/hint text used when no specific host mapping exists.
pub const URL_HINT_OPEN_WEBSITE: &str = "Open website";

/// About-tab open button label for normal links.
pub const ABOUT_OPEN_LINK_LABEL: &str = "Open Link";
/// About-tab open button label for the Discord invite.
pub const ABOUT_OPEN_INVITE_LABEL: &str = "Open Invite";
/// About-tab copy button label for normal links.
pub const ABOUT_COPY_LINK_LABEL: &str = "Copy Link";
/// About-tab copy button label for the Discord invite.
pub const ABOUT_COPY_INVITE_LABEL: &str = "Copy Invite";
/// Reference copy-button success label from `copy_text_button`.
pub const ABOUT_COPY_SUCCESS_LABEL: &str = "Copied!";
/// Reference copy-button reset delay in milliseconds.
pub const ABOUT_COPY_RESET_DELAY_MS: u64 = 3_000;

/// Rust-owned resource path for the application icon copied from the reference.
pub const IMAGE_ICON_256_RESOURCE_PATH: &str = "resources/images/icon-256.png";
/// Rust-owned resource path for the Nexus Mods logo copied from the reference.
pub const IMAGE_LOGO_NEXUSMODS_RESOURCE_PATH: &str = "resources/images/logo-nexusmods.png";
/// Rust-owned resource path for the Discord logo copied from the reference.
pub const IMAGE_LOGO_DISCORD_RESOURCE_PATH: &str = "resources/images/logo-discord.png";
/// Rust-owned resource path for the GitHub logo copied from the reference.
pub const IMAGE_LOGO_GITHUB_RESOURCE_PATH: &str = "resources/images/logo-github.png";
/// All Rust-owned About image resources required by S05.
pub const IMAGE_RESOURCE_PATHS: [&str; 4] = [
    IMAGE_ICON_256_RESOURCE_PATH,
    IMAGE_LOGO_NEXUSMODS_RESOURCE_PATH,
    IMAGE_LOGO_DISCORD_RESOURCE_PATH,
    IMAGE_LOGO_GITHUB_RESOURCE_PATH,
];

/// Stable action id for each Tools-tab entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolActionId {
    /// Deferred Downgrade Manager utility entry.
    DowngradeManager,
    /// Archive Patcher utility entry.
    ArchivePatcher,
    /// Bethini Pie external link.
    BethiniPie,
    /// CLASSIC Crash Log Scanner external link.
    ClassicCrashLogScanner,
    /// Vault-Tec Enhanced FaceGen System external link.
    VaultTecEnhancedFaceGenSystem,
    /// PJM precombine/previs scripts external link.
    PjmsPrecombinePrevisPatchingScripts,
    /// DDS Texture Scanner external link.
    DdsTextureScanner,
    /// xEdit / FO4Edit external link.
    XeditFo4Edit,
    /// Creation Kit Platform Extended external link.
    CreationKitPlatformExtended,
    /// Cathedral Assets Optimizer external link.
    CathedralAssetsOptimizer,
    /// BA2 Merging Automation Tool external link.
    Ba2MergingAutomationTool,
    /// IceStorm's Texture Tools external link.
    IceStormsTextureTools,
    /// CapFrameX external link.
    CapFrameX,
}

impl ToolActionId {
    /// Returns the stable string id used by UI callbacks and later tracing.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DowngradeManager => "tools.downgrade_manager",
            Self::ArchivePatcher => "tools.archive_patcher",
            Self::BethiniPie => "tools.bethini_pie",
            Self::ClassicCrashLogScanner => "tools.classic_crash_log_scanner",
            Self::VaultTecEnhancedFaceGenSystem => "tools.vault_tec_enhanced_facegen_system",
            Self::PjmsPrecombinePrevisPatchingScripts => {
                "tools.pjms_precombine_previs_patching_scripts"
            }
            Self::DdsTextureScanner => "tools.dds_texture_scanner",
            Self::XeditFo4Edit => "tools.xedit_fo4edit",
            Self::CreationKitPlatformExtended => "tools.creation_kit_platform_extended",
            Self::CathedralAssetsOptimizer => "tools.cathedral_assets_optimizer",
            Self::Ba2MergingAutomationTool => "tools.ba2_merging_automation_tool",
            Self::IceStormsTextureTools => "tools.icestorms_texture_tools",
            Self::CapFrameX => "tools.capframex",
        }
    }
}

/// Stable action id for each About-tab button callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AboutActionId {
    /// Open the Nexus Mods project link.
    OpenNexus,
    /// Copy the Nexus Mods project link.
    CopyNexus,
    /// Open the Discord invite.
    OpenDiscord,
    /// Copy the Discord invite.
    CopyDiscord,
    /// Open the GitHub repository link.
    OpenGithub,
    /// Copy the GitHub repository link.
    CopyGithub,
}

impl AboutActionId {
    /// Returns the stable string id used by UI callbacks and later tracing.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpenNexus => "about.nexus.open",
            Self::CopyNexus => "about.nexus.copy",
            Self::OpenDiscord => "about.discord.open",
            Self::CopyDiscord => "about.discord.copy",
            Self::OpenGithub => "about.github.open",
            Self::CopyGithub => "about.github.copy",
        }
    }
}

/// Stable id for each About-tab link row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AboutLinkId {
    /// Nexus Mods link row.
    Nexus,
    /// Discord invite row.
    Discord,
    /// GitHub repository row.
    Github,
}

impl AboutLinkId {
    /// Returns a stable row id for models and tests.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nexus => "about.nexus",
            Self::Discord => "about.discord",
            Self::Github => "about.github",
        }
    }
}

/// A reference Tools-tab group with entries in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolGroup {
    /// Reference labelframe text.
    pub label: &'static str,
    /// Entries in the exact order displayed within this group.
    pub entries: &'static [ToolEntry],
}

/// A reference Tools-tab button entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolEntry {
    /// Stable callback/tracing id.
    pub id: ToolActionId,
    /// Button label, preserving intentional multi-line spacing.
    pub label: &'static str,
    /// Link or deferred-utility action metadata.
    pub action: ToolEntryAction,
    /// Optional info tooltip shown beside the button.
    pub help_text: Option<&'static str>,
}

impl ToolEntry {
    /// Returns true when the entry should be enabled for direct user action.
    pub const fn is_enabled(self) -> bool {
        matches!(
            self.action,
            ToolEntryAction::ExternalLink(_) | ToolEntryAction::InternalUtility(_)
        )
    }

    /// Returns the internal utility metadata when this entry opens an in-app workflow.
    pub const fn internal_utility(self) -> Option<ToolInternalUtility> {
        match self.action {
            ToolEntryAction::InternalUtility(utility) => Some(utility),
            ToolEntryAction::DeferredUtility(_) | ToolEntryAction::ExternalLink(_) => None,
        }
    }

    /// Returns the deferred metadata when this entry is intentionally disabled.
    pub const fn deferred_utility(self) -> Option<ToolDeferredUtility> {
        match self.action {
            ToolEntryAction::DeferredUtility(utility) => Some(utility),
            ToolEntryAction::InternalUtility(_) | ToolEntryAction::ExternalLink(_) => None,
        }
    }

    /// Returns the external-link metadata when this entry opens a static URL.
    pub const fn external_link(self) -> Option<ToolExternalLink> {
        match self.action {
            ToolEntryAction::ExternalLink(link) => Some(link),
            ToolEntryAction::InternalUtility(_) | ToolEntryAction::DeferredUtility(_) => None,
        }
    }
}

/// Tools-tab action metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolEntryAction {
    /// Utility is implemented inside the Rust port and opens an in-app workflow.
    InternalUtility(ToolInternalUtility),
    /// Utility exists in the reference but is deferred/disabled in this slice.
    DeferredUtility(ToolDeferredUtility),
    /// Static external URL action from the reference.
    ExternalLink(ToolExternalLink),
}

/// Metadata for an in-app utility workflow that is available in this slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolInternalUtility {
    /// Internal utility key for diagnostics and routing.
    pub key: &'static str,
    /// Safe status text that can be displayed in the UI.
    pub status_text: &'static str,
}

/// Metadata for a utility entry that must remain disabled until its workflow is ported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDeferredUtility {
    /// Internal deferred key for diagnostics and future routing.
    pub key: &'static str,
    /// Safe status text that can be displayed in the UI.
    pub status_text: &'static str,
}

/// Metadata for a static external Tools-tab link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolExternalLink {
    /// Static URL from the Python reference.
    pub url: &'static str,
    /// Safe host hint derived from the same mapping as the reference tooltip code.
    pub host_hint: &'static str,
}

/// A reference About-tab link row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AboutLink {
    /// Stable row id.
    pub id: AboutLinkId,
    /// Static URL or invite from the Python reference.
    pub url: &'static str,
    /// Rust-owned logo resource path for this link row.
    pub image_resource_path: &'static str,
    /// Stable action id for the open button.
    pub open_action_id: AboutActionId,
    /// Stable action id for the copy button.
    pub copy_action_id: AboutActionId,
    /// Reference open button label.
    pub open_button_label: &'static str,
    /// Reference copy button label.
    pub copy_button_label: &'static str,
}

/// Returns the reference host tooltip/hint for a static URL.
pub fn url_host_hint(url: &str) -> &'static str {
    if url.contains("nexusmods") {
        URL_HINT_NEXUS_MODS
    } else if url.contains("github") {
        URL_HINT_GITHUB
    } else {
        URL_HINT_OPEN_WEBSITE
    }
}

/// Returns true when a resource path is owned by the Rust port, not the Python reference tree.
pub fn is_rust_owned_resource_path(path: impl AsRef<Path>) -> bool {
    !path.as_ref().components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("CMT")
    })
}

const TOOLKIT_UTILITIES: &[ToolEntry] = &[
    ToolEntry {
        id: ToolActionId::DowngradeManager,
        label: "Downgrade Manager",
        action: ToolEntryAction::InternalUtility(ToolInternalUtility {
            key: "downgrade_manager",
            status_text: "Open the Downgrade Manager workflow.",
        }),
        help_text: None,
    },
    ToolEntry {
        id: ToolActionId::ArchivePatcher,
        label: "Archive Patcher",
        action: ToolEntryAction::InternalUtility(ToolInternalUtility {
            key: "archive_patcher",
            status_text: "Open the Archive Patcher workflow.",
        }),
        help_text: None,
    },
];

const OTHER_CM_AUTHORS_TOOLS: &[ToolEntry] = &[
    ToolEntry {
        id: ToolActionId::BethiniPie,
        label: "Bethini Pie",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://www.nexusmods.com/site/mods/631",
            host_hint: URL_HINT_NEXUS_MODS,
        }),
        help_text: Some(
            "Bethini Pie (Performance INI Editor) makes editing INI config files simple.\nDiscord channel: #bethini-doubleyou-etc",
        ),
    },
    ToolEntry {
        id: ToolActionId::ClassicCrashLogScanner,
        label: "CLASSIC Crash Log Scanner",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://www.nexusmods.com/fallout4/mods/56255",
            host_hint: URL_HINT_NEXUS_MODS,
        }),
        help_text: Some(
            "Scans Buffout crash logs for key indicators of crashes.\nYou can also post crash logs to the CM Discord for assistance.\nDiscord channel: #fo4-crash-logs",
        ),
    },
    ToolEntry {
        id: ToolActionId::VaultTecEnhancedFaceGenSystem,
        label: "  Vault-Tec Enhanced\nFaceGen System (VEFS)",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://www.nexusmods.com/fallout4/mods/86374",
            host_hint: URL_HINT_NEXUS_MODS,
        }),
        help_text: Some(
            "Automates the process of generating FaceGen models and textures with xEdit/CK.\nDiscord channel: #bethini-doubleyou-etc",
        ),
    },
    ToolEntry {
        id: ToolActionId::PjmsPrecombinePrevisPatchingScripts,
        label: "PJM's Precombine/Previs\n    Patching Scripts",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://www.nexusmods.com/fallout4/mods/69978",
            host_hint: URL_HINT_NEXUS_MODS,
        }),
        help_text: Some(
            "Scripts to find precombine/previs (flickering/occlusion) errors in your mod list, and optionally generate a patch to fix those problems.",
        ),
    },
    ToolEntry {
        id: ToolActionId::DdsTextureScanner,
        label: "DDS Texture Scanner",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://www.nexusmods.com/fallout4/mods/71588",
            host_hint: URL_HINT_NEXUS_MODS,
        }),
        help_text: Some(
            "Sniff out textures that might CTD your game. With BA2 support.\nDiscord channel: #nistonmakemod",
        ),
    },
];

const OTHER_USEFUL_TOOLS: &[ToolEntry] = &[
    ToolEntry {
        id: ToolActionId::XeditFo4Edit,
        label: "xEdit / FO4Edit",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://github.com/TES5Edit/TES5Edit#xedit",
            host_hint: URL_HINT_GITHUB,
        }),
        help_text: Some(
            "Module editor and conflict detector for Bethesda games.\nFO4Edit/SSEEdit are xEdit, renamed to auto-set a game mode.",
        ),
    },
    ToolEntry {
        id: ToolActionId::CreationKitPlatformExtended,
        label: "Creation Kit Platform\n   Extended (CKPE)",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://www.nexusmods.com/fallout4/mods/51165",
            host_hint: URL_HINT_NEXUS_MODS,
        }),
        help_text: Some("Various patches and bug fixes for the Creation Kit to make life easier."),
    },
    ToolEntry {
        id: ToolActionId::CathedralAssetsOptimizer,
        label: "Cathedral Assets\nOptimizer (CAO)",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://www.nexusmods.com/skyrimspecialedition/mods/23316",
            host_hint: URL_HINT_NEXUS_MODS,
        }),
        help_text: Some(
            "An automation tool used to optimize BSAs, meshes, textures and animations.",
        ),
    },
    ToolEntry {
        id: ToolActionId::Ba2MergingAutomationTool,
        label: "BA2 Merging Automation\n     Tool (BMAT)",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://www.nexusmods.com/fallout4/mods/89306",
            host_hint: URL_HINT_NEXUS_MODS,
        }),
        help_text: Some("Automated BA2 files repackaging and merging."),
    },
    ToolEntry {
        id: ToolActionId::IceStormsTextureTools,
        label: "IceStorm's Texture Tools",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://storage.icestormng-mods.de/s/QG43aExydefeGXy",
            host_hint: URL_HINT_OPEN_WEBSITE,
        }),
        help_text: Some(
            "Converts textures from various formats into a Fallout 4 compatible format.",
        ),
    },
    ToolEntry {
        id: ToolActionId::CapFrameX,
        label: "CapFrameX",
        action: ToolEntryAction::ExternalLink(ToolExternalLink {
            url: "https://www.capframex.com/",
            host_hint: URL_HINT_OPEN_WEBSITE,
        }),
        help_text: Some(
            "Benchmarking tool - Record FPS, frametime, and sensors; analyse and plot the results.",
        ),
    },
];

/// Reference Tools-tab groups in column/display order.
pub const TOOL_GROUPS: [ToolGroup; 3] = [
    ToolGroup {
        label: "Toolkit Utilities",
        entries: TOOLKIT_UTILITIES,
    },
    ToolGroup {
        label: "Other CM Authors' Tools",
        entries: OTHER_CM_AUTHORS_TOOLS,
    },
    ToolGroup {
        label: "Other Useful Tools",
        entries: OTHER_USEFUL_TOOLS,
    },
];

/// Reference About-tab link rows in display order.
pub const ABOUT_LINKS: [AboutLink; 3] = [
    AboutLink {
        id: AboutLinkId::Nexus,
        url: NEXUS_LINK,
        image_resource_path: IMAGE_LOGO_NEXUSMODS_RESOURCE_PATH,
        open_action_id: AboutActionId::OpenNexus,
        copy_action_id: AboutActionId::CopyNexus,
        open_button_label: ABOUT_OPEN_LINK_LABEL,
        copy_button_label: ABOUT_COPY_LINK_LABEL,
    },
    AboutLink {
        id: AboutLinkId::Discord,
        url: DISCORD_INVITE,
        image_resource_path: IMAGE_LOGO_DISCORD_RESOURCE_PATH,
        open_action_id: AboutActionId::OpenDiscord,
        copy_action_id: AboutActionId::CopyDiscord,
        open_button_label: ABOUT_OPEN_INVITE_LABEL,
        copy_button_label: ABOUT_COPY_INVITE_LABEL,
    },
    AboutLink {
        id: AboutLinkId::Github,
        url: GITHUB_LINK,
        image_resource_path: IMAGE_LOGO_GITHUB_RESOURCE_PATH,
        open_action_id: AboutActionId::OpenGithub,
        copy_action_id: AboutActionId::CopyGithub,
        open_button_label: ABOUT_OPEN_LINK_LABEL,
        copy_button_label: ABOUT_COPY_LINK_LABEL,
    },
];

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, path::Path};

    use super::*;

    #[test]
    fn s05_reference_contract_tool_group_labels_and_button_order_match_reference() {
        assert_eq!(
            TOOL_GROUPS.map(|group| group.label),
            [
                "Toolkit Utilities",
                "Other CM Authors' Tools",
                "Other Useful Tools",
            ]
        );

        assert_eq!(
            TOOL_GROUPS[0]
                .entries
                .iter()
                .map(|entry| entry.label)
                .collect::<Vec<_>>(),
            ["Downgrade Manager", "Archive Patcher"]
        );
        assert_eq!(
            TOOL_GROUPS[1]
                .entries
                .iter()
                .map(|entry| entry.label)
                .collect::<Vec<_>>(),
            [
                "Bethini Pie",
                "CLASSIC Crash Log Scanner",
                "  Vault-Tec Enhanced\nFaceGen System (VEFS)",
                "PJM's Precombine/Previs\n    Patching Scripts",
                "DDS Texture Scanner",
            ]
        );
        assert_eq!(
            TOOL_GROUPS[2]
                .entries
                .iter()
                .map(|entry| entry.label)
                .collect::<Vec<_>>(),
            [
                "xEdit / FO4Edit",
                "Creation Kit Platform\n   Extended (CKPE)",
                "Cathedral Assets\nOptimizer (CAO)",
                "BA2 Merging Automation\n     Tool (BMAT)",
                "IceStorm's Texture Tools",
                "CapFrameX",
            ]
        );
    }

    #[test]
    fn s05_reference_contract_tool_urls_help_text_and_hints_match_reference() {
        let external_entries = TOOL_GROUPS
            .iter()
            .flat_map(|group| group.entries.iter())
            .filter_map(|entry| {
                entry
                    .external_link()
                    .map(|link| (entry.label, entry.help_text, link))
            })
            .collect::<Vec<_>>();

        assert_eq!(external_entries.len(), 11);
        assert_eq!(
            external_entries
                .iter()
                .map(|(label, _, link)| (*label, link.url, link.host_hint))
                .collect::<Vec<_>>(),
            [
                (
                    "Bethini Pie",
                    "https://www.nexusmods.com/site/mods/631",
                    URL_HINT_NEXUS_MODS,
                ),
                (
                    "CLASSIC Crash Log Scanner",
                    "https://www.nexusmods.com/fallout4/mods/56255",
                    URL_HINT_NEXUS_MODS,
                ),
                (
                    "  Vault-Tec Enhanced\nFaceGen System (VEFS)",
                    "https://www.nexusmods.com/fallout4/mods/86374",
                    URL_HINT_NEXUS_MODS,
                ),
                (
                    "PJM's Precombine/Previs\n    Patching Scripts",
                    "https://www.nexusmods.com/fallout4/mods/69978",
                    URL_HINT_NEXUS_MODS,
                ),
                (
                    "DDS Texture Scanner",
                    "https://www.nexusmods.com/fallout4/mods/71588",
                    URL_HINT_NEXUS_MODS,
                ),
                (
                    "xEdit / FO4Edit",
                    "https://github.com/TES5Edit/TES5Edit#xedit",
                    URL_HINT_GITHUB,
                ),
                (
                    "Creation Kit Platform\n   Extended (CKPE)",
                    "https://www.nexusmods.com/fallout4/mods/51165",
                    URL_HINT_NEXUS_MODS,
                ),
                (
                    "Cathedral Assets\nOptimizer (CAO)",
                    "https://www.nexusmods.com/skyrimspecialedition/mods/23316",
                    URL_HINT_NEXUS_MODS,
                ),
                (
                    "BA2 Merging Automation\n     Tool (BMAT)",
                    "https://www.nexusmods.com/fallout4/mods/89306",
                    URL_HINT_NEXUS_MODS,
                ),
                (
                    "IceStorm's Texture Tools",
                    "https://storage.icestormng-mods.de/s/QG43aExydefeGXy",
                    URL_HINT_OPEN_WEBSITE,
                ),
                (
                    "CapFrameX",
                    "https://www.capframex.com/",
                    URL_HINT_OPEN_WEBSITE
                ),
            ]
        );

        assert_eq!(
            external_entries
                .iter()
                .map(|(label, help_text, _)| (*label, help_text.expect("reference tooltip")))
                .collect::<Vec<_>>(),
            [
                (
                    "Bethini Pie",
                    "Bethini Pie (Performance INI Editor) makes editing INI config files simple.\nDiscord channel: #bethini-doubleyou-etc",
                ),
                (
                    "CLASSIC Crash Log Scanner",
                    "Scans Buffout crash logs for key indicators of crashes.\nYou can also post crash logs to the CM Discord for assistance.\nDiscord channel: #fo4-crash-logs",
                ),
                (
                    "  Vault-Tec Enhanced\nFaceGen System (VEFS)",
                    "Automates the process of generating FaceGen models and textures with xEdit/CK.\nDiscord channel: #bethini-doubleyou-etc",
                ),
                (
                    "PJM's Precombine/Previs\n    Patching Scripts",
                    "Scripts to find precombine/previs (flickering/occlusion) errors in your mod list, and optionally generate a patch to fix those problems.",
                ),
                (
                    "DDS Texture Scanner",
                    "Sniff out textures that might CTD your game. With BA2 support.\nDiscord channel: #nistonmakemod",
                ),
                (
                    "xEdit / FO4Edit",
                    "Module editor and conflict detector for Bethesda games.\nFO4Edit/SSEEdit are xEdit, renamed to auto-set a game mode.",
                ),
                (
                    "Creation Kit Platform\n   Extended (CKPE)",
                    "Various patches and bug fixes for the Creation Kit to make life easier.",
                ),
                (
                    "Cathedral Assets\nOptimizer (CAO)",
                    "An automation tool used to optimize BSAs, meshes, textures and animations.",
                ),
                (
                    "BA2 Merging Automation\n     Tool (BMAT)",
                    "Automated BA2 files repackaging and merging.",
                ),
                (
                    "IceStorm's Texture Tools",
                    "Converts textures from various formats into a Fallout 4 compatible format.",
                ),
                (
                    "CapFrameX",
                    "Benchmarking tool - Record FPS, frametime, and sensors; analyse and plot the results.",
                ),
            ]
        );

        for (_, _, link) in external_entries {
            assert_eq!(link.host_hint, url_host_hint(link.url));
        }
        assert_eq!(
            url_host_hint("https://example.invalid/tool"),
            URL_HINT_OPEN_WEBSITE
        );
    }

    #[test]
    fn s05_reference_contract_about_title_credit_links_and_copy_labels_match_reference() {
        assert_eq!(APP_TITLE, "Collective Modding Toolkit");
        assert_eq!(APP_VERSION, "0.6.1");
        assert_eq!(ABOUT_TITLE_LABEL, "Collective Modding\nToolkit");
        assert_eq!(
            ABOUT_CREDIT_LABEL,
            "v0.6.1\n\nCreated by wxMichael for the\nCollective Modding Community\n#cm-toolkit on Discord"
        );
        assert_eq!(ABOUT_COPY_SUCCESS_LABEL, "Copied!");
        assert_eq!(ABOUT_COPY_RESET_DELAY_MS, 3_000);

        assert_eq!(
            ABOUT_LINKS.map(|link| {
                (
                    link.id.as_str(),
                    link.url,
                    link.image_resource_path,
                    link.open_action_id.as_str(),
                    link.copy_action_id.as_str(),
                    link.open_button_label,
                    link.copy_button_label,
                )
            }),
            [
                (
                    "about.nexus",
                    NEXUS_LINK,
                    IMAGE_LOGO_NEXUSMODS_RESOURCE_PATH,
                    "about.nexus.open",
                    "about.nexus.copy",
                    ABOUT_OPEN_LINK_LABEL,
                    ABOUT_COPY_LINK_LABEL,
                ),
                (
                    "about.discord",
                    DISCORD_INVITE,
                    IMAGE_LOGO_DISCORD_RESOURCE_PATH,
                    "about.discord.open",
                    "about.discord.copy",
                    ABOUT_OPEN_INVITE_LABEL,
                    ABOUT_COPY_INVITE_LABEL,
                ),
                (
                    "about.github",
                    GITHUB_LINK,
                    IMAGE_LOGO_GITHUB_RESOURCE_PATH,
                    "about.github.open",
                    "about.github.copy",
                    ABOUT_OPEN_LINK_LABEL,
                    ABOUT_COPY_LINK_LABEL,
                ),
            ]
        );
    }

    #[test]
    fn s05_reference_contract_toolkit_utilities_track_available_and_deferred_workflows() {
        let entries = TOOL_GROUPS
            .iter()
            .flat_map(|group| group.entries.iter())
            .collect::<Vec<_>>();

        let mut seen_tool_ids = HashSet::new();
        for entry in &entries {
            assert!(
                seen_tool_ids.insert(entry.id.as_str()),
                "duplicate tool action id: {}",
                entry.id.as_str()
            );
        }

        let internal_entries = entries
            .iter()
            .filter_map(|entry| {
                entry
                    .internal_utility()
                    .map(|utility| (entry.label, utility))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            internal_entries,
            [(
                "Downgrade Manager",
                ToolInternalUtility {
                    key: "downgrade_manager",
                    status_text: "Open the Downgrade Manager workflow.",
                },
            )]
        );
        assert!(TOOL_GROUPS[0].entries[0].is_enabled());
        assert!(TOOL_GROUPS[0].entries[0].external_link().is_none());
        assert!(TOOL_GROUPS[0].entries[0].deferred_utility().is_none());

        let deferred_entries = entries
            .iter()
            .filter_map(|entry| {
                entry
                    .deferred_utility()
                    .map(|utility| (entry.label, utility))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            deferred_entries,
            [(
                "Archive Patcher",
                ToolDeferredUtility {
                    key: "archive_patcher",
                    status_text: "Archive Patcher is not available in this Rust port yet.",
                },
            )]
        );
        assert!(!TOOL_GROUPS[0].entries[1].is_enabled());
        assert!(TOOL_GROUPS[0].entries[1].external_link().is_none());

        for entry in entries
            .iter()
            .filter(|entry| entry.external_link().is_some())
        {
            assert!(entry.is_enabled());
            assert!(entry.deferred_utility().is_none());
            assert!(entry.internal_utility().is_none());
        }

        let mut seen_about_ids = HashSet::new();
        for link in ABOUT_LINKS {
            assert!(
                seen_about_ids.insert(link.id.as_str()),
                "duplicate about link id: {}",
                link.id.as_str()
            );
            assert!(
                seen_about_ids.insert(link.open_action_id.as_str()),
                "duplicate about open action id: {}",
                link.open_action_id.as_str()
            );
            assert!(
                seen_about_ids.insert(link.copy_action_id.as_str()),
                "duplicate about copy action id: {}",
                link.copy_action_id.as_str()
            );
        }
    }

    #[test]
    fn s05_reference_contract_image_resources_are_rust_owned_and_present() {
        for resource_path in IMAGE_RESOURCE_PATHS {
            assert!(
                is_rust_owned_resource_path(resource_path),
                "resource path must not reference CMT/: {resource_path}"
            );

            let absolute_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(resource_path);
            assert!(
                absolute_path.is_file(),
                "missing copied Rust-owned image resource: {}",
                absolute_path.display()
            );
        }
    }
}
