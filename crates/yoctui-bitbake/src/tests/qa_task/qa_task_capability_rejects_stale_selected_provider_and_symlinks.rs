use super::*;

#[test]
fn qa_task_capability_rejects_stale_selected_provider_and_symlinks() {
    let fixture = Fixture::new();
    let stale = input(&fixture);
    fs::remove_file(&stale.selected.file).unwrap();
    assert_eq!(
        QaTaskCapabilityInspector::new(stale).inspect(),
        Err(QaTaskCapabilityError::UnsafeSelectedScope)
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let mut linked = input(&fixture);
        let real = linked.selected.file.clone();
        let link = fixture.root.join("linked-provider.bb");
        symlink(real, &link).unwrap();
        linked.selected.file = link.clone();
        linked.scopes[0].identity.file = link;
        assert_eq!(
            QaTaskCapabilityInspector::new(linked).inspect(),
            Err(QaTaskCapabilityError::UnsafeSelectedScope)
        );
    }
}
