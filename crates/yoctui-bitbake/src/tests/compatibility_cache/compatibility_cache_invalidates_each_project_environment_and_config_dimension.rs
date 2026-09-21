use super::*;

#[test]
fn compatibility_cache_invalidates_each_project_environment_and_config_dimension() {
    let base_environment = environment("/poky/build", "2.8.1");
    let base = material("poky-one", "daemon-one")
        .key(base_environment.clone())
        .unwrap();
    let mut changed_material = material("poky-one", "daemon-one");
    changed_material.layer_configuration = b"BBLAYERS=/poky/meta /external/meta-openembedded";
    let layer = changed_material.key(base_environment.clone()).unwrap();
    let changed_environment = environment("/poky/build", "2.10.0");
    let bitbake = material("poky-one", "daemon-one")
        .key(changed_environment)
        .unwrap();
    let project = material("poky-two", "daemon-two")
        .key(environment("/other/build", "2.8.1"))
        .unwrap();

    let mut cache = CapabilitySnapshotCache::default();
    for (expected, key) in [(1, base.clone()), (2, layer), (3, bitbake), (4, project)] {
        let selected = cache.select(key).unwrap();
        assert_eq!(selected.generation, expected);
        assert!(selected.snapshot.is_none());
        assert!(cache.lookup(&base).is_none());
    }
}
