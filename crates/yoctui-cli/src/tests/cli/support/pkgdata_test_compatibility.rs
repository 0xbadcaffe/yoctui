use super::*;

pub(crate) fn pkgdata_test_compatibility(
    build_directory: &Path,
    tool: &Path,
) -> yoctui_model::DaemonCompatibilitySnapshot {
    let capabilities = [
        (
            yoctui_model::CapabilityId::PkgDataGenerated,
            "pkgdata.generated",
            yoctui_model::CapabilityImplementationKind::ProcessAdapter,
        ),
        (
            yoctui_model::CapabilityId::PkgDataListPackages,
            yoctui_bitbake::PKGDATA_LIST_PACKAGES_IMPLEMENTATION,
            yoctui_model::CapabilityImplementationKind::Command,
        ),
        (
            yoctui_model::CapabilityId::PkgDataPackageInfo,
            yoctui_bitbake::PKGDATA_PACKAGE_INFO_IMPLEMENTATION,
            yoctui_model::CapabilityImplementationKind::Command,
        ),
        (
            yoctui_model::CapabilityId::PkgDataListPackageFiles,
            yoctui_bitbake::PKGDATA_LIST_PACKAGE_FILES_IMPLEMENTATION,
            yoctui_model::CapabilityImplementationKind::Command,
        ),
        (
            yoctui_model::CapabilityId::PkgDataReadValue,
            yoctui_bitbake::PKGDATA_READ_VALUE_IMPLEMENTATION,
            yoctui_model::CapabilityImplementationKind::Command,
        ),
    ];
    yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: yoctui_model::CapabilitySnapshot {
            generation: 1,
            environment: yoctui_model::YoctoEnvironmentIdentity {
                build_directory: yoctui_model::AuthoritativeValue::detected(
                    build_directory.to_owned(),
                    yoctui_model::IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: yoctui_model::AuthoritativeValue::detected(
                    vec![yoctui_model::ToolIdentity {
                        id: "oe-pkgdata-util".into(),
                        executable: tool.to_owned(),
                        version: None,
                    }],
                    yoctui_model::IdentityAuthority::ExecutableProbe,
                ),
                ..yoctui_model::YoctoEnvironmentIdentity::default()
            },
            capabilities: capabilities
                .iter()
                .map(|(id, _, _)| yoctui_model::CapabilityRecord {
                    id: *id,
                    state: yoctui_model::CapabilityState::Available,
                    evidence: vec![yoctui_model::CapabilityEvidence {
                        kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
                        outcome: yoctui_model::CapabilityEvidenceOutcome::Positive,
                        subject: format!("{} test probe", id.as_str()),
                        detail: "Fixture exposes this exact pkgdata behavior.".into(),
                        argv: vec![tool.display().to_string(), "--help".into()],
                    }],
                })
                .collect(),
        },
        implementations: capabilities
            .into_iter()
            .map(|(id, implementation, kind)| {
                (
                    id,
                    yoctui_model::CapabilityImplementation {
                        id: implementation.into(),
                        kind,
                    },
                )
            })
            .collect(),
    }
    .normalize()
    .unwrap()
}
