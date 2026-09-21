use super::*;

#[test]
fn visible_guidance_popup_owns_its_advertised_enter_and_escape_controls() {
    let mut app = yoctui_model::App::new(10, 1_000);
    app.screen = Screen::Images;
    app.notification = Some("Select an image first with i.".into());

    assert_eq!(
        notification_popup_action(&app, Input::Esc),
        Some(Action::DismissNotification)
    );
    assert_eq!(
        notification_popup_action(&app, Input::Enter),
        Some(Action::ActivateNotification)
    );
    assert_eq!(notification_popup_action(&app, Input::Down), None);

    app.notification = Some("Package inventory refreshed.".into());
    assert_eq!(notification_popup_action(&app, Input::Esc), None);
}
