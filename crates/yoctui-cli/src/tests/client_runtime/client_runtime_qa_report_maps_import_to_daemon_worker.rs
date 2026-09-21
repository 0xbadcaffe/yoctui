use super::*;

#[test]
fn client_runtime_qa_report_maps_import_to_daemon_worker() {
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = Some("/build".into());
    let request = yoctui_model::QaReportRequest::new(1, vec!["/tmp/report.json".into()]).unwrap();
    let effect = Effect::Qa(yoctui_model::QaEffect::ImportReports(request));
    assert!(matches!(
        daemon_command_for_effect(&app, &effect).unwrap(),
        Some(DaemonCommand::StartQaReportScan { generation: 1, build_directory, .. }) if build_directory == "/build"
    ));
}
