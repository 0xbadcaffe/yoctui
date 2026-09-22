impl BridgeBackend {
    pub(crate) fn event(event: Event) -> Result<BackendEvent, BackendError> {
        Ok(match event {
            Event::Workspace { data } => BackendEvent::Workspace(Workspace {
                build_dir: data.build_dir.map(PathBuf::from),
                source_dir: data.source_dir.map(PathBuf::from),
                variables: data.variables,
                variable_provenance: data.variable_provenance,
                variable_provenance_chain: data.variable_provenance_chain,
                bitbake_version: data.bitbake_version,
                release: data.release,
                layers: data
                    .layers
                    .into_iter()
                    .map(|layer| Layer {
                        name: layer.name,
                        path: PathBuf::from(layer.path),
                        priority: layer.priority,
                    })
                    .collect(),
                recipes: data
                    .recipes
                    .into_iter()
                    .map(|recipe| Recipe {
                        name: recipe.name,
                        version: recipe.version,
                        layer: recipe.layer,
                        preferred_version: recipe.preferred_version,
                        file: recipe.file.map(PathBuf::from),
                        append_count: recipe.append_count,
                    })
                    .collect(),
            }),
            Event::Recipes { recipes } => BackendEvent::Recipes(
                recipes
                    .into_iter()
                    .map(
                        |RecipeData {
                             name,
                             version,
                             layer,
                             preferred_version,
                             file,
                             append_count,
                         }| Recipe {
                            name,
                            version,
                            layer,
                            preferred_version,
                            file: file.map(PathBuf::from),
                            append_count,
                        },
                    )
                    .collect(),
            ),
            Event::RecipesChunk { .. } => {
                return Err(BackendError::Bridge(
                    "recipe chunk outside an active inventory request".into(),
                ));
            }
            Event::Layers { layers } => BackendEvent::Layers(
                layers
                    .into_iter()
                    .map(
                        |LayerData {
                             name,
                             path,
                             priority,
                         }| Layer {
                            name,
                            path: PathBuf::from(path),
                            priority,
                        },
                    )
                    .collect(),
            ),
            Event::Variable {
                name,
                recipe,
                value,
                provenance,
                unexpanded_value,
                operations,
                active_overrides,
            } => BackendEvent::Variable {
                name,
                recipe,
                value,
                provenance,
                unexpanded_value,
                operations: operations
                    .into_iter()
                    .map(|operation| VariableOperation {
                        operation: operation.operation,
                        file: operation.file.map(PathBuf::from),
                        line: operation.line,
                        value: operation.value,
                    })
                    .collect(),
                active_overrides,
            },
            Event::Dependencies {
                recipe,
                build,
                runtime,
            } => BackendEvent::Dependencies {
                recipe,
                build,
                runtime,
            },
            Event::DependencyGraph { data } => {
                let DependencyGraphData {
                    root,
                    nodes,
                    edges,
                    mut limitations,
                } = data;
                let root = dependency_node_id(root)?;
                let mut dropped_paths = 0;
                let nodes = nodes
                    .into_iter()
                    .map(|DependencyNodeData { id, provider, log }| {
                        let provider = provider.map(PathBuf::from).and_then(|path| {
                            if path.is_absolute() {
                                Some(path)
                            } else {
                                dropped_paths += 1;
                                None
                            }
                        });
                        let log = log.map(PathBuf::from).and_then(|path| {
                            if path.is_absolute() {
                                Some(path)
                            } else {
                                dropped_paths += 1;
                                None
                            }
                        });
                        Ok(DependencyNode {
                            id: dependency_node_id(id)?,
                            provider,
                            log,
                        })
                    })
                    .collect::<Result<Vec<_>, BackendError>>()?;
                let edges = edges
                    .into_iter()
                    .map(|DependencyEdgeData { from, to, kind }| {
                        Ok(DependencyEdge {
                            from: dependency_node_id(from)?,
                            to: dependency_node_id(to)?,
                            kind: match kind {
                                DependencyEdgeKindData::Build => DependencyEdgeKind::Build,
                                DependencyEdgeKindData::Runtime => DependencyEdgeKind::Runtime,
                                DependencyEdgeKindData::Task => DependencyEdgeKind::Task,
                            },
                        })
                    })
                    .collect::<Result<Vec<_>, BackendError>>()?;
                let (graph, report) = DependencyGraph::normalize(
                    root,
                    nodes,
                    edges,
                    MAX_DEPENDENCY_NODES,
                    MAX_DEPENDENCY_EDGES,
                );
                if report.is_partial() {
                    limitations.push(format!(
                        "Rust adapter bounds dropped {} nodes and {} edges",
                        report.truncated_nodes, report.truncated_edges
                    ));
                }
                if dropped_paths > 0 {
                    limitations.push(format!(
                        "Rust adapter dropped {dropped_paths} non-absolute provider or log paths"
                    ));
                }
                BackendEvent::DependencyGraph { graph, limitations }
            }
            Event::RecipeSources { recipe, paths } => BackendEvent::RecipeSources {
                recipe,
                paths: paths.into_iter().map(PathBuf::from).collect(),
            },
            Event::RecipeMetadata { data } => BackendEvent::RecipeMetadata(RecipeMetadata {
                recipe: data.recipe,
                workspace_status: data.workspace_status.map(|status| match status {
                    RecipeWorkspaceStatusData::Clean => RecipeWorkspaceStatus::Clean,
                    RecipeWorkspaceStatusData::Modified => RecipeWorkspaceStatus::Modified,
                }),
                build_status: data.build_status.map(|status| match status {
                    RecipeBuildStatusData::Idle => RecipeBuildStatus::Idle,
                    RecipeBuildStatusData::Queued => RecipeBuildStatus::Queued,
                    RecipeBuildStatusData::Running => RecipeBuildStatus::Running,
                    RecipeBuildStatusData::Succeeded => RecipeBuildStatus::Succeeded,
                    RecipeBuildStatusData::Failed => RecipeBuildStatus::Failed,
                    RecipeBuildStatusData::Cancelled => RecipeBuildStatus::Cancelled,
                }),
                tasks: data.tasks,
                sources: data
                    .sources
                    .map(|paths| paths.into_iter().map(PathBuf::from).collect()),
                patches: data.patches,
                packages: data.packages,
                history: data.history,
            }),
            Event::LayerRelationships { layers } => BackendEvent::LayerRelationships(
                layers
                    .into_iter()
                    .map(
                        |LayerRelationshipData {
                             name,
                             priority,
                             compatible,
                             depends,
                             overlays,
                             appends,
                         }| LayerRelationship {
                            name,
                            priority,
                            compatible,
                            depends,
                            overlays,
                            appends,
                        },
                    )
                    .collect(),
            ),
            Event::BuildStarted => BackendEvent::BuildStarted,
            Event::TaskStats { stats } => BackendEvent::TaskStats(TaskStats {
                completed: stats.completed,
                total: stats.total,
                active: stats.active,
                failed: stats.failed,
            }),
            Event::ParseProgress { current, total } => {
                BackendEvent::ParseProgress { current, total }
            }
            Event::TaskQueued {
                recipe,
                task,
                worker,
                stats,
            } => BackendEvent::TaskQueued {
                recipe,
                task,
                worker,
                stats: task_stats(stats),
            },
            Event::TaskStarted {
                recipe,
                task,
                pid,
                worker,
                log_path,
                stats,
            } => BackendEvent::TaskStarted {
                recipe,
                task,
                pid,
                worker,
                log_path: log_path.map(PathBuf::from),
                stats: task_stats(stats),
            },
            Event::TaskProgress {
                recipe,
                task,
                progress,
            } => BackendEvent::TaskProgress {
                recipe,
                task,
                progress,
            },
            Event::TaskCompleted {
                recipe,
                task,
                success,
            } => BackendEvent::TaskCompleted {
                recipe,
                task,
                success,
            },
            Event::Log {
                level,
                message,
                recipe,
                task,
                path,
            } => {
                let severity = match level.as_str() {
                    "warning" => Severity::Warning,
                    "error" | "critical" | "fatal" => Severity::Error,
                    _ => Severity::Info,
                };
                if severity == Severity::Info
                    && recipe.is_none()
                    && task.is_none()
                    && let Some(summary) = crate::build_cache::parse_sstate_summary(&message)
                {
                    return Ok(BackendEvent::SstateSummary(summary));
                }
                BackendEvent::Log(LogEntry {
                    id: 0,
                    severity,
                    message,
                    recipe,
                    task,
                    path: path.map(PathBuf::from),
                    timestamp: SystemTime::now(),
                    build: None,
                    protected: false,
                    diagnostic: None,
                })
            }
            Event::Warning { message } => BackendEvent::Log(LogEntry {
                id: 0,
                severity: Severity::Warning,
                message,
                recipe: None,
                task: None,
                path: None,
                timestamp: SystemTime::now(),
                build: None,
                protected: true,
                diagnostic: None,
            }),
            Event::Error { message } => BackendEvent::Log(LogEntry {
                id: 0,
                severity: Severity::Error,
                message,
                recipe: None,
                task: None,
                path: None,
                timestamp: SystemTime::now(),
                build: None,
                protected: true,
                diagnostic: None,
            }),
            Event::BuildCompleted { success, exit_code } => {
                BackendEvent::BuildCompleted { success, exit_code }
            }
            Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                BackendEvent::CommandFailed { code, message }
            }
            Event::BridgeShutdown | Event::ServerTerminated => BackendEvent::Disconnected,
            Event::HelloAck { .. } | Event::Unknown => BackendEvent::Ignored,
        })
    }
}
