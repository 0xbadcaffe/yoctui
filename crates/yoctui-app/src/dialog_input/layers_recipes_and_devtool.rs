pub fn layer_tree_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::CtrlU => Some(Action::ClearMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectLayerBrowserEntry { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectLayerBrowserEntry { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectLayerBrowserEntry { delta: 1 }),
        Input::Enter => Some(Action::LayerBrowserEnter),
        Input::Right | Input::Char('l') => Some(Action::LayerBrowserExpand),
        Input::Left | Input::Char('h') => Some(Action::LayerBrowserUp),
        Input::Esc => Some(Action::CloseLayerBrowser),
        Input::Char('r') => Some(Action::RefreshLayerBrowser),
        Input::Char('e') => Some(Action::EditSelectedLayerBrowserFile),
        Input::Char('.') => Some(Action::ToggleLayerBrowserHidden),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::CtrlU => Some(Action::ClearMetadataQuery),
        Input::Char('i') => Some(Action::SetLayerInspectorMode(LayerInspectorMode::Metadata)),
        Input::Char('[') => Some(Action::ScrollLayerBrowserPreview { delta: -10 }),
        Input::Char(']') => Some(Action::ScrollLayerBrowserPreview { delta: 10 }),
        Input::Char('m') => Some(Action::SetLayerInspectorMode(LayerInspectorMode::Metadata)),
        Input::Char('d') => Some(Action::SetLayerInspectorMode(
            LayerInspectorMode::Dependencies,
        )),
        _ => None,
    }
}
pub fn recipes_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::CtrlU => Some(Action::ClearMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectRecipe { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectRecipe { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectRecipe { delta: 1 }),
        Input::Char('[') => Some(Action::ScrollRecipePreview { delta: -10 }),
        Input::Char(']') => Some(Action::ScrollRecipePreview { delta: 10 }),
        Input::Enter => Some(Action::BeginSelectedRecipeMetadata),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::CtrlU => Some(Action::ClearMetadataQuery),
        Input::Char('e') => Some(Action::OpenSelectedRecipeProvider),
        Input::Char('o') => Some(Action::BeginSelectedRecipeTaskLog),
        Input::Char('p') => Some(Action::BeginSelectedRecipePatchReview),
        Input::Char('g') | Input::Char('A') => Some(Action::BeginSelectedRecipeDependencies),
        Input::Char('f') => Some(Action::BeginSelectedRecipeForceTask),
        Input::Char('v') => Some(Action::BeginSelectedRecipeDevshell),
        Input::Char('K') => Some(Action::BeginSelectedRecipeDiffconfig),
        Input::Char('z') => Some(Action::BeginSelectedRecipeDiffsigs),
        Input::Char('Z') => Some(Action::BeginSelectedRecipeSignatures),
        Input::Char('V') => Some(Action::BeginSelectedRecipeCveCheck),
        Input::Char('X') => Some(Action::BeginSelectedRecipeSpdx),
        Input::Char('d') => Some(Action::BeginSelectedRecipeDevtoolModify),
        Input::Char('t') => Some(Action::BeginSelectedRecipeDevtoolStatus),
        Input::Char('u') => Some(Action::BeginSelectedRecipeDevtoolUpdateRecipe),
        Input::Char('F') => Some(Action::BeginSelectedRecipeDevtoolFinish),
        Input::Char('P') => Some(Action::BeginSelectedRecipeDevtoolDeploy),
        Input::Char('N') => Some(Action::BeginSelectedRecipeDevtoolUndeploy),
        Input::Char('U') => Some(Action::BeginSelectedRecipeDevtoolUpgrade),
        Input::Char('D') => Some(Action::BeginSelectedRecipeDevtoolReset),
        Input::Char('s') => Some(Action::BeginSelectedRecipeDevtoolWorkspaceShell),
        Input::Char('E') => Some(Action::BeginSelectedRecipeDevtoolEditRecipe),
        _ => None,
    }
}

pub fn devtool_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::CtrlU => Some(Action::ClearMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    if matches!(key, Input::Char('G') | Input::Char('J')) {
        return Some(Action::BeginSelectedRecipeDevtoolGitUi);
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectRecipe { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectRecipe { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectRecipe { delta: 1 }),
        Input::Char('[') => Some(Action::ScrollRecipePreview { delta: -10 }),
        Input::Char(']') => Some(Action::ScrollRecipePreview { delta: 10 }),
        Input::Enter | Input::Char('t') | Input::Char('r') => {
            Some(Action::BeginSelectedRecipeDevtoolStatus)
        }
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::CtrlU => Some(Action::ClearMetadataQuery),
        Input::Char('d') | Input::Char('e') => Some(Action::BeginSelectedRecipeDevtoolModify),
        Input::Char('b') => Some(Action::BeginSelectedRecipeBuild),
        Input::Char('P') => Some(Action::BeginSelectedRecipeDevtoolDeploy),
        Input::Char('u') => Some(Action::BeginSelectedRecipeDevtoolUpdateRecipe),
        Input::Char('F') => Some(Action::BeginSelectedRecipeDevtoolFinish),
        Input::Char('D') => Some(Action::BeginSelectedRecipeDevtoolReset),
        Input::Char('s') => Some(Action::BeginSelectedRecipeDevtoolWorkspaceShell),
        Input::Char('E') => Some(Action::BeginSelectedRecipeDevtoolEditRecipe),
        _ => None,
    }
}

pub fn terminal_launch_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Left | Input::Char('k') | Input::Char('h') => {
            Some(Action::SelectTerminalLaunchDestination { delta: -1 })
        }
        Input::Down | Input::Right | Input::Char('j') | Input::Char('l') | Input::Tab => {
            Some(Action::SelectTerminalLaunchDestination { delta: 1 })
        }
        Input::Enter => Some(Action::ConfirmTerminalLaunch),
        Input::Esc => Some(Action::CancelTerminalLaunch),
        _ => None,
    }
}

