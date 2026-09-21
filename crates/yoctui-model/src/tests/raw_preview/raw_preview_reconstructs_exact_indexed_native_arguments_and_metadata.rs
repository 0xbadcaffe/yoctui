use super::*;

#[test]
fn raw_preview_reconstructs_exact_indexed_native_arguments_and_metadata() {
    let preview = catalog()
        .preview(&request(), Some(&authority(9, true)))
        .unwrap();
    assert_eq!(preview.executable, RawExecutable::BitBake);
    assert_eq!(
        preview.arguments,
        [
            "-c",
            "do_compile",
            "mc:lib32:virtual/kernel",
            "",
            "--verbose",
            "café value",
        ]
    );
    assert_eq!(
        preview
            .indexed_arguments
            .iter()
            .map(|argument| (argument.index, argument.value.as_str()))
            .collect::<Vec<_>>(),
        [
            (0, "bitbake"),
            (1, "-c"),
            (2, "do_compile"),
            (3, "mc:lib32:virtual/kernel"),
            (4, ""),
            (5, "--verbose"),
            (6, "café value"),
        ]
    );
    assert_eq!(preview.catalog_version, 7);
    assert_eq!(preview.capability_generation, 9);
    assert_eq!(preview.build_directory, Path::new("/work/build"));
    assert_eq!(preview.interaction, RawInteractionMode::NoninteractiveJob);
    assert_eq!(preview.safety, RawSafetyClass::Build);
    assert_eq!(preview.limitations, ["Exact fixture limitation."]);
    assert_eq!(preview.capability_issues.len(), 1);
    assert_eq!(
        preview.implementations,
        [(CapabilityId::BitBakeRawCli, "bitbake.raw.argv".into())]
    );
}
