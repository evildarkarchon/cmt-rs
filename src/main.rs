pub mod app;
pub mod domain;
pub mod platform;
pub mod workers;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = MainWindow::new()?;
    app.run()
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
            include_str!("../ui/settings_tab.slint"),
        ),
        (
            "ui/about_tab.slint",
            "AboutTab",
            "About",
            include_str!("../ui/about_tab.slint"),
        ),
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

        for (file, component, label, source) in TAB_COMPONENTS {
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
    fn shell_contract_boundary_markers_construct_as_no_ops() {
        let _controller = ShellController;
        let _domain = DomainState;
        let _platform = PlatformServices;
        let _workers = WorkerRuntime;
    }
}
