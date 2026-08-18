use crate::{CapabilityId, CapabilitySnapshot, CapabilityState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtilityCompatibilityState {
    Available,
    AvailableWithLimitations,
    Unavailable,
    IntentionallyUnsupported,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtilityCompatibilityDefinition {
    pub id: &'static str,
    pub executables: &'static [&'static str],
    pub capabilities: &'static [CapabilityId],
    pub intentionally_unsupported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtilityCompatibilityStatus {
    pub id: &'static str,
    pub state: UtilityCompatibilityState,
    pub reason: String,
}

pub const UTILITY_COMPATIBILITY_CATALOG: &[UtilityCompatibilityDefinition] = &[
    definition(
        "environment",
        &[
            "oe-init-build-env",
            "oe-setup-builddir",
            "oe-buildenv-internal",
        ],
        &[CapabilityId::BitBakeWorkspaceInspection],
    ),
    definition(
        "bitbake",
        &["bitbake"],
        &[
            CapabilityId::BitBakeBuild,
            CapabilityId::BitBakeForceTask,
            CapabilityId::BitBakeGraphGeneration,
            CapabilityId::BitBakeEnvironmentDump,
        ],
    ),
    definition(
        "devtool",
        &["devtool"],
        &[
            CapabilityId::DevtoolStatus,
            CapabilityId::DevtoolEditRecipe,
            CapabilityId::DevtoolModify,
            CapabilityId::DevtoolUpdateRecipe,
            CapabilityId::DevtoolFinish,
            CapabilityId::DevtoolDeployTarget,
            CapabilityId::DevtoolUndeployTarget,
            CapabilityId::DevtoolReset,
            CapabilityId::DevtoolUpgrade,
        ],
    ),
    definition(
        "recipetool",
        &["recipetool"],
        &[
            CapabilityId::RecipetoolCreate,
            CapabilityId::RecipetoolCreateOutfile,
            CapabilityId::RecipetoolAppendFile,
        ],
    ),
    definition(
        "bitbake-layers",
        &["bitbake-layers"],
        &[
            CapabilityId::BitBakeLayersShowLayers,
            CapabilityId::BitBakeLayersCreateLayer,
            CapabilityId::BitBakeLayersCreateAndAddLayer,
            CapabilityId::BitBakeLayersAddLayer,
            CapabilityId::BitBakeLayersRemoveLayer,
        ],
    ),
    definition(
        "pkgdata",
        &["oe-pkgdata-util"],
        &[
            CapabilityId::PkgDataGenerated,
            CapabilityId::PkgDataListPackages,
            CapabilityId::PkgDataLookupPackage,
            CapabilityId::PkgDataPackageInfo,
            CapabilityId::PkgDataFindPath,
            CapabilityId::PkgDataListPackageFiles,
            CapabilityId::PkgDataReadValue,
        ],
    ),
    definition(
        "signatures",
        &[
            "bitbake-getvar",
            "bitbake-dumpsig",
            "bitbake-diffsigs",
            "dumpsig",
            "diffsigs",
            "whatchanged",
        ],
        &[
            CapabilityId::BitBakeGetVar,
            CapabilityId::BitBakeDumpSig,
            CapabilityId::BitBakeDiffSigs,
            CapabilityId::LockedSignatures,
        ],
    ),
    definition(
        "image-runtime",
        &["runqemu", "wic", "runqemu-extract-sdk"],
        &[CapabilityId::RunQemu, CapabilityId::WicCreate],
    ),
    definition(
        "native-tools",
        &["oe-find-native-sysroot", "oe-run-native"],
        &[],
    ),
    definition("kas", &["kas"], &[]),
    definition(
        "testing",
        &[
            "oe-selftest",
            "bitbake-selftest",
            "testimage",
            "testsdk",
            "ptest",
        ],
        &[CapabilityId::OeSelftest],
    ),
    definition("resulttool", &["resulttool"], &[CapabilityId::ResultTool]),
    definition(
        "security",
        &["cve-check", "create-spdx", "create-sbom"],
        &[CapabilityId::CveCheck, CapabilityId::SpdxCreate],
    ),
    definition(
        "yocto-tools",
        &[
            "yocto-check-layer",
            "yocto-layer",
            "yocto-bsp",
            "yocto-kernel",
        ],
        &[CapabilityId::YoctoCheckLayer],
    ),
    definition(
        "sstate",
        &["sstate-cache-management.sh", "cleanup-workdir"],
        &[],
    ),
    definition(
        "buildhistory",
        &["buildhistory-diff", "build-compare"],
        &[CapabilityId::BuildHistory, CapabilityId::LockedSignatures],
    ),
    definition(
        "release",
        &["oe-git-archive", "create-pull-request", "send-pull-request"],
        &[],
    ),
    definition("services", &["toaster", "pybootchartgui"], &[]),
    unsupported(
        "internal-workers",
        &["bitbake-worker", "bitbake-prserv", "bitbake-hashserv"],
    ),
];

const fn definition(
    id: &'static str,
    executables: &'static [&'static str],
    capabilities: &'static [CapabilityId],
) -> UtilityCompatibilityDefinition {
    UtilityCompatibilityDefinition {
        id,
        executables,
        capabilities,
        intentionally_unsupported: false,
    }
}

const fn unsupported(
    id: &'static str,
    executables: &'static [&'static str],
) -> UtilityCompatibilityDefinition {
    UtilityCompatibilityDefinition {
        id,
        executables,
        capabilities: &[],
        intentionally_unsupported: true,
    }
}

