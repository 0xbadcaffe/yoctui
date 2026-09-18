use super::*;

#[cfg(unix)]
pub(crate) async fn wic_device_write_start_effect(
    app: &mut App,
    inspector: &WicDeviceInspector,
) -> (WicSessionId, WicOperation) {
    let effect =
        update(app, Action::BeginSelectedWicDeviceWrite).expect("expected device discovery effect");
    let mut discovery = None;
    begin_wic_device_operation(inspector, &mut discovery, effect);
    tokio::time::timeout(Duration::from_secs(2), async {
        while discovery.is_some() {
            poll_wic_device_operation(app, &mut discovery).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let _ = update(
        app,
        wic_device_picker_action(Input::Enter).expect("device picker action"),
    );
    for character in "WRITE /dev/sdz".chars() {
        let _ = update(
            app,
            wic_write_phrase_action(Input::Char(character)).expect("phrase action"),
        );
    }
    let _ = update(
        app,
        wic_write_phrase_action(Input::Enter).expect("phrase preview action"),
    );
    let effect = update(
        app,
        wic_write_confirmation_action(Input::Enter).expect("write confirmation action"),
    );
    let Some(Effect::StartWicSession { id, operation }) = effect else {
        panic!("expected Wic write start effect");
    };
    (id, operation)
}
