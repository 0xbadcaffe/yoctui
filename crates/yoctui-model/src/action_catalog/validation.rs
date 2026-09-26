pub fn operator_action_catalog() -> Vec<OperatorActionDefinition> {
    let mut catalog = global_operator_action_definitions();
    for destination in WorkspaceDestination::ALL {
        catalog.extend(workspace_operator_action_definitions(destination));
    }
    catalog
}

pub fn validate_operator_action_catalog() -> Result<(), Vec<String>> {
    let catalog = operator_action_catalog();
    let mut errors = Vec::new();
    let mut ids = HashSet::new();
    for action in &catalog {
        let id = action.id.as_str();
        if !ids.insert(id) {
            errors.push(format!("duplicate action ID: {id}"));
        }
        if id.is_empty()
            || id.starts_with('.')
            || id.ends_with('.')
            || !id.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_.".contains(&byte)
            })
        {
            errors.push(format!("invalid stable action ID: {id}"));
        }
        if action.label.trim().is_empty()
            || action.description.trim().is_empty()
            || action.menu_path.len() < 2
            || action.menu_path.iter().any(|part| part.trim().is_empty())
            || action.palette_keywords.is_empty()
        {
            errors.push(format!("incomplete action metadata: {id}"));
        }
        if action.footer_priority > 100 {
            errors.push(format!("invalid footer priority: {id}"));
        }
        if action.safety == OperatorActionSafety::ReadOnly
            && [
                ".remove",
                ".reset",
                ".cleanup",
                ".write",
                ".undeploy",
                ".cancel",
            ]
            .iter()
            .any(|fragment| id.contains(fragment))
        {
            errors.push(format!("unsafe action is classified read-only: {id}"));
        }
    }
    for command in GLOBAL_COMMANDS {
        let count = catalog
            .iter()
            .filter(|action| action.target == OperatorActionTarget::Command(command))
            .count();
        if count != 1 {
            errors.push(format!("command {command:?} has {count} catalog entries"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

impl WorkspaceDestination {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Recipes => "Recipes",
            Self::Layers => "Layers",
            Self::Configuration => "Configuration",
            Self::Tasks => "Tasks",
            Self::BuildHistory => "Build History",
            Self::Logs => "Logs",
            Self::Errors => "Errors",
            Self::Dependencies => "Dependencies",
            Self::Signatures => "Signatures",
            Self::Packages => "Packages",
            Self::Images => "Images",
            Self::Hardware => "Hardware",
            Self::Kernel => "Kernel",
            Self::Firmware => "U-Boot / BIOS",
            Self::Sdk => "SDK",
            Self::Testing => "Testing",
            Self::Security => "Security",
            Self::Qa => "QA",
            Self::RawMode => "Raw Mode",
            Self::Devtool => "Devtool",
            Self::QemuWic => "QEMU / Wic",
            Self::Maintenance => "Maintenance",
            Self::ProjectProfiles => "Project Profiles",
            Self::TerminalSessions => "Terminal Sessions",
            Self::BuildEnvironment => "Build Environment",
            Self::Compatibility => "Compatibility",
            Self::Settings => "Settings",
            Self::Help => "Help",
        }
    }
}
