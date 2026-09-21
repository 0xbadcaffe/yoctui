use super::*;

#[test]
fn rootfs_query_validates_full_instance_generation_machine_and_containment() {
    let (build, query, compatibility, _) = fixture();
    assert_eq!(
        validate_authority(&query, query.daemon_instance_id, &compatibility).unwrap(),
        build
    );
    let mut different = query.daemon_instance_id;
    different.0[15] ^= 1;
    assert!(validate_authority(&query, different, &compatibility).is_err());
    let mut bad = query.clone();
    bad.compatibility_generation += 1;
    assert!(validate_authority(&bad, query.daemon_instance_id, &compatibility).is_err());
    let mut bad = query.clone();
    bad.request.image.machine = "other".into();
    assert!(validate_authority(&bad, query.daemon_instance_id, &compatibility).is_err());
    let link = build.join("linked");
    std::os::unix::fs::symlink("image.manifest", &link).unwrap();
    let mut bad = query.clone();
    bad.request.image.path = link.display().to_string();
    assert!(validate_authority(&bad, query.daemon_instance_id, &compatibility).is_err());
    let mut bad = query.clone();
    bad.request.image.path = "/etc/passwd".into();
    assert!(validate_authority(&bad, query.daemon_instance_id, &compatibility).is_err());
    fs::remove_dir_all(build).unwrap();
}
