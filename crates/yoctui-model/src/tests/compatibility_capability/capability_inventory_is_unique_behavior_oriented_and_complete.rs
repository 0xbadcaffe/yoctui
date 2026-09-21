use super::*;

#[test]
fn capability_inventory_is_unique_behavior_oriented_and_complete() {
    let ids = CapabilityId::ALL
        .into_iter()
        .map(CapabilityId::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), CapabilityId::ALL.len());
    assert!(ids.contains("bitbake.force_task"));
    assert!(ids.contains("devtool.upgrade"));
    assert!(ids.contains("recipetool.appendfile"));
    assert!(ids.contains("pkgdata.find_path"));
    assert!(ids.contains("hashserv.diagnostics"));
    assert!(ids.iter().all(|id| !id.chars().any(char::is_whitespace)));
    assert!(
        ids.iter()
            .all(|id| !id.chars().any(|ch| ch.is_ascii_digit()))
    );
}
