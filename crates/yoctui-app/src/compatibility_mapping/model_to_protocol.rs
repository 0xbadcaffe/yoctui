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
