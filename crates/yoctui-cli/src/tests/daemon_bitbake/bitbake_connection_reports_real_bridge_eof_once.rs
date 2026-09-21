use super::*;

#[tokio::test]
async fn bitbake_connection_reports_real_bridge_eof_once() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-bitbake-real-eof-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let build = root.join("build");
    let python = root.join("python");
    fs::create_dir_all(&build).unwrap();
    fs::create_dir_all(&python).unwrap();
    let build = build.canonicalize().unwrap();
    fs::write(
        python.join("bb.py"),
        format!(
            r#"import os
__version__ = "2.18.0"
class Connection:
 native_event_stream = True
 def inspect_workspace(self):
  return {{"build_dir": {build:?}, "source_dir": None, "variables": {{}}, "variable_provenance": {{}}, "variable_provenance_chain": {{}}, "bitbake_version": "2.18.0", "release": "6.0.2", "layers": [], "recipes": []}}
 def list_recipes(self, filter_value): return []
 def list_layers(self): return []
 def start_build(self, targets, task, force=False): os._exit(17)
 def drain_events(self): return []
 def shutdown(self): pass
class Server:
 def connect(self): return Connection()
server = Server()
"#,
            build = build.display().to_string(),
        ),
    )
    .unwrap();

    let mut supervisor =
        DaemonBitBakeSupervisor::new(Default::default()).with_bridge_environment(BTreeMap::from([
            ("PYTHONPATH".into(), python.display().to_string()),
        ]));
    supervisor
        .replace_compatibility(Some(cancellation_authority(&build)))
        .unwrap();
    supervisor
        .start(
            build,
            BuildRequest {
                targets: vec!["base-files".into()],
                task: None,
                force: false,
            },
        )
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut disconnects = 0;
    while Instant::now() < deadline && disconnects == 0 {
        while let Some(event) = supervisor.try_event() {
            match event {
                DaemonBitBakeEvent::Backend { event, .. }
                    if matches!(event.as_ref(), BackendEvent::Disconnected) =>
                {
                    disconnects += 1;
                }
                DaemonBitBakeEvent::Failed { message, .. } => panic!("{message}"),
                _ => {}
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert_eq!(disconnects, 1);
    assert!(supervisor.try_event().is_none());
    fs::remove_dir_all(root).unwrap();
}
