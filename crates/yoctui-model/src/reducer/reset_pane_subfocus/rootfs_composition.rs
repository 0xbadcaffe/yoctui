use super::*;
use crate::image_updates::rootfs_image_identity;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::BeginSelectedRootfsComposition => {
            let Some(image) = rootfs_image_identity(app) else {
                app.notification =
                    Some("Select a deployed artifact owned by an image recipe first.".into());
                return None;
            };
            app.image_artifact_selection = Some(image.clone());
            app.images_view = ImagesView::RootfsPackages;
            app.focus = FocusTarget::Workspace;
            if app.zoomed_pane.is_some() {
                app.zoomed_pane = Some(FocusTarget::Workspace);
            }
            return begin_rootfs_composition(app, image);
        }
        Action::ShiftImagesView { delta } => {
            app.images_view = app.images_view.shifted(delta);
            app.image_artifact_searching = false;
            if app
                .layer_browser
                .as_ref()
                .is_some_and(LayerBrowser::is_rootfs)
            {
                app.layer_browser = None;
                app.metadata_query.clear();
                app.metadata_searching = false;
            }
            if app.images_view != ImagesView::Artifacts {
                let Some(image) = rootfs_image_identity(app) else {
                    app.notification = Some(
                        "Select a deployed artifact owned by an image recipe before opening rootfs composition."
                            .into(),
                    );
                    app.images_view = ImagesView::Artifacts;
                    return None;
                };
                app.focus = FocusTarget::Workspace;
                if app.zoomed_pane.is_some() {
                    app.zoomed_pane = Some(FocusTarget::Workspace);
                }
                app.image_artifact_selection = Some(image.clone());
                let is_current = app
                    .rootfs_composition
                    .request()
                    .is_some_and(|request| request.image == image);
                if !is_current {
                    return begin_rootfs_composition(app, image);
                }
                if app.images_view == ImagesView::RootfsFilesystem {
                    return update(app, Action::BrowseRootfsFilesystem);
                }
            }
        }
        Action::RefreshRootfsComposition => {
            let image = app
                .rootfs_composition
                .request()
                .map(|request| request.image.clone())
                .or_else(|| {
                    app.selected_image_artifact()
                        .map(|artifact| artifact.identity.clone())
                });
            let Some(image) = image else {
                app.notification =
                    Some("No rootfs composition image is available to refresh.".into());
                return None;
            };
            return begin_rootfs_composition(app, image);
        }
        Action::RootfsCompositionLoaded {
            request,
            composition,
        } => {
            if matches!(
                &app.rootfs_composition,
                RootfsCompositionState::Loading { request: pending } if pending == &request
            ) {
                set_rootfs_composition(app, request, composition, Vec::new());
                if app.images_view == ImagesView::RootfsFilesystem {
                    return update(app, Action::BrowseRootfsFilesystem);
                }
            }
        }
        Action::RootfsCompositionPartial {
            request,
            composition,
            limitations,
        } => {
            if matches!(
                &app.rootfs_composition,
                RootfsCompositionState::Loading { request: pending } if pending == &request
            ) {
                set_rootfs_composition(app, request, composition, limitations);
                if app.images_view == ImagesView::RootfsFilesystem {
                    return update(app, Action::BrowseRootfsFilesystem);
                }
            }
        }
        Action::RootfsCompositionUnavailable { request, reason } => {
            if matches!(
                &app.rootfs_composition,
                RootfsCompositionState::Loading { request: pending } if pending == &request
            ) {
                app.rootfs_composition = RootfsCompositionState::Unavailable { request, reason };
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
