use super::*;

#[test]
fn devwork_editor_reports_structural_diagnostics_without_claiming_compiler_authority() {
    let diagnostics = source_structural_validation(SourceLanguage::C, "int main( {\n");
    assert!(
        diagnostics
            .iter()
            .any(|span| span.message.contains("parentheses"))
    );
    assert!(
        diagnostics
            .iter()
            .any(|span| span.message.contains("braces"))
    );
    let bitbake = source_structural_validation(SourceLanguage::BitBake, "SUMMARY =\n");
    assert!(
        bitbake
            .iter()
            .any(|span| span.message.contains("assignment has no value"))
    );
}
