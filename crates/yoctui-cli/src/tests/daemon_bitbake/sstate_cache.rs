use super::*;
#[tokio::test]
async fn cache_profile_rejects_wrong_build_or_invalid_literals_before_spawn() {
    let mut supervisor =
        DaemonBitBakeSupervisor::new(crate::daemon_job_ids::DaemonJobIds::default());
    let request = BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    };
    let mut cache = yoctui_model::SstateCacheProfile {
        build_directory: "/other".into(),
        settings: yoctui_model::SstateCacheSettings {
            hash_server: "auto".into(),
            ..Default::default()
        },
    };
    assert!(
        supervisor
            .start_with_optimizations("/build".into(), request.clone(), None, Some(cache.clone()))
            .unwrap_err()
            .contains("different build")
    );
    cache.build_directory = "/build".into();
    cache.settings.mirror_url = "https://user:secret@host/cache".into();
    assert!(
        supervisor
            .start_with_optimizations("/build".into(), request, None, Some(cache))
            .unwrap_err()
            .contains("credentials")
    );
    assert!(supervisor.active.is_empty());
}
