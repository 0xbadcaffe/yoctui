use super::*;

#[tokio::test]
async fn process_backend_rejects_unavailable_variable_detail() {
    let mut backend = ProcessBackend::new(std::env::temp_dir());

    let error = backend
        .get_variable("MACHINE".into(), None)
        .await
        .expect_err("process mode must not fabricate an empty variable detail");

    assert!(
        error
            .to_string()
            .contains("cannot inspect authoritative variable detail")
    );
}
