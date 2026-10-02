use super::*;
use tempfile::TempDir;
use yoctui_bitbake::BridgeBackend;
use yoctui_model::PlatformFileKind;

async fn fixture(staging: Option<&str>, fail_metadata: bool) -> (TempDir, BridgeBackend, PathBuf) {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let shared = root.path().join("shared-kernel-build");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&shared).unwrap();
    fs::write(source.join("board.dts"), "/dts-v1/;\n").unwrap();
    fs::write(shared.join(".config"), "CONFIG_MATCHED=y\n").unwrap();
    let script = root.path().join("bridge.py");
    fs::write(
        &script,
        r#"import json, sys
from pathlib import Path
settings = json.loads(Path(__file__).with_suffix('.json').read_text())
for line in sys.stdin:
    request = json.loads(line)
    message = request['message']
    kind = message['type']
    if kind == 'shutdown':
        response = {'type': 'bridge_shutdown'}
    elif kind == 'hello':
        response = {'type': 'hello_ack', 'bitbake_version': 'fixture'}
    else:
        assert message.get('recipe') == 'virtual/kernel', message
        if kind == 'get_recipe_metadata':
            if settings['fail_metadata']:
                response = {'type': 'command_failed', 'code': 'fixture.metadata', 'message': 'metadata unavailable'}
            else:
                response = {'type': 'recipe_metadata', 'data': {'recipe': 'virtual/kernel', 'tasks': ['do_menuconfig']}}
        elif kind == 'get_variable':
            assert message['name'] in settings['variables'], message
            response = {'type': 'variable', 'name': message['name'], 'recipe': message['recipe'], 'value': settings['variables'][message['name']]}
        else:
            raise AssertionError(message)
    print(json.dumps({'protocol_version': 1, 'sequence': request['sequence'], 'correlation_id': request['correlation_id'], 'message': response}), flush=True)
    if kind == 'shutdown':
        break
"#,
    )
    .unwrap();
    let staging = staging.map(|value| {
        if value == "absolute" {
            shared.display().to_string()
        } else {
            value.to_owned()
        }
    });
    fs::write(
        script.with_extension("json"),
        serde_json::to_vec(&serde_json::json!({
            "fail_metadata": fail_metadata,
            "variables": {
                "FILE": source.join("kernel.bb"), "S": source,
                "B": root.path().join("purged-build"),
                "WORKDIR": root.path().join("purged-work"),
                "STAGING_KERNEL_BUILDDIR": staging
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let backend = BridgeBackend::spawn("python3", script, root.path().to_owned())
        .await
        .unwrap()
        .with_platform_inspection_scope();
    (root, backend, shared)
}

#[tokio::test]
async fn kernel_inspection_queries_authoritative_staging_config_after_workdir_is_purged() {
    let (_root, mut backend, shared) = fixture(Some("absolute"), false).await;
    let action = inspect_kernel_workbench(None, &mut backend).await;
    backend.shutdown().await.unwrap();
    let Action::KernelLoaded(inventory) = action else {
        panic!("exact staging metadata must yield a typed kernel inventory");
    };
    assert_eq!(inventory.target, "virtual/kernel");
    assert_eq!(inventory.tasks, ["do_menuconfig"]);
    assert!(inventory.provider.unwrap().ends_with("source/kernel.bb"));
    assert!(inventory.roots.contains(&shared));
    let config = inventory
        .files
        .iter()
        .find(|file| file.kind == PlatformFileKind::DotConfig)
        .unwrap();
    assert_eq!(config.path, shared.join(".config"));
    assert_eq!(config.size_bytes, 17);
    assert!(
        inventory
            .files
            .iter()
            .any(|file| file.kind == PlatformFileKind::Dts)
    );
}

#[tokio::test]
async fn kernel_inspection_keeps_missing_or_relative_staging_explicit_without_guessing() {
    for staging in [None, Some("relative/other-build")] {
        let (_root, mut backend, shared) = fixture(staging, false).await;
        let action = inspect_kernel_workbench(None, &mut backend).await;
        backend.shutdown().await.unwrap();
        let Action::KernelLoaded(inventory) = action else {
            panic!("available source should remain browsable with a partial staging query");
        };
        assert!(!inventory.roots.contains(&shared));
        assert!(
            !inventory
                .files
                .iter()
                .any(|file| file.kind == PlatformFileKind::DotConfig)
        );
        assert!(
            inventory
                .limitations
                .iter()
                .any(|text| text.contains("STAGING_KERNEL_BUILDDIR"))
        );
    }
}

#[tokio::test]
async fn kernel_inspection_retains_recipe_metadata_failure_without_scanning_guessed_roots() {
    let (_root, mut backend, _shared) = fixture(Some("absolute"), true).await;
    let action = inspect_kernel_workbench(None, &mut backend).await;
    backend.shutdown().await.unwrap();
    assert!(
        matches!(action, Action::KernelFailed(message) if message.contains("metadata unavailable"))
    );
}
