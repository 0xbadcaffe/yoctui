//! Content search.
use super::*;

pub(crate) struct GlobalContentSearchOperation {
    pub(crate) generation: u64,
    pub(crate) query: String,
    pub(crate) cancellation: GlobalSearchCancellation,
    pub(crate) handle: tokio::task::JoinHandle<Result<GlobalSearchScanResult, String>>,
}

pub(crate) fn begin_global_content_search(
    app: &mut App,
    build_dir: &Path,
    operation: &mut Option<GlobalContentSearchOperation>,
) {
    if let Some(previous) = operation.take() {
        previous.cancellation.cancel();
    }
    if app.command_palette_query.trim().is_empty() || app.command_palette_regex_error().is_some() {
        return;
    }
    let _ = update(app, Action::BeginGlobalContentSearch);
    let yoctui_model::GlobalSearchContentState::Loading { generation, query } =
        &app.global_search_content
    else {
        return;
    };
    let generation = *generation;
    let query = query.clone();
    let plan = GlobalSearchPlan::for_app(app, build_dir, query.clone());
    let cancellation = GlobalSearchCancellation::default();
    let worker_cancellation = cancellation.clone();
    let handle =
        tokio::task::spawn_blocking(move || scan_global_content(&plan, &worker_cancellation));
    *operation = Some(GlobalContentSearchOperation {
        generation,
        query,
        cancellation,
        handle,
    });
}

pub(crate) async fn poll_global_content_search(
    app: &mut App,
    operation: &mut Option<GlobalContentSearchOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(operation) = operation.take() else {
        return;
    };
    let action = match operation.handle.await {
        Ok(Ok(result)) => Action::GlobalContentSearchLoaded {
            generation: operation.generation,
            query: operation.query,
            hits: result.hits,
            truncated: result.truncated,
            searched_scopes: result.searched_scopes,
        },
        Ok(Err(message)) => Action::GlobalContentSearchFailed {
            generation: operation.generation,
            query: operation.query,
            message,
        },
        Err(error) => Action::GlobalContentSearchFailed {
            generation: operation.generation,
            query: operation.query,
            message: format!("global search task was lost: {error}"),
        },
    };
    let _ = update(app, action);
}
