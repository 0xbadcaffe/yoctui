use super::*;

#[test]
fn client_sampling_is_fast_only_where_host_telemetry_is_visible() {
    let mut app = App::new(8, 1024);
    assert_eq!(client_telemetry_interval(&app), CLIENT_VISIBLE_INTERVAL);
    app.screen = Screen::Tasks;
    assert_eq!(client_telemetry_interval(&app), CLIENT_VISIBLE_INTERVAL);
    app.screen = Screen::Recipes;
    assert!(!client_telemetry_visible(&app));
    assert_eq!(client_telemetry_interval(&app), CLIENT_BACKGROUND_INTERVAL);
}
