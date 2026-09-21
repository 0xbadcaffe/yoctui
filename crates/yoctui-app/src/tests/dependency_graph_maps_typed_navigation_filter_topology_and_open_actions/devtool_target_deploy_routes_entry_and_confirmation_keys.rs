use super::*;

#[test]
fn devtool_target_deploy_routes_entry_and_confirmation_keys() {
    assert_eq!(
        devtool_deploy_dialog_action(Input::Char('q')),
        Some(Action::AppendDevtoolDeployTarget('q'))
    );
    assert_eq!(
        devtool_deploy_dialog_action(Input::Backspace),
        Some(Action::BackspaceDevtoolDeployTarget)
    );
    assert_eq!(
        devtool_deploy_dialog_action(Input::Enter),
        Some(Action::PreviewDevtoolDeploy)
    );
    assert_eq!(
        devtool_deploy_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolDeploy)
    );
    assert_eq!(
        devtool_deploy_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolDeployConfirmation)
    );
}
