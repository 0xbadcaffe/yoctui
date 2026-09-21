use super::*;

#[test]
fn compatibility_utilities_rejects_stale_unknown_and_wrong_implementation() {
    let authority = authority(9, Path::new("/yocto/build"), Path::new("/poky/runqemu"));
    assert!(matches!(
        UtilityCompatibilityAuthority::new(&authority, 8, Path::new("/yocto/build"), "runqemu"),
        Err(UtilityCompatibilityError::StaleGeneration { .. })
    ));
    assert!(matches!(
        UtilityCompatibilityAuthority::new(&authority, 9, Path::new("/yocto/build"), "wic"),
        Err(UtilityCompatibilityError::ToolIdentityUnknown(_))
    ));
    let planner =
        UtilityCompatibilityAuthority::new(&authority, 9, Path::new("/yocto/build"), "runqemu")
            .unwrap();
    assert!(matches!(
        planner.command(
            CapabilityId::RunQemu,
            "runqemu.legacy",
            vec!["qemux86-64".into()],
            UtilityRisk::Mutating
        ),
        Err(UtilityCompatibilityError::ImplementationMismatch { .. })
    ));
}
