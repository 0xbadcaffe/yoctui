fn yocto_utility_global_metadata(command: CommandId) -> Option<GlobalMetadata> {
    use OperatorActionSafety as Safety;
    Some(match command {
        CommandId::OpenBitBakeConfigBuild => yocto_utility_metadata(
            "tools.config-build",
            "BitBake config build",
            "List, inspect, enable, or disable BitBake configuration fragments",
            &["bitbake-config-build", "fragments", "toolcfg"],
            Safety::ConfirmationRequired,
        ),
        CommandId::OpenBitBakeLayersShowLayers => yocto_utility_metadata(
            "tools.layers-show-layers",
            "Show configured layers",
            "Run bitbake-layers show-layers in the active build",
            &["bitbake-layers", "show-layers", "layers"],
            Safety::ReadOnly,
        ),
        CommandId::OpenBitBakeLayersShowRecipes => yocto_utility_metadata(
            "tools.layers-show-recipes",
            "Show matching recipes",
            "Run bitbake-layers show-recipes with optional filters",
            &["bitbake-layers", "show-recipes", "providers"],
            Safety::ReadOnly,
        ),
        CommandId::OpenBitBakeLayersShowOverlayed => yocto_utility_metadata(
            "tools.layers-show-overlayed",
            "Show overlayed recipes",
            "Run bitbake-layers show-overlayed with optional filters",
            &["bitbake-layers", "show-overlayed", "overlays"],
            Safety::ReadOnly,
        ),
        CommandId::OpenBitBakeLayersShowAppends => yocto_utility_metadata(
            "tools.layers-show-appends",
            "Show recipe appends",
            "Run bitbake-layers show-appends with optional recipe patterns",
            &["bitbake-layers", "show-appends", "bbappend"],
            Safety::ReadOnly,
        ),
        CommandId::OpenBitBakeLayersShowCrossDepends => yocto_utility_metadata(
            "tools.layers-show-cross-depends",
            "Show cross-layer dependencies",
            "Inspect recipe dependencies that cross layer boundaries",
            &["bitbake-layers", "show-cross-depends", "dependencies"],
            Safety::ReadOnly,
        ),
        CommandId::OpenBitBakeLayersAddLayer => yocto_utility_metadata(
            "tools.layers-add-layer",
            "Add layers",
            "Add one or more layer directories to bblayers.conf",
            &["bitbake-layers", "add-layer", "bblayers"],
            Safety::ConfirmationRequired,
        ),
        CommandId::OpenBitBakeLayersRemoveLayer => yocto_utility_metadata(
            "tools.layers-remove-layer",
            "Remove layers",
            "Remove one or more layer paths or patterns from bblayers.conf",
            &["bitbake-layers", "remove-layer", "bblayers"],
            Safety::DestructiveConfirmation,
        ),
        CommandId::OpenBitBakeLayersFlatten => yocto_utility_metadata(
            "tools.layers-flatten",
            "Flatten layers",
            "Write selected or configured layers into one output directory",
            &["bitbake-layers", "flatten", "output"],
            Safety::ConfirmationRequired,
        ),
        CommandId::OpenBitBakeLayersLayerIndexFetch => yocto_utility_metadata(
            "tools.layers-index-fetch",
            "Fetch from layer index",
            "Fetch indexed layers and dependencies into the active build",
            &["bitbake-layers", "layerindex-fetch", "clone"],
            Safety::ConfirmationRequired,
        ),
        CommandId::OpenBitBakeLayersLayerIndexShowDepends => yocto_utility_metadata(
            "tools.layers-index-depends",
            "Show layer-index dependencies",
            "Query dependencies for named layers in the layer index",
            &["bitbake-layers", "layerindex-show-depends", "index"],
            Safety::ReadOnly,
        ),
        CommandId::OpenBitBakeLayersCreateLayer => yocto_utility_metadata(
            "tools.layers-create-layer",
            "Create layer",
            "Create a layer skeleton and optionally add it to bblayers.conf",
            &["bitbake-layers", "create-layer", "layer"],
            Safety::ConfirmationRequired,
        ),
        CommandId::OpenBitBakeLayersShowMachines => yocto_utility_metadata(
            "tools.layers-show-machines",
            "Show machines",
            "List machines provided by configured layers",
            &["bitbake-layers", "show-machines", "machine"],
            Safety::ReadOnly,
        ),
        CommandId::OpenBitBakeLayersSaveBuildConf => yocto_utility_metadata(
            "tools.layers-save-build-conf",
            "Save build configuration",
            "Save local.conf and bblayers.conf as a layer template",
            &["bitbake-layers", "save-build-conf", "template"],
            Safety::ConfirmationRequired,
        ),
        CommandId::OpenBitBakeLayersCreateLayersSetup => yocto_utility_metadata(
            "tools.layers-create-setup",
            "Create layers setup",
            "Write reproducible layer checkout configuration and script files",
            &["bitbake-layers", "create-layers-setup", "setup"],
            Safety::ConfirmationRequired,
        ),
        _ => return None,
    })
}

fn yocto_utility_metadata(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    keywords: &'static [&'static str],
    safety: OperatorActionSafety,
) -> GlobalMetadata {
    GlobalMetadata {
        id,
        scope: OperatorActionScope::Global,
        menu_path: vec!["Tools", label],
        label,
        description,
        aliases: keywords,
        keywords,
        bindings: &[],
        local_requirement: OperatorActionLocalRequirement::WorkspaceLoaded,
        safety,
        footer_priority: 45,
        help_group: OperatorActionHelpGroup::Operate,
    }
}
