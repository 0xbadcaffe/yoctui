pub fn workspace_requirement_availability(
    authority: Option<&DaemonCompatibilitySnapshot>,
    requirement: &WorkspaceEffectRequirement,
) -> WorkspaceAvailability {
    match requirement {
        WorkspaceEffectRequirement::ClientLocal => WorkspaceAvailability {
            state: WorkspaceAvailabilityState::Available,
            issues: Vec::new(),
            implementations: Vec::new(),
        },
        WorkspaceEffectRequirement::DaemonProbe { capabilities } => WorkspaceAvailability {
            state: WorkspaceAvailabilityState::Unsupported,
            issues: vec![WorkspaceCapabilityIssue {
                capability: None,
                reason: format!(
                    "Environment probing is daemon-owned; request a correlated reprobe for: {}.",
                    capability_names(capabilities)
                ),
            }],
            implementations: Vec::new(),
        },
        WorkspaceEffectRequirement::Capabilities { all, any } => {
            capability_requirement_availability(authority, all, any)
        }
    }
}

fn capability_requirement_availability(
    authority: Option<&DaemonCompatibilitySnapshot>,
    all: &[CapabilityId],
    any: &[CapabilityId],
) -> WorkspaceAvailability {
    let Some(authority) = authority else {
        let capabilities = all.iter().chain(any).copied().collect::<Vec<_>>();
        return WorkspaceAvailability {
            state: WorkspaceAvailabilityState::Unknown,
            issues: capabilities
                .into_iter()
                .map(|capability| WorkspaceCapabilityIssue {
                    capability: Some(capability),
                    reason: format!(
                        "No current environment capability snapshot: {}.",
                        capability.as_str()
                    ),
                })
                .collect(),
            implementations: Vec::new(),
        };
    };

    let all_results = all
        .iter()
        .map(|id| capability_result(authority, *id))
        .collect::<Vec<_>>();
    let any_results = any
        .iter()
        .map(|id| capability_result(authority, *id))
        .collect::<Vec<_>>();
    let all_satisfied = all_results.iter().all(CapabilityResult::is_enabled);
    let selected_any = any_results.iter().find(|result| result.is_enabled());
    let any_satisfied = any.is_empty() || selected_any.is_some();

    if all_satisfied && any_satisfied {
        let selected = all_results.iter().chain(selected_any).collect::<Vec<_>>();
        let limited = selected.iter().any(|result| result.limited);
        let issues = selected
            .iter()
            .filter_map(|result| {
                result
                    .reason
                    .as_ref()
                    .map(|reason| WorkspaceCapabilityIssue {
                        capability: Some(result.id),
                        reason: reason.clone(),
                    })
            })
            .collect();
        let implementations = selected
            .iter()
            .filter_map(|result| {
                result
                    .implementation
                    .as_ref()
                    .map(|implementation| (result.id, implementation.clone()))
            })
            .collect();
        return WorkspaceAvailability {
            state: if limited {
                WorkspaceAvailabilityState::AvailableWithLimitations
            } else {
                WorkspaceAvailabilityState::Available
            },
            issues,
            implementations,
        };
    }

    let failures = all_results
        .into_iter()
        .filter(|result| !result.is_enabled())
        .chain(
            (!any_satisfied)
                .then_some(any_results)
                .into_iter()
                .flatten(),
        )
        .collect::<Vec<_>>();
    let state = if failures
        .iter()
        .any(|result| result.state == WorkspaceAvailabilityState::Unknown)
    {
        WorkspaceAvailabilityState::Unknown
    } else if !failures.is_empty()
        && failures
            .iter()
            .all(|result| result.state == WorkspaceAvailabilityState::Unsupported)
    {
        WorkspaceAvailabilityState::Unsupported
    } else {
        WorkspaceAvailabilityState::Unavailable
    };
    WorkspaceAvailability {
        state,
        issues: failures
            .into_iter()
            .map(|result| WorkspaceCapabilityIssue {
                capability: Some(result.id),
                reason: result
                    .reason
                    .unwrap_or_else(|| format!("{} is not enabled.", result.id.as_str())),
            })
            .collect(),
        implementations: Vec::new(),
    }
}

