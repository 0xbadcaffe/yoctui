use super::*;

#[test]
fn archive_round_trip_retains_distinct_builds_and_private_bounds() {
    let root = Root::new();
    assert!(read(&root.0).unwrap().builds.is_empty());
    for id in 0..35 {
        save(&root.0, record(id)).unwrap();
    }
    let archive = read(&root.0).unwrap();
    assert_eq!(archive.builds.len(), 32);
    assert_eq!(archive.builds[0].id, "34");
    assert_eq!(archive.builds[31].id, "3");
    assert_ne!(archive.builds[0].logs, archive.builds[1].logs);
    let file = root.0.join("yoctui/build-history/history.json");
    assert_eq!(fs::metadata(file).unwrap().mode() & 0o777, 0o600);
    save(&root.0, record(34)).unwrap();
    assert_eq!(read(&root.0).unwrap().builds.len(), 32);
}
