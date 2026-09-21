use super::*;

impl InteractiveRuntime {
    pub(super) fn handle_paste(&mut self, text: String) -> Result<()> {
        let runtime = self;
        if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::EnvironmentSetup(_))
        ) {
            let _ = update(
                &mut runtime.app,
                Action::EnvironmentSetup(yoctui_model::EnvironmentSetupAction::Insert(text)),
            );
            return Ok(());
        }
        if runtime.app.screen == Screen::TerminalSessions
            && runtime.app.active_dialog().is_none()
            && !runtime.app.menu.is_open()
            && !runtime.app.command_palette_open
        {
            let _ =
                compatibility_workspace_action(&mut runtime.app, Action::TerminalStagePaste(text));
            return Ok(());
        }
        if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::RecipeEditor(editor))
                if editor.focus == yoctui_model::RecipeEditorFocus::Document
                    && editor.document.editing
        ) {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::EditRecipeEditor(yoctui_model::PopupEditorCommand::PasteText {
                    text,
                    source: yoctui_model::TextAreaPasteSource::BracketedPaste,
                }),
            );
            return Ok(());
        }
        if active_popup_accepts_paste(&runtime.app) {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::EditActivePopup(yoctui_model::PopupEditorCommand::PasteText {
                    text,
                    source: yoctui_model::TextAreaPasteSource::BracketedPaste,
                }),
            );
            return Ok(());
        }
        if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::Security(yoctui_model::SecurityDialog::Import { editor, .. }))
                if editor.editing
        ) {
            for character in text.chars().filter(|character| !character.is_control()) {
                let _ = compatibility_workspace_action(
                    &mut runtime.app,
                    Action::EditActivePopup(yoctui_model::PopupEditorCommand::Insert(character)),
                );
            }
            return Ok(());
        }
        if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::Qa(yoctui_model::QaDialog::Import { editor, .. }))
                if editor.editing
        ) {
            for character in text.chars().filter(|character| !character.is_control()) {
                let _ = compatibility_workspace_action(
                    &mut runtime.app,
                    Action::EditActivePopup(yoctui_model::PopupEditorCommand::Insert(character)),
                );
            }
            return Ok(());
        }
        if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::Maintenance(dialog))
                if matches!(
                    dialog.as_ref(),
                    yoctui_model::MaintenanceDialog::ReadinessToml { editor, .. }
                        | yoctui_model::MaintenanceDialog::CleanupToml { editor, .. }
                        | yoctui_model::MaintenanceDialog::PrServiceToml { editor, .. }
                        | yoctui_model::MaintenanceDialog::LockedCacheToml { editor, .. }
                        | yoctui_model::MaintenanceDialog::BuildHistoryToml { editor, .. }
                        | yoctui_model::MaintenanceDialog::GitArchiveToml { editor, .. }
                        if editor.editing
                )
        ) {
            for character in text.chars().filter(|character| !character.is_control()) {
                let _ = compatibility_workspace_action(
                    &mut runtime.app,
                    Action::EditActivePopup(yoctui_model::PopupEditorCommand::Insert(character)),
                );
            }
            return Ok(());
        }
        let popup_action = match runtime.app.active_dialog() {
            Some(Dialog::BuildEnvironmentEditor(editor)) if editor.editing => {
                Some(Action::AppendBuildEnvironmentEditor as fn(char) -> Action)
            }
            Some(Dialog::BuildEnvironmentCloneEditor(editor)) if editor.editing => {
                Some(Action::AppendBuildEnvironmentCloneEditor as fn(char) -> Action)
            }
            Some(Dialog::ConfigEdit { editor, .. }) if editor.editing => {
                Some(Action::AppendConfigEdit as fn(char) -> Action)
            }
            Some(Dialog::BbmaskEdit(editor)) if editor.editing => {
                Some(Action::AppendBbmask as fn(char) -> Action)
            }
            Some(Dialog::BuildTarget { editor, .. }) if editor.editing => {
                Some(Action::AppendBuildTarget as fn(char) -> Action)
            }
            Some(Dialog::WicCreateTomlEditor { editor, .. }) if editor.editing => {
                Some(Action::AppendWicCreateTomlEditor as fn(char) -> Action)
            }
            Some(Dialog::SdkPublishTomlEditor(editor)) if editor.editing => {
                Some(Action::AppendSdkPublishTomlEditor as fn(char) -> Action)
            }
            Some(Dialog::SdkNativeTomlEditor(editor)) if editor.editing => {
                Some(Action::AppendSdkNativeTomlEditor as fn(char) -> Action)
            }
            Some(Dialog::TestLaunchTomlEditor { editor, .. }) if editor.editing => {
                Some(Action::AppendTestLaunchTomlEditor as fn(char) -> Action)
            }
            Some(Dialog::TestResultImportTomlEditor { editor, .. }) if editor.editing => {
                Some(Action::AppendTestResultImportTomlEditor as fn(char) -> Action)
            }
            Some(Dialog::TestComparisonTomlEditor { editor, .. }) if editor.editing => {
                Some(Action::AppendTestComparisonTomlEditor as fn(char) -> Action)
            }
            Some(Dialog::TestJunitTomlEditor { editor, .. }) if editor.editing => {
                Some(Action::AppendTestJunitTomlEditor as fn(char) -> Action)
            }
            _ => None,
        };
        if let Some(action) = popup_action {
            for character in text.chars().filter(|character| !character.is_control()) {
                let _ = compatibility_workspace_action(&mut runtime.app, action(character));
            }
        } else if runtime.app.screen == Screen::BuildEnvironment
            && runtime
                .app
                .build_environment_draft
                .as_ref()
                .is_some_and(|draft| draft.editing)
        {
            for character in text.chars() {
                let _ = compatibility_workspace_action(
                    &mut runtime.app,
                    Action::AppendBuildEnvironmentField(character),
                );
            }
        }
        Ok(())
    }
}
