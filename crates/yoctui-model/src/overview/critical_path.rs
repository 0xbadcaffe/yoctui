fn longest_path(
    id: &str,
    tasks: &BTreeMap<String, &TaskInfo>,
    now: SystemTime,
    visiting: &mut std::collections::BTreeSet<String>,
    memo: &mut BTreeMap<String, (u64, Option<String>)>,
) -> (u64, Option<String>) {
    if let Some(result) = memo.get(id) {
        return result.clone();
    }
    let Some(task) = tasks.get(id) else {
        return (0, None);
    };
    let own = task
        .elapsed_at(now)
        .map_or(0, |value| value.as_millis() as u64);
    if !visiting.insert(id.to_owned()) {
        return (own, None);
    }
    let mut predecessor: Option<(String, u64)> = None;
    for dependency in &task.dependencies {
        if !tasks.contains_key(&dependency.0) || visiting.contains(&dependency.0) {
            continue;
        }
        let candidate = (
            dependency.0.clone(),
            longest_path(&dependency.0, tasks, now, visiting, memo).0,
        );
        if predecessor.as_ref().is_none_or(|current| {
            candidate.1 > current.1 || (candidate.1 == current.1 && candidate.0 < current.0)
        }) {
            predecessor = Some(candidate);
        }
    }
    visiting.remove(id);
    let result = (
        own.saturating_add(predecessor.as_ref().map_or(0, |(_, duration)| *duration)),
        predecessor.map(|(id, _)| id),
    );
    memo.insert(id.to_owned(), result.clone());
    result
}
