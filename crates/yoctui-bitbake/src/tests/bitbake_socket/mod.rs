use super::*;
use crate::{BitBakeServerController, BitBakeServerLifecycle};
use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt};
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
    CapabilitySnapshot, CapabilityState, DaemonCompatibilitySnapshot, IdentityAuthority,
    YoctoEnvironmentIdentity,
};

fn fixture() -> (PathBuf, PathBuf, BitBakeServerContext) {
    let nonce = std::time::SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "yoctui-bitbake-socket-{}-{nonce}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("build/conf")).unwrap();
    fs::create_dir_all(root.join("source")).unwrap();
    let script = root.join("fake-bridge.py");
    fs::write(
            &script,
            r#"#!/usr/bin/env python3
import json, os, socket, sys
path = os.path.join(os.getcwd(), "bitbake.sock")
try: os.unlink(path)
except FileNotFoundError: pass
server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(path)
sequence = 0
for raw in sys.stdin:
    request = json.loads(raw)
    kind = request["message"]["type"]
    sequence += 1
    if kind == "hello":
        compatibility = request["message"].get("compatibility")
        message = {"type":"hello_ack","bitbake_version":"2.8.1","compatibility_generation":compatibility["generation"],"capabilities":[capability["id"] for capability in compatibility["capabilities"]]}
    elif kind == "inspect_workspace": message = {"type":"workspace","data":{"build_dir":os.getcwd(),"source_dir":os.path.dirname(os.getcwd()),"variables":{},"bitbake_version":"2.8.1","layers":[],"recipes":[]}}
    elif kind == "shutdown": message = {"type":"bridge_shutdown"}
    elif kind == "terminate_server": message = {"type":"server_terminated"}
    else: message = {"type":"command_failed","code":"unsupported","message":kind}
    print(json.dumps({"protocol_version":1,"sequence":sequence,"correlation_id":request.get("correlation_id"),"message":message}), flush=True)
    if kind in ("shutdown", "terminate_server"):
        if kind == "terminate_server":
            server.close()
            try: os.unlink(path)
            except FileNotFoundError: pass
        break
"#,
        )
        .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let context = BitBakeServerContext {
        source_dir: root.join("source"),
        build_dir: root.join("build"),
        init_script: root.join("source/oe-init-build-env"),
    };
    (root, script, context)
}

fn test_adapter(script: PathBuf, context: &BitBakeServerContext) -> BitBakeSocketAdapter {
    let capabilities = [
        (
            CapabilityId::BitBakeWorkspaceInspection,
            "tinfoil.workspace",
        ),
        (CapabilityId::BitBakeBuild, "tinfoil.build"),
        (CapabilityId::BitBakeCancellation, "tinfoil.cancel"),
        (CapabilityId::BitBakeNativeEvents, "tinfoil.native_events"),
        (CapabilityId::BitBakeServerSocket, "bitbake.server_socket"),
    ];
    let compatibility = DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 1,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    context.build_dir.clone(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: capabilities
                .iter()
                .map(|(id, _)| CapabilityRecord {
                    id: *id,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::BackendNegotiation,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: id.as_str().into(),
                        detail: "The fake bridge explicitly exposes this operation.".into(),
                        argv: Vec::new(),
                    }],
                })
                .collect(),
        },
        implementations: capabilities
            .iter()
            .map(|(id, implementation)| {
                (
                    *id,
                    CapabilityImplementation {
                        id: (*implementation).into(),
                        kind: CapabilityImplementationKind::BackendApi,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
    .normalize()
    .unwrap();
    BitBakeSocketAdapter::new("python3", script, BTreeMap::new())
        .unwrap()
        .with_compatibility(compatibility)
        .unwrap()
}

mod bitbake_socket_starts_connects_correlates_and_stops_supported_server;

mod bitbake_socket_rejects_non_socket_and_reports_server_loss;
