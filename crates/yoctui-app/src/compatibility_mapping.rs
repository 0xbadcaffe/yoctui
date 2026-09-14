//! Compatibility mapping.

pub(crate) fn daemon_compatibility_protocol(
    compatibility: &yoctui_model::DaemonCompatibilitySnapshot,
) -> yoctui_protocol::daemon::CompatibilitySnapshotData {
    use yoctui_protocol::daemon::{
        COMPATIBILITY_SCHEMA_VERSION, CompatibilityCapabilityData, CompatibilityEvidenceData,
        CompatibilityEvidenceKind, CompatibilityEvidenceOutcome, CompatibilityImplementationData,
        CompatibilitySnapshotData, CompatibilityStateData,
    };

    let capabilities = compatibility
        .snapshot
        .capabilities
        .iter()
        .map(|record| CompatibilityCapabilityData {
            id: record.id.as_str().into(),
            state: match &record.state {
                yoctui_model::CapabilityState::Available => CompatibilityStateData::Available,
                yoctui_model::CapabilityState::AvailableWithLimitations {
                    reason,
                    limitations,
                } => CompatibilityStateData::AvailableWithLimitations {
                    reason: compatibility_reason_protocol(reason),
                    limitations: limitations.clone(),
                },
                yoctui_model::CapabilityState::Unavailable { reason } => {
                    CompatibilityStateData::Unavailable {
                        reason: compatibility_reason_protocol(reason),
                    }
                }
                yoctui_model::CapabilityState::Unknown { reason } => {
                    CompatibilityStateData::Unknown {
                        reason: compatibility_reason_protocol(reason),
                    }
                }
                yoctui_model::CapabilityState::Unsupported { reason } => {
                    CompatibilityStateData::Unsupported {
                        reason: compatibility_reason_protocol(reason),
                    }
                }
            },
            evidence: record
                .evidence
                .iter()
                .map(|evidence| CompatibilityEvidenceData {
                    kind: match evidence.kind {
                        yoctui_model::CapabilityEvidenceKind::DirectProbe => {
                            CompatibilityEvidenceKind::DirectProbe
                        }
                        yoctui_model::CapabilityEvidenceKind::BackendNegotiation => {
                            CompatibilityEvidenceKind::BackendNegotiation
                        }
                        yoctui_model::CapabilityEvidenceKind::ProtocolNegotiation => {
                            CompatibilityEvidenceKind::ProtocolNegotiation
                        }
                        yoctui_model::CapabilityEvidenceKind::Metadata => {
                            CompatibilityEvidenceKind::Metadata
                        }
                        yoctui_model::CapabilityEvidenceKind::ExecutableIdentity => {
                            CompatibilityEvidenceKind::ExecutableIdentity
                        }
                        yoctui_model::CapabilityEvidenceKind::ReleaseVersionFallback => {
                            CompatibilityEvidenceKind::ReleaseVersionFallback
                        }
                    },
                    outcome: match evidence.outcome {
                        yoctui_model::CapabilityEvidenceOutcome::Positive => {
                            CompatibilityEvidenceOutcome::Positive
                        }
                        yoctui_model::CapabilityEvidenceOutcome::Negative => {
                            CompatibilityEvidenceOutcome::Negative
                        }
                        yoctui_model::CapabilityEvidenceOutcome::Inconclusive => {
                            CompatibilityEvidenceOutcome::Inconclusive
                        }
                    },
                    subject: evidence.subject.clone(),
                    detail: evidence.detail.clone(),
                    argv: evidence.argv.clone(),
                })
                .collect(),
            implementation: compatibility
                .implementations
                .get(&record.id)
                .map(|implementation| CompatibilityImplementationData {
                    id: implementation.id.clone(),
                    kind: match implementation.kind {
                        yoctui_model::CapabilityImplementationKind::BackendApi => "backend_api",
                        yoctui_model::CapabilityImplementationKind::Command => "command",
                        yoctui_model::CapabilityImplementationKind::MetadataTask => "metadata_task",
                        yoctui_model::CapabilityImplementationKind::ProcessAdapter => {
                            "process_adapter"
                        }
                        yoctui_model::CapabilityImplementationKind::Protocol => "protocol",
                    }
                    .into(),
                }),
        })
        .collect();
    CompatibilitySnapshotData {
        schema_version: COMPATIBILITY_SCHEMA_VERSION,
        generation: compatibility.snapshot.generation,
        environment: compatibility_environment_protocol(&compatibility.snapshot.environment),
        capabilities,
    }
}

