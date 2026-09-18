pub(crate) fn project_profile_fixture() -> yoctui_model::ProjectProfile {
    yoctui_model::ProjectProfile {
        schema_version: yoctui_model::PROJECT_PROFILE_SCHEMA_VERSION,
        favorites: yoctui_model::ProjectFavorites {
            images: vec!["core-image-minimal".into()],
            ..yoctui_model::ProjectFavorites::default()
        },
        build_presets: Vec::new(),
        workflows: Vec::new(),
    }
}
