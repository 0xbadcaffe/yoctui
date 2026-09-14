//! Capability actions.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipetoolActionAvailability {
    pub capability: yoctui_model::CapabilityId,
    pub available: bool,
    pub reason: Option<String>,
}

pub fn compatibility_recipetool_actions(
    compatibility: Option<&yoctui_model::DaemonCompatibilitySnapshot>,
) -> Vec<RecipetoolActionAvailability> {
    [
        yoctui_model::CapabilityId::RecipetoolCreateOutfile,
        yoctui_model::CapabilityId::RecipetoolAppendFile,
    ]
    .into_iter()
    .map(|capability| {
        let state = compatibility
            .and_then(|snapshot| snapshot.snapshot.capability(capability))
            .map(|record| &record.state);
        let available = state.is_some_and(yoctui_model::CapabilityState::is_enabled);
        let reason = if available {
            None
        } else {
            Some(
                state
                    .and_then(yoctui_model::CapabilityState::reason)
                    .map(|reason| reason.message.clone())
                    .unwrap_or_else(|| {
                        format!(
                            "{} requires the current environment capability snapshot.",
                            capability.as_str()
                        )
                    }),
            )
        };
        RecipetoolActionAvailability {
            capability,
            available,
            reason,
        }
    })
    .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerActionAvailability {
    pub capability: yoctui_model::CapabilityId,
    pub available: bool,
    pub reason: Option<String>,
}

pub fn compatibility_layer_actions(
    compatibility: Option<&yoctui_model::DaemonCompatibilitySnapshot>,
) -> Vec<LayerActionAvailability> {
    [
        yoctui_model::CapabilityId::BitBakeLayerInventory,
        yoctui_model::CapabilityId::BitBakeLayerRelationships,
        yoctui_model::CapabilityId::BitBakeLayersShowLayers,
        yoctui_model::CapabilityId::BitBakeLayersCreateLayer,
        yoctui_model::CapabilityId::BitBakeLayersCreateAndAddLayer,
        yoctui_model::CapabilityId::BitBakeLayersAddLayer,
        yoctui_model::CapabilityId::BitBakeLayersRemoveLayer,
    ]
    .into_iter()
    .map(|capability| {
        let state = compatibility
            .and_then(|snapshot| snapshot.snapshot.capability(capability))
            .map(|record| &record.state);
        let available = state.is_some_and(yoctui_model::CapabilityState::is_enabled);
        let reason = (!available).then(|| {
            state
                .and_then(yoctui_model::CapabilityState::reason)
                .map(|reason| reason.message.clone())
                .unwrap_or_else(|| {
                    format!(
                        "{} requires the current environment capability snapshot.",
                        capability.as_str()
                    )
                })
        });
        LayerActionAvailability {
            capability,
            available,
            reason,
        }
    })
    .collect()
}

pub fn compatibility_pkgdata_actions(
    compatibility: Option<&yoctui_model::DaemonCompatibilitySnapshot>,
) -> Vec<LayerActionAvailability> {
    [
        yoctui_model::CapabilityId::PkgDataGenerated,
        yoctui_model::CapabilityId::PkgDataListPackages,
        yoctui_model::CapabilityId::PkgDataPackageInfo,
        yoctui_model::CapabilityId::PkgDataListPackageFiles,
        yoctui_model::CapabilityId::PkgDataReadValue,
        yoctui_model::CapabilityId::PkgDataLookupPackage,
        yoctui_model::CapabilityId::PkgDataFindPath,
    ]
    .into_iter()
    .map(|capability| {
        let state = compatibility
            .and_then(|snapshot| snapshot.snapshot.capability(capability))
            .map(|record| &record.state);
        let available = state.is_some_and(yoctui_model::CapabilityState::is_enabled);
        LayerActionAvailability {
            capability,
            available,
            reason: (!available).then(|| {
                state
                    .and_then(yoctui_model::CapabilityState::reason)
                    .map(|reason| reason.message.clone())
                    .unwrap_or_else(|| {
                        format!(
                            "{} requires the current environment capability snapshot.",
                            capability.as_str()
                        )
                    })
            }),
        }
    })
    .collect()
}
