use std::{collections::BTreeMap, fs};

use yoctui_app::{
    PtyContextAuthority, PtyContextEntry, PtyDevtoolAction, PtyDevtoolRouter,
    VerifiedPtyEnvironment,
};
use yoctui_model::{
    DevtoolCapability, DevtoolGitState, DevtoolStatus, DevtoolWorkspace, PtySessionKind,
    RecipeIdentity,
};

#[test]
fn pty_devtool_cli_composition_preserves_exact_interactive_routes() {
    let nonce = std::time::SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "yoctui-cli-pty-devtool-{}-{nonce}",
        std::process::id()
    ));
    for path in ["source", "build", "workspace"] {
        fs::create_dir_all(root.join(path)).unwrap();
    }
    let devtool = root.join("devtool");
    fs::write(&devtool, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&devtool, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let contexts = PtyContextAuthority::new(
        "workspace".into(),
        root.join("source"),
        root.join("build"),
        VerifiedPtyEnvironment {
            identity: "build-env".into(),
            shell: fs::canonicalize("/bin/sh").unwrap(),
            environment: BTreeMap::new(),
        },
        Vec::new(),
        Vec::new(),
        vec![PtyContextEntry {
            identity: "busybox-workspace".into(),
            directory: root.join("workspace"),
        }],
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let router = PtyDevtoolRouter::new(contexts, devtool).unwrap();
    let status = DevtoolStatus {
        identity: RecipeIdentity {
            name: "busybox".into(),
            file: root.join("source/busybox.bb"),
        },
        capability: DevtoolCapability::Available,
        workspace: DevtoolWorkspace::Present {
            source_path: root.join("workspace"),
            recipe_file: None,
        },
        git: DevtoolGitState::NotApplicable,
        error: None,
    };
    let shell = router
        .preview(
            &status,
            PtyDevtoolAction::WorkspaceShell {
                workspace_identity: "busybox-workspace".into(),
            },
        )
        .unwrap();
    assert_eq!(shell.kind, PtySessionKind::DevtoolShell);
    let edit = router
        .preview(&status, PtyDevtoolAction::EditRecipe)
        .unwrap();
    assert_eq!(edit.command.arguments, vec!["edit-recipe", "busybox"]);
    assert_eq!(
        router.preview(&status, PtyDevtoolAction::Modify),
        Err(yoctui_app::PtyDevtoolError::UseBackgroundJob)
    );
    fs::remove_dir_all(root).unwrap();
}
