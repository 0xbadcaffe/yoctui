use super::*;

#[test]
fn project_profile_items_resolve_only_against_authoritative_workspace() {
    let profile = profile();
    let state = ProjectProfileState::Loaded(profile);
    let workspace = Workspace {
        recipes: vec![
            Recipe {
                name: "busybox".into(),
                ..Recipe::default()
            },
            Recipe {
                name: "core-image-minimal".into(),
                ..Recipe::default()
            },
        ],
        layers: vec![Layer {
            name: "meta-poky".into(),
            path: PathBuf::from("/src/meta-poky"),
            priority: Some(5),
        }],
        ..Workspace::default()
    };
    let items = project_profile_items(&state, &workspace, &["core-image-minimal".into()]);
    assert!(
        items
            .iter()
            .all(|item| matches!(item.status, ProjectProfileItemStatus::Resolved))
    );

    let stale = project_profile_items(&state, &Workspace::default(), &[]);
    assert!(
        stale
            .iter()
            .all(|item| matches!(item.status, ProjectProfileItemStatus::Unavailable(_)))
    );
}
