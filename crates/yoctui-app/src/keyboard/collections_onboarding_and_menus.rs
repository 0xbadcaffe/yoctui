pub(crate) const DEFAULT_COLLECTION_PAGE_ROWS: isize = 10;

/// Resolve the common collection vocabulary into one signed selection delta.
/// Reducers still clamp against their authoritative filtered inventory.
pub fn collection_scroll_delta(key: Input) -> Option<isize> {
    match key {
        Input::Up | Input::Char('k') => Some(-1),
        Input::Down | Input::Char('j') => Some(1),
        Input::PageUp => Some(-DEFAULT_COLLECTION_PAGE_ROWS),
        Input::PageDown => Some(DEFAULT_COLLECTION_PAGE_ROWS),
        Input::Home => Some(isize::MIN),
        Input::End | Input::Char('G') => Some(isize::MAX),
        _ => None,
    }
}

pub fn keymap_preferences_action(app: &yoctui_model::App, key: Input) -> Option<Action> {
    if !app.keymap_preferences_ui.open {
        return None;
    }
    if app.keymap_preferences_ui.capture.is_some() {
        return Some(match key {
            Input::CtrlS => Action::ConfirmKeymapCapture,
            Input::Esc => Action::CancelKeymapCapture,
            Input::Backspace => Action::BackspaceKeymapCapture,
            key => Action::AppendKeymapCapture(input_key_stroke(key)),
        });
    }
    if app.keymap_preferences_ui.searching {
        return match key {
            Input::Esc | Input::Enter => Some(Action::FinishKeymapPreferenceSearch),
            Input::CtrlU => Some(Action::ClearKeymapPreferenceQuery),
            Input::Backspace => Some(Action::BackspaceKeymapPreferenceQuery),
            Input::Char(character) => Some(Action::AppendKeymapPreferenceQuery(character)),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectKeymapPreference { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectKeymapPreference { delta: 1 }),
        Input::Char('/') => Some(Action::BeginKeymapPreferenceSearch),
        Input::Enter | Input::Char('c') => Some(Action::BeginKeymapCapture),
        Input::Char('x') | Input::Char('d') => Some(Action::RemoveKeymapBinding),
        Input::Char('r') => Some(Action::ResetKeymapBinding),
        Input::Char('R') => Some(Action::ResetAllKeymapBindings),
        Input::Char('e') => Some(Action::ExportEffectiveKeymap),
        Input::Char('p') if app.settings_dirty => Some(Action::RetrySettingsPersistence),
        Input::Esc => Some(Action::CloseKeymapPreferences),
        _ => None,
    }
}

pub fn onboarding_action(app: &yoctui_model::App, key: Input) -> Option<Action> {
    if !app.onboarding.open {
        return None;
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectOnboarding { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectOnboarding { delta: 1 }),
        Input::Enter => Some(Action::ActivateOnboardingStep),
        Input::Right | Input::Char('n') => Some(Action::AdvanceOnboarding),
        Input::Char('s') => Some(Action::SkipOnboardingStep),
        Input::Char('r') => Some(Action::RestartOnboarding),
        Input::Esc | Input::Char('q') => Some(Action::DismissOnboarding),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuInputResult {
    Reduce(Box<Action>),
    ActivateCommand(yoctui_model::CommandId),
    ActivateContext(Input),
}

pub fn menu_action(app: &yoctui_model::App, key: Input) -> Option<MenuInputResult> {
    if !app.menu.is_open() {
        return None;
    }
    let reduce = |action| Some(MenuInputResult::Reduce(Box::new(action)));
    match key {
        Input::Esc | Input::F12 => reduce(Action::CloseMenu),
        Input::Left => reduce(Action::SelectMenuGroup { delta: -1 }),
        Input::Right => reduce(Action::SelectMenuGroup { delta: 1 }),
        Input::Up | Input::Char('k') => reduce(Action::SelectMenuItem { delta: -1 }),
        Input::Down | Input::Char('j') => reduce(Action::SelectMenuItem { delta: 1 }),
        Input::Backspace => reduce(Action::BackspaceMenuPrefix),
        Input::Enter => {
            let item = app.selected_menu_item()?;
            if !item.enabled() {
                return None;
            }
            match item.target {
                yoctui_model::OperatorActionTarget::Command(command) => {
                    Some(MenuInputResult::ActivateCommand(command))
                }
                yoctui_model::OperatorActionTarget::Workspace { legacy_id, .. } => {
                    context_menu_activation_input(legacy_id).map(MenuInputResult::ActivateContext)
                }
            }
        }
        Input::Char(character) => reduce(Action::AppendMenuPrefix(character)),
        _ => None,
    }
}

pub fn context_menu_activation_input(action_id: &str) -> Option<Input> {
    let input = match action_id {
        "dashboard.build" => Input::Char('B'),
        "dashboard.cancel" => Input::Char('c'),
        "dashboard.tasks" => Input::F2,
        "dashboard.logs" => Input::Char('l'),
        "dashboard.errors" => Input::Char('e'),
        "dashboard.history" => Input::F3,
        "dashboard.artifacts" => Input::F8,
        "dashboard.environment" => Input::Char('E'),
        "dashboard.maintenance" => Input::Char('M'),
        "dashboard.favorites" => Input::Char('f'),
        "dashboard.terminals" => Input::Char('t'),
        "recipes.metadata" => Input::Enter,
        "recipes.dependencies" => Input::Char('A'),
        "recipes.build" => Input::Char('b'),
        "recipes.force_task" => Input::Char('f'),
        "recipes.signatures" => Input::Char('z'),
        "recipes.cve" => Input::Char('V'),
        "recipes.spdx" => Input::Char('X'),
        "recipes.devtool_modify" => Input::Char('d'),
        "recipes.devtool_gitui" => Input::Char('J'),
        "recipes.devtool_update" => Input::Char('u'),
        "recipes.devtool_finish" => Input::Char('F'),
        "recipes.devtool_deploy" => Input::Char('P'),
        "recipes.devtool_reset" => Input::Char('D'),
        "recipes.open" => Input::Enter,
        "layers.inventory" => Input::Char('r'),
        "layers.relationships" => Input::Char('R'),
        "layers.create" => Input::Char('c'),
        "layers.add" => Input::Char('a'),
        "layers.remove" => Input::Char('x'),
        "layers.open" => Input::Enter,
        "configuration.getvar" => Input::Char('r'),
        "configuration.inspect" => Input::Enter,
        "configuration.edit" => Input::Char('E'),
        "tasks.inventory" => Input::F2,
        "tasks.build" => Input::Char('B'),
        "tasks.cancel" => Input::Char('c'),
        "tasks.logs" => Input::Char('l'),
        "tasks.history" => Input::Char('h'),
        "build_history.inspect"
        | "errors.inspect"
        | "dependencies.open"
        | "packages.detail"
        | "raw.inspect" => Input::Enter,
        "logs.inspect" => Input::Char('/'),
        "dependencies.refresh" | "signatures.dump" | "devtool.status" => Input::Char('r'),
        "signatures.compare" => Input::Char('c'),
        "signatures.open" => Input::Char('e'),
        "packages.inventory" => Input::Char('R'),
        "packages.navigate" => Input::Char('d'),
        "packages.cancel" => Input::Char('c'),
        "images.build" => Input::Char('b'),
        "images.qemu" | "qemu_wic.qemu" => Input::Char('Q'),
        "images.console" => Input::Char('T'),
        "images.wic" | "qemu_wic.wic" => Input::Char('W'),
        "images.device_write" | "qemu_wic.write" => Input::Char('D'),
        "images.artifacts" | "sdk.artifacts" => Input::Char('R'),
        "images.rootfs" => Input::Char('p'),
        "images.cancel" | "qemu_wic.cancel" => Input::Char('x'),
        "kernel.refresh" => Input::Char('r'),
        "kernel.menuconfig" => Input::Char('m'),
        "kernel.view" => Input::Enter,
        "kernel.explore" => Input::Char('o'),
        "kernel.compile" => Input::Char('c'),
        "kernel.decompile" => Input::Char('d'),
        "firmware.refresh" => Input::Char('r'),
        "firmware.menuconfig" => Input::Char('m'),
        "firmware.view" => Input::Enter,
        "firmware.explore" => Input::Char('o'),
        "firmware.compile" => Input::Char('c'),
        "firmware.decompile" => Input::Char('d'),
        "sdk.standard" => Input::Char('s'),
        "sdk.extensible" => Input::Char('E'),
        "sdk.testsdk" => Input::Char('t'),
        "sdk.testsdkext" => Input::Char('T'),
        "sdk.publish" => Input::Char('P'),
        "sdk.native" => Input::Char('n'),
        "sdk.cancel" | "security.cancel" | "qa.cancel" => Input::Char('c'),
        "testing.oe_selftest"
        | "testing.bitbake_selftest"
        | "testing.testimage"
        | "testing.testsdk"
        | "testing.testsdkext"
        | "testing.ptest"
        | "qa.recipe"
        | "qa.layer" => Input::Char('r'),
        "testing.compare" => Input::Char('c'),
        "testing.import" | "security.reports" | "qa.reports" => Input::Char('I'),
        "testing.cancel" => Input::Char('x'),
        "security.cve" => Input::Char('V'),
        "security.spdx" => Input::Char('X'),
        "security.package_map" => Input::Char('M'),
        "devtool.edit" => Input::Char('e'),
        "devtool.gitui" => Input::Char('J'),
        "devtool.modify" => Input::Char('d'),
        "devtool.update" => Input::Char('u'),
        "devtool.finish" => Input::Char('F'),
        "devtool.deploy" | "devtool.undeploy" => Input::Char('P'),
        "devtool.reset" => Input::Char('D'),
        "devtool.upgrade" => Input::Char('U'),
        "maintenance.readiness" => Input::Char('c'),
        "maintenance.cleanup" => Input::Char('d'),
        "maintenance.prserv" => Input::Char('e'),
        "maintenance.locked" => Input::Char('l'),
        "maintenance.history" => Input::Char('h'),
        "maintenance.archive" => Input::Char('a'),
        "maintenance.cancel" => Input::Char('x'),
        "maintenance.evidence" => Input::Char('o'),
        "terminal.devshell" => Input::Char('s'),
        "terminal.menuconfig" => Input::Char('m'),
        "terminal.shell" => Input::Char('n'),
        "terminal.control" => Input::Char('o'),
        "terminal.cancel" => Input::Char('c'),
        _ => return None,
    };
    Some(input)
}

pub(crate) fn catalog_routed_action(action: &Action) -> bool {
    matches!(
        action,
        Action::Open(
            Screen::Dashboard
                | Screen::Layers
                | Screen::Recipes
                | Screen::Images
                | Screen::Tasks
                | Screen::Logs
                | Screen::Errors
                | Screen::Configuration
                | Screen::RawMode
                | Screen::Compatibility
                | Screen::Settings
                | Screen::Help
        ) | Action::OpenBuildOptions
            | Action::BeginSelectedRecipeBuild
            | Action::BeginBbmaskEdit
            | Action::OpenThemePicker
    )
}
