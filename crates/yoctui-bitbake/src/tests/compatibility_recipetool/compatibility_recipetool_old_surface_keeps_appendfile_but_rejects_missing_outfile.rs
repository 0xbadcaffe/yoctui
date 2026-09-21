use super::*;

#[test]
fn compatibility_recipetool_old_surface_keeps_appendfile_but_rejects_missing_outfile() {
    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/scripts/recipetool");
    let authority = authority(
        build,
        executable,
        8,
        &[
            (
                CapabilityId::RecipetoolCreate,
                RECIPETOOL_CREATE_IMPLEMENTATION,
            ),
            (
                CapabilityId::RecipetoolAppendFile,
                RECIPETOOL_APPEND_FILE_IMPLEMENTATION,
            ),
        ],
        &[CapabilityId::RecipetoolCreateOutfile],
    );
    let planner = RecipetoolCommandPlanner::new(&authority, 8, build, executable).unwrap();
    planner.operation(&appendfile()).unwrap();
    assert!(matches!(
        planner.operation(&create()),
        Err(RecipetoolCompatibilityError::Unavailable {
            capability: CapabilityId::RecipetoolCreateOutfile,
            reason,
        }) if reason.contains("recipetool.create_outfile")
    ));
}
