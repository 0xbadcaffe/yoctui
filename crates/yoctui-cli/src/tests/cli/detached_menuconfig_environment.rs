use super::*;

struct Fixture {
    root: tempfile::TempDir,
    source: PathBuf,
    request: yoctui_model::TerminalLaunchRequest,
}

impl Fixture {
    fn new(script: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("selected-source");
        let build = root.path().join("build");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(build.join("conf")).unwrap();
        fs::write(build.join("conf/local.conf"), "MACHINE = \"qemux86-64\"\n").unwrap();
        fs::write(build.join("conf/bblayers.conf"), "BBLAYERS = \"\"\n").unwrap();
        fs::write(source.join("oe-init-build-env"), script).unwrap();
        let request = yoctui_model::TerminalLaunchRequest {
            name: "kernel menuconfig".into(),
            kind: yoctui_model::TerminalCreationKind::Menuconfig,
            cwd: build,
            program: "/usr/bin/env".into(),
            arguments: vec!["virtual/kernel".into(), "-c".into(), "menuconfig".into()],
            completion: None,
        };
        Self {
            root,
            source,
            request,
        }
    }
}

#[tokio::test]
async fn detached_menuconfig_initializes_selected_build_before_gui_launch() {
    let fixture = Fixture::new(
        "export BUILDDIR=\"$1\"\nexport BBPATH=\"$1\"\nexport BB_ENV_PASSTHROUGH_ADDITIONS='MACHINE DISTRO'\nexport YOCTUI_DETACHED_TEST=selected\n",
    );
    let parent_before: BTreeMap<_, _> = env::vars_os().collect();
    let environment = detached_terminal_environment(&fixture.request, Some(&fixture.source))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        environment.get("BUILDDIR"),
        Some(&fixture.request.cwd.display().to_string())
    );
    assert_eq!(
        environment.get("BBPATH"),
        Some(&fixture.request.cwd.display().to_string())
    );
    assert_eq!(
        environment.get("BB_ENV_PASSTHROUGH_ADDITIONS").unwrap(),
        "MACHINE DISTRO"
    );
    assert_eq!(environment.get("YOCTUI_DETACHED_TEST").unwrap(), "selected");
    assert_eq!(env::vars_os().collect::<BTreeMap<_, _>>(), parent_before);

    let launcher = DetachedTerminalLauncher {
        program: "/usr/bin/gnome-terminal".into(),
        prefix_arguments: vec!["--wait".into(), "--".into()],
    };
    let command =
        detached_terminal_command(&launcher, &fixture.request, Some(&environment)).unwrap();
    let exported: BTreeMap<_, _> = command
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.unwrap().to_string_lossy().into_owned(),
            )
        })
        .collect();
    assert_eq!(exported, environment);
    assert_eq!(
        command.get_current_dir(),
        Some(fixture.request.cwd.as_path())
    );
    let arguments: Vec<_> = command
        .get_args()
        .map(|value| value.to_string_lossy())
        .collect();
    assert_eq!(&arguments[..2], &["--wait", "--"]);
    assert!(
        arguments
            .iter()
            .any(|argument| argument == "__menuconfig-relay")
    );
    assert_eq!(
        &arguments[arguments.len() - 3..],
        &["virtual/kernel", "-c", "menuconfig"]
    );
}

#[tokio::test]
async fn detached_menuconfig_rejects_initializer_retargeting_the_build() {
    let fixture = Fixture::new("export BUILDDIR=\"$PWD\"\n");
    let error = detached_terminal_environment(&fixture.request, Some(&fixture.source))
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("different BUILDDIR"));
}

#[tokio::test]
async fn detached_menuconfig_missing_selected_initializer_does_not_use_ancestor() {
    let fixture = Fixture::new("export BUILDDIR=\"$1\"\n");
    fs::remove_file(fixture.source.join("oe-init-build-env")).unwrap();
    fs::write(
        fixture.root.path().join("oe-init-build-env"),
        "export BUILDDIR=\"$1\"\n",
    )
    .unwrap();
    let error = detached_terminal_environment(&fixture.request, Some(&fixture.source))
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("cannot locate oe-init-build-env"));
}

#[tokio::test]
async fn detached_menuconfig_reports_initializer_failure_without_spawning_terminal() {
    let fixture = Fixture::new("echo 'selected setup failed' >&2\nreturn 3\n");
    let error = detached_terminal_environment(&fixture.request, Some(&fixture.source))
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("selected setup failed"));
}

#[tokio::test]
async fn detached_non_menuconfig_keeps_existing_environment_behavior() {
    let mut fixture = Fixture::new("return 3\n");
    for kind in [
        yoctui_model::TerminalCreationKind::Utility,
        yoctui_model::TerminalCreationKind::QemuConsole,
        yoctui_model::TerminalCreationKind::GitUi,
    ] {
        fixture.request.kind = kind;
        assert!(
            detached_terminal_environment(&fixture.request, Some(&fixture.source))
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn detached_initialized_environment_replaces_inheritance_not_just_overlays() {
    let mut fixture = Fixture::new("export BUILDDIR=\"$1\"\n");
    fixture.request.kind = yoctui_model::TerminalCreationKind::Utility;
    fixture.request.arguments.clear();
    let launcher = DetachedTerminalLauncher {
        program: "/usr/bin/env".into(),
        prefix_arguments: vec![],
    };
    let environment = BTreeMap::from([("YOCTUI_DETACHED_ONLY".to_owned(), "exact".to_owned())]);
    let mut command =
        detached_terminal_command(&launcher, &fixture.request, Some(&environment)).unwrap();
    let output = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "YOCTUI_DETACHED_ONLY=exact\n"
    );
}
