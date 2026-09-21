fn command(
    tool: CapabilityToolId,
    subcommand: Option<&str>,
    options: &[&str],
) -> CommandRequirement {
    CommandRequirement {
        tool,
        subcommand: subcommand.map(str::to_owned),
        options: options.iter().map(|value| (*value).to_owned()).collect(),
    }
}

fn help(tool: CapabilityToolId, subcommand: Option<&str>) -> CapabilityProbeSpec {
    CapabilityProbeSpec::CommandHelp {
        tool,
        subcommand: subcommand.map(str::to_owned),
    }
}

fn implementation(
    value: &str,
    kind: CapabilityImplementationKind,
) -> CapabilityImplementation {
    CapabilityImplementation {
        id: value.into(),
        kind,
    }
}

fn executable(tool: CapabilityToolId) -> CapabilityProbeSpec {
    CapabilityProbeSpec::Executable { tool }
}

fn tool_command(
    label: &'static str,
    tool: CapabilityToolId,
    subcommand: Option<&str>,
    options: &[&str],
    impl_id: &str,
) -> Definition {
    let mut probes = vec![executable(tool), help(tool, subcommand)];
    probes.extend(
        options
            .iter()
            .map(|option| CapabilityProbeSpec::CommandOption {
                tool,
                subcommand: subcommand.map(str::to_owned),
                option: (*option).to_owned(),
            }),
    );
    (
        label,
        vec![tool],
        vec![command(tool, subcommand, options)],
        Vec::new(),
        probes,
        implementation(impl_id, CapabilityImplementationKind::Command),
        None,
    )
}

fn bitbake_option(label: &'static str, option: &str, impl_id: &str) -> Definition {
    use CapabilityToolId as Tool;
    (
        label,
        vec![Tool::BitBake],
        vec![command(Tool::BitBake, None, &[option])],
        Vec::new(),
        vec![CapabilityProbeSpec::CommandOption {
            tool: Tool::BitBake,
            subcommand: None,
            option: option.into(),
        }],
        implementation(impl_id, CapabilityImplementationKind::Command),
        None,
    )
}

fn backend(label: &'static str, name: &str, impl_id: &str) -> Definition {
    (
        label,
        Vec::new(),
        Vec::new(),
        vec![MetadataRequirement::Api { name: name.into() }],
        vec![CapabilityProbeSpec::BackendCapability { name: name.into() }],
        implementation(impl_id, CapabilityImplementationKind::BackendApi),
        None,
    )
}

fn backend_with_version_fallback(
    label: &'static str,
    name: &str,
    impl_id: &str,
) -> Definition {
    let mut value = backend(label, name, impl_id);
    value.6 = Some(FallbackImplementation {
        implementation: implementation(
            "tinfoil.version_correlated",
            CapabilityImplementationKind::BackendApi,
        ),
        selector: FallbackSelector::VersionInferenceWhenUnprobeable {
            map_key: "bitbake.tinfoil_adapter".into(),
        },
    });
    value
}

fn task(label: &'static str, names: &[&str], impl_id: &str) -> Definition {
    use CapabilityToolId as Tool;
    let names = names
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    (
        label,
        vec![Tool::BitBake],
        Vec::new(),
        vec![MetadataRequirement::AnyTask {
            names: names.clone(),
        }],
        vec![CapabilityProbeSpec::MetadataAnyTask { names }],
        implementation(impl_id, CapabilityImplementationKind::MetadataTask),
        None,
    )
}
