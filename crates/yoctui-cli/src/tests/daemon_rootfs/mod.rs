use super::*;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Semaphore, oneshot};
use yoctui_model::*;
use yoctui_protocol::{
    daemon::{DaemonInstanceId, RequestId},
    rootfs::{RootfsSourcesData, RootfsSourcesRequestData},
};

fn fixture() -> (
    PathBuf,
    RootfsSourcesRequestData,
    DaemonCompatibilitySnapshot,
    BTreeMap<String, String>,
) {
    let build = std::env::temp_dir().join(format!(
        "yoctui-rootfs-query-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&build).unwrap();
    let artifact = build.join("image.manifest");
    fs::write(&artifact, "busybox machine 1\n").unwrap();
    let fake = build.join("python-fixture");
    fs::write(&fake, r#"#!/usr/bin/python3
import json, os, sys, time
from pathlib import Path
sequence=0
for line in sys.stdin:
    envelope=json.loads(line); command=envelope['message']; kind=command['type']
    if kind=='hello':
        authority=command['compatibility']
        result={'type':'hello_ack','bitbake_version':'2.18.0','compatibility_generation':authority['generation'],'capabilities':[x['id'] for x in authority['capabilities']]}
    elif kind=='get_variable':
        assert command['recipe']=='image'
        Path('query.pid').write_text(str(os.getpid()))
        mode=os.environ.get('ROOTFS_TEST_MODE','ok')
        if mode=='hang': time.sleep(60)
        if mode=='error': result={'type':'command_failed','code':'fixture_error','message':'exact image query failed'}
        else:
            value=None if mode=='absent' else str(Path.cwd()/({'IMAGE_MANIFEST':'image.manifest','PKGDATA_DIR':'pkgdata','IMAGE_ROOTFS':'retained-rootfs'}[command['name']]))
            result={'type':'variable','name':command['name'],'recipe':command['recipe'],'value':value,'provenance':None}
    elif kind=='shutdown': result={'type':'bridge_shutdown'}
    else: raise RuntimeError(kind)
    sequence+=1
    print(json.dumps({'protocol_version':1,'sequence':sequence,'correlation_id':envelope['correlation_id'],'message':result}), flush=True)
    if kind=='shutdown': break
"#).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let mut compatibility = yoctui_bitbake::release_capability_fixtures()
        .into_iter()
        .find(|f| f.role == yoctui_bitbake::CompatibilityFixtureRole::CurrentStableCandidate)
        .unwrap()
        .command_authority(9);
    compatibility.snapshot.environment.build_directory =
        AuthoritativeValue::detected(build.clone(), IdentityAuthority::InitializedEnvironment);
    compatibility.snapshot.environment.machine =
        AuthoritativeValue::detected("machine".into(), IdentityAuthority::BitBakeDatastore);
    let record = compatibility
        .snapshot
        .capabilities
        .iter_mut()
        .find(|r| r.id == CapabilityId::BitBakeGetVar)
        .unwrap();
    record.state = CapabilityState::Available;
    compatibility.implementations.insert(
        CapabilityId::BitBakeGetVar,
        CapabilityImplementation {
            id: "tinfoil.getvar".into(),
            kind: CapabilityImplementationKind::BackendApi,
        },
    );
    let query = RootfsSourcesRequestData {
        request: yoctui_protocol::rootfs::RootfsCompositionRequestData {
            generation: 3,
            image: yoctui_protocol::rootfs::RootfsImageIdentityData {
                machine: "machine".into(),
                image: "image".into(),
                path: artifact.display().to_string(),
            },
        },
        daemon_instance_id: DaemonInstanceId([4; 16]),
        compatibility_generation: 9,
    };
    (
        build,
        query,
        compatibility,
        BTreeMap::from([("PYTHON".into(), fake.display().to_string())]),
    )
}

mod rootfs_client_ipc_negotiates_retries_only_stale_and_checks_reply_identity;
mod rootfs_client_query_requires_current_full_identity_and_compatibility;
mod rootfs_query_cancellation_timeout_and_single_worker_bound_reap_bridge;
mod rootfs_query_fake_bridge_returns_exact_or_absent_sources_and_failures;
mod rootfs_query_honors_command_implementation_and_bounds_owned_process;
mod rootfs_query_validates_full_instance_generation_machine_and_containment;
