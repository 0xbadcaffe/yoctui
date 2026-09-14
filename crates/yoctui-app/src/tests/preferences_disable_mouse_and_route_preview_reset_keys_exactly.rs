//! Regression tests grouped around ux_preferences_disable_mouse_and_route_preview_reset_keys_exactly.
use super::*;

#[test]
fn ux_preferences_disable_mouse_and_route_preview_reset_keys_exactly() {
    let mut app = yoctui_model::App::new(8, 1_000);
    app.preferences.mouse_enabled = false;
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                column: 30,
                row: 10,
                kind: MouseKind::Down,
            },
            &app,
            100,
            30,
        ),
        None
    );
    assert_eq!(
        settings_action(Input::Right),
        Some(Action::ChangeSelectedSetting { backwards: false })
    );
    assert_eq!(
        settings_action(Input::Char('R')),
        Some(Action::ResetPreferences)
    );
}

#[test]
fn kernel_workbench_keys_route_to_typed_actions() {
    assert_eq!(
        platform_workspace_action(Input::Tab),
        Some(Action::CycleKernelView)
    );
    assert_eq!(
        platform_workspace_action(Input::Char('m')),
        Some(Action::LaunchKernelMenuconfig)
    );
    assert_eq!(
        platform_workspace_action(Input::Char('c')),
        Some(Action::CompileSelectedKernelDts)
    );
    assert_eq!(
        platform_workspace_action(Input::Char('d')),
        Some(Action::DecompileSelectedKernelDtb)
    );
}

#[test]
fn firmware_workbench_keys_route_to_typed_actions() {
    assert_eq!(
        firmware_workspace_action(Input::Tab),
        Some(Action::CycleFirmwareView)
    );
    assert_eq!(
        firmware_workspace_action(Input::Char('m')),
        Some(Action::LaunchFirmwareMenuconfig)
    );
    assert_eq!(
        firmware_workspace_action(Input::Char('c')),
        Some(Action::CompileSelectedFirmwareDts)
    );
    assert_eq!(
        firmware_workspace_action(Input::Char('d')),
        Some(Action::DecompileSelectedFirmwareDtb)
    );
}

#[test]
fn overview_workspace_routes_tabs_and_numbered_views() {
    assert_eq!(
        overview_workspace_action(Input::Char('7')),
        Some(Action::SelectOverviewView(
            yoctui_model::OverviewView::SupplyChain
        ))
    );
    assert_eq!(
        overview_workspace_action(Input::Char(']')),
        Some(Action::ShiftOverviewView { delta: 1 })
    );
    let mut app = yoctui_model::App::new(8, 1_000);
    app.screen = Screen::Insights;
    let _ = yoctui_model::update(&mut app, Action::ShiftOverviewView { delta: -1 });
    assert_eq!(app.overview_view, yoctui_model::OverviewView::DiskUsage);
}
