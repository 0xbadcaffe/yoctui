fn builtin_entry(id: CapabilityId) -> CapabilityCatalogEntry {
    let (label, tools, commands, metadata, probes, preferred, fallback) = definition(id);
    CapabilityCatalogEntry {
        id,
        label: label.into(),
        required_tools: tools,
        required_commands: commands,
        required_metadata: metadata,
        probes,
        preferred,
        fallback,
        known_release_boundaries: Vec::new(),
        unavailable_reason: CapabilityReason::new(
            "environment.capability_unavailable",
            format!("Connected environment does not expose {label}."),
            Some(format!("Required capability: {}", id.as_str())),
        )
        .expect("built-in capability reason must be valid"),
    }
}

type Definition = (
    &'static str,
    Vec<CapabilityToolId>,
    Vec<CommandRequirement>,
    Vec<MetadataRequirement>,
    Vec<CapabilityProbeSpec>,
    CapabilityImplementation,
    Option<FallbackImplementation>,
);

