impl App {
    pub fn active_dialog(&self) -> Option<&Dialog> {
        self.dialogs.front()
    }
    pub fn active_dialog_mut(&mut self) -> Option<&mut Dialog> {
        self.dialogs.front_mut()
    }
    pub fn command_palette_commands(&self) -> Vec<PaletteCommand> {
        global_operator_action_definitions()
            .into_iter()
            .map(|definition| {
                let OperatorActionTarget::Command(id) = definition.target else {
                    unreachable!("global catalog entries target command IDs")
                };
                let pane_disabled_reason = match id {
                    CommandId::OpenGitUi if self.gitui_program.is_none() => {
                        Some("GitUI is not installed; install gitui and restart Yoctui")
                    }
                    CommandId::OpenGitUi if self.source_repository_path().is_none() => {
                        Some("Select a source directory in Build Environment")
                    }
                    CommandId::OpenGitUi
                        if !matches!(self.source_git_status, SourceGitStatus::Ready(_)) =>
                    {
                        Some("Source Git status is unavailable or still being inspected")
                    }
                    CommandId::FocusWorkspace
                        if !focus_target_is_relevant(self, FocusTarget::Workspace) =>
                    {
                        Some("The current Workspace is read-only")
                    }
                    CommandId::FocusInspector => Some("The Inspector is read-only"),
                    _ => None,
                };
                let local_disabled_reason =
                    pane_disabled_reason.or(match definition.local_requirement {
                        OperatorActionLocalRequirement::None => None,
                        OperatorActionLocalRequirement::WorkspaceLoaded => self
                            .workspace
                            .build_dir
                            .is_none()
                            .then_some("Load a Yocto workspace first"),
                        OperatorActionLocalRequirement::ImageRecipeAvailable => (!self
                            .workspace
                            .recipes
                            .iter()
                            .any(|recipe| recipe.name.contains("image")))
                        .then_some("No image recipes are available"),
                        OperatorActionLocalRequirement::SelectedRecipe => (self.screen
                            != Screen::Recipes
                            || self.workspace.recipes.get(self.recipe_selection).is_none())
                        .then_some("Open Recipes and select a recipe"),
                    });
                let compatibility =
                    compatibility_ui_command_action_availability(&self.workspace_compatibility, id);
                let compatibility_reason = compatibility.exact_reason();
                let disabled_reason = local_disabled_reason.map(str::to_owned).or_else(|| {
                    (!compatibility.enabled).then(|| {
                        compatibility_reason.clone().unwrap_or_else(|| {
                            "The connected environment does not enable this operation.".into()
                        })
                    })
                });
                PaletteCommand {
                    action_id: definition.id,
                    id,
                    label: definition.label,
                    description: definition.description,
                    shortcut: definition.shortcut,
                    menu_path: definition.menu_path,
                    aliases: definition.aliases,
                    palette_keywords: definition.palette_keywords,
                    safety: definition.safety,
                    footer_priority: definition.footer_priority,
                    help_group: definition.help_group,
                    disabled_reason,
                    compatibility_state: compatibility.state,
                    compatibility_reason,
                    compatibility_limitations: compatibility.limitations,
                    implementations: compatibility.implementations,
                }
            })
            .collect()
    }
    pub fn filtered_command_palette_commands(&self) -> Vec<PaletteCommand> {
        let query = self.command_palette_query.trim();
        if self.command_palette_mode == CommandPaletteMode::GlobalRegexSearch {
            return Vec::new();
        }
        let literal_query = query.to_lowercase();
        self.command_palette_commands()
            .into_iter()
            .filter(|command| {
                let values = std::iter::once(command.label)
                    .chain(std::iter::once(command.description.as_str()))
                    .chain(std::iter::once(command.shortcut))
                    .chain(std::iter::once(command.action_id.as_str()))
                    .chain(command.menu_path.iter().copied())
                    .chain(command.aliases.iter().copied())
                    .chain(command.palette_keywords.iter().copied());
                query.is_empty()
                    || values
                        .into_iter()
                        .any(|value| value.to_lowercase().contains(&literal_query))
            })
            .collect()
    }
    pub fn command_palette_regex_error(&self) -> Option<String> {
        let query = self.command_palette_query.trim();
        (self.command_palette_mode == CommandPaletteMode::GlobalRegexSearch && !query.is_empty())
            .then(|| {
                regex::RegexBuilder::new(query)
                    .case_insensitive(true)
                    .build()
                    .err()
                    .map(|error| error.to_string())
            })
            .flatten()
    }
    pub fn application_menu_items(&self, group: ApplicationMenuGroup) -> Vec<MenuItem> {
        if group == ApplicationMenuGroup::Actions {
            return self.context_menu_items(workspace_screen_destination(self.screen));
        }
        let mut items = self
            .command_palette_commands()
            .into_iter()
            .filter(|command| ApplicationMenuGroup::for_command(command.id) == group)
            .map(|command| MenuItem {
                action_id: command.action_id,
                target: OperatorActionTarget::Command(command.id),
                label: command.label,
                description: command.description,
                shortcut: command.shortcut,
                disabled_reason: command.disabled_reason,
                safety: command.safety,
            })
            .collect::<Vec<_>>();
        if group == ApplicationMenuGroup::Build
            && let Some(definition) =
                workspace_operator_action_definitions(WorkspaceDestination::Tasks)
                    .into_iter()
                    .find(|definition| definition.id.as_str() == "tasks.cancel")
        {
            let availability = compatibility_ui_action_availability(
                &self.workspace_compatibility,
                &definition.requirement,
            );
            let disabled_reason = context_action_local_disabled_reason(
                self,
                WorkspaceDestination::Tasks,
                definition.id.as_str(),
            )
            .or_else(|| {
                (!availability.enabled).then(|| {
                    availability.exact_reason().unwrap_or_else(|| {
                        "The connected environment does not enable this operation.".into()
                    })
                })
            });
            items.push(MenuItem {
                action_id: definition.id,
                target: definition.target,
                label: definition.label,
                description: definition.description,
                shortcut: definition.shortcut,
                disabled_reason,
                safety: definition.safety,
            });
        }
        items
    }
    pub fn context_menu_items(&self, destination: WorkspaceDestination) -> Vec<MenuItem> {
        workspace_operator_action_definitions(destination)
            .into_iter()
            .map(|definition| {
                let availability = compatibility_ui_action_availability(
                    &self.workspace_compatibility,
                    &definition.requirement,
                );
                let disabled_reason =
                    context_action_local_disabled_reason(self, destination, definition.id.as_str())
                        .or_else(|| {
                            (!availability.enabled).then(|| {
                                availability.exact_reason().unwrap_or_else(|| {
                                    "The connected environment does not enable this operation."
                                        .into()
                                })
                            })
                        });
                MenuItem {
                    action_id: definition.id,
                    target: definition.target,
                    label: definition.label,
                    description: definition.description,
                    shortcut: definition.shortcut,
                    disabled_reason,
                    safety: definition.safety,
                }
            })
            .collect()
    }
    pub fn active_menu_items(&self) -> Vec<MenuItem> {
        match self.menu.kind {
            Some(MenuKind::Application) => self.application_menu_items(self.menu.group()),
            Some(MenuKind::Context(destination)) => self.context_menu_items(destination),
            None => Vec::new(),
        }
    }
    pub fn selected_menu_item(&self) -> Option<MenuItem> {
        self.active_menu_items()
            .get(self.menu.item_selection)
            .cloned()
    }
    pub fn pane_focus_label(&self) -> &'static str {
        pane_focus_label(self.focus, self.workspace_subfocus, self.inspector_subfocus)
    }
    pub fn zoom_label(&self) -> Option<&'static str> {
        self.zoomed_pane
            .map(|focus| pane_focus_label(focus, self.workspace_subfocus, self.inspector_subfocus))
    }

}
