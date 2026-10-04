//! Focused typed paste mapping. Payload characters can never become shortcuts.
use yoctui_model::{
    Action, App, Dialog, FocusTarget, KernelDebugAction, KernelDebugField, PopupEditorCommand,
    RecipeEditorFocus, Screen, TerminalWorkbenchMode, TextAreaPasteSource,
};

enum Target {
    Popup,
    FieldPopup,
    Recipe,
    Kernel,
    Setup,
    Raw(Box<yoctui_model::RawModeAction>),
    Single(fn(char) -> Action),
}

pub fn popup_accepts_text_paste(app: &App) -> bool {
    match app.active_dialog() {
        Some(
            Dialog::BuildEnvironmentEditor(editor)
            | Dialog::BuildEnvironmentCloneEditor(editor)
            | Dialog::BbmaskEdit(editor)
            | Dialog::SdkPublishTomlEditor(editor)
            | Dialog::SdkNativeTomlEditor(editor),
        ) => editor.editing,
        Some(
            Dialog::ConfigEdit { editor, .. }
            | Dialog::BuildTarget { editor, .. }
            | Dialog::WicCreateTomlEditor { editor, .. }
            | Dialog::TestLaunchTomlEditor { editor, .. }
            | Dialog::TestResultImportTomlEditor { editor, .. }
            | Dialog::TestComparisonTomlEditor { editor, .. }
            | Dialog::TestJunitTomlEditor { editor, .. },
        ) => editor.editing,
        Some(Dialog::Security(yoctui_model::SecurityDialog::Import { editor, .. }))
        | Some(Dialog::Qa(yoctui_model::QaDialog::Import { editor, .. })) => editor.editing,
        Some(Dialog::Maintenance(dialog)) => match dialog.as_ref() {
            yoctui_model::MaintenanceDialog::ReadinessToml { editor, .. }
            | yoctui_model::MaintenanceDialog::CleanupToml { editor, .. }
            | yoctui_model::MaintenanceDialog::PrServiceToml { editor, .. }
            | yoctui_model::MaintenanceDialog::LockedCacheToml { editor, .. }
            | yoctui_model::MaintenanceDialog::BuildHistoryToml { editor, .. }
            | yoctui_model::MaintenanceDialog::GitArchiveToml { editor, .. } => editor.editing,
            _ => false,
        },
        _ => false,
    }
}

