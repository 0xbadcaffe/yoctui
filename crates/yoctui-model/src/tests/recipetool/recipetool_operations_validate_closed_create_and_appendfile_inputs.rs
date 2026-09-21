use super::*;

#[test]
fn recipetool_operations_validate_closed_create_and_appendfile_inputs() {
    RecipetoolOperation::Create {
        source: "https://example.invalid/demo.tar.gz".into(),
        outfile: "/layers/meta-demo/recipes-demo/demo.bb".into(),
    }
    .validate()
    .unwrap();
    RecipetoolOperation::AppendFile {
        destination_layer: "/layers/meta-demo".into(),
        target_path: "/etc/motd".into(),
        replacement_file: "/work/motd".into(),
    }
    .validate()
    .unwrap();
    assert!(
        RecipetoolOperation::Create {
            source: "--help".into(),
            outfile: "demo.bb".into(),
        }
        .validate()
        .is_err()
    );
}
