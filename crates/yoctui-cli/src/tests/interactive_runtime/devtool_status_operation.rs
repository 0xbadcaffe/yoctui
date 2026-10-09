use super::*;

#[tokio::test]
async fn devtool_status_deadline_bounds_busy_bitbake_without_fabricating_nonmembership() {
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        status_with_deadline(std::future::pending(), Duration::from_millis(10)),
    )
    .await
    .unwrap();
    assert!(
        result
            .unwrap_err()
            .contains("Existing workspace status was retained")
    );
}
