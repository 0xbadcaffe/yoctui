pub fn security_workspace_action(
    view: SecurityView,
    drilled: bool,
    searching: bool,
    key: Input,
) -> Option<Action> {
    let security = |action| Some(Action::Security(action));
    if searching {
        return match key {
            Input::Char(character) => security(SecurityAction::AppendQuery(character)),
            Input::Backspace => security(SecurityAction::BackspaceQuery),
            Input::CtrlU => security(SecurityAction::ClearQuery),
            Input::Enter | Input::Esc => security(SecurityAction::FinishSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return security(if view == SecurityView::Cves {
            SecurityAction::SelectFinding(delta)
        } else if drilled {
            SecurityAction::SelectComponent(delta)
        } else {
            SecurityAction::SelectReport(delta)
        });
    }
    match key {
        Input::Tab => security(SecurityAction::CycleView),
        Input::Up | Input::Char('k') => security(if view == SecurityView::Cves {
            SecurityAction::SelectFinding(-1)
        } else if drilled {
            SecurityAction::SelectComponent(-1)
        } else {
            SecurityAction::SelectReport(-1)
        }),
        Input::Down | Input::Char('j') => security(if view == SecurityView::Cves {
            SecurityAction::SelectFinding(1)
        } else if drilled {
            SecurityAction::SelectComponent(1)
        } else {
            SecurityAction::SelectReport(1)
        }),
        Input::Enter => security(SecurityAction::Drill),
        Input::Esc if drilled => security(SecurityAction::LeaveDrill),
        Input::Char('s') => security(SecurityAction::CycleScope),
        Input::Char('/') => security(SecurityAction::BeginSearch),
        Input::CtrlU => security(SecurityAction::ClearQuery),
        Input::Char('f') => security(SecurityAction::CycleCveFilter),
        Input::Char('V') => security(SecurityAction::BeginCveCheck),
        Input::Char('M') => security(SecurityAction::BeginPackageMap),
        Input::Char('X') => security(SecurityAction::BeginSbomGeneration),
        Input::Char('I') => security(SecurityAction::BeginImport),
        Input::Char('R') => security(SecurityAction::RefreshReports),
        Input::Char('o') => security(SecurityAction::OpenSelectedReport),
        Input::Char('e') => security(SecurityAction::OpenSelectedRecipe),
        Input::Char('v') => security(SecurityAction::OpenSelectedAdvisory),
        Input::Char('c') => security(SecurityAction::BeginCancellation),
        _ => None,
    }
}

pub fn security_dialog_action(dialog: &SecurityDialog, key: Input) -> Option<Action> {
    let security = |action| Some(Action::Security(action));
    match dialog {
        SecurityDialog::Operation(preview) => match key {
            Input::Enter => security(SecurityAction::ConfirmOperation(preview.clone())),
            Input::Esc => security(SecurityAction::CancelDialog),
            _ => None,
        },
        SecurityDialog::Cancellation(id) => match key {
            Input::Enter => security(SecurityAction::ConfirmCancellation(*id)),
            Input::Esc => security(SecurityAction::CancelDialog),
            _ => None,
        },
        SecurityDialog::Import { editor, .. } => match key {
            Input::Enter => security(SecurityAction::ConfirmImport(editor.text.clone())),
            Input::Char('q') | Input::Esc if !editor.editing => {
                security(SecurityAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
    }
}

pub fn qa_workspace_action(
    view: QaView,
    drilled: bool,
    searching: bool,
    key: Input,
) -> Option<Action> {
    let qa = |action| Some(Action::Qa(action));
    if searching {
        return match key {
            Input::Char(character) => qa(QaAction::AppendQuery(character)),
            Input::Backspace => qa(QaAction::BackspaceQuery),
            Input::CtrlU => qa(QaAction::ClearQuery),
            Input::Enter | Input::Esc => qa(QaAction::FinishSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return qa(if drilled {
            QaAction::SelectFinding(delta)
        } else if view == QaView::LayerQa {
            QaAction::SelectLayer(delta)
        } else {
            QaAction::SelectCheck(delta)
        });
    }
    match key {
        Input::Tab => qa(QaAction::CycleView),
        Input::Up | Input::Char('k') => qa(if drilled {
            QaAction::SelectFinding(-1)
        } else if view == QaView::LayerQa {
            QaAction::SelectLayer(-1)
        } else {
            QaAction::SelectCheck(-1)
        }),
        Input::Down | Input::Char('j') => qa(if drilled {
            QaAction::SelectFinding(1)
        } else if view == QaView::LayerQa {
            QaAction::SelectLayer(1)
        } else {
            QaAction::SelectCheck(1)
        }),
        Input::Enter => qa(QaAction::Drill),
        Input::Esc if drilled => qa(QaAction::LeaveDrill),
        Input::Char('s') => qa(if view == QaView::LayerQa {
            QaAction::SelectLayer(1)
        } else {
            QaAction::CycleScope
        }),
        Input::Char('/') => qa(QaAction::BeginSearch),
        Input::CtrlU => qa(QaAction::ClearQuery),
        Input::Char('f') => qa(QaAction::CycleStatusFilter),
        Input::Char('r') => qa(if view == QaView::LayerQa {
            QaAction::BeginSelectedLayerCheck
        } else {
            QaAction::BeginSelectedCheck
        }),
        Input::Char('I') => qa(QaAction::BeginImport),
        Input::Char('R') => qa(QaAction::RefreshReports),
        Input::Char('o') => qa(QaAction::OpenSelectedReport),
        Input::Char('e') => qa(if view == QaView::LayerQa {
            QaAction::OpenSelectedLayerRoot
        } else {
            QaAction::OpenProvider
        }),
        Input::Char('l') => qa(QaAction::OpenSelectedSource),
        Input::Char('c') => qa(if view == QaView::LayerQa {
            QaAction::BeginLayerCancellation
        } else {
            QaAction::BeginCancellation
        }),
        _ => None,
    }
}

pub fn qa_dialog_action(dialog: &QaDialog, key: Input) -> Option<Action> {
    let qa = |action| Some(Action::Qa(action));
    match dialog {
        QaDialog::Operation(preview) => match key {
            Input::Enter => qa(QaAction::ConfirmOperation(preview.clone())),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::LayerOperation(preview) => match key {
            Input::Enter => qa(QaAction::ConfirmLayerOperation(preview.clone())),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::Cancellation { session, .. } => match key {
            Input::Enter => qa(QaAction::ConfirmCancellation(*session)),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::LayerCancellation(session) => match key {
            Input::Enter => qa(QaAction::ConfirmLayerCancellation(*session)),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::Import { editor, .. } => match key {
            Input::Enter => qa(QaAction::ConfirmImport(editor.text.clone())),
            Input::Char('q') | Input::Esc if !editor.editing => qa(QaAction::CancelDialog),
            input => popup_editor_action(editor.editing, input),
        },
    }
}
