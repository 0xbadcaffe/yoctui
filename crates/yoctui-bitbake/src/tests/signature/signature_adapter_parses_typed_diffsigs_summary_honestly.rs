use super::*;

#[test]
fn signature_adapter_parses_typed_diffsigs_summary_honestly() {
    let (differences, limitations) = parse_diffsigs_output(
        "basehash changed from old to new\n\
             Variable CC value changed from 'gcc' to 'clang'\n\
             Dependency on variable CFLAGS was added\n\
             recursive detail",
    );
    assert_eq!(differences.len(), 3);
    assert_eq!(limitations.len(), 1);
}
