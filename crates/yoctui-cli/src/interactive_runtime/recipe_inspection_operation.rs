use super::*;

#[derive(Clone, Copy)]
pub(super) enum RecipeMetadataFollowup {
    PatchReview,
}

#[derive(Clone)]
enum RecipeInspectionRequest {
    Metadata {
        recipe: String,
        followup: Option<RecipeMetadataFollowup>,
    },
    DependencyGraph {
        recipe: String,
    },
}

enum RecipeInspectionResult {
    Metadata {
        recipe: String,
        followup: Option<RecipeMetadataFollowup>,
        result: std::result::Result<yoctui_model::RecipeMetadata, String>,
    },
    DependencyGraph {
        recipe: String,
        result: std::result::Result<yoctui_bitbake::DependencyGraphResponse, String>,
    },
}

type WorkerResult = (Box<dyn BitBakeBackend>, bool, RecipeInspectionResult);

pub(super) struct RecipeInspectionOperation {
    request: RecipeInspectionRequest,
    handle: tokio::task::JoinHandle<WorkerResult>,
}

impl InteractiveRuntime {
    pub(super) fn begin_recipe_metadata(
        &mut self,
        recipe: String,
        followup: Option<RecipeMetadataFollowup>,
    ) {
        self.begin_recipe_inspection(RecipeInspectionRequest::Metadata { recipe, followup });
    }

    pub(super) fn begin_recipe_dependency_graph(&mut self, recipe: String) {
        self.begin_recipe_inspection(RecipeInspectionRequest::DependencyGraph { recipe });
    }

    fn begin_recipe_inspection(&mut self, request: RecipeInspectionRequest) {
        if self.recipe_inspection_operation.is_some() {
            self.fail_recipe_inspection(
                request,
                "another recipe inspection is already running".into(),
            );
            return;
        }

        let build_dir = self.session_build_dir.clone();
        let cancellation_timeout = self.cancellation_timeout;
        let authoritative = self.metadata_backend_authoritative;
        let placeholder: Box<dyn BitBakeBackend> = Box::new(ProcessBackend::new(build_dir.clone()));
        let mut backend = std::mem::replace(&mut self.backend, placeholder);
        let worker_request = request.clone();
        let handle = tokio::spawn(async move {
            let mut ready = authoritative;
            if !ready {
                match select_backend_with_timeout(
                    Backend::Bridge,
                    build_dir,
                    Some(cancellation_timeout),
                )
                .await
                {
                    Ok(replacement) => {
                        let _ = backend.shutdown().await;
                        backend = replacement;
                        ready = true;
                    }
                    Err(error) => {
                        return (
                            backend,
                            false,
                            failed_result(worker_request, format!("{error:#}")),
                        );
                    }
                }
            }
            let result = match worker_request {
                RecipeInspectionRequest::Metadata { recipe, followup } => {
                    let result = backend
                        .get_recipe_metadata(recipe.clone())
                        .await
                        .map_err(|error| error.to_string());
                    RecipeInspectionResult::Metadata {
                        recipe,
                        followup,
                        result,
                    }
                }
                RecipeInspectionRequest::DependencyGraph { recipe } => {
                    let result = backend
                        .get_dependency_graph(recipe.clone())
                        .await
                        .map_err(|error| error.to_string());
                    RecipeInspectionResult::DependencyGraph { recipe, result }
                }
            };
            (backend, ready, result)
        });
        self.recipe_inspection_operation = Some(RecipeInspectionOperation { request, handle });
    }

    pub(super) async fn poll_recipe_inspection(&mut self) -> bool {
        if !self
            .recipe_inspection_operation
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return false;
        }
        let operation = self
            .recipe_inspection_operation
            .take()
            .expect("finished recipe inspection operation");
        match operation.handle.await {
            Ok((backend, authoritative, result)) => {
                self.backend = backend;
                self.metadata_backend_authoritative = authoritative;
                self.apply_recipe_inspection(result);
            }
            Err(error) => self.fail_recipe_inspection(
                operation.request,
                format!("recipe inspection task failed: {error}"),
            ),
        }
        true
    }

    fn apply_recipe_inspection(&mut self, result: RecipeInspectionResult) {
        match result {
            RecipeInspectionResult::Metadata {
                recipe,
                followup,
                result,
            } => match result {
                Ok(metadata) => {
                    let _ = compatibility_workspace_action(
                        &mut self.app,
                        Action::RecipeMetadataLoaded(metadata),
                    );
                    self.apply_recipe_metadata_followup(&recipe, followup);
                }
                Err(message) => self.fail_recipe_inspection(
                    RecipeInspectionRequest::Metadata { recipe, followup },
                    message,
                ),
            },
            RecipeInspectionResult::DependencyGraph { recipe, result } => match result {
                Ok(response) => {
                    let action = if response.limitations.is_empty() {
                        Action::DependencyGraphLoaded(response.graph)
                    } else {
                        Action::DependencyGraphPartial {
                            graph: response.graph,
                            limitations: response.limitations,
                        }
                    };
                    let _ = compatibility_workspace_action(&mut self.app, action);
                }
                Err(message) => self.fail_recipe_inspection(
                    RecipeInspectionRequest::DependencyGraph { recipe },
                    message,
                ),
            },
        }
    }

    fn apply_recipe_metadata_followup(
        &mut self,
        recipe: &str,
        followup: Option<RecipeMetadataFollowup>,
    ) {
        let selected_recipe = self
            .app
            .workspace
            .recipes
            .get(self.app.recipe_selection)
            .map(|selected| selected.name.as_str());
        if matches!(followup, Some(RecipeMetadataFollowup::PatchReview))
            && self.app.screen == Screen::Recipes
            && selected_recipe == Some(recipe)
        {
            let _ = compatibility_workspace_action(
                &mut self.app,
                Action::BeginSelectedRecipePatchReview,
            );
        } else if followup.is_some() {
            self.app.notification = Some(format!(
                "Metadata for {recipe} loaded; patch review was not opened because the selection changed."
            ));
        }
    }

    fn fail_recipe_inspection(&mut self, request: RecipeInspectionRequest, message: String) {
        let action = match request {
            RecipeInspectionRequest::Metadata { recipe, .. } => {
                Action::RecipeMetadataFailed { recipe, message }
            }
            RecipeInspectionRequest::DependencyGraph { recipe } => Action::DependencyGraphFailed {
                root: yoctui_model::DependencyNodeId::recipe(recipe),
                message,
            },
        };
        let _ = compatibility_workspace_action(&mut self.app, action);
    }

    pub(super) async fn stop_recipe_inspection(&mut self) {
        if let Some(operation) = self.recipe_inspection_operation.take() {
            operation.handle.abort();
            let _ = operation.handle.await;
        }
    }
}

fn failed_result(request: RecipeInspectionRequest, message: String) -> RecipeInspectionResult {
    match request {
        RecipeInspectionRequest::Metadata { recipe, followup } => {
            RecipeInspectionResult::Metadata {
                recipe,
                followup,
                result: Err(message),
            }
        }
        RecipeInspectionRequest::DependencyGraph { recipe } => {
            RecipeInspectionResult::DependencyGraph {
                recipe,
                result: Err(message),
            }
        }
    }
}