fn target(app: &App) -> Option<Target> {
    if app.menu.is_open()
        || app.onboarding.open
        || app.saved_builds.environment.loading
        || app.terminal.mode == TerminalWorkbenchMode::KillConfirmation
    {
        return None;
    }
    if popup_accepts_text_paste(app) {
        return Some(
            if matches!(
                app.active_dialog(),
                Some(
                    Dialog::ConfigEdit { .. } | Dialog::BuildTarget { .. } | Dialog::BbmaskEdit(_)
                )
            ) {
                Target::FieldPopup
            } else {
                Target::Popup
            },
        );
    }
    match app.active_dialog() {
        Some(Dialog::KernelDebug(dialog))
            if app.kernel_debug.pending.is_none()
                && (dialog.draft.tool.program().is_some()
                    || dialog.draft.tool.configuration_prep()) =>
        {
            let field = dialog.draft.fields().get(dialog.selection).copied()?;
            return (!matches!(
                field,
                KernelDebugField::Destination
                    | KernelDebugField::QemuBootMode
                    | KernelDebugField::InstrumentationPreset
            ))
            .then_some(Target::Kernel);
        }
        Some(Dialog::RecipeEditor(editor)) if editor.searching => {
            return Some(Target::Single(Action::AppendRecipeEditorSearch));
        }
        Some(Dialog::RecipeEditor(editor))
            if editor.focus == RecipeEditorFocus::Document && editor.document.editing =>
        {
            return Some(Target::Recipe);
        }
        Some(Dialog::EnvironmentSetup(setup)) if setup.editor.is_some() => {
            return Some(Target::Setup);
        }
        Some(Dialog::QemuLaunch(dialog)) if dialog.editing => {
            return Some(Target::Single(Action::AppendQemuLaunchField));
        }
        Some(Dialog::WicCreate(dialog)) if dialog.editing => {
            return Some(Target::Single(Action::AppendWicCreateField));
        }
        Some(_) => return None,
        None => {}
    }
    if app.command_palette_open {
        return Some(Target::Single(Action::AppendCommandPaletteQuery));
    }
    if app.screen == Screen::RawMode
        && matches!(app.focus, FocusTarget::Workspace | FocusTarget::Dialog)
        && let Some(
            action @ (yoctui_model::RawModeAction::EditParameterInput { .. }
            | yoctui_model::RawModeAction::EditAdditionalArguments(_)),
        ) = crate::raw_mode_input(app, crate::Input::CtrlV)
    {
        return Some(Target::Raw(Box::new(action)));
    }
    if app.focus != FocusTarget::Workspace {
        return None;
    }
    if app.screen == Screen::Hardware
        && app.hardware.projects.visible
        && matches!(
            app.hardware.projects.form,
            Some(yoctui_model::HardwareProjectForm::Name { .. })
        )
    {
        return Some(Target::Single(|c| {
            Action::Hardware(yoctui_model::HardwareAction::Project(
                yoctui_model::HardwareProjectAction::EditName(c),
            ))
        }));
    }
    if app.screen == Screen::BuildEnvironment
        && app
            .build_environment_draft
            .as_ref()
            .is_some_and(|draft| draft.editing)
    {
        return Some(Target::Single(Action::AppendBuildEnvironmentField));
    }
    let action: fn(char) -> Action = match app.screen {
        Screen::Tasks if app.task_filter_editing => Action::AppendTaskFilter,
        Screen::Logs if app.internal_logs.searching => Action::AppendInternalLogQuery,
        Screen::Logs if app.logs.searching => Action::AppendLogQuery,
        Screen::Recipes | Screen::Layers | Screen::Configuration | Screen::Images
            if app.metadata_searching =>
        {
            Action::AppendMetadataQuery
        }
        Screen::Dependencies if app.dependency_graph_searching => {
            Action::AppendDependencyGraphQuery
        }
        Screen::Packages if app.package_searching => Action::AppendPackageQuery,
        Screen::Images if app.image_artifact_searching => Action::AppendImageArtifactQuery,
        Screen::Sdk if app.sdk_artifact_searching => Action::AppendSdkArtifactQuery,
        Screen::Testing if app.test_result_searching => Action::AppendTestResultQuery,
        Screen::Compatibility if app.compatibility_ui.searching => Action::AppendCompatibilityQuery,
        Screen::Security if app.security.searching => {
            |c| Action::Security(yoctui_model::SecurityAction::AppendQuery(c))
        }
        Screen::Qa if app.qa.searching => |c| Action::Qa(yoctui_model::QaAction::AppendQuery(c)),
        Screen::TerminalSessions if app.terminal.mode == TerminalWorkbenchMode::Search => {
            Action::TerminalAppendSearch
        }
        Screen::TerminalSessions if app.terminal.mode == TerminalWorkbenchMode::Rename => {
            Action::TerminalAppendRename
        }
        _ => return None,
    };
    Some(Target::Single(action))
}

pub fn text_paste_active(app: &App) -> bool {
    target(app).is_some()
}

pub fn text_paste_actions(
    app: &App,
    text: String,
    source: TextAreaPasteSource,
) -> Result<Option<Vec<Action>>, String> {
    let Some(target) = target(app) else {
        return Ok(None);
    };
    if text.len() > yoctui_model::TEXTAREA_MAX_PASTE_BYTES {
        return Err("Paste exceeds the 256 KiB limit.".into());
    }
    let actions = match target {
        Target::Popup => vec![Action::EditActivePopup(PopupEditorCommand::PasteText {
            text,
            source,
        })],
        Target::Recipe => vec![Action::EditRecipeEditor(PopupEditorCommand::PasteText {
            text,
            source,
        })],
        Target::Raw(action) => {
            let command = PopupEditorCommand::PasteText { text, source };
            vec![Action::RawMode(match *action {
                yoctui_model::RawModeAction::EditParameterInput { parameter, .. } => {
                    yoctui_model::RawModeAction::EditParameterInput { parameter, command }
                }
                yoctui_model::RawModeAction::EditAdditionalArguments(_) => {
                    yoctui_model::RawModeAction::EditAdditionalArguments(command)
                }
                _ => unreachable!(),
            })]
        }
        target => {
            if text.chars().any(char::is_control)
                || text.len() > yoctui_model::MAX_KERNEL_DEBUG_FIELD_BYTES
            {
                return Err("Single-line paste must be at most 4096 bytes and contain no control characters/newlines.".into());
            }
            match target {
                Target::Kernel => vec![Action::KernelDebug(KernelDebugAction::Insert(text))],
                Target::Setup => vec![Action::EnvironmentSetup(
                    yoctui_model::EnvironmentSetupAction::Insert(text),
                )],
                Target::FieldPopup => {
                    vec![Action::EditActivePopup(PopupEditorCommand::PasteText {
                        text,
                        source,
                    })]
                }
                Target::Single(action) => text.chars().map(action).collect(),
                _ => unreachable!(),
            }
        }
    };
    Ok(Some(actions))
}
