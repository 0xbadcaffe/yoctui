use super::*;

#[test]
fn function_key_image_navigation_routes_its_local_inventory_effect() {
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let effect = yoctui_model::update(
        &mut app,
        yoctui_model::Action::Open(yoctui_model::Screen::Images),
    )
    .expect("opening an unloaded Images workspace requests its inventory");
    assert_eq!(
        local_workspace_effect_route(&effect),
        LocalWorkspaceEffectRoute::ImageArtifacts
    );
}
