use super::*;
use yoctui_model::{VariableDetail, Workspace};

fn identity() -> VariableIdentity {
    VariableIdentity {
        name: "EXTRA_IMAGE_FEATURES".into(),
        recipe: None,
    }
}

fn app_with_config() -> App {
    let mut app = App::new(20, 4096);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert(identity().name, "old".into());
    app.config_selection = 0;
    app
}

#[test]
fn configuration_snapshot_updates_values_without_replacing_build_or_recipe_inventory() {
    let mut app = app_with_config();
    app.workspace.source_dir = Some("/source".into());
    app.workspace.build_dir = Some("/build".into());
    let mut values = app.workspace.variables.clone();
    values.insert(
        identity().name,
        "allow-empty-password empty-root-password allow-root-login".into(),
    );
    apply_snapshot(
        &mut app,
        Workspace {
            variables: values.clone(),
            ..Default::default()
        },
    );
    assert_eq!(app.workspace.variables, values);
    assert_eq!(
        app.workspace.source_dir.as_deref(),
        Some(Path::new("/source"))
    );
    assert_eq!(
        app.workspace.build_dir.as_deref(),
        Some(Path::new("/build"))
    );
    assert_eq!(selected_identity(&app), Some(identity()));
}

#[test]
fn configuration_snapshot_retains_comparison_detail_when_values_are_unchanged() {
    let mut app = app_with_config();
    for recipe in [None, Some("core-image-minimal".into())] {
        let mut identity = identity();
        identity.recipe = recipe;
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity,
                effective_value: Some("old".into()),
                unexpanded_value: Some("old".into()),
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
    }
    let variables = app.workspace.variables.clone();
    apply_snapshot(
        &mut app,
        Workspace {
            variables,
            ..Default::default()
        },
    );
    assert_eq!(app.variable_details.len(), 2);
    let mut values = app.workspace.variables.clone();
    values.insert(identity().name, "new".into());
    apply_snapshot(
        &mut app,
        Workspace {
            variables: values,
            ..Default::default()
        },
    );
    assert!(app.variable_details.is_empty());
}

#[tokio::test]
async fn configuration_io_never_starts_bitbake_offline() {
    let mut io = ConfigurationIo::default();
    let mut app = app_with_config();
    app.screen = Screen::Configuration;
    io.inspect(identity());
    assert!(!io.poll(&mut app, Path::new("/missing/build"), false).await);
    assert!(!io.active());
}

#[tokio::test]
async fn configuration_io_discards_result_after_an_external_change() {
    let build = Path::new("/selected/build");
    let mut app = app_with_config();
    let mut io = ConfigurationIo {
        context: Some((build.into(), None)),
        running: Some(identity()),
        ..Default::default()
    };
    io.worker = Some(crate::daemon_metadata::StartupMetadata::spawn(|_| async {
        Ok(Some(query::FreshConfiguration {
            workspace: Workspace::default(),
            detail: None,
        }))
    }));
    tokio::task::yield_now().await;
    io.refresh();
    assert!(io.poll(&mut app, build, true).await);
    assert_eq!(
        app.workspace.variables.get(&identity().name).unwrap(),
        "old"
    );
    assert!(!io.active());
    assert!(io.dirty);
}

#[tokio::test]
async fn configuration_watcher_detects_atomic_local_conf_save_without_querying_offscreen() {
    let build = std::env::temp_dir().join(format!("yoctui-live-config-{}", std::process::id()));
    fs::create_dir_all(build.join("conf")).unwrap();
    let mut app = app_with_config();
    let mut io = ConfigurationIo {
        context: Some((build.clone(), None)),
        ..Default::default()
    };
    io.start_watching(&app, &build);
    assert!(io.watcher.is_some());
    fs::write(
        build.join("conf/local.conf.new"),
        "EXTRA_IMAGE_FEATURES = \"test\"\n",
    )
    .unwrap();
    fs::rename(
        build.join("conf/local.conf.new"),
        build.join("conf/local.conf"),
    )
    .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !io.dirty {
            io.poll(&mut app, &build, true).await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(!io.active());
    drop(io);
    fs::remove_dir_all(build).unwrap();
}

#[test]
fn configuration_image_features_edit_is_validated_and_preserves_other_assignments() {
    let build = std::env::temp_dir().join(format!("yoctui-config-features-{}", std::process::id()));
    fs::create_dir_all(build.join("conf")).unwrap();
    fs::write(
        build.join("conf/local.conf"),
        "MACHINE = \"qemux86-64\"\nEXTRA_IMAGE_FEATURES += \"old\"\n",
    )
    .unwrap();
    let value = "allow-empty-password empty-root-password allow-root-login";
    let request = ConfigEditRequest {
        identity: identity(),
        value: value.into(),
        destination: build.join("conf/local.conf"),
        assignment: yoctui_model::config_edit_assignment("EXTRA_IMAGE_FEATURES", value).unwrap(),
    };
    write_config_assignment_atomic(&build, &request).unwrap();
    let content = fs::read_to_string(build.join("conf/local.conf")).unwrap();
    assert!(content.contains("MACHINE = \"qemux86-64\""));
    assert!(content.contains(&request.assignment));
    assert!(!content.contains("+= \"old\""));
    fs::remove_dir_all(build).unwrap();
}