pub fn utility_compatibility_statuses(
    snapshot: Option<&CapabilitySnapshot>,
) -> Vec<UtilityCompatibilityStatus> {
    UTILITY_COMPATIBILITY_CATALOG
        .iter()
        .map(|definition| evaluate_utility(definition, snapshot))
        .collect()
}

fn evaluate_utility(
    definition: &UtilityCompatibilityDefinition,
    snapshot: Option<&CapabilitySnapshot>,
) -> UtilityCompatibilityStatus {
    if definition.intentionally_unsupported {
        return status(
            definition,
            UtilityCompatibilityState::IntentionallyUnsupported,
            "Internal utility is intentionally not user-launchable.",
        );
    }
    let Some(snapshot) = snapshot else {
        return status(
            definition,
            UtilityCompatibilityState::Unknown,
            "No current environment capability snapshot is available.",
        );
    };
    if definition.capabilities.is_empty() {
        return status(
            definition,
            UtilityCompatibilityState::Unknown,
            "Yoctui has no maintained capability probe for this utility family.",
        );
    }

    let records = definition
        .capabilities
        .iter()
        .map(|id| (*id, snapshot.capability(*id)))
        .collect::<Vec<_>>();
    let enabled = records
        .iter()
        .filter(|(_, record)| record.is_some_and(|record| record.state.is_enabled()))
        .count();
    let limited = records.iter().any(|(_, record)| {
        record.is_some_and(|record| {
            matches!(
                record.state,
                CapabilityState::AvailableWithLimitations { .. }
            )
        })
    });
    let missing = records.iter().any(|(_, record)| record.is_none());
    let all_unsupported = records.iter().all(|(_, record)| {
        record.is_some_and(|record| matches!(record.state, CapabilityState::Unsupported { .. }))
    });

    let state = if enabled == records.len() && !limited {
        UtilityCompatibilityState::Available
    } else if enabled > 0 {
        UtilityCompatibilityState::AvailableWithLimitations
    } else if missing
        || records.iter().any(|(_, record)| {
            record.is_some_and(|record| matches!(record.state, CapabilityState::Unknown { .. }))
        })
    {
        UtilityCompatibilityState::Unknown
    } else if all_unsupported {
        UtilityCompatibilityState::IntentionallyUnsupported
    } else {
        UtilityCompatibilityState::Unavailable
    };

    let reason = match state {
        UtilityCompatibilityState::Available => {
            "All maintained utility behaviors are positively evidenced.".into()
        }
        UtilityCompatibilityState::AvailableWithLimitations => capability_reasons(&records)
            .unwrap_or_else(|| "Only part of this utility family is positively evidenced.".into()),
        UtilityCompatibilityState::Unavailable => capability_reasons(&records)
            .unwrap_or_else(|| "Required utility behaviors are unavailable.".into()),
        UtilityCompatibilityState::IntentionallyUnsupported => capability_reasons(&records)
            .unwrap_or_else(|| "This utility family is intentionally unsupported.".into()),
        UtilityCompatibilityState::Unknown => capability_reasons(&records)
            .unwrap_or_else(|| "One or more utility behaviors lack conclusive evidence.".into()),
    };
    UtilityCompatibilityStatus {
        id: definition.id,
        state,
        reason,
    }
}

