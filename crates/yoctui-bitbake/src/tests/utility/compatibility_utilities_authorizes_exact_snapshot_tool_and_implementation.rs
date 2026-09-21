use super::*;

#[test]
fn compatibility_utilities_authorizes_exact_snapshot_tool_and_implementation() {
    let authority = authority(9, Path::new("/yocto/build"), Path::new("/poky/runqemu"));
    let planner =
        UtilityCompatibilityAuthority::new(&authority, 9, Path::new("/yocto/build"), "runqemu")
            .unwrap();
    let command = planner
        .command(
            CapabilityId::RunQemu,
            "runqemu.argv",
            vec!["qemux86-64".into()],
            UtilityRisk::Mutating,
        )
        .unwrap();
    assert_eq!(command.executable, Path::new("/poky/runqemu"));
    assert_eq!(command.argv, ["qemux86-64"]);
    assert_eq!(command.cwd, Path::new("/yocto/build"));
}
