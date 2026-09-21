pub fn errors_action(key: Input) -> Option<Action> {
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectError { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectError { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectError { delta: 1 }),
        Input::Enter => Some(Action::JumpToSelectedError),
        Input::Char('o') => Some(Action::OpenSelectedErrorSource),
        _ => None,
    }
}
pub fn dependency_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendDependencyGraphQuery(character)),
            Input::Backspace => Some(Action::BackspaceDependencyGraphQuery),
            Input::CtrlU => Some(Action::ClearDependencyGraphQuery),
            Input::Enter | Input::Esc => Some(Action::FinishDependencyGraphSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectDependencyGraphNode { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectDependencyGraphNode { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectDependencyGraphNode { delta: 1 }),
        Input::Enter => Some(Action::OpenSelectedDependencyRecipe),
        Input::Char('o') => Some(Action::OpenSelectedDependencyProvider),
        Input::Char('L') => Some(Action::OpenSelectedDependencyTaskLog),
        Input::Char('r') => Some(Action::RefreshDependencyGraph),
        Input::Char('/') => Some(Action::BeginDependencyGraphSearch),
        Input::CtrlU => Some(Action::ClearDependencyGraphQuery),
        Input::Char('v') => Some(Action::ToggleDependencyGraphReverse),
        Input::Left | Input::Char('h') => Some(Action::CollapseSelectedDependencyGraphNode),
        Input::Right | Input::Char('l') => Some(Action::ExpandSelectedDependencyGraphNode),
        Input::Char(' ') => Some(Action::ToggleSelectedDependencyGraphNode),
        _ => None,
    }
}
pub fn signature_task_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSignatureTask { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSignatureTask { delta: 1 }),
        Input::Enter => Some(Action::ConfirmSignatureTask),
        Input::Esc => Some(Action::CancelSignatureTaskPicker),
        _ => None,
    }
}
pub fn signature_workspace_action(key: Input) -> Option<Action> {
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectSignatureRecord { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSignatureRecord { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSignatureRecord { delta: 1 }),
        Input::Char('1') => Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Left,
        )),
        Input::Char('2') => Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Right,
        )),
        Input::Char('c') => Some(Action::BeginSignatureComparison),
        Input::Char('r') => Some(Action::RefreshSignatureDump),
        Input::Char('e') => Some(Action::OpenSignatureProvider),
        Input::Esc => Some(Action::LeaveSignatureWorkspace),
        _ => None,
    }
}
pub fn package_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendPackageQuery(character)),
            Input::Backspace => Some(Action::BackspacePackageQuery),
            Input::CtrlU => Some(Action::ClearPackageQuery),
            Input::Enter | Input::Esc => Some(Action::FinishPackageSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectPackage { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectPackage { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectPackage { delta: 1 }),
        Input::Enter => Some(Action::BeginSelectedPackageDetail),
        Input::Char('/') => Some(Action::BeginPackageSearch),
        Input::CtrlU => Some(Action::ClearPackageQuery),
        Input::Char('R') => Some(Action::RefreshPackageInventory),
        Input::Char('c') => Some(Action::CancelPackageOperation),
        Input::Char('D') => Some(Action::TogglePackageDependencyKind),
        Input::Char('[') => Some(Action::SelectPackageDependency { delta: -1 }),
        Input::Char(']') => Some(Action::SelectPackageDependency { delta: 1 }),
        Input::Char('d') => Some(Action::OpenSelectedPackageDependency),
        Input::Char('u') => Some(Action::BackPackageNavigation),
        Input::Char('o') => Some(Action::OpenSelectedPackageRecipe),
        Input::Char('e') => Some(Action::OpenSelectedPackageProvider),
        _ => None,
    }
}

pub fn images_workspace_action(searching: bool, key: Input) -> Option<Action> {
    images_workspace_action_for_view(searching, yoctui_model::ImagesView::Artifacts, key)
}

