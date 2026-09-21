use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use yoctui_bitbake::{CapabilityFingerprintMaterial, CapabilityProbeContext};
use yoctui_model::{
    AuthoritativeValue, CapabilityCatalog, CapabilityToolId, DistroIdentity, IdentityAuthority,
    LayerSeriesIdentity, ProtocolIdentity, ReleaseIdentity, SourceRootIdentity, SourceRootKind,
    ToolIdentity, YoctoEnvironmentIdentity,
};

use super::{
    DaemonCompatibilityError, DaemonCompatibilityRuntime,
    process_helpers::{
        authoritative_token, authoritative_value, bounded_process_environment,
        canonical_initialized_build, discover_executable, fingerprint_environment,
        parse_bitbake_version, read_bounded, run_read_only,
    },
};

impl DaemonCompatibilityRuntime {
    pub async fn detect(
        process_environment: &BTreeMap<String, String>,
    ) -> Result<Option<Self>, DaemonCompatibilityError> {
        let Some(configured_build) = process_environment.get("BUILDDIR") else {
            return Ok(None);
        };
        let build_directory = canonical_initialized_build(configured_build)?;
        let catalog = CapabilityCatalog::builtin();
        catalog.validate()?;
        let tool_ids = catalog
            .entries
            .iter()
            .flat_map(|entry| entry.required_tools.iter().copied())
            .collect::<BTreeSet<_>>();
        let path = process_environment.get("PATH").ok_or(
            DaemonCompatibilityError::InvalidStartupEnvironment(
                "initialized environment has no PATH".into(),
            ),
        )?;
        let mut tools = BTreeMap::new();
        for tool in tool_ids {
            if let Some(executable) = discover_executable(path, tool.executable_name()) {
                tools.insert(tool, executable);
            }
        }

        let bitbake_version = if let Some(bitbake) = tools.get(&CapabilityToolId::BitBake) {
            run_read_only(
                bitbake,
                &["--version"],
                &build_directory,
                process_environment,
            )
            .await
            .ok()
            .and_then(|output| parse_bitbake_version(&output))
        } else {
            None
        };

        let mut datastore = BTreeMap::new();
        if let Some(getvar) = tools.get(&CapabilityToolId::BitBakeGetVar) {
            for variable in [
                "MACHINE",
                "DISTRO",
                "DISTRO_VERSION",
                "DISTRO_CODENAME",
                "OE_VERSION",
                "COREBASE",
                "LAYERSERIES_CORENAMES",
                "BBLAYERS",
                "BB_HASHSERVE",
                "PRSERV_HOST",
            ] {
                if let Ok(value) = run_read_only(
                    getvar,
                    &["--value", variable],
                    &build_directory,
                    process_environment,
                )
                .await
                    && let Some(value) = authoritative_value(&value)
                {
                    let value = if matches!(
                        variable,
                        "MACHINE" | "DISTRO" | "DISTRO_VERSION" | "DISTRO_CODENAME" | "OE_VERSION"
                    ) {
                        authoritative_token(&value).unwrap_or(value)
                    } else {
                        value
                    };
                    datastore.insert(variable.to_owned(), value);
                }
            }
        }

        let available_tools = tools
            .iter()
            .map(|(id, executable)| ToolIdentity {
                id: id.executable_name().into(),
                executable: executable.clone(),
                version: (*id == CapabilityToolId::BitBake)
                    .then(|| bitbake_version.clone())
                    .flatten(),
            })
            .collect::<Vec<_>>();
        let source_roots = datastore
            .get("BBLAYERS")
            .into_iter()
            .flat_map(|value| value.split_whitespace())
            .filter_map(|value| fs::canonicalize(value).ok())
            .filter(|path| path.is_dir())
            .map(|path| SourceRootIdentity {
                kind: SourceRootKind::Layer,
                path,
            })
            .collect::<Vec<_>>();
        let distro = datastore
            .get("DISTRO")
            .filter(|value| !value.is_empty())
            .map(|name| DistroIdentity {
                name: name.clone(),
                version: datastore
                    .get("DISTRO_VERSION")
                    .filter(|value| !value.is_empty())
                    .cloned(),
            });
        let poky = (datastore.get("DISTRO").map(String::as_str) == Some("poky"))
            .then(|| ReleaseIdentity {
                name: datastore
                    .get("DISTRO_CODENAME")
                    .filter(|value| !value.is_empty())
                    .cloned(),
                version: datastore
                    .get("DISTRO_VERSION")
                    .filter(|value| !value.is_empty())
                    .cloned(),
            })
            .filter(|release| release.name.is_some() || release.version.is_some());
        let oe_core = datastore
            .get("LAYERSERIES_CORENAMES")
            .or_else(|| datastore.get("OE_VERSION"))
            .map(|_| ReleaseIdentity {
                name: datastore
                    .get("LAYERSERIES_CORENAMES")
                    .and_then(|value| value.split_whitespace().next())
                    .map(str::to_owned),
                version: datastore
                    .get("OE_VERSION")
                    .filter(|value| !value.is_empty())
                    .cloned(),
            })
            .filter(|release| release.name.is_some() || release.version.is_some());
        let layer_series = datastore
            .get("COREBASE")
            .filter(|value| !value.is_empty())
            .and_then(|value| fs::canonicalize(Path::new(value).join("meta")).ok())
            .filter(|root| root.is_dir())
            .zip(
                datastore
                    .get("LAYERSERIES_CORENAMES")
                    .map(|value| {
                        value
                            .split_whitespace()
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .filter(|series| !series.is_empty()),
            )
            .map(|(root, compatible_series)| {
                vec![LayerSeriesIdentity {
                    layer: "core".into(),
                    root,
                    compatible_series,
                }]
            });
        let identity = YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                build_directory.clone(),
                IdentityAuthority::InitializedEnvironment,
            ),
            source_roots: if source_roots.is_empty() {
                AuthoritativeValue::Unknown
            } else {
                AuthoritativeValue::detected(
                    source_roots,
                    IdentityAuthority::ConfiguredLayerMetadata,
                )
            },
            bitbake_version: bitbake_version.map_or(AuthoritativeValue::Unknown, |version| {
                AuthoritativeValue::detected(version, IdentityAuthority::BitBakeVersionProbe)
            }),
            oe_core: oe_core.map_or(AuthoritativeValue::Unknown, |release| {
                AuthoritativeValue::detected(release, IdentityAuthority::BitBakeDatastore)
            }),
            poky: poky.map_or(AuthoritativeValue::Unknown, |release| {
                AuthoritativeValue::detected(release, IdentityAuthority::BitBakeDatastore)
            }),
            distro: distro.map_or(AuthoritativeValue::Unknown, |distro| {
                AuthoritativeValue::detected(distro, IdentityAuthority::BitBakeDatastore)
            }),
            machine: datastore
                .get("MACHINE")
                .filter(|value| !value.is_empty())
                .cloned()
                .map_or(AuthoritativeValue::Unknown, |machine| {
                    AuthoritativeValue::detected(machine, IdentityAuthority::BitBakeDatastore)
                }),
            layer_series: layer_series.map_or(AuthoritativeValue::Unknown, |layers| {
                AuthoritativeValue::detected(layers, IdentityAuthority::ConfiguredLayerMetadata)
            }),
            available_tools: AuthoritativeValue::detected(
                available_tools,
                IdentityAuthority::ExecutableProbe,
            ),
            protocol: AuthoritativeValue::detected(
                ProtocolIdentity {
                    name: "yoctui-daemon".into(),
                    version: format!(
                        "{}.{}",
                        yoctui_protocol::daemon::ProtocolVersion::CURRENT.major,
                        yoctui_protocol::daemon::ProtocolVersion::CURRENT.minor
                    ),
                },
                IdentityAuthority::ProtocolNegotiation,
            ),
            ..YoctoEnvironmentIdentity::default()
        }
        .normalize()?;

        let bounded_environment = bounded_process_environment(process_environment);
        let backend_capabilities = if let Some(version) = identity.bitbake_version.value() {
            let python = process_environment
                .get("PYTHON")
                .map(String::as_str)
                .unwrap_or("python3");
            match yoctui_bitbake::probe_bundled_backend_capabilities(
                Path::new(python),
                &build_directory,
                &bounded_environment,
                version,
            )
            .await
            {
                Ok(capabilities) => Some(capabilities),
                Err(reason) => {
                    tracing::warn!(%reason, "direct backend capability probe was inconclusive");
                    None
                }
            }
        } else {
            None
        };
        let metadata_variables = datastore.keys().cloned().collect();
        let artifacts = [
            ("pkgdata", build_directory.join("tmp/pkgdata")),
            ("wic", build_directory.join("tmp/deploy/images")),
        ]
        .into_iter()
        .filter(|(_, path)| path.is_dir())
        .map(|(name, _)| name.to_owned())
        .collect();
        let context = CapabilityProbeContext::new(
            identity.clone(),
            build_directory.clone(),
            tools,
            bounded_environment,
            None,
            Some(metadata_variables),
            backend_capabilities,
            Some(BTreeSet::from(["state_snapshots".into()])),
            Some(artifacts),
            None,
        )?;
        let layer_configuration = read_bounded(&build_directory.join("conf/bblayers.conf"))?;
        let build_configuration = read_bounded(&build_directory.join("conf/local.conf"))?;
        let initialized_environment = fingerprint_environment(process_environment)?;
        let workspace = build_directory.to_string_lossy().into_owned();
        let key = CapabilityFingerprintMaterial {
            workspace_identity: &workspace,
            initialized_environment: &initialized_environment,
            layer_configuration: &layer_configuration,
            build_configuration: &build_configuration,
            daemon_workspace_identity: &workspace,
        }
        .key(identity)?;
        Ok(Some(Self { key, context }))
    }
}