pub(crate) fn compatibility_reason_protocol(
    reason: &yoctui_model::CapabilityReason,
) -> yoctui_protocol::daemon::CompatibilityReasonData {
    yoctui_protocol::daemon::CompatibilityReasonData {
        code: reason.code.as_str().into(),
        message: reason.message.clone(),
        requirement: reason.requirement.clone(),
    }
}

pub(crate) fn compatibility_environment_protocol(
    environment: &yoctui_model::YoctoEnvironmentIdentity,
) -> yoctui_protocol::daemon::CompatibilityEnvironmentIdentity {
    use yoctui_protocol::daemon::{
        CompatibilityBackendIdentity, CompatibilityDistroIdentity,
        CompatibilityEnvironmentIdentity, CompatibilityLayerSeriesIdentity,
        CompatibilityProtocolIdentity, CompatibilityReleaseIdentity,
        CompatibilitySourceRootIdentity, CompatibilityToolIdentity,
    };
    CompatibilityEnvironmentIdentity {
        build_directory: compatibility_detected(&environment.build_directory, |path| {
            path.display().to_string()
        }),
        source_roots: compatibility_detected(&environment.source_roots, |roots| {
            roots
                .iter()
                .map(|root| CompatibilitySourceRootIdentity {
                    kind: match &root.kind {
                        yoctui_model::SourceRootKind::CoreBase => "core_base".into(),
                        yoctui_model::SourceRootKind::OpenEmbeddedCore => {
                            "openembedded_core".into()
                        }
                        yoctui_model::SourceRootKind::Poky => "poky".into(),
                        yoctui_model::SourceRootKind::Layer => "layer".into(),
                        yoctui_model::SourceRootKind::Other(kind) => kind.clone(),
                    },
                    path: root.path.display().to_string(),
                })
                .collect()
        }),
        bitbake_version: compatibility_detected(&environment.bitbake_version, Clone::clone),
        oe_core: compatibility_detected(&environment.oe_core, |release| {
            CompatibilityReleaseIdentity {
                name: release.name.clone(),
                version: release.version.clone(),
            }
        }),
        poky: compatibility_detected(&environment.poky, |release| CompatibilityReleaseIdentity {
            name: release.name.clone(),
            version: release.version.clone(),
        }),
        distro: compatibility_detected(&environment.distro, |distro| CompatibilityDistroIdentity {
            name: distro.name.clone(),
            version: distro.version.clone(),
        }),
        machine: compatibility_detected(&environment.machine, Clone::clone),
        layer_series: compatibility_detected(&environment.layer_series, |layers| {
            layers
                .iter()
                .map(|layer| CompatibilityLayerSeriesIdentity {
                    layer: layer.layer.clone(),
                    root: layer.root.display().to_string(),
                    compatible_series: layer.compatible_series.clone(),
                })
                .collect()
        }),
        available_tools: compatibility_detected(&environment.available_tools, |tools| {
            tools
                .iter()
                .map(|tool| CompatibilityToolIdentity {
                    id: tool.id.clone(),
                    executable: tool.executable.display().to_string(),
                    version: tool.version.clone(),
                })
                .collect()
        }),
        backend: compatibility_detected(&environment.backend, |backend| {
            CompatibilityBackendIdentity {
                name: backend.name.clone(),
                version: backend.version.clone(),
            }
        }),
        protocol: compatibility_detected(&environment.protocol, |protocol| {
            CompatibilityProtocolIdentity {
                name: protocol.name.clone(),
                version: protocol.version.clone(),
            }
        }),
    }
}

