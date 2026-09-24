//! Dependency render.
use super::*;

pub(crate) fn dependency_identity_text(identity: &DependencyNodeId) -> String {
    match identity {
        DependencyNodeId::Recipe(recipe) => recipe.clone(),
        DependencyNodeId::Task { recipe, task } => format!("{recipe}:{task}"),
    }
}

pub(crate) fn dependency_kind_text(identity: &DependencyNodeId) -> &'static str {
    match identity {
        DependencyNodeId::Recipe(_) => "recipe",
        DependencyNodeId::Task { .. } => "task",
    }
}

pub(crate) fn dependency_edge_kind_text(kind: DependencyEdgeKind) -> &'static str {
    match kind {
        DependencyEdgeKind::Build => "build",
        DependencyEdgeKind::Runtime => "runtime",
        DependencyEdgeKind::Task => "task",
    }
}

pub(crate) fn dependency_edge_context(
    graph: &DependencyGraph,
    selected: &DependencyNodeId,
    incoming: bool,
) -> String {
    let edges = if incoming {
        graph.incoming(selected)
    } else {
        graph.outgoing(selected)
    };
    if edges.is_empty() {
        return "none reported".into();
    }
    let total = edges.len();
    let mut values = edges
        .into_iter()
        .take(8)
        .map(|edge| {
            let identity = if incoming { &edge.from } else { &edge.to };
            format!(
                "{}: {}",
                dependency_edge_kind_text(edge.kind),
                dependency_identity_text(identity)
            )
        })
        .collect::<Vec<_>>();
    if total > values.len() {
        values.push(format!("… {} more", total - values.len()));
    }
    values.join("\n")
}

