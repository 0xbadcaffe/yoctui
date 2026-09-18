use super::*;

#[cfg(unix)]
#[tokio::test]
async fn daemon_job_identity_is_unique_across_report_supervisors() {
    let job_ids = daemon_job_ids::DaemonJobIds::default();
    let mut qa = daemon_qa::DaemonQaReportSupervisor::new(job_ids.clone());
    let mut security = daemon_security::DaemonSecuritySupervisor::new(job_ids);
    let paths = vec!["/yoctui-fixture-missing/report.json".to_owned()];
    let qa_job = qa
        .start(1, "/yoctui-fixture-missing".into(), paths.clone())
        .unwrap();
    let security_job = security.start(1, paths).unwrap();
    assert_ne!(
        qa_job, security_job,
        "different owners cannot share a job ID"
    );
}
