#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawInteractionMode {
    NoninteractiveJob,
    InteractivePty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawSafetyClass {
    Inspection,
    Build,
    MetadataMutation,
    Destructive,
    ServerLifecycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawExecutable {
    BitBake,
}

impl RawExecutable {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BitBake => "bitbake",
        }
    }
}

/// One argv token. Parameters are substituted as data, never as shell text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RawArgument {
    Literal {
        value: String,
    },
    Empty,
    Parameter {
        parameter: RawParameterId,
    },
    JoinedParameter {
        prefix: String,
        parameter: RawParameterId,
    },
    Composed {
        segments: Vec<RawArgumentSegment>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RawArgumentSegment {
    Literal { value: String },
    Parameter { parameter: RawParameterId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "requirement", rename_all = "snake_case")]
pub enum RawCapabilityRequirement {
    All { capabilities: Vec<CapabilityId> },
    Any { capabilities: Vec<CapabilityId> },
}

impl RawCapabilityRequirement {
    fn capabilities(&self) -> &[CapabilityId] {
        match self {
            Self::All { capabilities } | Self::Any { capabilities } => capabilities,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawReferenceKind {
    ShellPipeline,
    Conceptual,
    CompanionTool,
    UnsupportedBitBake,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawReference {
    pub id: RawReferenceId,
    pub heading: String,
    pub command: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutableTemplate {
    pub executable: RawExecutable,
    pub arguments: Vec<RawArgument>,
    pub capabilities: RawCapabilityRequirement,
    pub interaction: RawInteractionMode,
    pub safety: RawSafetyClass,
}

impl RawExecutableTemplate {
    pub fn display_template(&self, parameters: &[RawParameter]) -> Option<String> {
        let mut tokens = Vec::with_capacity(self.arguments.len() + 1);
        tokens.push(self.executable.as_str().to_owned());
        for argument in &self.arguments {
            let token = match argument {
                RawArgument::Literal { value } => value.clone(),
                RawArgument::Empty => "''".into(),
                RawArgument::Parameter { parameter } => {
                    parameter_placeholder(parameters, parameter)?.into()
                }
                RawArgument::JoinedParameter { prefix, parameter } => {
                    format!("{prefix}{}", parameter_placeholder(parameters, parameter)?)
                }
                RawArgument::Composed { segments } => {
                    let mut token = String::new();
                    for segment in segments {
                        match segment {
                            RawArgumentSegment::Literal { value } => token.push_str(value),
                            RawArgumentSegment::Parameter { parameter } => {
                                token.push_str(parameter_placeholder(parameters, parameter)?)
                            }
                        }
                    }
                    token
                }
            };
            tokens.push(token);
        }
        Some(tokens.join(" "))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum RawExecutionPolicy {
    Executable {
        template: RawExecutableTemplate,
    },
    ReferenceOnly {
        kind: RawReferenceKind,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCommand {
    pub id: RawCommandId,
    pub category: RawCategoryId,
    pub label: String,
    pub description: String,
    pub reference: RawReference,
    pub parameters: Vec<RawParameter>,
    pub execution: RawExecutionPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawAvailabilityState {
    Available,
    Limited,
    Unavailable,
    Unknown,
    Unsupported,
}

impl RawAvailabilityState {
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Available | Self::Limited)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCapabilityIssue {
    pub capability: Option<CapabilityId>,
    pub reason: String,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCommandAvailability {
    pub state: RawAvailabilityState,
    pub issues: Vec<RawCapabilityIssue>,
    pub implementations: Vec<(CapabilityId, String)>,
}

impl RawCommandAvailability {
    pub const fn is_enabled(&self) -> bool {
        self.state.is_enabled()
    }
}

impl RawCommand {
    /// Project this command only from daemon-owned connected-environment
    /// authority. The bundled reference version never participates.
    pub fn availability(
        &self,
        authority: Option<&DaemonCompatibilitySnapshot>,
    ) -> RawCommandAvailability {
        match &self.execution {
            RawExecutionPolicy::ReferenceOnly { reason, .. } => RawCommandAvailability {
                state: RawAvailabilityState::Unsupported,
                issues: vec![RawCapabilityIssue {
                    capability: None,
                    reason: reason.clone(),
                    limitations: Vec::new(),
                }],
                implementations: Vec::new(),
            },
            RawExecutionPolicy::Executable { template } => match &template.capabilities {
                RawCapabilityRequirement::All { capabilities } => {
                    project_raw_capabilities(authority, capabilities, RawRequirementOperator::All)
                }
                RawCapabilityRequirement::Any { capabilities } => {
                    project_raw_capabilities(authority, capabilities, RawRequirementOperator::Any)
                }
            },
        }
    }

    pub fn selector(
        &self,
        parameter: &RawParameterId,
        authority: &RawSelectorAuthority,
    ) -> Result<RawParameterSelector, RawSelectorError> {
        let definition = self
            .parameters
            .iter()
            .find(|definition| &definition.id == parameter)
            .ok_or_else(|| RawSelectorError::UnknownParameter {
                command: self.id.clone(),
                parameter: parameter.clone(),
            })?;
        let RawExecutionPolicy::Executable { template } = &self.execution else {
            return Err(RawSelectorError::ReferenceOnly(self.id.clone()));
        };
        let inventory = authority.inventory(definition.kind).ok_or_else(|| {
            RawSelectorError::NotInventoryBacked {
                command: self.id.clone(),
                parameter: parameter.clone(),
            }
        })?;

        // Manual entry is exposed only by executable definitions that actually
        // substitute this parameter. Catalog validation normally guarantees
        // this; keeping the check here also fails closed for constructed data.
        let manual_entry = template_references_parameter(template, parameter);
        Ok(RawParameterSelector {
            parameter: definition.clone(),
            inventory: inventory.clone(),
            manual_entry,
        })
    }
}