pub(crate) fn compatibility_detected<T, U>(
    value: &yoctui_model::AuthoritativeValue<T>,
    map: impl FnOnce(&T) -> U,
) -> yoctui_protocol::daemon::CompatibilityDetected<U> {
    match value {
        yoctui_model::AuthoritativeValue::Unknown => {
            yoctui_protocol::daemon::CompatibilityDetected::Unknown
        }
        yoctui_model::AuthoritativeValue::Detected { value, authority } => {
            yoctui_protocol::daemon::CompatibilityDetected::Detected {
                value: map(value),
                authority: match authority {
                    yoctui_model::IdentityAuthority::BackendHandshake => {
                        yoctui_protocol::daemon::CompatibilityIdentityAuthority::BackendHandshake
                    }
                    yoctui_model::IdentityAuthority::BitBakeDatastore => {
                        yoctui_protocol::daemon::CompatibilityIdentityAuthority::BitBakeDatastore
                    }
                    yoctui_model::IdentityAuthority::BitBakeVersionProbe => {
                        yoctui_protocol::daemon::CompatibilityIdentityAuthority::BitBakeVersionProbe
                    }
                    yoctui_model::IdentityAuthority::ConfiguredLayerMetadata => {
                        yoctui_protocol::daemon::CompatibilityIdentityAuthority::ConfiguredLayerMetadata
                    }
                    yoctui_model::IdentityAuthority::ExecutableProbe => {
                        yoctui_protocol::daemon::CompatibilityIdentityAuthority::ExecutableProbe
                    }
                    yoctui_model::IdentityAuthority::InitializedEnvironment => {
                        yoctui_protocol::daemon::CompatibilityIdentityAuthority::InitializedEnvironment
                    }
                    yoctui_model::IdentityAuthority::ProtocolNegotiation => {
                        yoctui_protocol::daemon::CompatibilityIdentityAuthority::ProtocolNegotiation
                    }
                    yoctui_model::IdentityAuthority::ReleaseMetadata => {
                        yoctui_protocol::daemon::CompatibilityIdentityAuthority::ReleaseMetadata
                    }
                },
            }
        }
    }
}