struct CapabilityResult {
    id: CapabilityId,
    state: WorkspaceAvailabilityState,
    limited: bool,
    reason: Option<String>,
    implementation: Option<String>,
}

impl CapabilityResult {
    fn is_enabled(&self) -> bool {
        self.state.is_enabled() && self.implementation.is_some()
    }
}

fn capability_result(
    authority: &DaemonCompatibilitySnapshot,
    id: CapabilityId,
) -> CapabilityResult {
    let Some(record) = authority.snapshot.capability(id) else {
        return CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unknown,
            limited: false,
            reason: Some(format!("{} has no capability evidence.", id.as_str())),
            implementation: None,
        };
    };
    let implementation = authority
        .implementations
        .get(&id)
        .map(|implementation| implementation.id.clone());
    match &record.state {
        CapabilityState::Available if implementation.is_some() => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Available,
            limited: false,
            reason: None,
            implementation,
        },
        CapabilityState::Available => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unknown,
            limited: false,
            reason: Some(format!(
                "{} is enabled but has no selected implementation.",
                id.as_str()
            )),
            implementation: None,
        },
        CapabilityState::AvailableWithLimitations { reason, .. } if implementation.is_some() => {
            CapabilityResult {
                id,
                state: WorkspaceAvailabilityState::AvailableWithLimitations,
                limited: true,
                reason: Some(reason.message.clone()),
                implementation,
            }
        }
        CapabilityState::AvailableWithLimitations { .. } => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unknown,
            limited: false,
            reason: Some(format!(
                "{} is limited but has no selected implementation.",
                id.as_str()
            )),
            implementation: None,
        },
        CapabilityState::Unavailable { reason } => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unavailable,
            limited: false,
            reason: Some(reason.message.clone()),
            implementation: None,
        },
        CapabilityState::Unknown { reason } => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unknown,
            limited: false,
            reason: Some(reason.message.clone()),
            implementation: None,
        },
        CapabilityState::Unsupported { reason } => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unsupported,
            limited: false,
            reason: Some(reason.message.clone()),
            implementation: None,
        },
    }
}

fn capability_names(capabilities: &[CapabilityId]) -> String {
    capabilities
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Screen rendering is always locally reachable so unavailable workflows can
/// remain discoverable. Effect-producing actions inside each destination are
/// authorized separately by `workspace_effect_requirement`.
pub const fn workspace_screen_destination(screen: Screen) -> WorkspaceDestination {
    match screen {
        Screen::Dashboard | Screen::Insights => WorkspaceDestination::Dashboard,
        Screen::Tasks => WorkspaceDestination::Tasks,
        Screen::BuildHistory => WorkspaceDestination::BuildHistory,
        Screen::Dependencies => WorkspaceDestination::Dependencies,
        Screen::Signatures => WorkspaceDestination::Signatures,
        Screen::LayerRelationships => WorkspaceDestination::Layers,
        Screen::Recipes => WorkspaceDestination::Recipes,
        Screen::Packages => WorkspaceDestination::Packages,
        Screen::Images => WorkspaceDestination::Images,
        Screen::Kernel => WorkspaceDestination::Kernel,
        Screen::Firmware => WorkspaceDestination::Firmware,
        Screen::Sdk => WorkspaceDestination::Sdk,
        Screen::Testing => WorkspaceDestination::Testing,
        Screen::Security => WorkspaceDestination::Security,
        Screen::Qa => WorkspaceDestination::Qa,
        Screen::Layers => WorkspaceDestination::Layers,
        Screen::Configuration | Screen::Bbmask => WorkspaceDestination::Configuration,
        Screen::RawMode => WorkspaceDestination::RawMode,
        Screen::TerminalSessions => WorkspaceDestination::TerminalSessions,
        Screen::Maintenance => WorkspaceDestination::Maintenance,
        Screen::Logs => WorkspaceDestination::Logs,
        Screen::Errors => WorkspaceDestination::Errors,
        Screen::Help => WorkspaceDestination::Help,
        Screen::BuildEnvironment => WorkspaceDestination::BuildEnvironment,
        Screen::Compatibility => WorkspaceDestination::Compatibility,
        Screen::Settings => WorkspaceDestination::Settings,
    }
}

