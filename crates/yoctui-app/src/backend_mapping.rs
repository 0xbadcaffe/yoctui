//! Backend mapping.
use super::*;

pub fn model_action_from_backend_event(event: BackendEvent) -> Option<Action> {
    match event {
        BackendEvent::Workspace(workspace) => Some(Action::WorkspaceLoaded(workspace)),
        BackendEvent::BuildStarted => Some(Action::BuildStarted),
        BackendEvent::TaskStats(stats) => Some(Action::TaskStats(stats)),
        BackendEvent::SstateSummary(summary) => Some(Action::SstateSummary(summary)),
        BackendEvent::ParseProgress { current, total } => {
            Some(Action::ParseProgress { current, total })
        }
        BackendEvent::Log(entry) => Some(Action::Log(entry)),
        BackendEvent::TaskQueued {
            recipe,
            task,
            worker,
            stats,
        } => {
            let id = TaskId(format!("{recipe}:{task}"));
            let mut info = TaskInfo::active(id, recipe, task);
            info.worker = worker;
            info.stats = stats;
            Some(Action::TaskQueued(info))
        }
        BackendEvent::TaskStarted {
            recipe,
            task,
            pid,
            worker,
            log_path,
            stats,
        } => {
            let id = TaskId(format!("{recipe}:{task}"));
            let mut info = TaskInfo::active(id, recipe, task);
            info.pid = pid;
            info.worker = worker;
            info.log_path = log_path;
            info.stats = stats;
            Some(Action::TaskStarted(info))
        }
        BackendEvent::TaskProgress {
            recipe,
            task,
            progress,
        } => Some(Action::TaskProgress {
            id: TaskId(format!("{recipe}:{task}")),
            progress,
        }),
        BackendEvent::TaskCompleted {
            recipe,
            task,
            success,
        } => Some(Action::TaskCompleted {
            id: TaskId(format!("{recipe}:{task}")),
            success,
        }),
        BackendEvent::BuildCompleted { success, exit_code } => {
            Some(Action::BuildCompleted { success, exit_code })
        }
        BackendEvent::CommandFailed { code, message } => Some(Action::Failure(AppError::new(
            "BitBake",
            format!("{code}: {message}"),
            "inspect the bridge or BitBake diagnostics",
        ))),
        BackendEvent::Disconnected => Some(Action::Failure(AppError::new(
            "Bridge",
            "backend disconnected",
            "restart Yoctui and inspect the backend diagnostics",
        ))),
        BackendEvent::Recipes(recipes) => Some(Action::RecipesLoaded(recipes)),
        BackendEvent::Layers(layers) => Some(Action::LayersLoaded(layers)),
        BackendEvent::Variable {
            name,
            recipe,
            value,
            provenance,
            unexpanded_value,
            operations,
            active_overrides,
        } => Some(Action::VariableLoaded(VariableDetail {
            identity: VariableIdentity { name, recipe },
            effective_value: value,
            unexpanded_value,
            provenance,
            operations,
            active_overrides,
        })),
        BackendEvent::Dependencies {
            recipe,
            build,
            runtime,
        } => Some(Action::DependenciesLoaded(RecipeDependencies {
            recipe,
            build,
            runtime,
        })),
        BackendEvent::DependencyGraph { graph, limitations } => {
            if limitations.is_empty() {
                Some(Action::DependencyGraphLoaded(graph))
            } else {
                Some(Action::DependencyGraphPartial { graph, limitations })
            }
        }
        BackendEvent::DependencyGraphFailed { root, message } => {
            Some(Action::DependencyGraphFailed { root, message })
        }
        BackendEvent::SignatureDump {
            target,
            records,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::SignatureDumpLoaded { target, records })
            } else {
                Some(Action::SignatureDumpPartial {
                    target,
                    records,
                    limitations,
                })
            }
        }
        BackendEvent::SignatureDumpFailed { target, message } => {
            Some(Action::SignatureDumpFailed { target, message })
        }
        BackendEvent::SignatureComparison {
            request,
            differences,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::SignatureComparisonLoaded {
                    request,
                    differences,
                })
            } else {
                Some(Action::SignatureComparisonPartial {
                    request,
                    differences,
                    limitations,
                })
            }
        }
        BackendEvent::SignatureComparisonFailed { request, message } => {
            Some(Action::SignatureComparisonFailed { request, message })
        }
        BackendEvent::PackageInventory {
            request,
            packages,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::PackageInventoryLoaded { request, packages })
            } else {
                Some(Action::PackageInventoryPartial {
                    request,
                    packages,
                    limitations,
                })
            }
        }
        BackendEvent::PackageInventoryFailed { request, message } => {
            Some(Action::PackageInventoryFailed { request, message })
        }
        BackendEvent::PackageDetail {
            request,
            detail,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::PackageDetailLoaded { request, detail })
            } else {
                Some(Action::PackageDetailPartial {
                    request,
                    detail,
                    limitations,
                })
            }
        }
        BackendEvent::PackageDetailFailed { request, message } => {
            Some(Action::PackageDetailFailed { request, message })
        }
        BackendEvent::ImageArtifacts {
            request,
            inventory,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::ImageArtifactInventoryLoaded { request, inventory })
            } else {
                Some(Action::ImageArtifactInventoryPartial {
                    request,
                    inventory,
                    limitations,
                })
            }
        }
        BackendEvent::ImageArtifactsFailed { request, message } => {
            Some(Action::ImageArtifactInventoryFailed { request, message })
        }
        BackendEvent::RootfsComposition {
            request,
            composition,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::RootfsCompositionLoaded {
                    request,
                    composition,
                })
            } else {
                Some(Action::RootfsCompositionPartial {
                    request,
                    composition,
                    limitations,
                })
            }
        }
        BackendEvent::RootfsCompositionUnavailable { request, reason } => {
            Some(Action::RootfsCompositionUnavailable { request, reason })
        }
        BackendEvent::RootfsCompositionFailed { request, message } => {
            Some(Action::RootfsCompositionFailed { request, message })
        }
        BackendEvent::RecipeSources { recipe, paths } => {
            Some(Action::RecipeSourcesLoaded { recipe, paths })
        }
        BackendEvent::RecipeMetadata(metadata) => Some(Action::RecipeMetadataLoaded(metadata)),
        BackendEvent::LayerRelationships(layers) => {
            Some(Action::LayerRelationshipsLoaded(LayerRelationships {
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
            }))
        }
        BackendEvent::Ignored => None,
    }
}
