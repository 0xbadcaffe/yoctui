use super::*;

pub(crate) fn wic_workspace_start_effect(app: &mut App) -> (WicSessionId, WicOperation) {
    let _ = update(app, Action::BeginSelectedWicCreate);
    let _ = update(app, Action::PreviewWicCreate);
    let Some(Effect::StartWicSession { id, operation }) = update(app, Action::ConfirmWicCreate)
    else {
        panic!("expected Wic start effect");
    };
    (id, operation)
}
