use super::*;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use yoctui_bitbake::CapabilityFingerprintMaterial;
use yoctui_model::{
    AuthoritativeValue, IdentityAuthority, LayerSeriesIdentity, YoctoEnvironmentIdentity,
};

static NEXT_RUNTIME: AtomicU64 = AtomicU64::new(1);

fn environment(build: PathBuf, version: &str) -> YoctoEnvironmentIdentity {
    YoctoEnvironmentIdentity {
        build_directory: AuthoritativeValue::detected(
            build,
            IdentityAuthority::InitializedEnvironment,
        ),
        bitbake_version: AuthoritativeValue::detected(
            version.into(),
            IdentityAuthority::BitBakeVersionProbe,
        ),
        ..YoctoEnvironmentIdentity::default()
    }
}

fn key(environment: YoctoEnvironmentIdentity, workspace: &str) -> CapabilityCacheKey {
    CapabilityFingerprintMaterial {
        workspace_identity: workspace,
        initialized_environment: b"PATH=/work/poky/bitbake/bin",
        layer_configuration: b"BBLAYERS=/work/poky/meta",
        build_configuration: b"MACHINE=qemux86-64\nDISTRO=poky",
        daemon_workspace_identity: workspace,
    }
    .key(environment)
    .unwrap()
}

fn context(environment: YoctoEnvironmentIdentity) -> CapabilityProbeContext {
    CapabilityProbeContext::new(
        environment.clone(),
        environment.build_directory.value().unwrap().clone(),
        BTreeMap::new(),
        BTreeMap::new(),
        Some(BTreeSet::from([
            "do_build".into(),
            "do_populate_sdk".into(),
        ])),
        Some(BTreeSet::from(["MACHINE".into(), "DISTRO".into()])),
        Some(BTreeSet::from(["workspace_inspection".into()])),
        Some(BTreeSet::from(["state_snapshots".into()])),
        Some(BTreeSet::new()),
        Some(BTreeSet::new()),
    )
    .unwrap()
}

fn temporary_build(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "yoctui-daemon-compatibility-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path.canonicalize().unwrap()
}

struct RuntimeFixture {
    root: PathBuf,
    build: PathBuf,
    bin: PathBuf,
    environment: BTreeMap<String, String>,
}

impl RuntimeFixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "yoctui-daemon-compatibility-runtime-{}-{}",
            std::process::id(),
            NEXT_RUNTIME.fetch_add(1, Ordering::Relaxed)
        ));
        let build = root.join("build");
        let bin = root.join("bin");
        let layer = root.join("layers/meta");
        fs::create_dir_all(build.join("conf")).unwrap();
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&layer).unwrap();
        fs::write(
            build.join("conf/local.conf"),
            "MACHINE = \"qemux86-64\"\nDISTRO = \"poky\"\n",
        )
        .unwrap();
        fs::write(
            build.join("conf/bblayers.conf"),
            format!("BBLAYERS = \"{}\"\n", layer.display()),
        )
        .unwrap();
        write_tool(
            &bin.join("bitbake"),
            "case \"$1\" in\n  --version) echo 'BitBake Build Tool Core version 2.18.0' ;;\n  --help) echo 'usage: bitbake -e -g -f -c --dry-run --status-only --server-only --kill-server' ;;\n  *) echo ok ;;\nesac",
        );
        write_tool(
            &bin.join("bitbake-getvar"),
            &format!(
                "if [ \"$1\" = --help ]; then echo 'usage: bitbake-getvar --value --recipe'; exit 0; fi\ncase \"$2\" in\n  MACHINE) echo qemux86-64 ;;\n  DISTRO) echo poky ;;\n  DISTRO_VERSION) echo 6.0.2 ;;\n  DISTRO_CODENAME) echo wrynose ;;\n  OE_VERSION) echo 5.3 ;;\n  COREBASE) echo '{}' ;;\n  LAYERSERIES_CORENAMES) echo wrynose ;;\n  BBLAYERS) echo '{}' ;;\n  BB_HASHSERVE|PRSERV_HOST) echo '' ;;\nesac",
                layer.parent().unwrap().display(),
                layer.display()
            ),
        );
        let environment = BTreeMap::from([
            ("BUILDDIR".into(), build.display().to_string()),
            ("PATH".into(), bin.display().to_string()),
            ("HOME".into(), root.display().to_string()),
        ]);
        Self {
            root,
            build,
            bin,
            environment,
        }
    }
}

impl Drop for RuntimeFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_tool(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions).unwrap();
}

mod authoritative_value_ignores_bitbake_diagnostics;
mod command_probes_are_grouped_by_executable_for_serial_scheduling;
mod daemon_compatibility_backend_probe_enables_future_api_only_with_current_evidence;
mod daemon_compatibility_directory_resolution_keeps_unsafe_tools_rejected;
mod daemon_compatibility_discovers_directory_symlinks_and_preserves_tool_alias_name;
mod daemon_compatibility_probes_once_and_reuses_one_snapshot_for_all_clients;
mod daemon_compatibility_runtime_bounds_and_rejects_invalid_initialized_input;
mod daemon_compatibility_runtime_doctor_compatibility_publishes_initialized_authority;
mod daemon_compatibility_runtime_refuses_host_path_without_initialized_build;
mod raw_capability_probe_daemon_publishes_and_reuses_option_authority;
mod raw_capability_probe_environment_change_rejects_stale_result;