pub fn compatibility_model_snapshot(
    wire: &yoctui_protocol::daemon::CompatibilitySnapshotData,
) -> Result<yoctui_model::DaemonCompatibilitySnapshot, String> {
    use yoctui_protocol::daemon::{
        CompatibilityEvidenceKind as WireEvidenceKind,
        CompatibilityEvidenceOutcome as WireEvidenceOutcome, CompatibilityStateData,
    };

    wire.validate().map_err(|error| error.to_string())?;
    let environment = compatibility_environment_model(&wire.environment)?;
    let mut capabilities = Vec::with_capacity(wire.capabilities.len());
    let mut implementations = std::collections::BTreeMap::new();
    for capability in &wire.capabilities {
        let id = yoctui_model::CapabilityId::from_stable_name(&capability.id)
            .ok_or_else(|| format!("unknown capability ID: {}", capability.id))?;
        let state = match &capability.state {
            CompatibilityStateData::Available => yoctui_model::CapabilityState::Available,
            CompatibilityStateData::AvailableWithLimitations {
                reason,
                limitations,
            } => yoctui_model::CapabilityState::AvailableWithLimitations {
                reason: compatibility_reason_model(reason)?,
                limitations: limitations.clone(),
            },
            CompatibilityStateData::Unavailable { reason } => {
                yoctui_model::CapabilityState::Unavailable {
                    reason: compatibility_reason_model(reason)?,
                }
            }
            CompatibilityStateData::Unknown { reason } => yoctui_model::CapabilityState::Unknown {
                reason: compatibility_reason_model(reason)?,
            },
            CompatibilityStateData::Unsupported { reason } => {
                yoctui_model::CapabilityState::Unsupported {
                    reason: compatibility_reason_model(reason)?,
                }
            }
            CompatibilityStateData::UnknownWireState => {
                return Err(format!(
                    "unknown wire state for capability {}",
                    capability.id
                ));
            }
        };
        let evidence = capability
            .evidence
            .iter()
            .map(|evidence| {
                Ok(yoctui_model::CapabilityEvidence {
                    kind: match evidence.kind {
                        WireEvidenceKind::DirectProbe => {
                            yoctui_model::CapabilityEvidenceKind::DirectProbe
                        }
                        WireEvidenceKind::BackendNegotiation => {
                            yoctui_model::CapabilityEvidenceKind::BackendNegotiation
                        }
                        WireEvidenceKind::ProtocolNegotiation => {
                            yoctui_model::CapabilityEvidenceKind::ProtocolNegotiation
                        }
                        WireEvidenceKind::Metadata => {
                            yoctui_model::CapabilityEvidenceKind::Metadata
                        }
                        WireEvidenceKind::ExecutableIdentity => {
                            yoctui_model::CapabilityEvidenceKind::ExecutableIdentity
                        }
                        WireEvidenceKind::ReleaseVersionFallback => {
                            yoctui_model::CapabilityEvidenceKind::ReleaseVersionFallback
                        }
                        WireEvidenceKind::Unknown => {
                            return Err("unknown compatibility evidence kind".to_owned());
                        }
                    },
                    outcome: match evidence.outcome {
                        WireEvidenceOutcome::Positive => {
                            yoctui_model::CapabilityEvidenceOutcome::Positive
                        }
                        WireEvidenceOutcome::Negative => {
                            yoctui_model::CapabilityEvidenceOutcome::Negative
                        }
                        WireEvidenceOutcome::Inconclusive => {
                            yoctui_model::CapabilityEvidenceOutcome::Inconclusive
                        }
                        WireEvidenceOutcome::Unknown => {
                            return Err("unknown compatibility evidence outcome".to_owned());
                        }
                    },
                    subject: evidence.subject.clone(),
                    detail: evidence.detail.clone(),
                    argv: evidence.argv.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if let Some(implementation) = &capability.implementation {
            let kind = match implementation.kind.as_str() {
                "backend_api" => yoctui_model::CapabilityImplementationKind::BackendApi,
                "command" => yoctui_model::CapabilityImplementationKind::Command,
                "metadata_task" => yoctui_model::CapabilityImplementationKind::MetadataTask,
                "process_adapter" => yoctui_model::CapabilityImplementationKind::ProcessAdapter,
                "protocol" => yoctui_model::CapabilityImplementationKind::Protocol,
                value => return Err(format!("unknown capability implementation kind: {value}")),
            };
            implementations.insert(
                id,
                yoctui_model::CapabilityImplementation {
                    id: implementation.id.clone(),
                    kind,
                },
            );
        }
        capabilities.push(yoctui_model::CapabilityRecord {
            id,
            state,
            evidence,
        });
    }
    yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: yoctui_model::CapabilitySnapshot {
            generation: wire.generation,
            environment,
            capabilities,
        },
        implementations,
    }
    .normalize()
    .map_err(|error| error.to_string())
}

/// Applies a user-originated action through the single workspace capability
/// authority installed from the daemon snapshot. Local-only actions continue
/// to work without an environment snapshot; environment effects fail closed.
pub fn compatibility_workspace_action(
    app: &mut yoctui_model::App,
    action: yoctui_model::Action,
) -> Option<yoctui_model::Effect> {
    yoctui_model::update_with_workspace_authority(app, action)
}

pub(crate) fn compatibility_reason_model(
    reason: &yoctui_protocol::daemon::CompatibilityReasonData,
) -> Result<yoctui_model::CapabilityReason, String> {
    yoctui_model::CapabilityReason::new(
        reason.code.clone(),
        reason.message.clone(),
        reason.requirement.clone(),
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn compatibility_environment_model(
    wire: &yoctui_protocol::daemon::CompatibilityEnvironmentIdentity,
) -> Result<yoctui_model::YoctoEnvironmentIdentity, String> {
    Ok(yoctui_model::YoctoEnvironmentIdentity {
        build_directory: compatibility_detected_model(&wire.build_directory, |value| {
            Ok(value.into())
        })?,
        source_roots: compatibility_detected_model(&wire.source_roots, |roots| {
            roots
                .iter()
                .map(|root| {
                    Ok(yoctui_model::SourceRootIdentity {
                        kind: match root.kind.as_str() {
                            "core_base" => yoctui_model::SourceRootKind::CoreBase,
                            "openembedded_core" => yoctui_model::SourceRootKind::OpenEmbeddedCore,
                            "poky" => yoctui_model::SourceRootKind::Poky,
                            "layer" => yoctui_model::SourceRootKind::Layer,
                            other => yoctui_model::SourceRootKind::Other(other.into()),
                        },
                        path: root.path.clone().into(),
                    })
                })
                .collect()
        })?,
        bitbake_version: compatibility_detected_model(&wire.bitbake_version, |value| {
            Ok(value.clone())
        })?,
        oe_core: compatibility_detected_model(&wire.oe_core, |release| {
            Ok(yoctui_model::ReleaseIdentity {
                name: release.name.clone(),
                version: release.version.clone(),
            })
        })?,
        poky: compatibility_detected_model(&wire.poky, |release| {
            Ok(yoctui_model::ReleaseIdentity {
                name: release.name.clone(),
                version: release.version.clone(),
            })
        })?,
        distro: compatibility_detected_model(&wire.distro, |distro| {
            Ok(yoctui_model::DistroIdentity {
                name: distro.name.clone(),
                version: distro.version.clone(),
            })
        })?,
        machine: compatibility_detected_model(&wire.machine, |value| Ok(value.clone()))?,
        layer_series: compatibility_detected_model(&wire.layer_series, |layers| {
            Ok(layers
                .iter()
                .map(|layer| yoctui_model::LayerSeriesIdentity {
                    layer: layer.layer.clone(),
                    root: layer.root.clone().into(),
                    compatible_series: layer.compatible_series.clone(),
                })
                .collect())
        })?,
        available_tools: compatibility_detected_model(&wire.available_tools, |tools| {
            Ok(tools
                .iter()
                .map(|tool| yoctui_model::ToolIdentity {
                    id: tool.id.clone(),
                    executable: tool.executable.clone().into(),
                    version: tool.version.clone(),
                })
                .collect())
        })?,
        backend: compatibility_detected_model(&wire.backend, |backend| {
            Ok(yoctui_model::BackendIdentity {
                name: backend.name.clone(),
                version: backend.version.clone(),
            })
        })?,
        protocol: compatibility_detected_model(&wire.protocol, |protocol| {
            Ok(yoctui_model::ProtocolIdentity {
                name: protocol.name.clone(),
                version: protocol.version.clone(),
            })
        })?,
    })
}

pub(crate) fn compatibility_detected_model<T, U>(
    wire: &yoctui_protocol::daemon::CompatibilityDetected<T>,
    map: impl FnOnce(&T) -> Result<U, String>,
) -> Result<yoctui_model::AuthoritativeValue<U>, String> {
    use yoctui_protocol::daemon::{CompatibilityDetected, CompatibilityIdentityAuthority};
    match wire {
        CompatibilityDetected::Unknown => Ok(yoctui_model::AuthoritativeValue::Unknown),
        CompatibilityDetected::Detected { value, authority } => {
            let authority = match authority {
                CompatibilityIdentityAuthority::BackendHandshake => {
                    yoctui_model::IdentityAuthority::BackendHandshake
                }
                CompatibilityIdentityAuthority::BitBakeDatastore => {
                    yoctui_model::IdentityAuthority::BitBakeDatastore
                }
                CompatibilityIdentityAuthority::BitBakeVersionProbe => {
                    yoctui_model::IdentityAuthority::BitBakeVersionProbe
                }
                CompatibilityIdentityAuthority::ConfiguredLayerMetadata => {
                    yoctui_model::IdentityAuthority::ConfiguredLayerMetadata
                }
                CompatibilityIdentityAuthority::ExecutableProbe => {
                    yoctui_model::IdentityAuthority::ExecutableProbe
                }
                CompatibilityIdentityAuthority::InitializedEnvironment => {
                    yoctui_model::IdentityAuthority::InitializedEnvironment
                }
                CompatibilityIdentityAuthority::ProtocolNegotiation => {
                    yoctui_model::IdentityAuthority::ProtocolNegotiation
                }
                CompatibilityIdentityAuthority::ReleaseMetadata => {
                    yoctui_model::IdentityAuthority::ReleaseMetadata
                }
                CompatibilityIdentityAuthority::Unknown => {
                    return Err("unknown compatibility identity authority".into());
                }
            };
            Ok(yoctui_model::AuthoritativeValue::detected(
                map(value)?,
                authority,
            ))
        }
    }
}

pub(crate) fn stable_workspace_hash(source: &str, build: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in source.bytes().chain([0]).chain(build.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub(crate) fn daemon_bitbake_lifecycle(
    lifecycle: yoctui_model::DaemonBitBakeLifecycle,
) -> yoctui_protocol::daemon::LifecycleState {
    use yoctui_protocol::daemon::LifecycleState;
    match lifecycle {
        yoctui_model::DaemonBitBakeLifecycle::Disconnected => LifecycleState::Disconnected,
        yoctui_model::DaemonBitBakeLifecycle::Connecting => LifecycleState::Connecting,
        yoctui_model::DaemonBitBakeLifecycle::Connected => LifecycleState::Running,
        yoctui_model::DaemonBitBakeLifecycle::Stopping => LifecycleState::Stopping,
        yoctui_model::DaemonBitBakeLifecycle::Failed => LifecycleState::Failed,
        yoctui_model::DaemonBitBakeLifecycle::Recovering => LifecycleState::Connecting,
    }
}

pub(crate) fn daemon_job_kind(
    kind: yoctui_model::BackgroundJobKind,
) -> yoctui_protocol::daemon::JobKind {
    use yoctui_protocol::daemon::JobKind;
    match kind {
        yoctui_model::BackgroundJobKind::Build => JobKind::BitBakeBuild,
        yoctui_model::BackgroundJobKind::CveCheck => JobKind::Security,
        yoctui_model::BackgroundJobKind::Spdx => JobKind::Qa,
        yoctui_model::BackgroundJobKind::Qemu => JobKind::Qemu,
        yoctui_model::BackgroundJobKind::Wic => JobKind::Wic,
        yoctui_model::BackgroundJobKind::Sdk => JobKind::Sdk,
        yoctui_model::BackgroundJobKind::Test => JobKind::Testing,
        yoctui_model::BackgroundJobKind::Devtool => JobKind::Devtool,
        yoctui_model::BackgroundJobKind::Maintenance => JobKind::Maintenance,
    }
}

pub(crate) fn daemon_job_lifecycle(
    status: yoctui_model::BackgroundJobStatus,
) -> yoctui_protocol::daemon::LifecycleState {
    use yoctui_protocol::daemon::LifecycleState;
    match status {
        yoctui_model::BackgroundJobStatus::Queued | yoctui_model::BackgroundJobStatus::Starting => {
            LifecycleState::Connecting
        }
        yoctui_model::BackgroundJobStatus::Running => LifecycleState::Running,
        yoctui_model::BackgroundJobStatus::Cancelling => LifecycleState::Stopping,
        yoctui_model::BackgroundJobStatus::Succeeded => LifecycleState::Exited,
        yoctui_model::BackgroundJobStatus::Failed => LifecycleState::Failed,
        yoctui_model::BackgroundJobStatus::Cancelled => LifecycleState::Exited,
        yoctui_model::BackgroundJobStatus::Lost => LifecycleState::Lost,
    }
}
