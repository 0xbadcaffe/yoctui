use super::*;

#[test]
fn browser_git_cache_expires_and_invalidates_after_changes_or_mid_query_edits() {
    let mut cache = GitCache::default();
    let root = PathBuf::from("/source");
    cache.put(root.clone(), b"old".to_vec(), 0);
    assert_eq!(cache.get(&root, Instant::now()), Some(b"old".to_vec()));
    assert!(cache.get(&root, Instant::now() + TTL).is_none());
    cache.epoch.fetch_add(1, Ordering::Relaxed);
    assert!(cache.get(&root, Instant::now()).is_none());
    cache.put(root.clone(), b"raced".to_vec(), 0);
    assert!(cache.get(&root, Instant::now()).is_none());
    cache.put(root.clone(), b"fresh".to_vec(), 1);
    assert_eq!(cache.get(&root, Instant::now()), Some(b"fresh".to_vec()));
}

#[test]
fn browser_git_cache_has_byte_entry_and_oversized_output_bounds() {
    let mut cache = GitCache::default();
    for i in 0..40 {
        cache.put(
            PathBuf::from(format!("/source-{i}")),
            vec![0; 1024 * 1024],
            0,
        );
    }
    assert!(cache.entries.len() <= MAX_ENTRIES);
    assert!(cache.bytes <= MAX_CACHE_BYTES);
    let count = cache.entries.len();
    cache.put("/huge".into(), vec![0; MAX_CACHE_BYTES + 1], 0);
    assert_eq!(cache.entries.len(), count);
}

#[tokio::test]
async fn browser_git_cache_watch_invalidates_file_content_edits_with_ttl_fallback() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("file.txt");
    std::fs::write(&path, "first").unwrap();
    let mut cache = GitCache::default();
    cache.observe(directory.path());
    let epoch = cache.epoch.load(Ordering::Relaxed);
    cache.put(directory.path().to_owned(), b"clean".to_vec(), epoch);
    if cache.watcher.is_some() {
        std::fs::write(path, "changed").unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            while cache.epoch.load(Ordering::Relaxed) == epoch {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert!(cache.get(directory.path(), Instant::now()).is_none());
    } else {
        assert!(cache.get(directory.path(), Instant::now() + TTL).is_none());
    }
    assert!(cache.watched.len() <= MAX_WATCHES);
}

#[test]
fn browser_git_decorations_handle_nested_paths_unicode_newlines_and_priority() {
    use crate::layer_browser::decorate_entries;
    let root = PathBuf::from("/source");
    let mut entries: Vec<_> = ["dir", "工程\nfile.bb", "unchanged"]
        .iter()
        .map(|name| LayerBrowserEntry {
            path: root.join(name),
            ..Default::default()
        })
        .collect();
    decorate_entries(
        &root,
        &mut entries,
        Some("?? dir/new\0 M dir/tracked\0!! 工程\nfile.bb\0".as_bytes()),
    );
    assert_eq!(entries[0].git, crate::GitFileState::Modified);
    assert_eq!(entries[1].git, crate::GitFileState::Ignored);
    assert_eq!(entries[2].git, crate::GitFileState::Clean);
    for malformed in [
        b" M ../outside\0".as_slice(),
        b" M \xff\0",
        b" M missing-delimiter",
        "☃ file\0".as_bytes(),
        b"wrong\0",
    ] {
        decorate_entries(&root, &mut entries, Some(malformed));
        assert!(
            entries
                .iter()
                .all(|entry| entry.git == crate::GitFileState::Unavailable)
        );
    }
}

#[tokio::test]
async fn browser_git_real_repository_forces_directory_relative_nul_paths() {
    use crate::layer_browser::{decorate_entries, scan_layer_directory_stream};
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "--quiet"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.invalid"]);
    git(&["config", "status.relativePaths", "false"]);
    let nested = root.join("nested 工程\n");
    std::fs::create_dir(&nested).unwrap();
    std::fs::write(nested.join("tracked"), "initial").unwrap();
    std::fs::write(root.join(".gitignore"), "*.ignored\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "fixture"]);
    std::fs::write(nested.join("tracked"), "changed").unwrap();
    std::fs::write(nested.join("工程\nfile"), "new").unwrap();
    std::fs::write(nested.join("build.ignored"), "ignored").unwrap();
    let output = crate::source_git::directory_git_status(&nested)
        .await
        .unwrap();
    let mut entries = scan_layer_directory_stream(&nested, |_| true).unwrap();
    decorate_entries(&nested, &mut entries, Some(&output));
    for (name, expected) in [
        ("tracked", crate::GitFileState::Modified),
        ("工程\nfile", crate::GitFileState::Untracked),
        ("build.ignored", crate::GitFileState::Ignored),
    ] {
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry.path == nested.join(name))
                .unwrap()
                .git,
            expected
        );
    }
    for (name, expected) in [
        ("untracked", crate::GitFileState::Untracked),
        ("cache.ignored", crate::GitFileState::Ignored),
    ] {
        let directory = root.join(name);
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("file"), "contents").unwrap();
        let output = crate::source_git::directory_git_status(&directory)
            .await
            .unwrap();
        let mut entries = scan_layer_directory_stream(&directory, |_| true).unwrap();
        decorate_entries(&directory, &mut entries, Some(&output));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].git, expected, "whole directory status: {name}");
    }
}

#[test]
fn browser_git_prefix_normalization_rejects_incomplete_or_unrelated_paths() {
    use crate::layer_browser::directory_relative_git_output as relative;
    assert_eq!(
        relative(b"nested/\n", b" M nested/file\0"),
        Some(b" M file\0".to_vec())
    );
    assert_eq!(
        relative(b"nested/\n", b"?? nested/\0"),
        Some(b"?? .\0".to_vec())
    );
    assert_eq!(relative(b"\n", b" M file\0"), Some(b" M file\0".to_vec()));
    assert!(relative(b"nested/\n", b" M outside/file\0").is_none());
    assert!(relative(b"nested/\n", b" M nested/file").is_none());
    assert!(relative(b"../\n", b" M ../file\0").is_none());
    assert!(relative(b"nested/", b"").is_none());
    assert!(relative(b"\xff/\n", b"").is_none());
}
