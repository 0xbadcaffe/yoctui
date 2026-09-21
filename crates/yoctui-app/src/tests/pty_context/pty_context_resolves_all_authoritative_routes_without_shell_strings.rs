use super::*;

#[test]
fn pty_context_resolves_all_authoritative_routes_without_shell_strings() {
    let (root, authority) = fixture();
    let cases = [
        (
            PtyContextAction::BuildDirectory,
            PtySessionKind::BuildShell,
            "build",
            "build-env-7",
        ),
        (
            PtyContextAction::SourceTree,
            PtySessionKind::SourceShell,
            "source",
            "build-env-7",
        ),
        (
            PtyContextAction::SelectedLayer {
                identity: "meta-test".into(),
            },
            PtySessionKind::LayerShell,
            "layer",
            "build-env-7",
        ),
        (
            PtyContextAction::SelectedRecipeSource {
                identity: "busybox".into(),
            },
            PtySessionKind::RecipeShell,
            "recipe",
            "build-env-7",
        ),
        (
            PtyContextAction::DevtoolWorkspace {
                identity: "devtool:busybox".into(),
            },
            PtySessionKind::DevtoolShell,
            "devtool",
            "build-env-7",
        ),
        (
            PtyContextAction::DeployDirectory {
                identity: "qemux86-64".into(),
            },
            PtySessionKind::DeployShell,
            "deploy",
            "build-env-7",
        ),
        (
            PtyContextAction::SdkEnvironment {
                identity: "sdk-x86_64".into(),
            },
            PtySessionKind::SdkShell,
            "sdk",
            "sdk-env-3",
        ),
    ];
    for (action, kind, directory, environment) in cases {
        let launch = authority.resolve(action).unwrap();
        assert_eq!(launch.kind, kind);
        assert_eq!(launch.cwd, fs::canonicalize(root.join(directory)).unwrap());
        assert_eq!(launch.command.arguments, vec!["-i"]);
        assert_eq!(launch.environment_identity, environment);
        assert_eq!(launch.workspace.owner_identity, "workspace-1");
        PtySession::new(
            PtySessionSpec {
                id: PtySessionId(1),
                name: launch.name.clone(),
                kind: launch.kind,
                cwd: launch.cwd.clone(),
                command: launch.command.clone(),
                dimensions: PtyDimensions {
                    columns: 80,
                    rows: 24,
                },
                restartable: true,
                workspace: launch.workspace.clone(),
            },
            10,
        )
        .unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}
