pub(crate) fn dialog_mouse_action(
    mouse: MouseInput,
    app: &yoctui_model::App,
    terminal_width: u16,
    terminal_height: u16,
) -> Option<Action> {
    if matches!(mouse.kind, MouseKind::Down | MouseKind::Drag)
        && let Some(editor) = active_dialog_textarea(app)
    {
        let width = 110u16.min(terminal_width.saturating_sub(2)).max(1);
        let preferred_height = terminal_height.saturating_sub(4).min(34);
        let height = preferred_height
            .min(terminal_height.saturating_sub(2))
            .max(1);
        let x = terminal_width.saturating_sub(width) / 2;
        let y = terminal_height.saturating_sub(height) / 2;
        let content_y = y.saturating_add(2);
        let gutter = editor.line_count().to_string().len() as u16 + 3;
        let content_x = x.saturating_add(1).saturating_add(gutter);
        if mouse.row >= content_y
            && mouse.row < y.saturating_add(height).saturating_sub(3)
            && mouse.column >= content_x
            && mouse.column < x.saturating_add(width).saturating_sub(1)
        {
            return Some(Action::EditActivePopup(
                PopupEditorCommand::SelectPosition {
                    line: usize::from(mouse.row - content_y),
                    column: usize::from(mouse.column - content_x),
                    extend: mouse.kind == MouseKind::Drag,
                },
            ));
        }
    }
    if matches!(mouse.kind, MouseKind::Down | MouseKind::ContextDown) {
        return Some(Action::Focus(FocusTarget::Dialog));
    }
    let delta = match mouse.kind {
        MouseKind::ScrollUp => -1,
        MouseKind::ScrollDown => 1,
        MouseKind::Drag | MouseKind::Up | MouseKind::Down | MouseKind::ContextDown => return None,
    };
    match app.active_dialog()? {
        yoctui_model::Dialog::EnvironmentSetup(setup) if setup.editor.is_none() => {
            environment_setup_action(setup, if delta < 0 { Input::Up } else { Input::Down })
        }
        yoctui_model::Dialog::ThemePicker { .. } => Some(Action::SelectTheme { delta }),
        yoctui_model::Dialog::ImagePicker(_) => Some(Action::SelectImage { delta }),
        yoctui_model::Dialog::RecipeTaskPicker(_) => Some(Action::SelectRecipeTask { delta }),
        yoctui_model::Dialog::RecipeTaskLogPicker(_) => Some(Action::SelectRecipeTaskLog { delta }),
        yoctui_model::Dialog::RecipePatchPicker(_) => Some(Action::SelectRecipePatch { delta }),
        yoctui_model::Dialog::SignatureTaskPicker(_) => Some(Action::SelectSignatureTask { delta }),
        yoctui_model::Dialog::ConfigSourcePicker(_) => Some(Action::SelectConfigSource { delta }),
        yoctui_model::Dialog::ConfigScopePicker(_) => Some(Action::SelectConfigScope { delta }),
        yoctui_model::Dialog::DevtoolFinishPicker(_) => {
            Some(Action::SelectDevtoolFinishLayer { delta })
        }
        yoctui_model::Dialog::WicDevicePicker(_) => Some(Action::SelectWicDevice { delta }),
        yoctui_model::Dialog::DtcCompile(_) => Some(Action::SelectDtcCompileOption { delta }),
        _ => None,
    }
}

pub(crate) fn active_dialog_textarea(
    app: &yoctui_model::App,
) -> Option<&yoctui_model::TextAreaState> {
    match app.active_dialog()? {
        yoctui_model::Dialog::BuildEnvironmentEditor(editor)
        | yoctui_model::Dialog::BuildEnvironmentCloneEditor(editor)
        | yoctui_model::Dialog::BbmaskEdit(editor)
        | yoctui_model::Dialog::SdkPublishTomlEditor(editor)
        | yoctui_model::Dialog::SdkNativeTomlEditor(editor) => Some(editor),
        yoctui_model::Dialog::ConfigEdit { editor, .. }
        | yoctui_model::Dialog::BuildTarget { editor, .. } => Some(editor),
        yoctui_model::Dialog::WicCreateTomlEditor { editor, .. }
        | yoctui_model::Dialog::TestLaunchTomlEditor { editor, .. }
        | yoctui_model::Dialog::TestResultImportTomlEditor { editor, .. }
        | yoctui_model::Dialog::TestComparisonTomlEditor { editor, .. }
        | yoctui_model::Dialog::TestJunitTomlEditor { editor, .. } => Some(editor),
        yoctui_model::Dialog::Security(yoctui_model::SecurityDialog::Import { editor, .. })
        | yoctui_model::Dialog::Qa(yoctui_model::QaDialog::Import { editor, .. }) => Some(editor),
        yoctui_model::Dialog::Maintenance(dialog) => match dialog.as_ref() {
            yoctui_model::MaintenanceDialog::ReadinessToml { editor, .. }
            | yoctui_model::MaintenanceDialog::CleanupToml { editor, .. }
            | yoctui_model::MaintenanceDialog::PrServiceToml { editor, .. }
            | yoctui_model::MaintenanceDialog::LockedCacheToml { editor, .. }
            | yoctui_model::MaintenanceDialog::BuildHistoryToml { editor, .. }
            | yoctui_model::MaintenanceDialog::GitArchiveToml { editor, .. } => Some(editor),
            _ => None,
        },
        _ => None,
    }
}

