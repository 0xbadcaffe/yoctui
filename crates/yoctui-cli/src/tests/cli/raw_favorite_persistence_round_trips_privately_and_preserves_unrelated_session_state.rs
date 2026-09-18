use super::*;

#[test]
fn raw_favorite_persistence_round_trips_privately_and_preserves_unrelated_session_state() {
    let directory = std::env::temp_dir().join(format!(
        "yoctui-raw-favorite-persistence-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    let path = directory.join("session.toml");
    let favorite = persistent_raw_favorite();
    let mut session = Session {
        last_target: Some("core-image-minimal".into()),
        theme: Some(Theme::MatrixGreen),
        ..Session::default()
    };
    let mut app = App::new(8, 1024);
    app.raw_mode.favorites = vec![favorite.clone()];
    persist_raw_favorites(Some(&path), &mut session, &app.raw_mode.favorites).unwrap();

    let loaded = read_session(Some(&path)).unwrap();
    assert_eq!(loaded.raw_favorites.len(), 1);
    assert_eq!(&loaded.raw_favorites[0], &favorite);
    assert_eq!(loaded.last_target.as_deref(), Some("core-image-minimal"));
    assert_eq!(loaded.theme, Some(Theme::MatrixGreen));
    let mut restored = App::new(8, 1024);
    install_session_raw_favorites(&loaded, &mut restored).unwrap();
    assert_eq!(restored.raw_mode.favorites, [favorite]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            fs::symlink_metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let serialized = fs::read_to_string(&path).unwrap();
    for prohibited in [
        "process_group",
        "raw-job:",
        "raw-session:",
        "stdout",
        "stderr",
        "capability_generation",
        "build_directory",
        "preview_digest",
        "request_id",
    ] {
        assert!(!serialized.contains(prohibited), "retained {prohibited}");
    }
    fs::remove_file(path).unwrap();
    fs::remove_dir(directory).unwrap();
}
