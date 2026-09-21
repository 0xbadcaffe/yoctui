use super::*;

#[test]
fn client_runtime_qa_report_rejects_invalid_generation() {
    let mut supervisor = DaemonQaReportSupervisor::new(Default::default());
    assert!(
        supervisor
            .start(0, "/build".into(), vec!["/tmp/report.json".into()])
            .is_err()
    );
}
