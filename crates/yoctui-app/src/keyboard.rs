//! Keyboard.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Char(char),
    Esc,
    Enter,
    CtrlC,
    CtrlV,
    CtrlB,
    CtrlP,
    CtrlU,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    Tab,
    BackTab,
    CtrlS,
    Up,
    Down,
    Backspace,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

/// Resolve terminal-workbench control keys. `None` deliberately means the
/// input remains eligible for forwarding to the daemon PTY writer.
pub fn terminal_workspace_action(app: &yoctui_model::App, input: Input) -> Option<Action> {
    use yoctui_model::TerminalWorkbenchMode as Mode;
    match app.terminal.mode {
        Mode::Search => match input {
            Input::Enter | Input::Esc => Some(Action::TerminalFinishSearch),
            Input::Backspace => Some(Action::TerminalBackspaceSearch),
            Input::CtrlU => Some(Action::TerminalClearSearch),
            Input::Char(character) => Some(Action::TerminalAppendSearch(character)),
            _ => None,
        },
        Mode::Rename => match input {
            Input::Enter => Some(Action::TerminalConfirmRename),
            Input::Esc => Some(Action::TerminalCancelMode),
            Input::Backspace => Some(Action::TerminalBackspaceRename),
            Input::Char(character) => Some(Action::TerminalAppendRename(character)),
            _ => None,
        },
        Mode::PasteReview => match input {
            Input::Enter => Some(Action::TerminalConfirmPaste),
            Input::Esc => Some(Action::TerminalCancelMode),
            _ => None,
        },
        Mode::KillConfirmation => match input {
            Input::Enter => Some(Action::TerminalConfirmKill),
            Input::Esc => Some(Action::TerminalCancelMode),
            _ => None,
        },
        Mode::Help => match input {
            Input::Esc | Input::Char('?') => Some(Action::TerminalToggleHelp),
            _ => None,
        },
        Mode::Copy => match input {
            Input::Up | Input::Char('k') => Some(Action::TerminalMoveCopyRow { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::TerminalMoveCopyRow { delta: 1 }),
            Input::Enter | Input::Char('y') => Some(Action::TerminalCopyViewport),
            Input::Char('/') => Some(Action::TerminalBeginSearch),
            Input::Esc | Input::Char('v') => Some(Action::TerminalCancelMode),
            _ => None,
        },
        Mode::Live if app.selected_terminal_is_writer() => None,
        Mode::Live => match input {
            Input::Char('n') => Some(Action::TerminalCreateBuildShell),
            Input::Char('s') => Some(Action::TerminalCreateSelectedDevshell),
            Input::Char('m') => Some(Action::TerminalCreateSelectedMenuconfig),
            Input::Enter => Some(Action::TerminalTakeControl),
            Input::Char('/') => Some(Action::TerminalBeginSearch),
            Input::Char('v') => Some(Action::TerminalEnterCopyMode),
            Input::Char('r') => Some(Action::TerminalBeginRename),
            Input::Char('o') => Some(Action::TerminalTakeControl),
            Input::Char('O') => Some(Action::TerminalReleaseControl),
            Input::Char('c') => Some(Action::TerminalReleaseControl),
            Input::Char('x') => Some(Action::TerminalBeginKill),
            Input::Char('?') => Some(Action::TerminalToggleHelp),
            Input::PageUp => Some(Action::TerminalScroll { delta: 10 }),
            Input::PageDown => Some(Action::TerminalScroll { delta: -10 }),
            Input::Home => Some(Action::TerminalScroll { delta: isize::MAX }),
            Input::End => Some(Action::TerminalScroll {
                delta: isize::MIN + 1,
            }),
            _ => None,
        },
    }
}

/// Resolve an explicitly activated Terminal Sessions context-menu item. This
/// separate route prevents ordinary writer keystrokes from becoming controls.
pub fn terminal_context_action(input: Input) -> Option<Action> {
    match input {
        Input::Char('n') => Some(Action::TerminalCreateBuildShell),
        Input::Char('s') => Some(Action::TerminalCreateSelectedDevshell),
        Input::Char('m') => Some(Action::TerminalCreateSelectedMenuconfig),
        Input::Char('o') | Input::Enter => Some(Action::TerminalTakeControl),
        Input::Char('c') => Some(Action::TerminalReleaseControl),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxInputAction {
    Toggle,
    Move(isize),
    Primary,
}

pub fn checkbox_input_action(input: Input) -> Option<CheckboxInputAction> {
    match input {
        Input::Char(' ') => Some(CheckboxInputAction::Toggle),
        Input::Up | Input::Char('k') => Some(CheckboxInputAction::Move(-1)),
        Input::Down | Input::Char('j') => Some(CheckboxInputAction::Move(1)),
        Input::Enter => Some(CheckboxInputAction::Primary),
        _ => None,
    }
}

/// Route the controls advertised by a visible guidance/failure popup before
/// screen-local Enter or Esc handling can consume them.
pub fn notification_popup_action(app: &App, input: Input) -> Option<Action> {
    let message = app.notification.as_deref()?;
    if !yoctui_model::notification_requires_acknowledgement(message)
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.onboarding.open
        || app.keymap_preferences_ui.open
        || app.command_palette_open
        || (app.screen == Screen::RawMode
            && matches!(
                app.raw_mode.view,
                yoctui_model::RawModeView::Form | yoctui_model::RawModeView::Preview
            ))
    {
        return None;
    }
    match input {
        Input::Enter => Some(Action::ActivateNotification),
        Input::Esc => Some(Action::DismissNotification),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapInputResult {
    Action(Box<Action>),
    Pending,
    Unmatched,
}

impl KeymapInputResult {
    pub fn action(self) -> Option<Action> {
        match self {
            Self::Action(action) => Some(*action),
            Self::Pending | Self::Unmatched => None,
        }
    }
}

pub fn keymap_action_for_app(app: &mut yoctui_model::App, key: Input) -> KeymapInputResult {
    if app.onboarding.open
        || app.keymap_preferences_ui.open
        || app.menu.is_open()
        || app.active_dialog().is_some()
        || app.command_palette_open
        || matches!(app.focus, FocusTarget::Dialog | FocusTarget::CommandPalette)
    {
        app.keymap_chord.clear();
        return KeymapInputResult::Unmatched;
    }
    let stroke = input_key_stroke(key);
    let workspace = yoctui_model::workspace_screen_destination(app.screen);
    match app
        .effective_keymap
        .resolve_input(&mut app.keymap_chord, workspace, stroke)
    {
        yoctui_model::KeymapResolution::Pending => KeymapInputResult::Pending,
        yoctui_model::KeymapResolution::Unmatched => key_action(key)
            .filter(|action| !catalog_routed_action(action))
            .map(Box::new)
            .map_or(KeymapInputResult::Unmatched, KeymapInputResult::Action),
        yoctui_model::KeymapResolution::Activated(action_id) => {
            let definition = yoctui_model::global_operator_action_definitions()
                .into_iter()
                .find(|definition| definition.id == action_id)
                .expect("effective keymaps retain catalog action IDs");
            let yoctui_model::OperatorActionTarget::Command(command) = definition.target else {
                unreachable!("the current effective keymap contains command targets")
            };
            KeymapInputResult::Action(Box::new(yoctui_model::command_action(app, command)))
        }
    }
}

/// Route menuconfig-style global search without stealing `/` from an active
/// text editor, contextual search, or daemon-owned terminal session.
pub fn global_search_action(app: &yoctui_model::App, key: Input) -> Option<Action> {
    if key != Input::Char('/')
        || app.onboarding.open
        || app.keymap_preferences_ui.open
        || app.menu.is_open()
        || app.active_dialog().is_some()
        || app.command_palette_open
        || matches!(app.focus, FocusTarget::Dialog | FocusTarget::CommandPalette)
        || app.screen == yoctui_model::Screen::TerminalSessions
        || workspace_text_input_active(app)
    {
        None
    } else {
        Some(Action::OpenGlobalSearch)
    }
}

/// Screen-local editors and searches retain text, Escape and navigation keys.
pub fn workspace_text_input_active(app: &yoctui_model::App) -> bool {
    (app.screen == yoctui_model::Screen::BuildEnvironment
        && app
            .build_environment_draft
            .as_ref()
            .is_some_and(|draft| draft.editing))
        || (app.screen == yoctui_model::Screen::Tasks && app.task_filter_editing)
        || (app.screen == yoctui_model::Screen::Logs
            && (app.logs.searching || app.internal_logs.searching))
        || (matches!(
            app.screen,
            yoctui_model::Screen::Recipes
                | yoctui_model::Screen::Layers
                | yoctui_model::Screen::Configuration
        ) && app.metadata_searching)
        || (app.screen == yoctui_model::Screen::Dependencies && app.dependency_graph_searching)
        || (app.screen == yoctui_model::Screen::Packages && app.package_searching)
        || (app.screen == yoctui_model::Screen::Images && app.image_artifact_searching)
        || (app.screen == yoctui_model::Screen::Sdk && app.sdk_artifact_searching)
        || (app.screen == yoctui_model::Screen::Testing && app.test_result_searching)
        || (app.screen == yoctui_model::Screen::Security && app.security.searching)
        || (app.screen == yoctui_model::Screen::Qa && app.qa.searching)
        || (app.screen == yoctui_model::Screen::Compatibility && app.compatibility_ui.searching)
        || (app.screen == yoctui_model::Screen::RawMode
            && (app.raw_mode.search.editing || app.raw_mode.output.searching))
        || (app.screen == yoctui_model::Screen::RawMode
            && matches!(
                app.raw_mode.view,
                yoctui_model::RawModeView::Form | yoctui_model::RawModeView::Preview
            ))
}

pub fn terminal_owns_input(app: &yoctui_model::App) -> bool {
    app.screen == yoctui_model::Screen::TerminalSessions
        && app.focus == FocusTarget::Workspace
        && (app.selected_terminal_is_writer()
            || app.terminal.mode != yoctui_model::TerminalWorkbenchMode::Live)
}

pub fn input_key_stroke(key: Input) -> yoctui_model::KeyStroke {
    use yoctui_model::KeyStroke as Stroke;
    match key {
        Input::Char(character) => Stroke::Char(character),
        Input::Esc => Stroke::Esc,
        Input::Enter => Stroke::Enter,
        Input::CtrlC => Stroke::CtrlC,
        Input::CtrlV => Stroke::CtrlV,
        Input::CtrlB => Stroke::CtrlB,
        Input::CtrlP => Stroke::CtrlP,
        Input::CtrlU => Stroke::CtrlU,
        Input::F1 => Stroke::F1,
        Input::F2 => Stroke::F2,
        Input::F3 => Stroke::F3,
        Input::F4 => Stroke::F4,
        Input::F5 => Stroke::F5,
        Input::F6 => Stroke::F6,
        Input::F7 => Stroke::F7,
        Input::F8 => Stroke::F8,
        Input::F9 => Stroke::F9,
        Input::F10 => Stroke::F10,
        Input::Tab => Stroke::Tab,
        Input::BackTab => Stroke::BackTab,
        Input::CtrlS => Stroke::CtrlS,
        Input::Up => Stroke::Up,
        Input::Down => Stroke::Down,
        Input::Backspace => Stroke::Backspace,
        Input::Left => Stroke::Left,
        Input::Right => Stroke::Right,
        Input::Home => Stroke::Home,
        Input::End => Stroke::End,
        Input::PageUp => Stroke::PageUp,
        Input::PageDown => Stroke::PageDown,
    }
}

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
        Input::Esc | Input::F10 => reduce(Action::CloseMenu),
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
