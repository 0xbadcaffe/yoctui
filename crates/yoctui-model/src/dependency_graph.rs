//! Dependency graph.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecipeDependencies {
    pub recipe: String,
    pub build: Vec<String>,
    pub runtime: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyNodeId {
    Recipe(String),
    Task { recipe: String, task: String },
}
impl DependencyNodeId {
    pub fn recipe(name: impl Into<String>) -> Self {
        Self::Recipe(name.into())
    }

    pub fn task(recipe: impl Into<String>, task: impl Into<String>) -> Self {
        Self::Task {
            recipe: recipe.into(),
            task: task.into(),
        }
    }

    pub fn recipe_name(&self) -> &str {
        match self {
            Self::Recipe(recipe) | Self::Task { recipe, .. } => recipe,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyNode {
    pub id: DependencyNodeId,
    pub provider: Option<PathBuf>,
    pub log: Option<PathBuf>,
}
impl DependencyNode {
    pub fn identity(id: DependencyNodeId) -> Self {
        Self {
            id,
            provider: None,
            log: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyEdgeKind {
    Build,
    Runtime,
    Task,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DependencyEdge {
    pub from: DependencyNodeId,
    pub to: DependencyNodeId,
    pub kind: DependencyEdgeKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DependencyNormalizationReport {
    pub duplicate_nodes: usize,
    pub duplicate_edges: usize,
    pub self_edges: usize,
    pub synthesized_nodes: usize,
    pub truncated_nodes: usize,
    pub truncated_edges: usize,
}
impl DependencyNormalizationReport {
    pub fn is_partial(&self) -> bool {
        self.truncated_nodes > 0 || self.truncated_edges > 0
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraph {
    pub root: DependencyNodeId,
    pub nodes: Vec<DependencyNode>,
    pub edges: Vec<DependencyEdge>,
}
impl DependencyGraph {
    pub fn normalize(
        root: DependencyNodeId,
        mut nodes: Vec<DependencyNode>,
        mut edges: Vec<DependencyEdge>,
        max_nodes: usize,
        max_edges: usize,
    ) -> (Self, DependencyNormalizationReport) {
        let mut report = DependencyNormalizationReport::default();
        nodes.sort();

        let mut normalized_nodes = BTreeMap::new();
        for node in nodes {
            if normalized_nodes.contains_key(&node.id) {
                report.duplicate_nodes += 1;
                continue;
            }
            normalized_nodes.insert(node.id.clone(), node);
        }
        normalized_nodes
            .entry(root.clone())
            .or_insert_with(|| DependencyNode::identity(root.clone()));

        let node_limit = max_nodes.max(1);
        if normalized_nodes.len() > node_limit {
            let removable = normalized_nodes
                .keys()
                .filter(|id| **id != root)
                .skip(node_limit.saturating_sub(1))
                .cloned()
                .collect::<Vec<_>>();
            report.truncated_nodes += removable.len();
            for id in removable {
                normalized_nodes.remove(&id);
            }
        }

        edges.sort();
        let mut unique_edges = BTreeSet::new();
        for edge in edges {
            if edge.from == edge.to {
                report.self_edges += 1;
                continue;
            }
            if !unique_edges.insert(edge.clone()) {
                report.duplicate_edges += 1;
            }
        }

        let mut normalized_edges = Vec::new();
        for edge in unique_edges {
            let missing = [&edge.from, &edge.to]
                .into_iter()
                .filter(|id| !normalized_nodes.contains_key(*id))
                .cloned()
                .collect::<BTreeSet<_>>();
            if normalized_nodes.len() + missing.len() > node_limit {
                report.truncated_edges += 1;
                continue;
            }
            for id in missing {
                normalized_nodes.insert(id.clone(), DependencyNode::identity(id));
                report.synthesized_nodes += 1;
            }
            if normalized_edges.len() == max_edges {
                report.truncated_edges += 1;
                continue;
            }
            normalized_edges.push(edge);
        }

        (
            Self {
                root,
                nodes: normalized_nodes.into_values().collect(),
                edges: normalized_edges,
            },
            report,
        )
    }

    pub fn contains(&self, id: &DependencyNodeId) -> bool {
        self.nodes.iter().any(|node| &node.id == id)
    }

    pub fn incoming(&self, id: &DependencyNodeId) -> Vec<&DependencyEdge> {
        self.edges.iter().filter(|edge| &edge.to == id).collect()
    }

    pub fn outgoing(&self, id: &DependencyNodeId) -> Vec<&DependencyEdge> {
        self.edges.iter().filter(|edge| &edge.from == id).collect()
    }

    pub fn why_built(
        &self,
        target: &DependencyNodeId,
        max_depth: usize,
        max_visited: usize,
    ) -> DependencyPathResult {
        if !self.contains(target) {
            return DependencyPathResult::Unreachable;
        }
        if self.root == *target {
            return DependencyPathResult::Found(vec![self.root.clone()]);
        }
        if max_visited == 0 {
            return DependencyPathResult::LimitReached;
        }

        let mut queue = VecDeque::from([(self.root.clone(), 0_usize)]);
        let mut visited = BTreeSet::from([self.root.clone()]);
        let mut parents = BTreeMap::new();
        let mut limited = false;
        while let Some((current, depth)) = queue.pop_front() {
            if depth == max_depth {
                limited |= !self.outgoing(&current).is_empty();
                continue;
            }
            for edge in self.outgoing(&current) {
                if visited.contains(&edge.to) {
                    continue;
                }
                if visited.len() == max_visited {
                    limited = true;
                    break;
                }
                visited.insert(edge.to.clone());
                parents.insert(edge.to.clone(), current.clone());
                if edge.to == *target {
                    let mut path = vec![target.clone()];
                    let mut cursor = target;
                    while let Some(parent) = parents.get(cursor) {
                        path.push(parent.clone());
                        cursor = parent;
                    }
                    path.reverse();
                    return DependencyPathResult::Found(path);
                }
                queue.push_back((edge.to.clone(), depth + 1));
            }
        }
        if limited {
            DependencyPathResult::LimitReached
        } else {
            DependencyPathResult::Unreachable
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyPathResult {
    Found(Vec<DependencyNodeId>),
    Unreachable,
    LimitReached,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyProjectionRow {
    pub id: DependencyNodeId,
    pub parent: Option<DependencyNodeId>,
    pub edge_kind: Option<DependencyEdgeKind>,
    pub depth: usize,
    pub source_index: usize,
    pub has_children: bool,
    pub collapsed: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraphProjection {
    pub anchor: DependencyNodeId,
    pub reverse: bool,
    pub rows: Vec<DependencyProjectionRow>,
    pub source_total: usize,
    pub hidden_by_filter: usize,
    pub hidden_by_collapse: usize,
    pub truncated_depth: usize,
    pub truncated_rows: usize,
    pub cycle_edges: usize,
}
pub const DEPENDENCY_GRAPH_MAX_QUERY_BYTES: usize = 256;
impl DependencyGraph {
    /// Build a deterministic, reducer-owned tree projection over the graph.
    /// Cross-edges and cycles remain authoritative in `edges`; the projection
    /// visits each identity once and reports references that would revisit it.
    pub fn project(
        &self,
        anchor: &DependencyNodeId,
        reverse: bool,
        query: &str,
        collapsed: &BTreeSet<DependencyNodeId>,
        max_depth: usize,
        max_rows: usize,
    ) -> DependencyGraphProjection {
        let anchor = if self.contains(anchor) {
            anchor.clone()
        } else {
            self.root.clone()
        };
        let mut adjacency =
            BTreeMap::<DependencyNodeId, Vec<(DependencyNodeId, DependencyEdgeKind)>>::new();
        for edge in &self.edges {
            let (from, to) = if reverse {
                (&edge.to, &edge.from)
            } else {
                (&edge.from, &edge.to)
            };
            adjacency
                .entry(from.clone())
                .or_default()
                .push((to.clone(), edge.kind));
        }
        for values in adjacency.values_mut() {
            values.sort();
        }
        let neighbors = |id: &DependencyNodeId| adjacency.get(id).cloned().unwrap_or_default();

        let mut reachable = BTreeSet::from([anchor.clone()]);
        let mut reach_queue = VecDeque::from([anchor.clone()]);
        while let Some(current) = reach_queue.pop_front() {
            for (next, _) in neighbors(&current) {
                if reachable.insert(next.clone()) {
                    reach_queue.push_back(next);
                }
            }
        }

        let normalized_query = query.to_lowercase();
        let matches_query = |id: &DependencyNodeId| {
            if normalized_query.is_empty() {
                return true;
            }
            let label = match id {
                DependencyNodeId::Recipe(recipe) => recipe.clone(),
                DependencyNodeId::Task { recipe, task } => format!("{recipe}:{task}"),
            };
            label.to_lowercase().contains(&normalized_query)
        };
        let source_indexes = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let mut queue = VecDeque::from([(anchor.clone(), None, None, 0_usize)]);
        let mut seen = BTreeSet::new();
        let mut candidates = Vec::new();
        let mut hidden_by_collapse = 0;
        let mut truncated_depth = 0;
        let mut cycle_edges = 0;
        while let Some((id, parent, edge_kind, depth)) = queue.pop_front() {
            if !seen.insert(id.clone()) {
                cycle_edges += 1;
                continue;
            }
            let children = neighbors(&id);
            let is_collapsed = collapsed.contains(&id) && !children.is_empty();
            candidates.push(DependencyProjectionRow {
                source_index: source_indexes.get(&id).copied().unwrap_or(0),
                id: id.clone(),
                parent,
                edge_kind,
                depth,
                has_children: !children.is_empty(),
                collapsed: is_collapsed,
            });
            if is_collapsed {
                hidden_by_collapse += children.len();
            } else if depth >= max_depth {
                truncated_depth += children.len();
            } else {
                for (child, kind) in children {
                    if seen.contains(&child) {
                        cycle_edges += 1;
                    } else {
                        queue.push_back((child, Some(id.clone()), Some(kind), depth + 1));
                    }
                }
            }
        }
        // Preserve disconnected authoritative nodes as top-level rows. Nodes
        // hidden by a collapsed reachable branch must not leak back in here.
        for node in &self.nodes {
            if !reachable.contains(&node.id) && seen.insert(node.id.clone()) {
                candidates.push(DependencyProjectionRow {
                    id: node.id.clone(),
                    parent: None,
                    edge_kind: None,
                    depth: 0,
                    source_index: source_indexes.get(&node.id).copied().unwrap_or(0),
                    has_children: !neighbors(&node.id).is_empty(),
                    collapsed: collapsed.contains(&node.id),
                });
            }
        }
        let before_filter = candidates.len();
        candidates.retain(|row| row.id == anchor || matches_query(&row.id));
        let hidden_by_filter = before_filter.saturating_sub(candidates.len());
        let row_limit = max_rows.max(1);
        let truncated_rows = candidates.len().saturating_sub(row_limit);
        candidates.truncate(row_limit);

        DependencyGraphProjection {
            anchor,
            reverse,
            rows: candidates,
            source_total: self.nodes.len(),
            hidden_by_filter,
            hidden_by_collapse,
            truncated_depth,
            truncated_rows,
            cycle_edges,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DependencyGraphState {
    #[default]
    NotLoaded,
    Loading {
        root: DependencyNodeId,
    },
    AvailableEmpty {
        root: DependencyNodeId,
    },
    Available(DependencyGraph),
    Partial {
        graph: DependencyGraph,
        limitations: Vec<String>,
    },
    Failed {
        root: DependencyNodeId,
        message: String,
    },
}
impl DependencyGraphState {
    pub fn graph(&self) -> Option<&DependencyGraph> {
        match self {
            Self::Available(graph) | Self::Partial { graph, .. } => Some(graph),
            Self::NotLoaded
            | Self::Loading { .. }
            | Self::AvailableEmpty { .. }
            | Self::Failed { .. } => None,
        }
    }

    pub fn root(&self) -> Option<&DependencyNodeId> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { root } | Self::AvailableEmpty { root } | Self::Failed { root, .. } => {
                Some(root)
            }
            Self::Available(graph) | Self::Partial { graph, .. } => Some(&graph.root),
        }
    }
}
