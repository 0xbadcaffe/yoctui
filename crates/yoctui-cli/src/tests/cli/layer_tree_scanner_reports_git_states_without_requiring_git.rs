use super::*;

#[test]
fn layer_tree_scanner_reports_git_states_without_requiring_git() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-layer-tree-git-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(".gitignore"), "ignored.bin\n").unwrap();
    fs::write(directory.join("tracked.bb"), "SUMMARY = \"first\"\n").unwrap();
    let initialized = ProcessCommand::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(&directory)
        .status()
        .is_ok_and(|status| status.success());
    if initialized {
        assert!(
            ProcessCommand::new("git")
                .args(["add", ".gitignore", "tracked.bb"])
                .current_dir(&directory)
                .status()
                .unwrap()
                .success()
        );
        assert!(
            ProcessCommand::new("git")
                .args([
                    "-c",
                    "user.name=Yoctui Test",
                    "-c",
                    "user.email=yoctui@example.invalid",
                    "commit",
                    "-qm",
                    "fixture",
                ])
                .current_dir(&directory)
                .status()
                .unwrap()
                .success()
        );
        fs::write(directory.join("tracked.bb"), "SUMMARY = \"changed\"\n").unwrap();
        fs::write(directory.join("new.bb"), "SUMMARY = \"new\"\n").unwrap();
        fs::write(directory.join("ignored.bin"), [1, 2, 3]).unwrap();
        let entries = scan_layer_directory(&directory, true).unwrap();
        let state = |name: &str| {
            entries
                .iter()
                .find(|entry| entry.path.ends_with(name))
                .unwrap()
                .git
        };
        assert_eq!(state("tracked.bb"), GitFileState::Modified);
        assert_eq!(state("new.bb"), GitFileState::Untracked);
        assert_eq!(state("ignored.bin"), GitFileState::Ignored);
    } else {
        assert!(
            scan_layer_directory(&directory, true)
                .unwrap()
                .iter()
                .all(|entry| entry.git == GitFileState::Unavailable)
        );
    }
    fs::remove_dir_all(directory).unwrap();
}
