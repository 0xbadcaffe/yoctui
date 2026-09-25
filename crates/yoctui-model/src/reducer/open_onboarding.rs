//! State transitions beginning with OpenOnboarding.
use super::*;

fn selected_platform_dtc(
    app: &App,
    kernel: bool,
) -> Result<(PlatformFile, PathBuf, PlatformComponent), String> {
    let workbench = if kernel { &app.kernel } else { &app.firmware };
    let file = workbench
        .selected_file()
        .cloned()
        .ok_or_else(|| "Select a device-tree file first.".to_owned())?;
    let inventory = workbench
        .inventory()
        .ok_or_else(|| "Refresh the platform inventory before running dtc.".to_owned())?;
    if !yoctui_utils::is_absolute_normal_path(&file.root)
        || !yoctui_utils::is_absolute_normal_path(&file.path)
        || file.path.strip_prefix(&file.root).is_err()
        || !inventory.roots.iter().any(|root| root == &file.root)
    {
        return Err("The selected file is outside its authoritative root.".into());
    }
    let program = inventory
        .dtc
        .clone()
        .ok_or_else(|| "Install the dtc compiler to work with device-tree binaries.".to_owned())?;
    if !yoctui_utils::is_absolute_normal_path(&program) {
        return Err("The reported dtc executable path is unsafe.".into());
    }
    Ok((file, program, inventory.component))
}

fn ensure_output_absent(app: &mut App, output: &Path) -> bool {
    match yoctui_utils::path_entry_exists(output) {
        Ok(false) => true,
        Ok(true) => {
            app.notification = Some(format!(
                "Refusing to overwrite {}; move or remove it first.",
                output.display()
            ));
            false
        }
        Err(error) => {
            app.notification = Some(format!(
                "Cannot verify that {} is available: {error}",
                output.display()
            ));
            false
        }
    }
}

fn begin_platform_dtc_compile(app: &mut App, kernel: bool) {
    let (file, program, component) = match selected_platform_dtc(app, kernel) {
        Ok(values) => values,
        Err(message) => {
            app.notification = Some(message);
            return;
        }
    };
    if file.kind != PlatformFileKind::Dts {
        app.notification = Some("Select a DTS source before compiling.".into());
        return;
    }
    let dialog = DtcCompileDialog::new(component, &file, program);
    if !ensure_output_absent(app, &dialog.output) {
        return;
    }
    open_dialog(app, Dialog::DtcCompile(dialog));
}

fn begin_platform_dtc_decompile(app: &mut App, kernel: bool) {
    let (file, program, component) = match selected_platform_dtc(app, kernel) {
        Ok(values) => values,
        Err(message) => {
            app.notification = Some(message);
            return;
        }
    };
    if !matches!(file.kind, PlatformFileKind::Dtb | PlatformFileKind::Dtbo) {
        app.notification = Some("Select a DTB or DTBO before decompiling.".into());
        return;
    }
    let stem = file
        .path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("device-tree");
    let output = file.path.with_file_name(format!("{stem}.yoctui.dts"));
    if !ensure_output_absent(app, &output) {
        return;
    }
    open_dialog(
        app,
        Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest {
                name: format!(
                    "decompile {} device tree",
                    component.label().to_ascii_lowercase()
                ),
                kind: TerminalCreationKind::Utility,
                cwd: file.root,
                program,
                arguments: vec![
                    "-I".into(),
                    "dtb".into(),
                    "-O".into(),
                    "dts".into(),
                    "-o".into(),
                    output.display().to_string(),
                    file.path.display().to_string(),
                ],
            },
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: Some(output),
        }),
    );
}

mod explore_selected_firmware_root_to_terminal_move_copy_row;
mod open_onboarding_to_open_selected_firmware_file;
mod terminal_copy_viewport_to_terminal_begin_search;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::OpenOnboarding
        | Action::DismissOnboarding
        | Action::SelectOnboarding { .. }
        | Action::ActivateOnboardingStep
        | Action::AdvanceOnboarding
        | Action::SkipOnboardingStep
        | Action::RestartOnboarding
        | Action::OnboardingPersisted
        | Action::OnboardingPersistenceFailed(..)
        | Action::ScrollCurrent { .. }
        | Action::ProjectProfileAbsent
        | Action::ProjectProfileLoaded(..)
        | Action::ProjectProfileLoadFailed(..)
        | Action::PreviewProjectProfileGeneration(..)
        | Action::ConfirmProjectProfileGeneration { .. }
        | Action::ProjectProfileGenerated(..)
        | Action::ProjectProfileGenerationFailed(..)
        | Action::SelectProjectProfileItem { .. }
        | Action::ActivateProjectProfileItem
        | Action::OpenRawFavorites
        | Action::InspectKernel
        | Action::KernelLoaded(..)
        | Action::KernelFailed(..)
        | Action::CycleKernelView
        | Action::SetKernelView(..)
        | Action::SelectKernelFile { .. }
        | Action::LaunchKernelMenuconfig
        | Action::OpenSelectedKernelFile
        | Action::ExploreSelectedKernelRoot
        | Action::CompileSelectedKernelDts
        | Action::DecompileSelectedKernelDtb
        | Action::InspectFirmware
        | Action::FirmwareLoaded(..)
        | Action::FirmwareFailed(..)
        | Action::CycleFirmwareView
        | Action::SetFirmwareView(..)
        | Action::SelectFirmwareFile { .. }
        | Action::LaunchFirmwareMenuconfig
        | Action::OpenSelectedFirmwareFile => {
            open_onboarding_to_open_selected_firmware_file::reduce_actions(app, action)
        }
        Action::ExploreSelectedFirmwareRoot
        | Action::CompileSelectedFirmwareDts
        | Action::DecompileSelectedFirmwareDtb
        | Action::SelectDtcCompileOption { .. }
        | Action::AdjustDtcCompileOption { .. }
        | Action::ConfirmDtcCompileOptions
        | Action::CancelDtcCompileOptions
        | Action::ShiftOverviewView { .. }
        | Action::SelectOverviewView(..)
        | Action::Open(..)
        | Action::SelectNavigator { .. }
        | Action::SelectNavigatorAt { .. }
        | Action::ToggleNavigatorGroup { .. }
        | Action::CollapseNavigatorGroup
        | Action::ExpandNavigatorGroup
        | Action::SelectPtySession { .. }
        | Action::SelectPtyPane { .. }
        | Action::TerminalTakeControl
        | Action::TerminalReleaseControl
        | Action::TerminalCreateBuildShell
        | Action::TerminalCreateSelectedDevshell
        | Action::TerminalCreateSelectedMenuconfig
        | Action::DetachedTerminalAvailabilityDetected(..)
        | Action::SelectTerminalLaunchDestination { .. }
        | Action::ConfirmTerminalLaunch
        | Action::CancelTerminalLaunch
        | Action::TerminalEnterCopyMode
        | Action::TerminalMoveCopyRow { .. } => {
            explore_selected_firmware_root_to_terminal_move_copy_row::reduce_actions(app, action)
        }
        Action::TerminalCopyViewport | Action::TerminalBeginSearch => {
            terminal_copy_viewport_to_terminal_begin_search::reduce_actions(app, action)
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
