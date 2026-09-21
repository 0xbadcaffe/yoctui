use super::*;

#[test]
fn device_tree_compile_choices_are_bounded() {
    let file = PlatformFile {
        path: "/work/board.dts".into(),
        root: "/work".into(),
        kind: PlatformFileKind::Dts,
        size_bytes: 42,
    };
    let mut dialog = DtcCompileDialog::new(PlatformComponent::UBoot, &file, "/tools/dtc".into());
    dialog.select(99);
    dialog.adjust(99);
    assert_eq!(dialog.selection, DtcCompileOption::COUNT - 1);
    assert_eq!(dialog.reserve_entries, 16);
    dialog.select(-99);
    assert_eq!(dialog.selection, 0);
}
