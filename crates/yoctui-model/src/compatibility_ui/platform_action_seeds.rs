fn kernel_workspace_action_seeds() -> Vec<CompatibilityUiWorkspaceActionDefinition> {
    use CapabilityId as Id;
    use CompatibilityUiWorkspaceActionDefinition as Action;
    vec![
        Action::local("kernel.refresh", "Refresh kernel files", "r"),
        Action::capability(
            "kernel.menuconfig",
            "Open kernel menuconfig",
            "m",
            Id::MenuConfig,
        ),
        Action::local("kernel.view", "View selected text file", "Enter"),
        Action::local("kernel.explore", "Explore selected root", "o"),
        Action::local("kernel.compile", "Compile DTS with options", "c"),
        Action::local("kernel.decompile", "Decompile selected DTB", "d"),
    ]
}

fn firmware_workspace_action_seeds() -> Vec<CompatibilityUiWorkspaceActionDefinition> {
    use CapabilityId as Id;
    use CompatibilityUiWorkspaceActionDefinition as Action;
    vec![
        Action::local("firmware.refresh", "Refresh firmware files", "r"),
        Action::capability(
            "firmware.menuconfig",
            "Open firmware menuconfig",
            "m",
            Id::MenuConfig,
        ),
        Action::local("firmware.view", "View selected text file", "Enter"),
        Action::local("firmware.explore", "Explore selected root", "o"),
        Action::local("firmware.compile", "Compile DTS with options", "c"),
        Action::local("firmware.decompile", "Decompile selected DTB", "d"),
    ]
}
