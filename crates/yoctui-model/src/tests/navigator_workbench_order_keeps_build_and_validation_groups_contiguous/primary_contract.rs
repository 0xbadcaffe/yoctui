use super::*;

#[test]
fn navigator_workbench_order_keeps_build_and_validation_groups_contiguous() {
    assert_eq!(
        NAVIGATOR_SCREENS,
        [
            Screen::Dashboard,
            Screen::Insights,
            Screen::Layers,
            Screen::Recipes,
            Screen::Packages,
            Screen::Images,
            Screen::Kernel,
            Screen::Firmware,
            Screen::Sdk,
            Screen::Tasks,
            Screen::Logs,
            Screen::Errors,
            Screen::Configuration,
            Screen::Dependencies,
            Screen::Testing,
            Screen::Security,
            Screen::Qa,
            Screen::RawMode,
            Screen::TerminalSessions,
            Screen::Recipes,
            Screen::Images,
            Screen::Maintenance,
            Screen::BuildEnvironment,
            Screen::Compatibility,
            Screen::Settings,
        ]
    );
}
