pub mod app;
pub mod domain;
pub mod platform;
pub mod workers;

use std::{cell::RefCell, rc::Rc};

use app::settings_controller::SettingsController;
use domain::settings::AppSettings;
use platform::settings_store::{FileAssetResolver, SettingsStore};
use slint::ComponentHandle;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = MainWindow::new()?;
    let controller = load_settings_controller();

    app.set_update_source(controller.visible_update_source().into());
    app.set_log_level(controller.visible_log_level().into());
    bind_settings_callbacks(&app, controller);

    Ok(app.run()?)
}

fn load_settings_controller() -> SettingsController<FileAssetResolver> {
    SettingsController::load(SettingsStore::production()).unwrap_or_else(|error| {
        tracing::error!(%error, "Settings : Failed to load settings; using in-memory defaults");
        SettingsController::from_settings(SettingsStore::production(), AppSettings::default())
    })
}

fn bind_settings_callbacks(app: &MainWindow, controller: SettingsController<FileAssetResolver>) {
    let controller = Rc::new(RefCell::new(controller));

    app.on_update_source_selected({
        let app = app.as_weak();
        let controller = Rc::clone(&controller);

        move |selected| {
            let visible_value = controller
                .borrow_mut()
                .select_update_source(selected.as_str());
            if let Some(app) = app.upgrade() {
                app.set_update_source(visible_value.into());
            }
        }
    });

    app.on_log_level_selected({
        let app = app.as_weak();
        let controller = Rc::clone(&controller);

        move |selected| {
            let visible_value = controller.borrow_mut().select_log_level(selected.as_str());
            if let Some(app) = app.upgrade() {
                app.set_log_level(visible_value.into());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        app::{SHELL_TAB_LABELS, ShellController, shell_tab_labels},
        domain::DomainState,
        platform::PlatformServices,
        workers::WorkerRuntime,
    };

    const MAIN_SLINT: &str = include_str!("../ui/main.slint");
    const SETTINGS_SLINT: &str = include_str!("../ui/settings_tab.slint");
    const TAB_COMPONENTS: [(&str, &str, &str, &str); 6] = [
        (
            "ui/overview_tab.slint",
            "OverviewTab",
            "Overview",
            include_str!("../ui/overview_tab.slint"),
        ),
        (
            "ui/f4se_tab.slint",
            "F4seTab",
            "F4SE",
            include_str!("../ui/f4se_tab.slint"),
        ),
        (
            "ui/scanner_tab.slint",
            "ScannerTab",
            "Scanner",
            include_str!("../ui/scanner_tab.slint"),
        ),
        (
            "ui/tools_tab.slint",
            "ToolsTab",
            "Tools",
            include_str!("../ui/tools_tab.slint"),
        ),
        (
            "ui/settings_tab.slint",
            "SettingsTab",
            "Settings",
            SETTINGS_SLINT,
        ),
        (
            "ui/about_tab.slint",
            "AboutTab",
            "About",
            include_str!("../ui/about_tab.slint"),
        ),
    ];
    const INERT_TAB_COMPONENTS: [(&str, &str, &str, &str); 5] = [
        TAB_COMPONENTS[0],
        TAB_COMPONENTS[1],
        TAB_COMPONENTS[2],
        TAB_COMPONENTS[3],
        TAB_COMPONENTS[5],
    ];

    fn slint_string_property_values(source: &str, property: &str) -> Vec<String> {
        let prefix = format!("{property}:");

        source
            .lines()
            .filter_map(|line| line.trim().strip_prefix(&prefix))
            .filter_map(|value| value.trim().trim_end_matches(';').strip_prefix('"'))
            .filter_map(|value| value.strip_suffix('"'))
            .map(String::from)
            .collect()
    }

    fn assert_source_contains_in_order(source: &str, expected: &[&str]) {
        let mut search_from = 0;

        for value in expected {
            let relative_index = source[search_from..].find(value).unwrap_or_else(|| {
                panic!("expected source to contain {value:?} after byte {search_from}")
            });
            search_from += relative_index + value.len();
        }
    }

    #[test]
    fn shell_tab_labels_match_reference_order() {
        assert_eq!(
            shell_tab_labels(),
            ["Overview", "F4SE", "Scanner", "Tools", "Settings", "About"]
        );
    }

    #[test]
    fn shell_tab_labels_count_is_reference_count() {
        assert_eq!(SHELL_TAB_LABELS.len(), 6);
    }

    #[test]
    fn shell_contract_main_slint_title_and_tabs_match_rust_contract() {
        let titles = slint_string_property_values(MAIN_SLINT, "title");

        assert_eq!(
            titles.first().map(String::as_str),
            Some("Collective Modding Toolkit")
        );
        assert_eq!(
            titles
                .iter()
                .skip(1)
                .map(String::as_str)
                .collect::<Vec<_>>(),
            SHELL_TAB_LABELS.to_vec()
        );
    }

    #[test]
    fn shell_contract_inert_tab_components_are_static_placeholders() {
        let prohibited_markers = [
            "callback",
            "clicked",
            "changed",
            "=>",
            "Timer",
            "FileDialog",
            "fs::",
            "std::fs",
            "filesystem",
            "network",
            "http://",
            "https://",
            "process",
            "Command",
            "spawn",
        ];

        for (file, component, label, source) in INERT_TAB_COMPONENTS {
            assert_eq!(
                source.matches("export component ").count(),
                1,
                "{file} should export exactly one component"
            );
            assert!(
                source.contains(&format!("export component {component}")),
                "{file} should export {component}"
            );
            assert!(
                source.contains(&format!("text: \"{label}\";")),
                "{file} should keep the reference tab heading"
            );
            assert!(
                source.contains(&format!(
                    "text: \"{label} behavior is reserved for a later port phase.\";"
                )),
                "{file} should keep the inert scope note"
            );

            for marker in prohibited_markers {
                assert!(
                    !source.contains(marker),
                    "{file} should not contain behavior marker {marker:?}"
                );
            }
        }
    }

    #[test]
    fn settings_tab_update_channel_labels() {
        assert_source_contains_in_order(
            SETTINGS_SLINT,
            &[
                "title: \"Update Channel\"",
                "text: \"All: GitHub & Nexus Mods\"",
                "root.update-source = \"both\"",
                "root.update-source-selected(\"both\")",
                "text: \"Early: GitHub\"",
                "root.update-source = \"github\"",
                "root.update-source-selected(\"github\")",
                "text: \"Stable: Nexus Mods\"",
                "root.update-source = \"nexus\"",
                "root.update-source-selected(\"nexus\")",
                "text: \"Never: Don't Check\"",
                "root.update-source = \"none\"",
                "root.update-source-selected(\"none\")",
            ],
        );

        assert!(SETTINGS_SLINT.contains("in-out property <string> update-source"));
        assert!(SETTINGS_SLINT.contains("callback update-source-selected(string)"));
    }

    #[test]
    fn settings_tab_log_level_labels() {
        assert_source_contains_in_order(
            SETTINGS_SLINT,
            &[
                "title: \"Log Level\"",
                "text: \"Debug\"",
                "root.log-level = \"debug\"",
                "root.log-level-selected(\"debug\")",
                "text: \"Info\"",
                "root.log-level = \"info\"",
                "root.log-level-selected(\"info\")",
                "text: \"Warning\"",
                "root.log-level = \"warning\"",
                "root.log-level-selected(\"warning\")",
                "text: \"Error\"",
                "root.log-level = \"error\"",
                "root.log-level-selected(\"error\")",
            ],
        );

        assert!(SETTINGS_SLINT.contains("in-out property <string> log-level"));
        assert!(SETTINGS_SLINT.contains("callback log-level-selected(string)"));
    }

    #[test]
    fn settings_tab_uses_dark_mode_palette() {
        assert!(SETTINGS_SLINT.contains("background: #202020;"));
        assert!(SETTINGS_SLINT.contains("color: #f3f3f3;"));
        assert!(!SETTINGS_SLINT.contains("background: #f3f3f3;"));
    }

    #[test]
    fn main_window_forwards_settings_tab_api() {
        assert_source_contains_in_order(
            MAIN_SLINT,
            &[
                "in-out property <string> update-source",
                "in-out property <string> log-level",
                "callback update-source-selected(string)",
                "callback log-level-selected(string)",
                "SettingsTab {",
                "update-source <=> root.update-source",
                "log-level <=> root.log-level",
                "root.update-source-selected(value)",
                "root.log-level-selected(value)",
            ],
        );
    }

    #[test]
    fn shell_contract_boundary_markers_construct_as_no_ops() {
        let _controller = ShellController;
        let _domain = DomainState;
        let _platform = PlatformServices;
        let _workers = WorkerRuntime;
    }
}
