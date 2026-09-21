use super::*;

#[tokio::test]
async fn process_backend_collects_both_output_streams() {
    let script = fixture_script("fake-bitbake");
    fs::write(
        &script,
        "#!/bin/sh\nprintf 'NOTE: stdout line\\n'\nprintf 'WARNING: stderr line\\n' >&2\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let mut backend = shell_backend(script.clone());
    backend
        .start_build(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        })
        .await
        .unwrap();
    let mut messages = Vec::new();
    loop {
        match backend.next_event().await.unwrap() {
            BackendEvent::Log(entry) => messages.push(entry),
            BackendEvent::BuildCompleted { success, .. } => {
                assert!(success);
                break;
            }
            _ => {}
        }
    }
    fs::remove_file(script).unwrap();
    assert_eq!(messages.len(), 2);
    assert!(
        messages
            .iter()
            .any(|entry| entry.severity == Severity::Warning)
    );
}