pub fn workspace_collection_action(app: &yoctui_model::App, key: Input) -> Option<Action> {
    let delta = collection_scroll_delta(key)?;
    match app.screen {
        Screen::Dashboard | Screen::Tasks => tasks_action(app.task_filter_editing, key),
        Screen::Insights => None,
        Screen::BuildHistory => Some(Action::SelectBuildHistory { delta }),
        Screen::Dependencies => dependency_workspace_action(app.dependency_graph_searching, key),
        Screen::Signatures => signature_workspace_action(key),
        Screen::Recipes => recipes_workspace_action(app.metadata_searching, key),
        Screen::Packages => package_workspace_action(app.package_searching, key),
        Screen::Images => {
            images_workspace_action_for_view(app.image_artifact_searching, app.images_view, key)
        }
        Screen::Kernel => platform_workspace_action(key),
        Screen::Firmware => firmware_workspace_action(key),
        Screen::Sdk => sdk_workspace_action(app.sdk_artifact_searching, key),
        Screen::Testing => match app.test_view {
            TestWorkspaceView::Launches => testing_workspace_action(key),
            TestWorkspaceView::Results => test_results_workspace_action(
                app.test_result_searching,
                app.test_result_drilled,
                key,
            ),
            TestWorkspaceView::Comparison => test_comparison_workspace_action(key),
        },
        Screen::Security => security_workspace_action(
            app.security.view,
            app.security.drilled,
            app.security.searching,
            key,
        ),
        Screen::Qa => qa_workspace_action(app.qa.view, app.qa.drilled, app.qa.searching, key),
        Screen::Layers => {
            if app.layer_browser.is_some() {
                layer_tree_action(app.metadata_searching, key)
            } else {
                Some(Action::SelectLayer { delta })
            }
        }
        Screen::Configuration => config_workspace_action(app.metadata_searching, key),
        Screen::RawMode => raw_mode_input(app, key).map(Action::RawMode),
        Screen::TerminalSessions => terminal_workspace_action(app, key),
        Screen::Maintenance => maintenance_workspace_action(
            app.maintenance.view,
            match app.maintenance.view {
                MaintenanceView::Sstate => 2,
                MaintenanceView::Services => 1,
                MaintenanceView::Release | MaintenanceView::Integrations => 4,
            },
            key,
        ),
        Screen::Logs => log_workspace_action(app, key),
        Screen::Errors => errors_action(key),
        Screen::BuildEnvironment => build_environment_action(key),
        Screen::Compatibility => {
            compatibility_ui_inspector_action(app.compatibility_ui.searching, key)
        }
        Screen::Settings => settings_action(key),
        Screen::LayerRelationships | Screen::Bbmask | Screen::Help => None,
    }
}

pub fn platform_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectKernelFile { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectKernelFile { delta: 1 }),
        Input::PageUp => Some(Action::SelectKernelFile { delta: -10 }),
        Input::PageDown => Some(Action::SelectKernelFile { delta: 10 }),
        Input::Tab | Input::BackTab => Some(Action::CycleKernelView),
        Input::Char('m') => Some(Action::LaunchKernelMenuconfig),
        Input::Enter | Input::Char('e') => Some(Action::OpenSelectedKernelFile),
        Input::Char('o') => Some(Action::ExploreSelectedKernelRoot),
        Input::Char('c') => Some(Action::CompileSelectedKernelDts),
        Input::Char('d') => Some(Action::DecompileSelectedKernelDtb),
        Input::Char('r') => Some(Action::InspectKernel),
        _ => None,
    }
}

pub fn firmware_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectFirmwareFile { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectFirmwareFile { delta: 1 }),
        Input::PageUp => Some(Action::SelectFirmwareFile { delta: -10 }),
        Input::PageDown => Some(Action::SelectFirmwareFile { delta: 10 }),
        Input::Tab | Input::BackTab => Some(Action::CycleFirmwareView),
        Input::Char('m') => Some(Action::LaunchFirmwareMenuconfig),
        Input::Enter | Input::Char('e') => Some(Action::OpenSelectedFirmwareFile),
        Input::Char('o') => Some(Action::ExploreSelectedFirmwareRoot),
        Input::Char('c') => Some(Action::CompileSelectedFirmwareDts),
        Input::Char('d') => Some(Action::DecompileSelectedFirmwareDtb),
        Input::Char('r') => Some(Action::InspectFirmware),
        _ => None,
    }
}

pub fn dashboard_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Char('f') => Some(Action::OpenRawFavorites),
        Input::Char('t') => Some(Action::Open(Screen::TerminalSessions)),
        _ => None,
    }
}

pub fn overview_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Left | Input::Char('[') | Input::Char('h') => {
            Some(Action::ShiftOverviewView { delta: -1 })
        }
        Input::Right | Input::Char(']') | Input::Char('l') => {
            Some(Action::ShiftOverviewView { delta: 1 })
        }
        Input::Char(character @ '1'..='8') => {
            yoctui_model::OverviewView::from_number(character as u8 - b'0')
                .map(Action::SelectOverviewView)
        }
        _ => None,
    }
}
