pub(crate) fn task_stats(data: Option<TaskStatsData>) -> Option<TaskStats> {
    data.map(|stats| TaskStats {
        completed: stats.completed,
        total: stats.total,
        active: stats.active,
        failed: stats.failed,
    })
}

pub(crate) fn dependency_node_id(
    data: DependencyNodeIdData,
) -> Result<DependencyNodeId, BackendError> {
    if data.recipe.is_empty()
        || data.recipe.len() > 512
        || data.recipe.chars().any(char::is_whitespace)
        || data.recipe.chars().any(char::is_control)
        || data.task.as_ref().is_some_and(|task| {
            task.is_empty()
                || task.len() > 512
                || task.chars().any(char::is_whitespace)
                || task.chars().any(char::is_control)
        })
    {
        return Err(BackendError::Bridge(
            "protocol dependency graph contains an invalid node identity".into(),
        ));
    }
    Ok(match data.task {
        Some(task) => DependencyNodeId::task(data.recipe, task),
        None => DependencyNodeId::recipe(data.recipe),
    })
}

pub(crate) fn legacy_dependency_graph(
    recipe: String,
    build: Vec<String>,
    runtime: Vec<String>,
) -> DependencyGraphResponse {
    let root = DependencyNodeId::recipe(recipe);
    let edges = build
        .into_iter()
        .map(|dependency| DependencyEdge {
            from: root.clone(),
            to: DependencyNodeId::recipe(dependency),
            kind: DependencyEdgeKind::Build,
        })
        .chain(runtime.into_iter().map(|dependency| DependencyEdge {
            from: root.clone(),
            to: DependencyNodeId::recipe(dependency),
            kind: DependencyEdgeKind::Runtime,
        }))
        .collect();
    let (graph, _) = DependencyGraph::normalize(
        root,
        Vec::new(),
        edges,
        MAX_DEPENDENCY_NODES,
        MAX_DEPENDENCY_EDGES,
    );
    DependencyGraphResponse {
        graph,
        limitations: vec![
            "Legacy bridge supplied direct recipe edges only; task dependencies are unavailable."
                .into(),
        ],
    }
}

pub(crate) fn dot_quoted_id(line: &str) -> Result<(String, &str), BackendError> {
    let Some(content) = line.strip_prefix('"') else {
        return Err(BackendError::Bridge(
            "malformed dependency graph identifier".into(),
        ));
    };
    let mut value = String::new();
    let mut escaped = false;
    for (offset, character) in content.char_indices() {
        if escaped {
            match character {
                '"' | '\\' => value.push(character),
                _ => {
                    return Err(BackendError::Bridge(
                        "unsupported escape in dependency graph identifier".into(),
                    ));
                }
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Ok((value, &content[offset + character.len_utf8()..]));
        } else {
            value.push(character);
        }
    }
    Err(BackendError::Bridge(
        "unterminated dependency graph identifier".into(),
    ))
}

pub(crate) fn dependency_task_identity(value: String) -> Result<DependencyNodeId, BackendError> {
    let Some((recipe, task)) = value.rsplit_once('.') else {
        return Err(BackendError::Bridge(
            "dependency graph task identity has no task separator".into(),
        ));
    };
    if recipe.is_empty()
        || task.is_empty()
        || recipe.len() > 512
        || task.len() > 512
        || recipe.chars().any(char::is_whitespace)
        || task.chars().any(char::is_whitespace)
        || recipe.chars().any(char::is_control)
        || task.chars().any(char::is_control)
    {
        return Err(BackendError::Bridge(
            "dependency graph contains an invalid task identity".into(),
        ));
    }
    Ok(DependencyNodeId::task(recipe, task))
}

pub(crate) fn parse_task_dependency_dot(
    recipe: &str,
    bytes: &[u8],
) -> Result<DependencyGraphResponse, BackendError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| BackendError::Bridge("dependency graph is not valid UTF-8".into()))?;
    let root = DependencyNodeId::recipe(recipe);
    let mut nodes = vec![DependencyNode::identity(root.clone())];
    let mut edges = Vec::new();
    let mut opened = false;
    let mut closed = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !opened {
            if line != "digraph depends {" {
                return Err(BackendError::Bridge(
                    "dependency graph has an invalid header".into(),
                ));
            }
            opened = true;
            continue;
        }
        if line == "}" {
            closed = true;
            continue;
        }
        if closed {
            return Err(BackendError::Bridge(
                "dependency graph contains records after its closing brace".into(),
            ));
        }

        let (source, remainder) = dot_quoted_id(line)?;
        let remainder = remainder.trim_start();
        if remainder.starts_with('[') {
            if !remainder.trim_end_matches(';').ends_with(']') {
                return Err(BackendError::Bridge(
                    "dependency graph node contains malformed attributes".into(),
                ));
            }
            nodes.push(DependencyNode::identity(dependency_task_identity(source)?));
            continue;
        }
        let Some(remainder) = remainder.strip_prefix("->") else {
            return Err(BackendError::Bridge(
                "dependency graph contains an unsupported record".into(),
            ));
        };
        let (destination, trailing) = dot_quoted_id(remainder.trim_start())?;
        if !trailing.trim().trim_end_matches(';').is_empty() {
            return Err(BackendError::Bridge(
                "dependency graph edge contains unsupported attributes".into(),
            ));
        }
        let from = dependency_task_identity(source)?;
        let to = dependency_task_identity(destination)?;
        nodes.push(DependencyNode::identity(from.clone()));
        nodes.push(DependencyNode::identity(to.clone()));
        for task in [&from, &to] {
            let recipe_node = DependencyNodeId::recipe(task.recipe_name());
            nodes.push(DependencyNode::identity(recipe_node.clone()));
            // `bitbake -g` emits task-to-task edges but the workspace root is
            // a recipe node. Preserve an explicit recipe-to-task bridge so
            // task dependencies are reachable instead of becoming orphans.
            edges.push(DependencyEdge {
                from: recipe_node,
                to: task.clone(),
                kind: DependencyEdgeKind::Task,
            });
        }
        edges.push(DependencyEdge {
            from: from.clone(),
            to: to.clone(),
            kind: DependencyEdgeKind::Task,
        });
        if from.recipe_name() != to.recipe_name() {
            edges.push(DependencyEdge {
                from: DependencyNodeId::recipe(from.recipe_name()),
                to: DependencyNodeId::recipe(to.recipe_name()),
                kind: DependencyEdgeKind::Build,
            });
        }
    }
    if !opened || !closed {
        return Err(BackendError::Bridge(
            "dependency graph is incomplete".into(),
        ));
    }
    let (graph, report) = DependencyGraph::normalize(
        root,
        nodes,
        edges,
        MAX_DEPENDENCY_NODES,
        MAX_DEPENDENCY_EDGES,
    );
    let mut limitations = vec![
        "The process backend task graph does not report runtime dependency edges.".into(),
        "The process backend task graph does not report provider or task-log paths.".into(),
    ];
    if report.is_partial() {
        limitations.push(format!(
            "Dependency graph bounds dropped {} nodes and {} edges.",
            report.truncated_nodes, report.truncated_edges
        ));
    }
    Ok(DependencyGraphResponse { graph, limitations })
}
