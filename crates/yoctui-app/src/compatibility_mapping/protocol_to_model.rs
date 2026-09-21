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
