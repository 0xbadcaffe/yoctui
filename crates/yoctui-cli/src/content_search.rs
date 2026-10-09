//! Content search.
use super::*;

#[cfg(test)]
#[path = "tests/content_search.rs"]
mod tests;

pub(crate) struct GlobalContentSearchOperation {
    pub(crate) generation: u64,
    pub(crate) query: String,
    pub(crate) cancellation: GlobalSearchCancellation,
    pub(crate) handle: tokio::task::JoinHandle<Result<GlobalSearchScanResult, String>>,
    pub(crate) progress: tokio::sync::mpsc::UnboundedReceiver<yoctui_model::GlobalSearchHit>,
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
    // At most MAX_GLOBAL_SEARCH_HITS messages are sent; coalesce them on the UI thread.
    let (sender, progress) = tokio::sync::mpsc::unbounded_channel();
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if worker_cancellation.cancelled() {
            return Ok(GlobalSearchScanResult {
                hits: Vec::new(),
                truncated: false,
                searched_scopes: Vec::new(),
            });
        }
        tokio::task::spawn_blocking(move || {
            global_search::scan_global_content_streaming(&plan, &worker_cancellation, &|hit| {
                let _ = sender.send(hit.clone());
            })
        })
        .await
        .map_err(|error| format!("global search worker was lost: {error}"))?
    });
    *operation = Some(GlobalContentSearchOperation {
        generation,
        query,
        cancellation,
        handle,
        progress,
    });
}

pub(crate) async fn poll_global_content_search(
    app: &mut App,
    operation: &mut Option<GlobalContentSearchOperation>,
) {
    if let Some(operation) = operation.as_mut() {
        let mut hits = Vec::new();
        while let Ok(hit) = operation.progress.try_recv() {
            hits.push(hit);
        }
        if !hits.is_empty() {
            let _ = update(
                app,
                Action::GlobalContentSearchProgress {
                    generation: operation.generation,
                    query: operation.query.clone(),
                    hits,
                },
            );
        }
    }
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
