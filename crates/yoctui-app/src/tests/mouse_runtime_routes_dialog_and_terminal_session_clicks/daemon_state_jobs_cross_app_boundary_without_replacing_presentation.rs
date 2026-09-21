use super::*;

#[test]
fn daemon_state_jobs_cross_app_boundary_without_replacing_presentation() {
    let mut authoritative = yoctui_model::App::new(16, 4096);
    authoritative.qemu_session_generation = 9;
    authoritative.screen = Screen::Maintenance;
    let jobs = daemon_job_state_from_app(&authoritative);

    let mut client = yoctui_model::App::new(16, 4096);
    client.screen = Screen::Layers;
    client.focus = FocusTarget::Navigator;
    install_daemon_job_replica(&mut client, &jobs);
    assert_eq!(client.qemu_sessions, authoritative.qemu_sessions);
    assert_eq!(client.background_jobs, authoritative.background_jobs);
    assert_eq!(client.screen, Screen::Layers);
    assert_eq!(client.focus, FocusTarget::Navigator);
}