pub fn images_workspace_action_for_view(
    searching: bool,
    view: yoctui_model::ImagesView,
    key: Input,
) -> Option<Action> {
    if matches!(key, Input::Tab | Input::BackTab) {
        return Some(Action::ShiftImagesView {
            delta: if key == Input::Tab { 1 } else { -1 },
        });
    }
    if !(view == yoctui_model::ImagesView::Artifacts && searching)
        && let Input::Char(key @ ('1' | '2' | '3' | '4' | '5' | '6')) = key
    {
        let current = yoctui_model::ImagesView::ALL
            .iter()
            .position(|candidate| *candidate == view)
            .unwrap_or(0) as isize;
        let destination = key.to_digit(10).unwrap_or(1) as isize - 1;
        return Some(Action::ShiftImagesView {
            delta: destination - current,
        });
    }
    if view == yoctui_model::ImagesView::RootfsPackages {
        if let Some(delta) = collection_scroll_delta(key) {
            return Some(Action::SelectRootfsPackage { delta });
        }
        return match key {
            Input::Up | Input::Char('k') => Some(Action::SelectRootfsPackage { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::SelectRootfsPackage { delta: 1 }),
            Input::Left | Input::Char('h') => Some(Action::SelectRootfsGroup { delta: -1 }),
            Input::Right | Input::Char('l') => Some(Action::SelectRootfsGroup { delta: 1 }),
            Input::Char('r') | Input::Char('R') => Some(Action::RefreshRootfsComposition),
            _ => None,
        };
    }
    if view == yoctui_model::ImagesView::RootfsFilesystem {
        if let Some(delta) = collection_scroll_delta(key) {
            return Some(Action::SelectRootfsEntry { delta });
        }
        return match key {
            Input::Up | Input::Char('k') => Some(Action::SelectRootfsEntry { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::SelectRootfsEntry { delta: 1 }),
            Input::Enter | Input::Right => Some(Action::BrowseRootfsFilesystem),
            Input::Char('r') | Input::Char('R') => Some(Action::RefreshRootfsComposition),
            _ => None,
        };
    }
    if view == yoctui_model::ImagesView::UdevRules {
        if let Some(delta) = collection_scroll_delta(key) {
            return Some(Action::SelectRootfsUdevRule { delta });
        }
        return match key {
            Input::Char('[') => Some(Action::ScrollRootfsUdevPreview { delta: -1 }),
            Input::Char(']') => Some(Action::ScrollRootfsUdevPreview { delta: 1 }),
            Input::Enter | Input::Right => Some(Action::BrowseRootfsFilesystem),
            Input::Char('r') | Input::Char('R') => Some(Action::RefreshRootfsComposition),
            _ => None,
        };
    }
    if view == yoctui_model::ImagesView::SystemdServices {
        if let Some(delta) = collection_scroll_delta(key) {
            return Some(Action::SelectRootfsSystemdService { delta });
        }
        return match key {
            Input::Up | Input::Char('k') => Some(Action::SelectRootfsSystemdService { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::SelectRootfsSystemdService { delta: 1 }),
            Input::Enter | Input::Right => Some(Action::BrowseRootfsFilesystem),
            Input::Char('e') => Some(Action::EditSelectedRootfsSystemFile),
            Input::Char('r') | Input::Char('R') => Some(Action::RefreshRootfsComposition),
            _ => None,
        };
    }
    if view == yoctui_model::ImagesView::SystemDbus {
        if let Some(delta) = collection_scroll_delta(key) {
            return Some(Action::SelectRootfsDbusService { delta });
        }
        return match key {
            Input::Up | Input::Char('k') => Some(Action::SelectRootfsDbusService { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::SelectRootfsDbusService { delta: 1 }),
            Input::Enter | Input::Right => Some(Action::BrowseRootfsFilesystem),
            Input::Char('e') => Some(Action::EditSelectedRootfsSystemFile),
            Input::Char('r') | Input::Char('R') => Some(Action::RefreshRootfsComposition),
            _ => None,
        };
    }
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendImageArtifactQuery(character)),
            Input::Backspace => Some(Action::BackspaceImageArtifactQuery),
            Input::CtrlU => Some(Action::ClearImageArtifactQuery),
            Input::Enter | Input::Esc => Some(Action::FinishImageArtifactSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectImageArtifact { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectImageArtifact { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectImageArtifact { delta: 1 }),
        Input::Char('/') => Some(Action::BeginImageArtifactSearch),
        Input::CtrlU => Some(Action::ClearImageArtifactQuery),
        Input::Char('R') => Some(Action::RefreshImageArtifactInventory),
        Input::Char('c') => Some(Action::CancelImageArtifactOperation),
        Input::Char('b') => Some(Action::BeginSelectedImageArtifactBuild),
        Input::Char('T') => Some(Action::BeginSelectedImageConsole),
        Input::Char('Q') => Some(Action::BeginSelectedQemuLaunch),
        Input::Char('W') => Some(Action::BeginSelectedWicCreate),
        Input::Char('D') => Some(Action::BeginSelectedWicDeviceWrite),
        Input::Char('x') => Some(Action::BeginActiveImageRuntimeCancellation),
        Input::Char('[') => Some(Action::SelectWicOutput { delta: -1 }),
        Input::Char(']') => Some(Action::SelectWicOutput { delta: 1 }),
        Input::Char('O') => Some(Action::OpenSelectedWicOutput),
        Input::Char('o') => Some(Action::OpenSelectedImageArtifact),
        Input::Char('m') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Manifest,
        )),
        Input::Char('l') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::License,
        )),
        Input::Char('s') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Spdx,
        )),
        Input::Char('w') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Wic,
        )),
        Input::Char('p') | Input::Enter => Some(Action::BeginSelectedRootfsComposition),
        _ => None,
    }
}
