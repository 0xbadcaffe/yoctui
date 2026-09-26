use super::*;

#[tokio::test]
async fn devtool_workspace_git_reports_repository_tracking_and_dirty_state() {
    let root = fixture_script("devtool-workspace");
    let source = root.join("sources/busybox");
    fs::create_dir_all(&source).unwrap();
    let devtool = root.join("devtool");
    let git = root.join("git");
    fs::write(
        &devtool,
        format!(
            "#!/bin/sh\nprintf '%s\\n' 'busybox: {} (/layers/core/busybox_1.0.bb)'\n",
            source.display()
        ),
    )
    .unwrap();
    fs::write(
        &git,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *rev-parse*) printf '%s\\n' '{}' ;;\n  *) printf '%s\\n' '# branch.oid abc123' '# branch.head work' '# branch.upstream origin/work' '# branch.ab +2 -1' '1 .M N... 100644 100644 100644 abc abc file.c' '? new.txt' 'u UU N... 100644 100644 100644 100644 abc abc abc conflict.c' ;;\nesac\n",
            source.display()
        ),
    )
    .unwrap();
    for script in [&devtool, &git] {
        let mut permissions = fs::metadata(script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(script, permissions).unwrap();
    }
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/core/busybox_1.0.bb".into(),
    };
    let compatibility = devtool_compatibility(&root, &devtool);
    let status = DevtoolInspector::with_programs(devtool, git)
        .inspect_with_compatibility(&root, identity.clone(), &compatibility, 1)
        .await;
    assert_eq!(status.identity, identity);
    assert_eq!(status.capability, DevtoolCapability::Available);
    assert_eq!(
        status.workspace,
        DevtoolWorkspace::Present {
            source_path: source.clone(),
            recipe_file: Some("/layers/core/busybox_1.0.bb".into()),
        }
    );
    assert_eq!(
        status.git,
        DevtoolGitState::Available {
            repository_root: Some(source.clone()),
            branch: Some("work".into()),
            upstream: Some("origin/work".into()),
            ahead: 2,
            behind: 1,
            head: Some("abc123".into()),
            modified: 1,
            untracked: 1,
            conflicted: 1,
        }
    );
    fs::remove_dir_all(root).unwrap();
}
