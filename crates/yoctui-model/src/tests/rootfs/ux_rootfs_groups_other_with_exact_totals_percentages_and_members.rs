use super::*;

#[test]
fn ux_rootfs_groups_other_with_exact_totals_percentages_and_members() {
    let inventory = RootfsPackageInventory {
        packages: vec![
            package("a", "base", 50),
            package("b", "base", 10),
            package("c", "locale", 30),
            package("d", "debug", 10),
        ],
    };
    let rows = inventory.grouped(2);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].identity,
        RootfsGroupIdentity::Category("base".into())
    );
    assert_eq!(rows[0].installed_size_bytes, 60);
    assert_eq!(rows[0].percent_basis_points, 6_000);
    assert_eq!(rows[1].identity, RootfsGroupIdentity::Other);
    assert_eq!(rows[1].installed_size_bytes, 40);
    assert_eq!(rows[1].package_count, 2);
    assert_eq!(rows[1].percent_basis_points, 4_000);
    assert_eq!(
        rows[1].members,
        [PackageIdentity::new("c"), PackageIdentity::new("d")]
    );
}
