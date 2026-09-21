pub fn sdk_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendSdkArtifactQuery(character)),
            Input::Backspace => Some(Action::BackspaceSdkArtifactQuery),
            Input::CtrlU => Some(Action::ClearSdkArtifactQuery),
            Input::Enter | Input::Esc => Some(Action::FinishSdkArtifactSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectSdkArtifact { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSdkArtifact { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSdkArtifact { delta: 1 }),
        Input::Char('/') => Some(Action::BeginSdkArtifactSearch),
        Input::CtrlU => Some(Action::ClearSdkArtifactQuery),
        Input::Char('R') => Some(Action::RefreshSdkArtifactInventory),
        Input::Char('s') => Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
            SdkKind::Standard,
        ))),
        Input::Char('E') => Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
            SdkKind::Extensible,
        ))),
        Input::Char('t') => Some(Action::BeginSdkBuild(SdkBuildAction::Test(
            SdkKind::Standard,
        ))),
        Input::Char('T') => Some(Action::BeginSdkBuild(SdkBuildAction::Test(
            SdkKind::Extensible,
        ))),
        Input::Char('P') => Some(Action::BeginSelectedSdkPublish),
        Input::Char('n') => Some(Action::BeginSdkNative),
        Input::Char('o') => Some(Action::OpenSelectedSdkArtifact),
        Input::Char('c') => Some(Action::BeginActiveSdkSessionCancellation),
        _ => None,
    }
}

pub fn sdk_build_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkBuild),
        Input::Esc => Some(Action::CancelSdkBuild),
        _ => None,
    }
}

pub fn sdk_publish_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendSdkPublishDestination(character)),
        Input::Backspace => Some(Action::BackspaceSdkPublishDestination),
        Input::Enter => Some(Action::PreviewSdkPublish),
        Input::Esc => Some(Action::CancelSdkPublish),
        _ => None,
    }
}

pub fn sdk_publish_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkPublish),
        Input::Esc => Some(Action::CancelSdkPublishPreview),
        _ => None,
    }
}

pub fn sdk_native_dialog_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendSdkNativeField(character)),
            Input::Backspace => Some(Action::BackspaceSdkNativeField),
            Input::Enter => Some(Action::FinishSdkNativeFieldEdit),
            Input::Esc => Some(Action::CancelSdkNative),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSdkNativeField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSdkNativeField { delta: 1 }),
        Input::Left | Input::Right | Input::Char('h') | Input::Char('l') => {
            Some(Action::CycleSdkNativeMode)
        }
        Input::Enter => Some(Action::ActivateSdkNativeField),
        Input::Char('p') => Some(Action::PreviewSdkNative),
        Input::Esc => Some(Action::CancelSdkNative),
        _ => None,
    }
}

pub fn sdk_native_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkNative),
        Input::Esc => Some(Action::CancelSdkNativePreview),
        _ => None,
    }
}

pub fn sdk_cancellation_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkSessionCancellation),
        Input::Esc => Some(Action::CancelSdkSessionCancellation),
        _ => None,
    }
}

pub fn testing_workspace_action(key: Input) -> Option<Action> {
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectTestFamily { delta });
    }
    match key {
        Input::Tab => Some(Action::CycleTestView),
        Input::Up | Input::Char('k') => Some(Action::SelectTestFamily { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestFamily { delta: 1 }),
        Input::Enter | Input::Char('r') => Some(Action::BeginSelectedTestLaunch),
        Input::Char('x') => Some(Action::BeginActiveTestSessionCancellation),
        _ => None,
    }
}
