//! Signature operations.
use super::*;

#[derive(Debug, Clone)]
pub(crate) enum SignatureOperationRequest {
    Dump(SignatureTarget),
    Compare(SignatureComparisonRequest),
}

pub(crate) struct SignatureBackgroundOperation {
    pub(crate) request: SignatureOperationRequest,
    pub(crate) cancellation: SignatureCancellation,
    pub(crate) handle: tokio::task::JoinHandle<BackendEvent>,
}

pub(crate) fn begin_signature_operation(
    app: &mut App,
    adapter: &SignatureAdapter,
    operation: &mut Option<SignatureBackgroundOperation>,
    effect: Effect,
) {
    if operation.is_some() {
        let _ = update(
            app,
            Action::Notify("A signature operation is already running.".into()),
        );
        return;
    }
    let cancellation = SignatureCancellation::default();
    let worker_cancellation = cancellation.clone();
    let adapter = adapter.clone();
    let (request, handle) = match effect {
        Effect::GetSignatureDump(target) => {
            let worker_target = target.clone();
            let handle = tokio::spawn(async move {
                match adapter
                    .dump_with_cancellation(worker_target.clone(), worker_cancellation)
                    .await
                {
                    Ok(response) => response.into(),
                    Err(error) => BackendEvent::SignatureDumpFailed {
                        target: worker_target,
                        message: error.to_string(),
                    },
                }
            });
            (SignatureOperationRequest::Dump(target), handle)
        }
        Effect::CompareSignatures(request) => {
            let worker_request = request.clone();
            let handle = tokio::spawn(async move {
                match adapter
                    .compare_with_cancellation(worker_request.clone(), worker_cancellation)
                    .await
                {
                    Ok(response) => response.into(),
                    Err(error) => BackendEvent::SignatureComparisonFailed {
                        request: worker_request,
                        message: error.to_string(),
                    },
                }
            });
            (SignatureOperationRequest::Compare(request), handle)
        }
        _ => return,
    };
    *operation = Some(SignatureBackgroundOperation {
        request,
        cancellation,
        handle,
    });
}

pub(crate) async fn poll_signature_operation(
    app: &mut App,
    operation: &mut Option<SignatureBackgroundOperation>,
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
    let event = match operation.handle.await {
        Ok(event) => event,
        Err(error) => match operation.request {
            SignatureOperationRequest::Dump(target) => BackendEvent::SignatureDumpFailed {
                target,
                message: format!("signature background task was lost: {error}"),
            },
            SignatureOperationRequest::Compare(request) => {
                BackendEvent::SignatureComparisonFailed {
                    request,
                    message: format!("signature background task was lost: {error}"),
                }
            }
        },
    };
    if let Some(action) = model_action_from_backend_event(event) {
        let _ = update(app, action);
    }
}