fn capability_reasons(
    records: &[(CapabilityId, Option<&crate::CapabilityRecord>)],
) -> Option<String> {
    let messages = records
        .iter()
        .filter_map(|(id, record)| match record {
            Some(record) => record
                .state
                .reason()
                .map(|reason| format!("{id}: {}", reason.message)),
            None => Some(format!("{id}: no capability evidence")),
        })
        .collect::<Vec<_>>();
    (!messages.is_empty()).then(|| messages.join(" "))
}

fn status(
    definition: &UtilityCompatibilityDefinition,
    state: UtilityCompatibilityState,
    reason: &str,
) -> UtilityCompatibilityStatus {
    UtilityCompatibilityStatus {
        id: definition.id,
        state,
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityReason, CapabilityRecord, YoctoEnvironmentIdentity};

    const REQUIRED: &[&str] = &[
        "oe-init-build-env",
        "bitbake",
        "devtool",
        "recipetool",
        "bitbake-layers",
        "runqemu",
        "wic",
        "kas",
        "oe-pkgdata-util",
        "bitbake-getvar",
        "bitbake-diffsigs",
        "bitbake-dumpsig",
        "oe-find-native-sysroot",
        "sstate-cache-management.sh",
        "buildhistory-diff",
        "yocto-check-layer",
        "yocto-layer",
        "yocto-bsp",
        "yocto-kernel",
        "pybootchartgui",
        "toaster",
        "resulttool",
        "oe-selftest",
        "bitbake-selftest",
    ];

    fn reason(message: &str) -> CapabilityReason {
        CapabilityReason::new("test.unavailable", message, None).unwrap()
    }

    #[test]
    fn compatibility_utilities_catalog_covers_every_registered_executable() {
        for executable in REQUIRED {
            assert!(
                UTILITY_COMPATIBILITY_CATALOG
                    .iter()
                    .any(|entry| entry.executables.contains(executable)),
                "missing utility {executable}"
            );
        }
    }

    #[test]
    fn compatibility_utilities_unknown_snapshot_never_uses_host_path() {
        let statuses = utility_compatibility_statuses(None);
        assert!(
            statuses
                .iter()
                .filter(|status| status.id != "internal-workers")
                .all(|status| status.state == UtilityCompatibilityState::Unknown)
        );
        assert_eq!(
            statuses
                .iter()
                .find(|status| status.id == "internal-workers")
                .unwrap()
                .state,
            UtilityCompatibilityState::IntentionallyUnsupported
        );
    }

    #[test]
    fn compatibility_utilities_preserves_partial_and_unavailable_reasons() {
        let mut snapshot = CapabilitySnapshot {
            generation: 7,
            environment: YoctoEnvironmentIdentity::default(),
            capabilities: vec![
                CapabilityRecord {
                    id: CapabilityId::RunQemu,
                    state: CapabilityState::Available,
                    evidence: vec![],
                },
                CapabilityRecord {
                    id: CapabilityId::WicCreate,
                    state: CapabilityState::Unavailable {
                        reason: reason("wic create is absent"),
                    },
                    evidence: vec![],
                },
                CapabilityRecord {
                    id: CapabilityId::ResultTool,
                    state: CapabilityState::Unavailable {
                        reason: reason("resulttool executable was not detected"),
                    },
                    evidence: vec![],
                },
            ],
        };
        snapshot.capabilities.sort_by_key(|record| record.id);
        let statuses = utility_compatibility_statuses(Some(&snapshot));
        let image = statuses
            .iter()
            .find(|status| status.id == "image-runtime")
            .unwrap();
        assert_eq!(
            image.state,
            UtilityCompatibilityState::AvailableWithLimitations
        );
        assert!(image.reason.contains("wic create is absent"));
        let resulttool = statuses
            .iter()
            .find(|status| status.id == "resulttool")
            .unwrap();
        assert_eq!(resulttool.state, UtilityCompatibilityState::Unavailable);
        assert!(resulttool.reason.contains("executable was not detected"));
    }
}
