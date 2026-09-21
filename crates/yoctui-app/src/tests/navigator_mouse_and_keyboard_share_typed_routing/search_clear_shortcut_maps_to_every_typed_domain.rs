use super::*;

#[test]
fn search_clear_shortcut_maps_to_every_typed_domain() {
    for searching in [false, true] {
        assert_eq!(
            compatibility_ui_inspector_action(searching, Input::CtrlU),
            Some(Action::ClearCompatibilityQuery)
        );
        assert_eq!(
            logs_action(searching, Input::CtrlU),
            Some(Action::ClearLogQuery)
        );
        assert_eq!(
            package_workspace_action(searching, Input::CtrlU),
            Some(Action::ClearPackageQuery)
        );
        assert_eq!(
            images_workspace_action(searching, Input::CtrlU),
            Some(Action::ClearImageArtifactQuery)
        );
        assert_eq!(
            sdk_workspace_action(searching, Input::CtrlU),
            Some(Action::ClearSdkArtifactQuery)
        );
        assert_eq!(
            test_results_workspace_action(searching, false, Input::CtrlU),
            Some(Action::ClearTestResultQuery)
        );
        assert_eq!(
            layer_tree_action(searching, Input::CtrlU),
            Some(Action::ClearMetadataQuery)
        );
        assert_eq!(
            recipes_workspace_action(searching, Input::CtrlU),
            Some(Action::ClearMetadataQuery)
        );
        assert_eq!(
            config_workspace_action(searching, Input::CtrlU),
            Some(Action::ClearMetadataQuery)
        );
        assert_eq!(
            security_workspace_action(SecurityView::Cves, false, searching, Input::CtrlU),
            Some(Action::Security(SecurityAction::ClearQuery))
        );
        assert_eq!(
            qa_workspace_action(QaView::RecipeKernel, false, searching, Input::CtrlU),
            Some(Action::Qa(QaAction::ClearQuery))
        );
    }
}
