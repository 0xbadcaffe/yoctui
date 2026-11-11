use super::*;
use yoctui_model::{BuildEnvironmentProfile, BuildEnvironmentState};
#[test]
fn metadata_refresh_preserves_verified_custom_initializer_and_pending_path_selection() {
    let mut app = App::new(16, 4096);
    let verified = BuildEnvironmentProfile {
        source_dir: "/source".into(),
        build_dir: "/build".into(),
        init_script: "/build/init-build-env".into(),
    };
    app.workspace.build_dir = Some(verified.build_dir.clone());
    app.workspace.source_dir = Some(verified.source_dir.clone());
    app.build_environment = BuildEnvironmentState::Connected(verified.clone());
    crate::daemon_client::install_daemon_build_environment(&mut app);
    assert_eq!(
        app.build_environment,
        BuildEnvironmentState::Connected(verified.clone())
    );
    app.daemon.instance_id = Some(yoctui_model::DaemonModelInstanceId([3; 16]));
    app.saved_builds.environment.profile_instance = app.daemon.instance_id;
    app.workspace.source_dir = Some("/source/layers/openembedded-core".into());
    crate::daemon_client::install_daemon_build_environment(&mut app);
    assert_eq!(
        app.build_environment,
        BuildEnvironmentState::Connected(verified.clone()),
        "COREBASE can differ from the selected wrapper working directory"
    );
    let selected = BuildEnvironmentProfile {
        build_dir: "/new-build".into(),
        ..verified
    };
    app.build_environment = BuildEnvironmentState::Configured(selected.clone());
    app.saved_builds.environment.requested_profile = Some(selected.clone());
    crate::daemon_client::install_daemon_build_environment(&mut app);
    assert_eq!(
        app.build_environment,
        BuildEnvironmentState::Configured(selected)
    );
    app.saved_builds.environment.requested_profile = None;
    app.workspace.build_dir = Some("/different".into());
    crate::daemon_client::install_daemon_build_environment(&mut app);
    assert!(
        matches!(&app.build_environment,BuildEnvironmentState::Connected(p) if p.build_dir==std::path::Path::new("/different"))
    );
}
