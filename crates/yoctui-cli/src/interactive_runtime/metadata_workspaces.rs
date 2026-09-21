use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_recipe_and_layer_workspaces(
        &mut self,
        input: Input,
    ) -> Result<Option<KeyRouteOutcome>> {
        let runtime = self;
        if runtime.app.metadata_searching {
            match input {
                Input::Char(character) => {
                    let _ = compatibility_workspace_action(
                        &mut runtime.app,
                        Action::AppendMetadataQuery(character),
                    );
                }
                Input::Enter | Input::Esc => {
                    let _ = compatibility_workspace_action(
                        &mut runtime.app,
                        Action::FinishMetadataSearch,
                    );
                }
                Input::Backspace => {
                    let _ = compatibility_workspace_action(
                        &mut runtime.app,
                        Action::BackspaceMetadataQuery,
                    );
                }
                _ => {}
            }
        } else if input == Input::Char('!') {
            open_yocto_shell(&runtime.guard, &mut runtime.app).await;
        } else if input == Input::Char('i') {
            let images = runtime
                .app
                .workspace
                .recipes
                .iter()
                .map(|recipe| recipe.name.as_str())
                .filter(|name| name.contains("image"))
                .map(str::to_owned)
                .collect();
            let _ =
                compatibility_workspace_action(&mut runtime.app, Action::OpenImagePicker(images));
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('b') {
            let _ =
                compatibility_workspace_action(&mut runtime.app, Action::BeginSelectedRecipeBuild);
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('f') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeForceTask,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('v') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeDevshell,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('K') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeDiffconfig,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('z') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeDiffsigs,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('Z') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeSignatures,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('V') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeCveCheck,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('X') {
            let _ =
                compatibility_workspace_action(&mut runtime.app, Action::BeginSelectedRecipeSpdx);
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('e') {
            if let Some(Effect::OpenInEditor(path)) =
                compatibility_workspace_action(&mut runtime.app, Action::OpenSelectedRecipeProvider)
            {
                open_in_editor(
                    &runtime.guard,
                    &mut runtime.app,
                    path,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('o') {
            if let Some(Effect::OpenInEditor(path)) =
                compatibility_workspace_action(&mut runtime.app, Action::BeginSelectedRecipeTaskLog)
            {
                open_in_editor(
                    &runtime.guard,
                    &mut runtime.app,
                    path,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('p') {
            if let Some(Effect::OpenInEditor(path)) = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipePatchReview,
            ) {
                open_in_editor(
                    &runtime.guard,
                    &mut runtime.app,
                    path,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if runtime.app.screen == yoctui_model::Screen::Dashboard
            && let Some(action) = dashboard_workspace_action(input)
        {
            let _ = compatibility_workspace_action(&mut runtime.app, action);
        } else if runtime.app.screen == yoctui_model::Screen::Dashboard
            && collection_scroll_delta(input).is_some()
        {
            let delta = collection_scroll_delta(input).expect("scroll key was checked");
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::ScrollBuildTasks { delta },
            );
        } else if runtime.app.screen == yoctui_model::Screen::BuildHistory
            && collection_scroll_delta(input).is_some()
        {
            let delta = collection_scroll_delta(input).expect("scroll key was checked");
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::SelectBuildHistory { delta },
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('d') {
            let root = match compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeDevtoolModify,
            ) {
                Some(Effect::OpenWorkspaceEditor { label, root }) => Some((label, root)),
                _ => None,
            };
            if let Some((recipe, root)) = root {
                open_workspace_editor(&mut runtime.app, recipe, root).await;
            }
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('t') {
            inspect_selected_devtool(&mut runtime.app, &runtime.session_build_dir).await;
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('D') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeDevtoolReset,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('u') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeDevtoolUpdateRecipe,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('F') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeDevtoolFinish,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('P') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeDevtoolDeploy,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('A') {
            if let Some(Effect::GetDependencies(recipe)) = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeDependencies,
            ) {
                load_dependency_graph(&mut runtime.app, runtime.backend.as_mut(), recipe).await;
            }
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Enter {
            if let Some(Effect::GetRecipeMetadata(recipe)) = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeMetadata,
            ) {
                match runtime.backend.get_recipe_metadata(recipe.clone()).await {
                    Ok(metadata) => {
                        let _ = compatibility_workspace_action(
                            &mut runtime.app,
                            Action::RecipeMetadataLoaded(metadata),
                        );
                    }
                    Err(error) => {
                        let _ = compatibility_workspace_action(
                            &mut runtime.app,
                            Action::RecipeMetadataFailed {
                                recipe,
                                message: error.to_string(),
                            },
                        );
                    }
                }
            }
            inspect_selected_devtool(&mut runtime.app, &runtime.session_build_dir).await;
        } else if input == Input::Char('b') {
            let _ =
                compatibility_workspace_action(&mut runtime.app, Action::BeginCurrentImageBuild);
        } else if runtime.app.screen == yoctui_model::Screen::Recipes
            && collection_scroll_delta(input).is_some()
        {
            let delta = collection_scroll_delta(input).expect("scroll key was checked");
            let _ =
                compatibility_workspace_action(&mut runtime.app, Action::SelectRecipe { delta });
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('C') {
            let _ =
                compatibility_workspace_action(&mut runtime.app, Action::BeginSelectedRecipeClean);
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('M') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeMenuConfig,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Recipes && input == Input::Char('S') {
            let _ = compatibility_workspace_action(
                &mut runtime.app,
                Action::BeginSelectedRecipeCleanState,
            );
        } else if runtime.app.screen == yoctui_model::Screen::Layers
            && collection_scroll_delta(input).is_some()
        {
            let delta = collection_scroll_delta(input).expect("scroll key was checked");
            let _ = compatibility_workspace_action(&mut runtime.app, Action::SelectLayer { delta });
        } else if let Some(action) = layer_list_open_action(&runtime.app, input) {
            if let Some(Effect::LoadLayerBrowserDirectory {
                layer,
                root,
                directory,
            }) = compatibility_workspace_action(&mut runtime.app, action)
            {
                load_layer_browser_directory(&mut runtime.app, layer, root, directory).await;
            }
        } else if runtime.app.screen == yoctui_model::Screen::Layers && input == Input::Char('o') {
            if let Some(Effect::OpenInEditor(path)) =
                compatibility_workspace_action(&mut runtime.app, Action::OpenSelectedLayer)
            {
                open_in_editor(
                    &runtime.guard,
                    &mut runtime.app,
                    path,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if runtime.app.screen == yoctui_model::Screen::Layers && input == Input::Char('e') {
            if let Some(Effect::OpenWorkspaceEditor { label, root }) =
                compatibility_workspace_action(
                    &mut runtime.app,
                    Action::BeginSelectedLayerWorkspaceEditor,
                )
            {
                open_workspace_editor(&mut runtime.app, label, root).await;
            }
        } else if runtime.app.screen == yoctui_model::Screen::Layers && input == Input::Char('R') {
            if matches!(
                compatibility_workspace_action(&mut runtime.app, Action::BeginLayerRelationships),
                Some(Effect::GetLayerRelationships)
            ) {
                match runtime.backend.get_layer_relationships().await {
                    Ok(layers) => {
                        let _ = compatibility_workspace_action(
                            &mut runtime.app,
                            Action::LayerRelationshipsLoaded(LayerRelationships {
                                layers: layers
                                    .into_iter()
                                    .map(|layer| LayerRelationship {
                                        name: layer.name,
                                        priority: layer.priority,
                                        compatible: layer.compatible,
                                        depends: layer.depends,
                                        overlays: layer.overlays,
                                        appends: layer.appends,
                                    })
                                    .collect(),
                            }),
                        );
                    }
                    Err(error) => {
                        let _ = compatibility_workspace_action(
                            &mut runtime.app,
                            Action::Failure(AppError::new(
                                "Layers",
                                error.to_string(),
                                "use a bridge connected to a BitBake server that supports get_layer_relationships",
                            )),
                        );
                    }
                }
            }
        } else {
            return Ok(None);
        }
        Ok(Some(KeyRouteOutcome::Handled))
    }
}
