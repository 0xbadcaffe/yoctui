use super::*;

#[test]
fn raw_favorite_persistence_rejects_invalid_atomically_and_retains_stale_templates() {
    let directory = std::env::temp_dir().join(format!(
        "yoctui-raw-favorite-invalid-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&directory);
    let path = directory.join("session.toml");
    let favorite = persistent_raw_favorite();
    let valid = Session {
        raw_favorites: vec![favorite.clone()],
        ..Session::default()
    };
    write_session(Some(&path), &valid).unwrap();
    let before = fs::read(&path).unwrap();

    let mut future = favorite.clone();
    future.schema_version += 1;
    let invalid = Session {
        raw_favorites: vec![future],
        ..Session::default()
    };
    assert!(write_session(Some(&path), &invalid).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut app = App::new(8, 1024);
    app.raw_mode.favorites = vec![favorite.clone()];
    assert!(install_session_raw_favorites(&invalid, &mut app).is_err());
    assert_eq!(app.raw_mode.favorites.len(), 1);
    assert_eq!(&app.raw_mode.favorites[0], &favorite);

    let mut second = favorite.clone();
    second.order = 1;
    let duplicate = Session {
        raw_favorites: vec![favorite.clone(), second],
        ..Session::default()
    };
    assert!(write_session(Some(&path), &duplicate).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);

    let mut stale = favorite;
    stale.template_digest = yoctui_model::RawFavoriteTemplateDigest([9; 32]);
    let stale_session = Session {
        raw_favorites: vec![stale.clone()],
        ..Session::default()
    };
    write_session(Some(&path), &stale_session).unwrap();
    let loaded = read_session(Some(&path)).unwrap();
    assert_eq!(loaded.raw_favorites, [stale.clone()]);
    assert!(
        stale
            .project(yoctui_model::builtin_raw_catalog(), None)
            .stale
    );

    let mut malformed = stale_session;
    malformed.raw_favorites[0].schema_version += 1;
    fs::write(&path, toml::to_string(&malformed).unwrap()).unwrap();
    assert!(read_session(Some(&path)).is_err());
    fs::remove_file(path).unwrap();
    fs::remove_dir(directory).unwrap();
}
