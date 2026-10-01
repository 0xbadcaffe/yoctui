use super::*;

fn service_app(total: usize) -> App {
    let mut app = readme_systemd_services_app();
    let RootfsCompositionState::Partial { composition, .. } = &mut app.rootfs_composition else {
        panic!("fixture composition");
    };
    let yoctui_model::RootfsAuthority::Available(inventory) = &mut composition.system_inventory
    else {
        panic!("fixture inventory");
    };
    let template = inventory.systemd_services[0].clone();
    inventory.systemd_services = (0..total)
        .map(|index| {
            let mut service = template.clone();
            service.name = format!("service-{index:03}.service");
            service.logical_path = yoctui_model::RootfsPathIdentity(
                format!("/usr/lib/systemd/system/{}", service.name).into(),
            );
            service.host_path = PathBuf::from("/build/rootfs")
                .join(service.logical_path.0.strip_prefix("/").unwrap());
            service
        })
        .collect();
    app.rootfs_systemd_selection = 0;
    app
}

#[test]
fn rootfs_systemd_viewport_follows_down_up_pages_end_home_and_resize() {
    let mut app = service_app(80);
    for (width, height) in [(160, 50), (100, 25), (80, 24)] {
        update(
            &mut app,
            Action::SelectRootfsSystemdService { delta: isize::MIN },
        );
        for index in 0..80 {
            let text = rendered_text(&app, width, height);
            assert!(
                text.contains(&format!("service-{index:03}.service")),
                "{width}x{height}, {index}: {text}"
            );
            update(&mut app, Action::SelectRootfsSystemdService { delta: 1 });
        }
        for delta in [-1, -10, -10, isize::MIN, 10, isize::MAX] {
            update(&mut app, Action::SelectRootfsSystemdService { delta });
            let text = rendered_text(&app, width, height);
            assert!(
                text.contains(&format!(
                    "service-{:03}.service",
                    app.rootfs_systemd_selection
                )),
                "{text}"
            );
        }
        let text = rendered_text(&app, width, height);
        assert!(
            !text.contains("service-000.service"),
            "list did not scroll: {text}"
        );
    }
    for (width, height) in [(160, 50), (80, 24), (120, 30)] {
        let text = rendered_text(&app, width, height);
        assert!(
            text.contains("service-079.service"),
            "resize hid selection: {text}"
        );
        assert!(text.contains("80/80"), "missing position: {text}");
    }
}

#[test]
fn rootfs_systemd_empty_short_stale_selection_and_tiny_views_are_safe() {
    let empty = service_app(0);
    assert!(rendered_text(&empty, 160, 50).contains("No systemd .service files"));
    let mut short = service_app(2);
    let text = rendered_text(&short, 160, 50);
    assert!(
        text.contains("service-000.service") && text.contains("service-001.service"),
        "{text}"
    );
    assert!(
        !text.contains("1/2"),
        "unclipped list should not add a cue: {text}"
    );
    short.rootfs_systemd_selection = usize::MAX;
    assert!(rendered_text(&short, 80, 24).contains("service-001.service"));
    for app in [&empty, &short] {
        for (width, height) in [(30, 10), (10, 3), (1, 1)] {
            let _ = rendered_text(app, width, height);
        }
    }
}
