use super::*;

#[test]
fn udev_image_absolute_links_stay_in_image_and_loops_are_bounded() {
    let temp = TestRoot::new();
    fs::create_dir_all(temp.path().join("usr/lib/udev/rules.d")).unwrap();
    fs::write(
        temp.path().join("usr/lib/udev/rules.d/10-test.rules"),
        "# image only",
    )
    .unwrap();
    std::os::unix::fs::symlink("/usr/lib", temp.path().join("lib")).unwrap();
    assert_eq!(
        resolve(temp.path(), Path::new("lib/udev/rules.d/10-test.rules"), 0).unwrap(),
        Some(temp.path().join("usr/lib/udev/rules.d/10-test.rules"))
    );
    std::os::unix::fs::symlink("loop", temp.path().join("loop")).unwrap();
    assert!(resolve(temp.path(), Path::new("loop"), 0).is_err());
    assert!(resolve(temp.path(), Path::new("../outside"), 0).is_err());
}
