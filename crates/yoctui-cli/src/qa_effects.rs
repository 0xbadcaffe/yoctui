//! Qa effects.
use super::*;

pub(crate) async fn route_independent_qa_effect(
    guard: &TerminalGuard,
    app: &mut App,
    coordinator: &mut QaCliCoordinator,
    effect: Effect,
    editor: Option<&str>,
) -> bool {
    match &effect {
        Effect::Qa(QaEffect::StartBuild { .. } | QaEffect::CancelBuild { .. }) => false,
        Effect::Qa(QaEffect::OpenReport(identity)) => {
            match coordinator.revalidate_report(app, identity) {
                Ok(()) => {
                    open_in_editor(guard, app, identity.path.clone(), editor).await;
                }
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Qa(QaEffect::OpenProvider(identity)) => {
            match coordinator.revalidate_provider(app, identity) {
                Ok(()) => open_in_editor(guard, app, identity.file.clone(), editor).await,
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Qa(QaEffect::OpenSource(source)) => {
            match coordinator.revalidate_source(app, source) {
                Ok(()) => open_in_editor(guard, app, source.path.clone(), editor).await,
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Qa(QaEffect::OpenLayerRoot(layer)) => {
            match coordinator.revalidate_layer(app, layer) {
                Ok(()) => open_in_editor(guard, app, layer.root.clone(), editor).await,
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Qa(_) => coordinator.handle_effect(app, effect).await,
        _ => false,
    }
}