pub fn yocto_utility_dialog_action(
    dialog: &yoctui_model::YoctoUtilityDialog,
    key: Input,
) -> Option<Action> {
    let selected_kind = dialog
        .fields()
        .get(dialog.selected_field)
        .map(|(_, _, kind)| *kind);
    match key {
        Input::Up | Input::BackTab => Some(Action::SelectYoctoUtilityField { delta: -1 }),
        Input::Down | Input::Tab => Some(Action::SelectYoctoUtilityField { delta: 1 }),
        Input::Left => Some(Action::CycleYoctoUtilityChoice { delta: -1 }),
        Input::Right => Some(Action::CycleYoctoUtilityChoice { delta: 1 }),
        Input::Char(' ')
            if selected_kind == Some(yoctui_model::YoctoUtilityFieldKind::Choice) =>
        {
            Some(Action::CycleYoctoUtilityChoice { delta: 1 })
        }
        Input::Backspace => Some(Action::BackspaceYoctoUtilityField),
        Input::CtrlU => Some(Action::ClearYoctoUtilityField),
        Input::Enter => Some(Action::ReviewYoctoUtility),
        Input::Esc => Some(Action::CancelYoctoUtility),
        Input::Char(character) => Some(Action::AppendYoctoUtilityField(character)),
        _ => None,
    }
}

pub fn dtc_compile_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') | Input::BackTab => {
            Some(Action::SelectDtcCompileOption { delta: -1 })
        }
        Input::Down | Input::Char('j') | Input::Tab => {
            Some(Action::SelectDtcCompileOption { delta: 1 })
        }
        Input::Left | Input::Char('h') => Some(Action::AdjustDtcCompileOption { delta: -1 }),
        Input::Right | Input::Char('l') | Input::Char(' ') => {
            Some(Action::AdjustDtcCompileOption { delta: 1 })
        }
        Input::Enter => Some(Action::ConfirmDtcCompileOptions),
        Input::Esc => Some(Action::CancelDtcCompileOptions),
        _ => None,
    }
}

pub fn devtool_modify_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolModify),
        Input::Esc => Some(Action::CancelDevtoolModify),
        _ => None,
    }
}

pub fn devtool_update_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolUpdateRecipe),
        Input::Esc => Some(Action::CancelDevtoolUpdateRecipe),
        _ => None,
    }
}

pub fn devtool_finish_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectDevtoolFinishLayer { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectDevtoolFinishLayer { delta: 1 }),
        Input::Enter => Some(Action::PreviewDevtoolFinish),
        Input::Esc => Some(Action::CancelDevtoolFinish),
        _ => None,
    }
}

pub fn devtool_finish_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolFinish),
        Input::Esc => Some(Action::CancelDevtoolFinishConfirmation),
        _ => None,
    }
}

pub fn devtool_deploy_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendDevtoolDeployTarget(character)),
        Input::Backspace => Some(Action::BackspaceDevtoolDeployTarget),
        Input::Enter => Some(Action::PreviewDevtoolDeploy),
        Input::Esc => Some(Action::CancelDevtoolDeploy),
        _ => None,
    }
}

pub fn devtool_deploy_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolDeploy),
        Input::Esc => Some(Action::CancelDevtoolDeployConfirmation),
        _ => None,
    }
}

pub fn devtool_undeploy_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendDevtoolUndeployTarget(character)),
        Input::Backspace => Some(Action::BackspaceDevtoolUndeployTarget),
        Input::Enter => Some(Action::PreviewDevtoolUndeploy),
        Input::Esc => Some(Action::CancelDevtoolUndeploy),
        _ => None,
    }
}

pub fn devtool_undeploy_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolUndeploy),
        Input::Esc => Some(Action::CancelDevtoolUndeployConfirmation),
        _ => None,
    }
}

pub fn devtool_upgrade_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolUpgrade),
        Input::Esc => Some(Action::CancelDevtoolUpgrade),
        _ => None,
    }
}

pub fn devtool_reset_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolReset),
        Input::Esc => Some(Action::CancelDevtoolReset),
        _ => None,
    }
}
