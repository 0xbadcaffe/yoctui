use super::*;

fn recipe(app: &mut App) -> Option<String> {
    let s = app.workbench.current()?;
    if s.tabs.iter().any(WorkbenchTab::is_dirty) {
        app.notification = Some("Save open changes with Ctrl+s before recipe commands.".into());
        return None;
    }
    if s.build_dir != app.workspace.build_dir {
        app.notification = Some(
            "Attach this workspace's build environment before running recipe commands.".into(),
        );
        return None;
    }
    let Some(name) = s.recipe.clone() else {
        super::reduce(app, WorkbenchAction::Form(WorkbenchFormKind::Recipe));
        return None;
    };
    let Some(index) = app.workspace.recipes.iter().position(|r| r.name == name) else {
        app.notification = Some(format!("Recipe {name} is not available in this build."));
        return None;
    };
    app.recipe_selection = index;
    Some(name)
}

pub(super) fn reduce(app: &mut App, action: WorkbenchAction) -> Option<Effect> {
    use WorkbenchAction as W;
    match action {
        W::Compile => {
            if app.workbench.pending_compile.is_some() {
                return None;
            }
            let name = recipe(app)?;
            if !app.recipe_metadata.contains_key(&name) {
                let s = app.workbench.current()?;
                app.workbench.pending_compile = Some((s.id, name, s.build_dir.clone()));
                let effect =
                    update_with_workspace_authority(app, Action::BeginSelectedRecipeMetadata);
                if effect.is_none() {
                    app.workbench.pending_compile = None;
                }
                return effect;
            }
            return super::reduce(
                app,
                W::RecipeAction(Box::new(Action::BeginSelectedRecipeTask {
                    task: Some("compile".into()),
                    force: false,
                })),
            );
        }
        W::Utility(command @ (DevtoolUtilityCommand::Build | DevtoolUtilityCommand::IdeSdk)) => {
            app.workbench.pending_compile = None;
            let name = recipe(app)?;
            update_with_workspace_authority(
                app,
                Action::OpenYoctoUtility(YoctoUtilityCommand::Devtool(command)),
            );
            if let Some(Dialog::YoctoUtility(dialog)) = app.active_dialog_mut()
                && let YoctoUtilityDraft::Devtool(draft) = &mut dialog.draft
            {
                *draft.selected_text_mut(0)? = name;
                if command == DevtoolUtilityCommand::IdeSdk {
                    draft.cycle_choice(2, 1); // Native terminal workflow: IDE none.
                    draft.cycle_choice(11, 1); // Retain unstripped executables for GDB.
                }
            }
        }
        W::Debug => {
            app.workbench.pending_compile = None;
            let s = app.workbench.current()?;
            let mut config = s.debug.clone();
            if config.executable.is_empty()
                && s.focus == WorkbenchFocus::Files
                && let Some(entry) = s
                    .folders
                    .get(s.folder)
                    .and_then(|f| f.browser.selected_entry())
                && !entry.is_dir
                && entry
                    .path
                    .extension()
                    .is_none_or(|e| e == "elf" || e == "out")
            {
                config.executable = entry.path.display().to_string();
            }
            app.workbench.debug_dialog = Some(WorkbenchDebugDialog {
                config,
                field: 0,
                error: None,
                loading: false,
            });
        }
        W::DebugField(delta) => {
            let d = app.workbench.debug_dialog.as_mut()?;
            d.field = (d.field as isize + delta).rem_euclid(WORKBENCH_DEBUG_FIELDS.len() as isize)
                as usize;
        }
        W::DebugMode => {
            let d = app.workbench.debug_dialog.as_mut()?;
            d.config.local = !d.config.local;
            d.error = None;
        }
        W::DebugText(c) if !c.is_control() => {
            let d = app.workbench.debug_dialog.as_mut()?;
            if let Some(value) = d.config.field_mut(d.field)
                && value.len() + c.len_utf8() <= 4096
            {
                value.push(c);
                d.error = None;
            }
        }
        W::DebugBackspace | W::DebugClear => {
            let d = app.workbench.debug_dialog.as_mut()?;
            if let Some(value) = d.config.field_mut(d.field) {
                if action == W::DebugClear {
                    value.clear();
                } else {
                    value.pop();
                }
            }
            d.error = None;
        }
        W::DebugReview => {
            let d = app.workbench.debug_dialog.as_ref()?;
            if d.loading {
                return None;
            }
            if let Err(error) = d.config.validate() {
                app.workbench.debug_dialog.as_mut()?.error = Some(error);
                return None;
            }
            let config = d.config.clone();
            let s = app.workbench.current_mut()?;
            if s.tabs.iter().any(WorkbenchTab::is_dirty) {
                app.workbench.debug_dialog.as_mut()?.error =
                    Some("Save edited source files before launching GDB.".into());
                return None;
            }
            let session = s.id;
            let root = s.folders.get(s.folder)?.browser.root.clone();
            let breakpoints = s
                .tabs
                .iter()
                .flat_map(|t| t.breakpoints.iter().map(|line| (t.path.clone(), *line)))
                .collect();
            s.debug = config.clone();
            app.workbench.changed();
            app.workbench.debug_request = app.workbench.debug_request.saturating_add(1);
            app.workbench.debug_dialog.as_mut()?.loading = true;
            return Some(Effect::Workbench(WorkbenchRequest::PrepareDebug {
                session,
                generation: app.workbench.debug_request,
                root,
                config,
                breakpoints,
            }));
        }
        W::DebugPrepared {
            session,
            generation,
            result,
        } => {
            if app.workbench.active != Some(session)
                || generation != app.workbench.debug_request
                || !app
                    .workbench
                    .debug_dialog
                    .as_ref()
                    .is_some_and(|d| d.loading)
            {
                return None;
            }
            match result {
                Ok(request) => {
                    app.workbench.debug_dialog = None;
                    crate::open_dialog(
                        app,
                        Dialog::TerminalLaunch(TerminalLaunchDialog {
                            request,
                            destination: TerminalLaunchDestination::Embedded,
                            output_must_not_exist: None,
                        }),
                    );
                    crate::synchronize_focus(app);
                }
                Err(error) => {
                    let d = app.workbench.debug_dialog.as_mut()?;
                    d.loading = false;
                    d.error = Some(error);
                }
            }
        }
        _ => {}
    }
    None
}
