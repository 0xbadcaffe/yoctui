use super::*;

#[test]
fn devwork_editor_atomic_save_preserves_permissions_and_detects_conflicts() {
    let directory = DevworkTempDir::new();
    let source = directory.0.join("main.c");
    fs::write(&source, "int main() { return 0; }\n").unwrap();
    let permissions = fs::metadata(&source).unwrap().permissions();
    let revision = TextAreaRevision::of("int main() { return 0; }\n");
    write_recipe_editor_file_atomically(
        &directory.0,
        &source,
        "int main() { return 1; }\n",
        revision,
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        "int main() { return 1; }\n"
    );
    assert_eq!(
        fs::metadata(&source).unwrap().permissions().readonly(),
        permissions.readonly()
    );

    let error = write_recipe_editor_file_atomically(
        &directory.0,
        &source,
        "int main() { return 2; }\n",
        revision,
    )
    .unwrap_err();
    assert!(error.to_string().contains("changed on disk"));
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        "int main() { return 1; }\n"
    );
}
