//! Editor types.
use super::*;

pub type PopupEditor = TextAreaState;

impl TextAreaState {
    pub fn select_toml_value(&mut self, key: &str) -> Result<(), String> {
        let prefix = format!("{key} = ");
        let line_start = self
            .text
            .lines()
            .scan(0usize, |offset, line| {
                let start = *offset;
                *offset += line.len() + 1;
                Some((start, line))
            })
            .find_map(|(start, line)| line.trim_start().starts_with(&prefix).then_some(start))
            .ok_or_else(|| format!("Missing `{key}` TOML value."))?;
        let line_end = self.text[line_start..]
            .find('\n')
            .map_or(self.text.len(), |index| line_start + index);
        let line = &self.text[line_start..line_end];
        let (value_start, value_end) = popup_toml_value_range(line, line_start)?;
        self.select_range(value_start, value_end);
        Ok(())
    }
    pub fn select_toml_value_at_cursor(&mut self) -> Result<(), String> {
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let line_end = self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |index| self.cursor + index);
        let line = &self.text[line_start..line_end];
        let (value_start, value_end) = popup_toml_value_range(line, line_start)?;
        self.select_range(value_start, value_end);
        Ok(())
    }
}

pub(crate) fn popup_toml_value_range(
    line: &str,
    line_start: usize,
) -> Result<(usize, usize), String> {
    let equals = line
        .find('=')
        .ok_or_else(|| "The current TOML line has no value.".to_owned())?;
    let raw = &line[equals + 1..];
    let leading = raw.len() - raw.trim_start().len();
    let value = raw.trim_start();
    let value_start = line_start + equals + 1 + leading;
    if let Some(quoted) = value.strip_prefix('"') {
        let end = quoted
            .find('"')
            .ok_or_else(|| "The current TOML line has no closing quote.".to_owned())?;
        return Ok((value_start + 1, value_start + 1 + end));
    }
    let value = value.split_once('#').map_or(value, |(value, _)| value);
    let value = value.trim_end();
    if value.is_empty() {
        return Err("The current TOML line has no value.".into());
    }
    Ok((value_start, value_start + value.len()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupEditorCommand {
    ToggleInsert,
    ToggleVisual,
    Insert(char),
    Newline,
    Backspace,
    Delete,
    Left,
    Right,
    WordLeft,
    WordRight,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Undo,
    Redo,
    SelectPosition {
        line: usize,
        column: usize,
        extend: bool,
    },
    PasteText {
        text: String,
        source: TextAreaPasteSource,
    },
    SelectValue,
    Copy,
    Paste,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    EnvironmentSetup(Box<EnvironmentSetup>),
    BuildEnvironmentCloneEditor(PopupEditor),
    BuildEnvironmentCloneReview(BuildEnvironmentClonePlan),
    BuildEnvironmentEditor(PopupEditor),
    ThemePicker {
        selection: usize,
        original_theme: Theme,
        original_color_enabled: bool,
        original_settings_dirty: bool,
    },
    BuildOptions,
    BuildCompletion,
    BuildTarget {
        editor: PopupEditor,
        task: Option<String>,
    },
    ImagePicker(ImagePicker),
    ImageConsole(ImageConsoleDialog),
    QemuLaunch(QemuLaunchDialog),
    QemuLaunchConfirmation(QemuLaunchPreview),
    QemuCancellationConfirmation(QemuSessionId),
    WicCreate(WicCreateDialog),
    WicCreateTomlEditor {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    WicCreateConfirmation(WicCreatePreview),
    WicDevicePicker(WicDevicePickerDialog),
    WicWritePhrase(WicWritePhraseDialog),
    WicWriteConfirmation(WicWritePreview),
    WicCancellationConfirmation {
        id: WicSessionId,
        incomplete_device_warning: bool,
    },
    SdkBuildConfirmation(SdkBuildPreview),
    SdkPublish(SdkPublishDraft),
    SdkPublishTomlEditor(PopupEditor),
    SdkPublishConfirmation(SdkPublishPreview),
    SdkNative(SdkNativeDialog),
    SdkNativeTomlEditor(PopupEditor),
    SdkNativeConfirmation(SdkNativePreview),
    SdkCancellationConfirmation(SdkSessionId),
    TestLaunch(TestLaunchDialog),
    TestLaunchTomlEditor {
        family: TestFamily,
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    TestLaunchConfirmation(TestLaunchPreview),
    TestCancellationConfirmation(TestSessionId),
    TestResultImport(TestResultImportDialog),
    TestResultImportTomlEditor {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    TestComparison(TestComparisonPicker),
    TestComparisonTomlEditor {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    TestComparisonConfirmation(TestComparisonPreview),
    TestJunitExport(TestJunitExportDialog),
    TestJunitTomlEditor {
        result: TestResultIdentity,
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    TestJunitExportConfirmation(TestJunitExportPreview),
    Security(SecurityDialog),
    Qa(QaDialog),
    Maintenance(Box<MaintenanceDialog>),
    RecipeTaskConfirmation(BuildRequest),
    RecipeTaskPicker(RecipeTaskPicker),
    SignatureTaskPicker(SignatureTaskPicker),
    RecipeTaskLogPicker(RecipeTaskLogPicker),
    RecipePatchPicker(RecipePatchPicker),
    ConfigSourcePicker(ConfigSourcePicker),
    ConfigScopePicker(ConfigScopePicker),
    ConfigComparison(ConfigComparison),
    ConfigEdit {
        identity: VariableIdentity,
        editor: PopupEditor,
    },
    ConfigEditConfirmation(ConfigEditRequest),
    DevtoolModifyConfirmation(RecipeIdentity),
    DevtoolResetConfirmation(DevtoolResetPlan),
    DevtoolUpdateConfirmation(RecipeIdentity),
    DevtoolFinishPicker(DevtoolFinishPicker),
    DevtoolFinishConfirmation(DevtoolFinishPlan),
    DevtoolDeploy(DevtoolDeployDraft),
    DevtoolDeployConfirmation(DevtoolDeployPlan),
    BbmaskEdit(PopupEditor),
    BbmaskConfirmation(String),
    TerminalLaunch(TerminalLaunchDialog),
    RecipeEditor(RecipeEditor),
    BuildCancellationConfirmation,
    QuitConfirmation,
}

impl Dialog {
    pub fn is_confirmation(&self) -> bool {
        match self {
            Self::BuildEnvironmentCloneReview(_)
            | Self::BuildCompletion
            | Self::QemuLaunchConfirmation(_)
            | Self::QemuCancellationConfirmation(_)
            | Self::WicCreateConfirmation(_)
            | Self::WicWritePhrase(_)
            | Self::WicWriteConfirmation(_)
            | Self::WicCancellationConfirmation { .. }
            | Self::SdkBuildConfirmation(_)
            | Self::SdkPublishConfirmation(_)
            | Self::SdkNativeConfirmation(_)
            | Self::SdkCancellationConfirmation(_)
            | Self::TestLaunchConfirmation(_)
            | Self::TestCancellationConfirmation(_)
            | Self::TestComparisonConfirmation(_)
            | Self::TestJunitExportConfirmation(_)
            | Self::RecipeTaskConfirmation(_)
            | Self::ConfigEditConfirmation(_)
            | Self::DevtoolModifyConfirmation(_)
            | Self::DevtoolResetConfirmation(_)
            | Self::DevtoolUpdateConfirmation(_)
            | Self::DevtoolFinishConfirmation(_)
            | Self::DevtoolDeployConfirmation(_)
            | Self::BbmaskConfirmation(_)
            | Self::BuildCancellationConfirmation
            | Self::QuitConfirmation => true,
            Self::Security(dialog) => matches!(
                dialog,
                SecurityDialog::Operation(_) | SecurityDialog::Cancellation(_)
            ),
            Self::Qa(dialog) => !matches!(dialog, QaDialog::Import { .. }),
            Self::Maintenance(dialog) => matches!(
                dialog.as_ref(),
                MaintenanceDialog::Confirm(_)
                    | MaintenanceDialog::CleanupPhrase { .. }
                    | MaintenanceDialog::ConfirmNetworkPush(_)
                    | MaintenanceDialog::ConfirmCancellation(_)
            ),
            Self::BuildEnvironmentCloneEditor(_)
            | Self::EnvironmentSetup(_)
            | Self::BuildEnvironmentEditor(_)
            | Self::ThemePicker { .. }
            | Self::BuildOptions
            | Self::BuildTarget { .. }
            | Self::ImagePicker(_)
            | Self::ImageConsole(_)
            | Self::QemuLaunch(_)
            | Self::WicCreate(_)
            | Self::WicCreateTomlEditor { .. }
            | Self::WicDevicePicker(_)
            | Self::SdkPublish(_)
            | Self::SdkPublishTomlEditor(_)
            | Self::SdkNative(_)
            | Self::SdkNativeTomlEditor(_)
            | Self::TestLaunch(_)
            | Self::TestLaunchTomlEditor { .. }
            | Self::TestResultImport(_)
            | Self::TestResultImportTomlEditor { .. }
            | Self::TestComparison(_)
            | Self::TestComparisonTomlEditor { .. }
            | Self::TestJunitExport(_)
            | Self::TestJunitTomlEditor { .. }
            | Self::RecipeTaskPicker(_)
            | Self::SignatureTaskPicker(_)
            | Self::RecipeTaskLogPicker(_)
            | Self::RecipePatchPicker(_)
            | Self::ConfigSourcePicker(_)
            | Self::ConfigScopePicker(_)
            | Self::ConfigComparison(_)
            | Self::ConfigEdit { .. }
            | Self::DevtoolFinishPicker(_)
            | Self::DevtoolDeploy(_)
            | Self::BbmaskEdit(_)
            | Self::TerminalLaunch(_)
            | Self::RecipeEditor(_) => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransientStatusKind {
    Error,
    Confirmation,
    Success,
    Warning,
    Notification,
    Reconnecting,
    Activity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransientStatus {
    pub kind: TransientStatusKind,
    pub text: String,
}

impl RecipeEditor {
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.files
            .get(self.selection)
            .map(|path| self.root.join(path))
    }

    pub fn is_dirty(&self) -> bool {
        self.document.is_modified()
    }

    pub fn refresh_language_and_validation(&mut self) {
        self.language = self
            .files
            .get(self.selection)
            .map_or(SourceLanguage::PlainText, |path| {
                SourceLanguage::from_path(path)
            });
        self.document.set_validation(source_structural_validation(
            self.language,
            &self.document.text,
        ));
    }

    pub fn local_validation(&self) -> Vec<String> {
        self.document
            .validation()
            .iter()
            .map(|span| span.message.clone())
            .collect()
    }

    pub fn diff_preview(&self, maximum_lines: usize) -> Vec<String> {
        if !self.document.has_visual_diff() || maximum_lines == 0 {
            return Vec::new();
        }
        let mut document = self.document.clone();
        let mut preview = document
            .preview_diff()
            .lines
            .iter()
            .filter(|line| line.kind != TextAreaDiffKind::Context)
            .map(|line| {
                let marker = match line.kind {
                    TextAreaDiffKind::Removed => '-',
                    TextAreaDiffKind::Added => '+',
                    TextAreaDiffKind::Context => ' ',
                };
                let number = line.new_line.or(line.old_line).map_or(0, |line| line + 1);
                format!("{marker} {number:>3} {}", line.text)
            })
            .collect::<Vec<_>>();
        preview.truncate(maximum_lines);
        preview
    }
}

pub(crate) fn source_structural_validation(
    language: SourceLanguage,
    text: &str,
) -> Vec<TextAreaValidationSpan> {
    let mut diagnostics = Vec::new();
    if language == SourceLanguage::BitBake {
        let mut offset = 0usize;
        for (line, value) in text.lines().enumerate() {
            if value.trim_end().ends_with('=') {
                diagnostics.push(TextAreaValidationSpan {
                    start: offset,
                    end: offset + value.len(),
                    severity: TextAreaValidationSeverity::Error,
                    message: format!("line {}: assignment has no value", line + 1),
                });
            }
            offset = offset.saturating_add(value.len() + 1);
        }
    }

    if !matches!(
        language,
        SourceLanguage::PlainText | SourceLanguage::Markdown | SourceLanguage::Yaml
    ) {
        for (open, close, label) in [
            ('{', '}', "braces"),
            ('(', ')', "parentheses"),
            ('[', ']', "brackets"),
        ] {
            let opens = text.chars().filter(|character| *character == open).count();
            let closes = text.chars().filter(|character| *character == close).count();
            if opens != closes {
                diagnostics.push(TextAreaValidationSpan {
                    start: 0,
                    end: text.len().min(1),
                    severity: TextAreaValidationSeverity::Warning,
                    message: format!(
                        "structural {label} mismatch: {opens} opening / {closes} closing"
                    ),
                });
            }
        }
    }
    diagnostics
}

pub(crate) fn apply_recipe_editor_command(
    editor: &mut TextAreaState,
    command: PopupEditorCommand,
) -> Result<Option<String>, String> {
    match command {
        PopupEditorCommand::ToggleInsert => editor.toggle_insert(),
        PopupEditorCommand::ToggleVisual => {
            let mode = if editor.mode() == TextAreaMode::Visual {
                TextAreaMode::Normal
            } else {
                TextAreaMode::Visual
            };
            editor.set_mode(mode);
        }
        PopupEditorCommand::Insert(character) if editor.editing && !character.is_control() => {
            editor
                .try_insert(&character.to_string())
                .map_err(|error| format!("Editor input rejected: {error:?}"))?
        }
        PopupEditorCommand::Insert(_) => {}
        PopupEditorCommand::Newline if editor.editing => editor
            .try_insert("\n")
            .map_err(|error| format!("Editor input rejected: {error:?}"))?,
        PopupEditorCommand::Newline => {}
        PopupEditorCommand::Backspace if editor.editing => editor.backspace(),
        PopupEditorCommand::Backspace => {}
        PopupEditorCommand::Delete => editor.delete_forward(),
        PopupEditorCommand::Left => editor.left(),
        PopupEditorCommand::Right => editor.right(),
        PopupEditorCommand::WordLeft => editor.move_cursor(TextAreaMotion::WordLeft),
        PopupEditorCommand::WordRight => editor.move_cursor(TextAreaMotion::WordRight),
        PopupEditorCommand::Up => editor.up(),
        PopupEditorCommand::Down => editor.down(),
        PopupEditorCommand::Home => editor.home(),
        PopupEditorCommand::End => editor.end(),
        PopupEditorCommand::PageUp => editor.move_cursor(TextAreaMotion::PageUp),
        PopupEditorCommand::PageDown => editor.move_cursor(TextAreaMotion::PageDown),
        PopupEditorCommand::Undo => {
            editor.undo();
        }
        PopupEditorCommand::Redo => {
            editor.redo();
        }
        PopupEditorCommand::SelectPosition {
            line,
            column,
            extend,
        } => editor.select_position(line, column, extend),
        PopupEditorCommand::PasteText { text, source } if editor.editing => editor
            .paste_text(&text, source)
            .map_err(|error| format!("Editor paste rejected: {error:?}"))?,
        PopupEditorCommand::PasteText { .. } => {}
        PopupEditorCommand::Copy => return Ok(Some(editor.copy_selection_or_line())),
        PopupEditorCommand::Paste if editor.editing => editor
            .paste_internal_clipboard()
            .map_err(|error| format!("Editor paste rejected: {error:?}"))?,
        PopupEditorCommand::Paste | PopupEditorCommand::SelectValue => {}
    }
    Ok(None)
}
