use super::*;

#[test]
fn udev_normalization_preserves_distinct_paths_and_rejects_unsafe_records() {
    let rule = RootfsUdevRule {
        name: "10-test.rules".into(),
        logical_path: RootfsPathIdentity("/etc/udev/rules.d/10-test.rules".into()),
        masked: true,
        overridden_by: None,
        limitation: None,
        preview: String::new(),
        preview_truncated: false,
    };
    let mut vendor = rule.clone();
    vendor.logical_path = RootfsPathIdentity("/usr/lib/udev/rules.d/10-test.rules".into());
    vendor.overridden_by = Some(rule.logical_path.clone());
    assert_eq!(vendor.status(), "Overridden");
    let mut unsafe_rule = rule.clone();
    unsafe_rule.logical_path = RootfsPathIdentity("/../outside".into());
    let mut authority = RootfsAuthority::Available(RootfsSystemInventory {
        udev_rules: vec![rule.clone(), rule, vendor, unsafe_rule],
        ..Default::default()
    });
    let mut report = RootfsNormalizationReport::default();
    normalize_system_authority(&mut authority, &mut report);
    assert_eq!(authority.value().unwrap().udev_rules.len(), 2);
    assert_eq!(report.invalid_entries, 1);
    assert_eq!(ImagesView::UdevRules.shifted(1), ImagesView::Artifacts);
    assert_eq!(ImagesView::Artifacts.shifted(-1), ImagesView::UdevRules);
}
