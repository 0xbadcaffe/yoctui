use super::*;

#[test]
fn compatibility_recipetool_generates_exact_create_and_appendfile_argv() {
    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/scripts/recipetool");
    let available = [
        (
            CapabilityId::RecipetoolCreate,
            RECIPETOOL_CREATE_IMPLEMENTATION,
        ),
        (
            CapabilityId::RecipetoolCreateOutfile,
            RECIPETOOL_CREATE_OUTFILE_IMPLEMENTATION,
        ),
        (
            CapabilityId::RecipetoolAppendFile,
            RECIPETOOL_APPEND_FILE_IMPLEMENTATION,
        ),
    ];
    let authority = authority(build, executable, 7, &available, &[]);
    let planner = RecipetoolCommandPlanner::new(&authority, 7, build, executable).unwrap();
    let create = planner.operation(&create()).unwrap();
    assert_eq!(
        create.arguments(),
        [
            "create",
            "--outfile",
            "/layers/meta-demo/recipes-demo/demo.bb",
            "https://example.invalid/demo.tar.gz",
        ]
    );
    assert_eq!(
        create.required_capabilities(),
        [
            CapabilityId::RecipetoolCreate,
            CapabilityId::RecipetoolCreateOutfile,
        ]
    );
    assert_eq!(
        planner.operation(&appendfile()).unwrap().arguments(),
        ["appendfile", "/layers/meta-demo", "/etc/motd", "/work/motd"]
    );
}
