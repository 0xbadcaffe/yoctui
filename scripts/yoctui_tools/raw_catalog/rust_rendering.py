def render_parameter(parameter: Parameter) -> str:
    return (
        "RawParameter { "
        f"id: RawParameterId::new({quoted(parameter.id)}).unwrap(), "
        f"label: {quoted(parameter.label)}.into(), "
        f"placeholder: {quoted(parameter.placeholder)}.into(), "
        f"kind: RawParameterKind::{parameter.kind}, "
        "presence: RawParameterPresence::Required }"
    )


def render_command(entry: Entry) -> str:
    command_id = f"{entry.category_id}.l{entry.line:04d}"
    reference_id = f"wrynose-6-0.l{entry.line:04d}"
    common = f"""        RawCommand {{
            id: RawCommandId::new({quoted(command_id)}).unwrap(),
            category: RawCategoryId::new({quoted(entry.category_id)}).unwrap(),
            label: {quoted(entry.command)}.into(),
            description: {quoted(entry.description)}.into(),
            reference: RawReference {{
                id: RawReferenceId::new({quoted(reference_id)}).unwrap(),
                heading: {quoted(entry.heading)}.into(),
                command: {quoted(entry.command)}.into(),
                description: {quoted(entry.description)}.into(),
            }},
"""
    if not entry.executable:
        kind = "ShellPipeline" if entry.command.startswith("bitbake ") else "CompanionTool"
        reason = (
            "Requires a shell pipeline or redirection; Raw Mode never invokes a shell."
            if kind == "ShellPipeline"
            else "Uses a companion tool or shell command; Raw Mode executes only structured BitBake argv."
        )
        return (
            common
            + "            parameters: vec![],\n"
            + "            execution: RawExecutionPolicy::ReferenceOnly {\n"
            + f"                kind: RawReferenceKind::{kind},\n"
            + f"                reason: {quoted(reason)}.into(),\n"
            + "            },\n"
            + "        },\n"
        )

    parameters, arguments = command_parts(entry.command)
    capability_names = capabilities(entry.command)
    capability_rows = ", ".join(f"CapabilityId::{name}" for name in capability_names)
    parameters_text = ",\n                ".join(render_parameter(item) for item in parameters)
    arguments_text = ",\n                    ".join(arguments)
    return (
        common
        + "            parameters: vec![\n"
        + (f"                {parameters_text}\n" if parameters else "")
        + "            ],\n"
        + "            execution: RawExecutionPolicy::Executable {\n"
        + "                template: RawExecutableTemplate {\n"
        + "                    executable: RawExecutable::BitBake,\n"
        + "                    arguments: vec![\n"
        + (f"                    {arguments_text}\n" if arguments else "")
        + "                    ],\n"
        + "                    capabilities: RawCapabilityRequirement::All {\n"
        + f"                        capabilities: vec![{capability_rows}],\n"
        + "                    },\n"
        + f"                    interaction: RawInteractionMode::{interaction(entry.command)},\n"
        + f"                    safety: RawSafetyClass::{safety(entry.command, capability_names)},\n"
        + "                },\n"
        + "            },\n"
        + "        },\n"
    )
