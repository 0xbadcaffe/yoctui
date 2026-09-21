use super::*;

#[test]
fn device_tree_compile_options_build_deterministic_argument_vector() {
    let file = PlatformFile {
        path: "/work/board.dts".into(),
        root: "/work".into(),
        kind: PlatformFileKind::Dts,
        size_bytes: 42,
    };
    let mut dialog = DtcCompileDialog::new(PlatformComponent::Kernel, &file, "/tools/dtc".into());
    dialog.adjust(1);
    dialog.select(1);
    dialog.adjust(1);
    dialog.select(1);
    dialog.adjust(1);
    dialog.adjust(1);
    dialog.select(1);
    dialog.adjust(1);

    assert_eq!(dialog.selected_option(), DtcCompileOption::ReserveEntries);
    assert_eq!(dialog.padding_bytes, 1_024);
    assert_eq!(dialog.reserve_entries, 1);
    assert_eq!(
        dialog.arguments(),
        [
            "-I",
            "dts",
            "-O",
            "dtb",
            "-@",
            "-s",
            "-p",
            "1024",
            "-R",
            "1",
            "-o",
            "/work/board.yoctui.dtb",
            "/work/board.dts",
        ]
    );
    assert_eq!(
        dialog.terminal_request().program,
        PathBuf::from("/tools/dtc")
    );
}
