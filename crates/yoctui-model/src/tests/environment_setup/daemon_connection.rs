use super::*;
#[test]
fn saving_paths_prepares_daemon_review_and_cancel_restores_current_environment() {
    let mut app = App::new(20, 2000);
    app.screen = crate::Screen::BuildEnvironment;
    let old = crate::BuildEnvironmentProfile {
        source_dir: "/old/source".into(),
        build_dir: "/old/build".into(),
        init_script: "/old/source/oe-init-build-env".into(),
    };
    app.build_environment = BuildEnvironmentState::Connected(old.clone());
    app.available_images = vec!["core-image-old".into()];
    app.workspace.recipes = vec![crate::Recipe {
        name: "core-image-old".into(),
        ..Default::default()
    }];
    action(&mut app, EnvironmentSetupAction::Open { browse: false });
    action(&mut app, EnvironmentSetupAction::Edit);
    action(
        &mut app,
        EnvironmentSetupAction::Insert("/new/source".into()),
    );
    action(&mut app, EnvironmentSetupAction::AcceptEdit);
    let Some(Effect::SavedEnvironment(crate::SavedEnvironmentRequest::PrepareProfile {
        generation,
        profile,
    })) = action(&mut app, EnvironmentSetupAction::Save)
    else {
        panic!("save must dispatch a daemon connection review")
    };
    assert_eq!(profile.source_dir, PathBuf::from("/new/source"));
    assert_eq!(app.available_images, vec!["core-image-old"]);
    let plan = crate::SavedEnvironmentPlan {
        edited_profile: true,
        profile,
        target: String::new(),
        machine: None,
        mode: crate::SavedEnvironmentMode::Restart { instance: [1; 16] },
    };
    update(
        &mut app,
        Action::SavedBuild(crate::SavedBuildAction::Environment(
            crate::SavedEnvironmentAction::Prepared {
                generation,
                result: Ok(plan),
            },
        )),
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::SavedEnvironmentReview(_))
    ));
    // A retained metadata projection may clear image availability while selection is pending.
    app.refresh_image_targets();
    update(
        &mut app,
        Action::SavedBuild(crate::SavedBuildAction::Environment(
            crate::SavedEnvironmentAction::Cancel,
        )),
    );
    assert_eq!(app.build_environment, BuildEnvironmentState::Connected(old));
    assert_eq!(app.available_images, vec!["core-image-old"]);
}