pub(crate) fn dependency_why_built(graph: &DependencyGraph, selected: &DependencyNodeId) -> String {
    match graph.why_built(selected, 64, 4_096) {
        DependencyPathResult::Found(path) if path.len() == 1 => "root selected".into(),
        DependencyPathResult::Found(path) => {
            let mut text = dependency_identity_text(&path[0]);
            for pair in path.windows(2) {
                let kind = graph
                    .edges
                    .iter()
                    .find(|edge| edge.from == pair[0] && edge.to == pair[1])
                    .map_or("unknown", |edge| dependency_edge_kind_text(edge.kind));
                text.push_str(&format!(
                    "\n  --{kind}--> {}",
                    dependency_identity_text(&pair[1])
                ));
            }
            text
        }
        DependencyPathResult::Unreachable => "unreachable from root".into(),
        DependencyPathResult::LimitReached => "path limit reached".into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DependencyLayoutMode {
    Topology,
    Tree,
    Table,
}

pub(crate) fn dependency_layout_mode(terminal_width: u16) -> DependencyLayoutMode {
    if terminal_width >= 130 {
        DependencyLayoutMode::Topology
    } else if terminal_width >= 100 {
        DependencyLayoutMode::Tree
    } else {
        DependencyLayoutMode::Table
    }
}

pub(crate) fn dependency_tree_label(
    row: &yoctui_model::DependencyProjectionRow,
    selected: bool,
    ascii: bool,
) -> String {
    let indent = "  ".repeat(row.depth.min(64));
    let branch = if row.depth == 0 {
        ""
    } else if ascii {
        "`-"
    } else {
        "└─"
    };
    let expansion = match (row.has_children, row.collapsed) {
        (true, true) => "+ ",
        (true, false) => "- ",
        (false, _) => "  ",
    };
    let relation = row
        .edge_kind
        .map(|kind| format!("[{}] ", dependency_edge_kind_text(kind)))
        .unwrap_or_default();
    format!(
        "{}{}{}{}{}{}",
        if selected { "> " } else { "  " },
        indent,
        branch,
        expansion,
        relation,
        dependency_identity_text(&row.id)
    )
}

pub(crate) fn dependency_inspector(app: &App) -> String {
    match &app.dependency_graph {
        DependencyGraphState::NotLoaded => {
            "Dependency graph: not loaded\n\nPress r to choose a recipe, or use F12 > Actions."
                .into()
        }
        DependencyGraphState::Loading { root } => format!(
            "Dependency graph: loading\nRoot: {}\n\nNo stale graph is shown while the authoritative query runs.",
            dependency_identity_text(root)
        ),
        DependencyGraphState::AvailableEmpty { root } => format!(
            "Root: {}\nState: available-empty\n\nNo dependency edges reported.",
            dependency_identity_text(root)
        ),
        DependencyGraphState::Failed { root, message } => format!(
            "Root: {}\nState: failed\n\n{message}\n\nNo stale graph is presented as current.",
            dependency_identity_text(root)
        ),
        DependencyGraphState::Available(graph) | DependencyGraphState::Partial { graph, .. } => {
            let selected = app
                .dependency_graph_selection
                .as_ref()
                .and_then(|identity| graph.nodes.iter().find(|node| &node.id == identity));
            let Some(node) = selected else {
                return format!(
                    "Root: {}\n\nNo dependency node is selected.",
                    dependency_identity_text(&graph.root)
                );
            };
            let limitations = match &app.dependency_graph {
                DependencyGraphState::Partial { limitations, .. } if !limitations.is_empty() => {
                    limitations
                        .iter()
                        .map(|value| format!("- {value}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                _ => "none".into(),
            };
            let position = graph
                .nodes
                .iter()
                .position(|candidate| candidate.id == node.id)
                .map_or_else(
                    || "unavailable".into(),
                    |index| format!("{} of {}", index + 1, graph.nodes.len()),
                );
            format!(
                "Root: {}\nSelected: {} ({})\nPosition: {}\nView: {}\nFilter: {}{}\nProvider: {}\nTask log: {}\n\nReverse / incoming:\n{}\n\nDependencies / outgoing:\n{}\n\nWhy built:\n{}\n\nLimitations:\n{}\n\nControls: arrows/jk navigate; left/right collapse/expand; space toggles; / filters; v reverses; Enter opens recipe; o opens provider; L opens task log.",
                dependency_identity_text(&graph.root),
                dependency_identity_text(&node.id),
                dependency_kind_text(&node.id),
                position,
                if app.dependency_graph_reverse {
                    "reverse dependencies"
                } else {
                    "forward dependencies"
                },
                if app.dependency_graph_query.is_empty() {
                    "none"
                } else {
                    &app.dependency_graph_query
                },
                if app.dependency_graph_searching {
                    " (editing)"
                } else {
                    ""
                },
                node.provider
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                node.log
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                dependency_edge_context(graph, &node.id, true),
                dependency_edge_context(graph, &node.id, false),
                dependency_why_built(graph, &node.id),
                limitations,
            )
        }
    }
}

pub(crate) fn dependencies(frame: &mut Frame, app: &App, area: Rect) {
    let (graph, partial) = match &app.dependency_graph {
        DependencyGraphState::NotLoaded => {
            frame.render_widget(
                Paragraph::new(
                    "Dependency graph is not loaded.\n\nPress r to choose a recipe, or use F12 > Actions.",
                )
                .block(
                    Block::default()
                        .title("Dependency graph · not loaded")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::Loading { root } => {
            frame.render_widget(
                Paragraph::new(format!(
                    "Loading authoritative dependency graph for {}…\n\nStale rows are hidden.",
                    dependency_identity_text(root)
                ))
                .block(
                    Block::default()
                        .title("Dependency graph · loading")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::AvailableEmpty { root } => {
            frame.render_widget(
                Paragraph::new(format!(
                    "Root: {}\n\nNo dependency edges reported.",
                    dependency_identity_text(root)
                ))
                .block(
                    Block::default()
                        .title("Dependency graph · available-empty")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::Failed { root, message } => {
            frame.render_widget(
                Paragraph::new(format!(
                    "Root: {}\n\n{message}\n\nNo stale graph is presented as current.",
                    dependency_identity_text(root)
                ))
                .block(
                    Block::default()
                        .title("Dependency graph · failed")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::Available(graph) => (graph, false),
        DependencyGraphState::Partial { graph, .. } => (graph, true),
    };

    let mut counts: HashMap<&DependencyNodeId, (usize, usize)> = HashMap::new();
    for edge in &graph.edges {
        counts.entry(&edge.from).or_default().1 += 1;
        counts.entry(&edge.to).or_default().0 += 1;
    }
    let anchor = app.dependency_graph_anchor.as_ref().unwrap_or(&graph.root);
    let projection = graph.project(
        anchor,
        app.dependency_graph_reverse,
        &app.dependency_graph_query,
        &app.dependency_graph_collapsed,
        64,
        8_192,
    );
    let selected_index = app
        .dependency_graph_selection
        .as_ref()
        .and_then(|selected| projection.rows.iter().position(|row| &row.id == selected))
        .unwrap_or(0);
    let capacity = area.height.saturating_sub(3).max(1) as usize;
    let start = selected_index.saturating_add(1).saturating_sub(capacity);
    let end = projection.rows.len().min(start.saturating_add(capacity));
    let mode = dependency_layout_mode(frame.area().width);
    let rows = projection.rows[start..end].iter().map(|row| {
        let selected = app.dependency_graph_selection.as_ref() == Some(&row.id);
        let (incoming, outgoing) = counts.get(&row.id).copied().unwrap_or_default();
        let position = format!("{}/{}", row.source_index + 1, projection.source_total);
        let cells = match mode {
            DependencyLayoutMode::Topology => vec![
                Cell::from(position),
                Cell::from(dependency_tree_label(row, selected, app.color_forced_off)),
                Cell::from(incoming.to_string()),
                Cell::from(outgoing.to_string()),
            ],
            DependencyLayoutMode::Tree => vec![
                Cell::from(position),
                Cell::from(dependency_tree_label(row, selected, app.color_forced_off)),
                Cell::from(row.depth.to_string()),
            ],
            DependencyLayoutMode::Table => vec![
                Cell::from(position),
                Cell::from(dependency_kind_text(&row.id)),
                Cell::from(dependency_identity_text(&row.id)),
                Cell::from(
                    row.edge_kind
                        .map(dependency_edge_kind_text)
                        .unwrap_or("root"),
                ),
                Cell::from(incoming.to_string()),
                Cell::from(outgoing.to_string()),
            ],
        };
        Row::new(cells).style(selected_style(app, selected))
    });
    let (headers, constraints): (Vec<&str>, Vec<Constraint>) = match mode {
        DependencyLayoutMode::Topology => (
            vec!["Pos", "Topology / relationship", "In", "Out"],
            vec![
                Constraint::Length(9),
                Constraint::Min(24),
                Constraint::Length(4),
                Constraint::Length(4),
            ],
        ),
        DependencyLayoutMode::Tree => (
            vec!["Pos", "Tree / relationship", "Depth"],
            vec![
                Constraint::Length(9),
                Constraint::Min(20),
                Constraint::Length(5),
            ],
        ),
        DependencyLayoutMode::Table => (
            vec!["Pos", "Kind", "Identity", "Rel", "In", "Out"],
            vec![
                Constraint::Length(7),
                Constraint::Length(7),
                Constraint::Min(12),
                Constraint::Length(7),
                Constraint::Length(3),
                Constraint::Length(3),
            ],
        ),
    };
    let limitations = projection.hidden_by_filter
        + projection.hidden_by_collapse
        + projection.truncated_depth
        + projection.truncated_rows;
    frame.render_widget(
        Table::new(rows, constraints)
            .header(Row::new(headers).style(Style::default().add_modifier(Modifier::BOLD)))
            .block(
                Block::default()
                    .title(format!(
                        "Dependency {}: {} · {} nodes · {} edges · {}{}{}{}",
                        match mode {
                            DependencyLayoutMode::Topology => "topology",
                            DependencyLayoutMode::Tree => "tree",
                            DependencyLayoutMode::Table => "table",
                        },
                        dependency_identity_text(&graph.root),
                        graph.nodes.len(),
                        graph.edges.len(),
                        if app.dependency_graph_reverse {
                            "reverse"
                        } else {
                            "forward"
                        },
                        if app.dependency_graph_searching {
                            " · search editing"
                        } else {
                            ""
                        },
                        if app.dependency_graph_query.is_empty() {
                            String::new()
                        } else {
                            format!(" · filter={}", app.dependency_graph_query)
                        },
                        if partial || limitations > 0 || projection.cycle_edges > 0 {
                            format!(
                                " · partial/hidden={} cycles={}",
                                limitations, projection.cycle_edges
                            )
                        } else {
                            String::new()
                        }
                    ))
                    .borders(Borders::ALL),
            ),
        area,
    );
}

pub(crate) fn layer_relationships(frame: &mut Frame, app: &App, area: Rect) {
    let text = app.layer_relationships.as_ref().map_or_else(
        || "No layer relationship data is loaded. Open Layers and press i.".into(),
        |relationships| relationships.layers.iter().map(|layer| format!(
            "{} (priority: {})\n  compatible: {}\n  depends: {}\n  overlays: {}\n  appends: {}",
            layer.name, layer.priority.map_or_else(|| "unknown".into(), |value| value.to_string()),
            list_or_none(&layer.compatible), list_or_none(&layer.depends), list_or_none(&layer.overlays), list_or_none(&layer.appends)
        )).collect::<Vec<_>>().join("\n\n"),
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Layer relationships (server supplied)")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}
