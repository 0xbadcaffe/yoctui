use super::*;

#[test]
fn raw_favorite_persistence_defaults_legacy_and_rejects_oversized_session() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-raw-favorite-legacy-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("session.toml");
    fs::write(&path, "theme = 'dark-pro'\n").unwrap();
    assert!(read_session(Some(&path)).unwrap().raw_favorites.is_empty());

    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(MAX_SESSION_BYTES + 1).unwrap();
    assert!(read_session(Some(&path)).is_err());
    fs::remove_file(path).unwrap();
    fs::remove_dir(directory).unwrap();
}
