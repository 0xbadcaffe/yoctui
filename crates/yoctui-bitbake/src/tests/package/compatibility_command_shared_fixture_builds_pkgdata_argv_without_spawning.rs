use super::*;

#[test]
fn compatibility_command_shared_fixture_builds_pkgdata_argv_without_spawning() {
    let mut authority = release_capability_fixtures()
        .into_iter()
        .find(|fixture| fixture.role == CompatibilityFixtureRole::LatestSupportCandidate)
        .unwrap()
        .command_authority(39);
    let build_dir = authority
        .snapshot
        .environment
        .build_directory
        .value()
        .unwrap()
        .clone();
    let tool = authority
        .snapshot
        .environment
        .available_tools
        .value()
        .unwrap()
        .iter()
        .find(|tool| tool.id == "oe-pkgdata-util")
        .unwrap()
        .executable
        .clone();
    let pkgdata_dir = build_dir.join("tmp/pkgdata");
    let context = PackageDataContext {
        build_dir,
        pkgdata_dir: pkgdata_dir.clone(),
        tool: tool.clone(),
        compatibility: authority.clone(),
    };

    let list = context
        .command(
            CapabilityId::PkgDataListPackages,
            PKGDATA_LIST_PACKAGES_IMPLEMENTATION,
            "list-pkgs",
            [OsString::from("-r")],
        )
        .unwrap();
    assert_eq!(list.executable(), tool);
    assert_eq!(
        list.arguments(),
        [
            OsString::from("-p"),
            pkgdata_dir.as_os_str().to_owned(),
            OsString::from("list-pkgs"),
            OsString::from("-r"),
        ]
    );

    let read = context
        .command(
            CapabilityId::PkgDataReadValue,
            PKGDATA_READ_VALUE_IMPLEMENTATION,
            "read-value",
            [OsString::from("RDEPENDS"), OsString::from("-n")],
        )
        .unwrap();
    assert_eq!(
        read.arguments(),
        [
            OsString::from("-p"),
            pkgdata_dir.as_os_str().to_owned(),
            OsString::from("read-value"),
            OsString::from("RDEPENDS"),
            OsString::from("-n"),
        ]
    );

    let record = authority
        .snapshot
        .capabilities
        .iter_mut()
        .find(|record| record.id == CapabilityId::PkgDataReadValue)
        .unwrap();
    record.state = CapabilityState::Unavailable {
        reason: yoctui_model::CapabilityReason::new(
            "fixture.command_absent",
            "The fixture does not expose read-value.",
            Some("Required command: read-value".into()),
        )
        .unwrap(),
    };
    record.evidence[0].outcome = CapabilityEvidenceOutcome::Negative;
    authority
        .implementations
        .remove(&CapabilityId::PkgDataReadValue);
    let unavailable = PackageDataContext {
        build_dir: context.build_dir,
        pkgdata_dir: context.pkgdata_dir,
        tool: context.tool,
        compatibility: authority.normalize().unwrap(),
    };
    assert!(matches!(
        unavailable.command(
            CapabilityId::PkgDataReadValue,
            PKGDATA_READ_VALUE_IMPLEMENTATION,
            "read-value",
            [OsString::from("RDEPENDS")],
        ),
        Err(PackageDataAdapterError::CapabilityUnavailable {
            capability: CapabilityId::PkgDataReadValue,
            ..
        })
    ));
}
