use super::*;

#[cfg(unix)]
pub(crate) fn sdk_workflow_fixture(
    name: &str,
    publish_body: &str,
    find_body: &str,
    run_body: &str,
) -> (PathBuf, PathBuf, SdkArtifactAdapter, SdkToolAdapter, App) {
    use std::os::unix::fs::PermissionsExt;

    let directory = std::env::temp_dir().join(format!(
        "yoctui-sdk-workflow-{}-{name}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let build = directory.join("build");
    let source = directory.join("source");
    let scripts = source.join("scripts");
    let deploy = directory.join("deploy-sdk");
    fs::create_dir_all(&build).unwrap();
    fs::create_dir_all(&scripts).unwrap();
    fs::create_dir_all(&deploy).unwrap();
    for (tool, body) in [
        ("oe-publish-sdk", publish_body),
        ("oe-find-native-sysroot", find_body),
        ("oe-run-native", run_body),
    ] {
        let path = scripts.join(tool);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).unwrap();
    }
    fs::write(
        deploy.join("poky-glibc-x86_64-core-image-minimal-qemux86-64.sh"),
        b"installer",
    )
    .unwrap();

    let build = fs::canonicalize(build).unwrap();
    let source = fs::canonicalize(source).unwrap();
    let deploy = fs::canonicalize(deploy).unwrap();
    let mut app = App::new(100, 100_000);
    app.screen = Screen::Sdk;
    app.build.target = Some("core-image-minimal".into());
    app.workspace.build_dir = Some(build.clone());
    app.workspace.source_dir = Some(source.clone());
    app.workspace
        .variables
        .insert("SDK_DEPLOY".into(), deploy.display().to_string());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    let artifact_adapter = SdkArtifactAdapter::new(deploy.clone());
    let tool_adapter = SdkToolAdapter::new(build.clone(), deploy, vec![build.clone(), source]);
    (directory, build, artifact_adapter, tool_adapter, app)
}
