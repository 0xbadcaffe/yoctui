fn valid_command(command: &CommandRequirement, tools: &[CapabilityToolId]) -> bool {
    tools.contains(&command.tool)
        && command.subcommand.as_deref().is_none_or(valid_token)
        && command.options.len() <= MAX_CATALOG_REQUIREMENTS
        && command.options.iter().all(|option| valid_option(option))
}

fn valid_metadata(requirement: &MetadataRequirement) -> bool {
    match requirement {
        MetadataRequirement::AnyTask { names } => {
            !names.is_empty()
                && names.len() <= MAX_CATALOG_REQUIREMENTS
                && names.iter().all(|name| valid_token(name))
        }
        MetadataRequirement::Variable { name }
        | MetadataRequirement::Api { name }
        | MetadataRequirement::Artifact { kind: name }
        | MetadataRequirement::Configuration { name } => valid_token(name),
    }
}

fn valid_probe(probe: &CapabilityProbeSpec, tools: &[CapabilityToolId]) -> bool {
    match probe {
        CapabilityProbeSpec::Executable { tool } | CapabilityProbeSpec::CommandVersion { tool } => {
            tools.contains(tool)
        }
        CapabilityProbeSpec::CommandHelp { tool, subcommand } => {
            tools.contains(tool) && subcommand.as_deref().is_none_or(valid_token)
        }
        CapabilityProbeSpec::CommandOption {
            tool,
            subcommand,
            option,
        } => {
            tools.contains(tool)
                && subcommand.as_deref().is_none_or(valid_token)
                && valid_option(option)
        }
        CapabilityProbeSpec::CommandHelpText { tool, needle } => {
            tools.contains(tool) && valid_token(needle)
        }
        CapabilityProbeSpec::MetadataAnyTask { names } => {
            !names.is_empty()
                && names.len() <= MAX_CATALOG_REQUIREMENTS
                && names.iter().all(|name| valid_token(name))
        }
        CapabilityProbeSpec::MetadataVariable { name }
        | CapabilityProbeSpec::BackendCapability { name }
        | CapabilityProbeSpec::ProtocolCapability { name }
        | CapabilityProbeSpec::Artifact { kind: name }
        | CapabilityProbeSpec::Configuration { name } => valid_token(name),
    }
}

fn valid_boundary(boundary: &AdvisoryReleaseBoundary) -> bool {
    valid_token(&boundary.component)
        && boundary.introduced.as_deref().is_none_or(valid_text)
        && boundary.removed.as_deref().is_none_or(valid_text)
        && valid_text(&boundary.source)
        && valid_text(&boundary.note)
}

fn valid_id(value: &str) -> bool {
    valid_token(value)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_option(value: &str) -> bool {
    valid_token(value) && value.starts_with('-')
}

fn valid_token(value: &str) -> bool {
    valid_text(value) && !value.chars().any(char::is_whitespace)
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1_024
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

