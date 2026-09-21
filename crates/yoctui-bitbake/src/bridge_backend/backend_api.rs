#[async_trait]
impl BitBakeBackend for BridgeBackend {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        self.require_api(BitBakeApiOperation::Workspace)?;
        self.command(Command::InspectWorkspace).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Workspace(workspace) => return Ok(workspace),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(self.disconnected("bridge disconnected during inspection"));
                }
                _ => {}
            }
        }
    }
    async fn list_recipes(&mut self, filter: Option<String>) -> Result<Vec<Recipe>, BackendError> {
        self.require_api(BitBakeApiOperation::Recipes)?;
        self.command(Command::ListRecipes {
            filter,
            chunked: true,
        })
        .await?;
        let correlation = self.sequence.to_string();
        let mut inventory = recipe_inventory::RecipeInventory::default();
        loop {
            let Some(line) = self.next_line().await? else {
                return Err(
                    self.disconnected("bridge disconnected before recipe inventory completion")
                );
            };
            let envelope: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
            if envelope.correlation_id.as_deref() != Some(correlation.as_str()) {
                return Err(BackendError::Bridge(
                    "recipe inventory response has wrong request correlation".into(),
                ));
            }
            self.last_sequence = envelope.sequence;
            let completed = match envelope.message {
                Event::RecipesChunk {
                    offset,
                    total,
                    complete,
                    recipes,
                } => {
                    if line.len() > yoctui_protocol::MAX_RECIPE_CHUNK_BYTES {
                        return Err(BackendError::Bridge("recipe chunk exceeds 512 KiB".into()));
                    }
                    inventory.push(offset, total, complete, recipes)?
                }
                Event::Recipes { recipes } => inventory.push(0, recipes.len(), true, recipes)?,
                Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                _ => false,
            };
            if completed {
                let BackendEvent::Recipes(recipes) = Self::event(Event::Recipes {
                    recipes: inventory.finish()?,
                })?
                else {
                    unreachable!("recipe conversion")
                };
                return Ok(recipes);
            }
        }
    }
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError> {
        self.require_api(BitBakeApiOperation::Layers)?;
        self.command(Command::ListLayers).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Layers(layers) => return Ok(layers),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(self.disconnected("bridge disconnected while listing layers"));
                }
                _ => {}
            }
        }
    }
    async fn get_variable(
        &mut self,
        name: String,
        recipe: Option<String>,
    ) -> Result<VariableValue, BackendError> {
        self.require_api(BitBakeApiOperation::Variable)?;
        let requested_recipe = recipe.clone();
        self.command(Command::GetVariable {
            name: name.clone(),
            recipe,
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Variable {
                    name: returned,
                    recipe,
                    value,
                    provenance,
                    unexpanded_value,
                    operations,
                    active_overrides,
                } if returned == name && recipe == requested_recipe => {
                    return Ok(VariableValue {
                        recipe,
                        value,
                        provenance,
                        unexpanded_value,
                        operations,
                        active_overrides,
                    });
                }
                BackendEvent::Variable { .. } => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(self.disconnected("bridge disconnected while reading a variable"));
                }
                _ => {}
            }
        }
    }
    async fn get_dependencies(
        &mut self,
        recipe: String,
    ) -> Result<RecipeDependencies, BackendError> {
        self.require_api(BitBakeApiOperation::Dependencies)?;
        self.command(Command::GetDependencies {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Dependencies {
                    recipe: returned,
                    build,
                    runtime,
                } if returned == recipe => return Ok(RecipeDependencies { build, runtime }),
                BackendEvent::Dependencies { .. } => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading recipe dependencies")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn get_dependency_graph(
        &mut self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
        self.require_api(BitBakeApiOperation::DependencyGraph)?;
        self.command(Command::GetDependencyGraph {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::DependencyGraph { graph, limitations }
                    if graph.root.recipe_name() == recipe =>
                {
                    return Ok(DependencyGraphResponse { graph, limitations });
                }
                BackendEvent::DependencyGraph { graph, .. } => {
                    return Err(BackendError::Bridge(format!(
                        "bridge returned dependency graph root {} for requested recipe {recipe}",
                        graph.root.recipe_name()
                    )));
                }
                BackendEvent::Dependencies {
                    recipe: returned,
                    build,
                    runtime,
                } if returned == recipe => {
                    return Ok(legacy_dependency_graph(recipe, build, runtime));
                }
                BackendEvent::CommandFailed { code, .. }
                    if code == "invalid_request" || code == "unsupported_command" =>
                {
                    break;
                }
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading the dependency graph")
                    );
                }
                _ => continue,
            }
        }
        let dependencies = self.get_dependencies(recipe.clone()).await?;
        Ok(legacy_dependency_graph(
            recipe,
            dependencies.build,
            dependencies.runtime,
        ))
    }
    async fn get_signature_dump(
        &mut self,
        target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, BackendError> {
        self.signature_adapter
            .dump(target)
            .await
            .map_err(Into::into)
    }
    async fn compare_signatures(
        &mut self,
        request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, BackendError> {
        self.signature_adapter
            .compare(request)
            .await
            .map_err(Into::into)
    }
    async fn get_recipe_sources(&mut self, recipe: String) -> Result<Vec<PathBuf>, BackendError> {
        self.require_api(BitBakeApiOperation::RecipeSources)?;
        self.command(Command::GetRecipeSources {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::RecipeSources {
                    recipe: returned,
                    paths,
                } if returned == recipe => return Ok(paths),
                BackendEvent::RecipeSources { .. } => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading recipe source paths")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn get_recipe_metadata(
        &mut self,
        recipe: String,
    ) -> Result<RecipeMetadata, BackendError> {
        self.require_api(BitBakeApiOperation::RecipeMetadata)?;
        self.command(Command::GetRecipeMetadata {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::RecipeMetadata(metadata) if metadata.recipe == recipe => {
                    return Ok(metadata);
                }
                BackendEvent::RecipeMetadata(_) => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading recipe metadata")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError> {
        self.require_api(BitBakeApiOperation::LayerRelationships)?;
        self.command(Command::GetLayerRelationships).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::LayerRelationships(layers) => return Ok(layers),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading layer relationships")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError> {
        self.require_api(BitBakeApiOperation::Build)?;
        if request.force {
            self.require_api(BitBakeApiOperation::ForceTask)?;
        }
        self.command(Command::StartBuild {
            targets: request.targets,
            task: request.task,
            force: request.force,
        })
        .await
    }
    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        self.require_api(BitBakeApiOperation::Cancel)?;
        self.command(Command::CancelBuild).await
    }
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        let Some(line) = self.next_line().await? else {
            return Ok(BackendEvent::Disconnected);
        };
        let e: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.validate_correlation(&e)?;
        self.last_sequence = e.sequence;
        Self::event(e.message)
    }

    async fn shutdown(&mut self) -> Result<(), BackendError> {
        BridgeBackend::shutdown(self).await
    }
}
impl Drop for BridgeBackend {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }
    }
}
