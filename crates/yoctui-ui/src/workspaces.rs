//! Typed screen-to-workspace routing.

use super::*;

pub(super) fn workspace(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    terminal_width: u16,
    now: SystemTime,
    task_rows: Option<&[TaskRowRef<'_>]>,
) {
    match app.screen {
        Screen::Dashboard => tasks_workspace(frame, app, area, now, task_rows.unwrap_or_default()),
        Screen::Tasks => tasks_workspace(frame, app, area, now, task_rows.unwrap_or_default()),
        Screen::BuildHistory => build_history(frame, app, area, now),
        Screen::Dependencies => dependencies(frame, app, area),
        Screen::Signatures => signature_records(frame, app, area),
        Screen::LayerRelationships => layer_relationships(frame, app, area),
        Screen::Logs => logs(frame, app, area),
        Screen::Errors => errors(frame, app, area),
        Screen::Recipes => recipes(frame, app, area),
        Screen::Packages => packages_workspace(frame, app, area),
        Screen::Images => images_workspace(frame, app, area),
        Screen::Sdk => sdk_workspace(frame, app, area),
        Screen::Testing => testing_workspace(frame, app, area),
        Screen::Security => security_workspace(frame, app, area),
        Screen::Qa => qa_workspace(frame, app, area),
        Screen::Layers => {
            if let Some(browser) = app.layer_browser.as_ref() {
                layer_browser(frame, app, browser, area)
            } else {
                layers(frame, app, area)
            }
        }
        Screen::Configuration => config(frame, app, area),
        Screen::Bbmask => bbmask(frame, app, area),
        Screen::RawMode => raw_mode_workspace(frame, app, area, terminal_width),
        Screen::TerminalSessions => terminal_sessions_workspace(frame, app, area),
        Screen::Maintenance => maintenance_workspace(frame, app, area),
        Screen::Compatibility => compatibility_workspace(frame, app, area),
        Screen::Help => help(frame, app, area),
        Screen::Settings => settings_workspace(frame, app, area),
        Screen::BuildEnvironment => build_environment_workspace(frame, app, area),
    }
}
