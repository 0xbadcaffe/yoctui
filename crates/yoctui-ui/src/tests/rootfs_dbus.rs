use super::*;

fn bus_app(total: usize) -> App {
    let mut app = readme_system_dbus_app();
    let RootfsCompositionState::Partial { composition, .. } = &mut app.rootfs_composition else {
        panic!("fixture composition");
    };
    let yoctui_model::RootfsAuthority::Available(inventory) = &mut composition.system_inventory
    else {
        panic!("fixture inventory");
    };
    let template = inventory.dbus_services[0].clone();
    inventory.dbus_services = (0..total)
        .map(|index| {
            let mut service = template.clone();
            service.name = format!("org.t.B{index:03}");
            service.logical_path = yoctui_model::RootfsPathIdentity(
                format!("/usr/share/dbus-1/system-services/{}.service", service.name).into(),
            );
            service.host_path = PathBuf::from("/build/rootfs")
                .join(service.logical_path.0.strip_prefix("/").unwrap());
            service
        })
        .collect();
    app.rootfs_dbus_selection = 0;
    app
}

#[test]
fn rootfs_dbus_viewport_follows_down_up_pages_end_home_and_resize() {
    let mut app = bus_app(100);
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        update(
            &mut app,
            Action::SelectRootfsDbusService { delta: isize::MIN },
        );
        for index in 0..100 {
            assert_eq!(app.rootfs_dbus_selection, index);
            let text = rendered_text(&app, width, height);
            assert!(
                text.contains(&format!("org.t.B{index:03}")),
                "{width}x{height}, {index}: {text}"
            );
            update(&mut app, Action::SelectRootfsDbusService { delta: 1 });
        }
        for index in (0..100).rev() {
            let text = rendered_text(&app, width, height);
            assert!(text.contains(&format!("org.t.B{index:03}")), "{text}");
            update(&mut app, Action::SelectRootfsDbusService { delta: -1 });
        }
        for delta in [10, 10, -10, isize::MAX, -10, isize::MIN, isize::MAX] {
            update(&mut app, Action::SelectRootfsDbusService { delta });
            let text = rendered_text(&app, width, height);
            assert!(
                text.contains(&format!("org.t.B{:03}", app.rootfs_dbus_selection)),
                "{text}"
            );
        }
        let text = rendered_text(&app, width, height);
        assert!(!text.contains("org.t.B000"), "list did not scroll: {text}");
    }
    for (width, height) in [(160, 50), (80, 24), (120, 30)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("org.t.B099"), "resize hid selection: {text}");
        assert!(text.contains("100/100"), "missing position: {text}");
    }
}

#[test]
fn rootfs_dbus_unzoomed_workspace_keeps_last_and_previous_rows_visible() {
    let mut app = bus_app(100);
    app.zoomed_pane = None;
    for inspector in [false, true] {
        app.inspector_visible = inspector;
        for (width, height) in [(160, 50), (100, 30), (80, 24)] {
            update(
                &mut app,
                Action::SelectRootfsDbusService { delta: isize::MAX },
            );
            let text = rendered_text(&app, width, height);
            assert!(text.contains("100/100"), "{text}");
            // Narrow tables can truncate names; verify the selected row directly
            // in the production table region, independent of inspector text.
            let area = Rect::new(0, 0, width, height);
            let rows = rendered_region_rows(width, height, |frame, _| {
                rootfs_dbus_workspace(frame, &app, area);
            });
            assert!(
                rows.iter().any(|row| row.contains("org.t.B099")),
                "{rows:?}"
            );
            update(&mut app, Action::SelectRootfsDbusService { delta: -1 });
            let rows = rendered_region_rows(width, height, |frame, _| {
                rootfs_dbus_workspace(frame, &app, area);
            });
            assert!(
                rows.iter().any(|row| row.contains("org.t.B098")),
                "{rows:?}"
            );
        }
    }
}

#[test]
fn rootfs_dbus_empty_short_stale_selection_and_tiny_views_are_safe() {
    let empty = bus_app(0);
    assert!(rendered_text(&empty, 160, 50).contains("No system-bus activation files"));
    let mut short = bus_app(2);
    let text = rendered_text(&short, 160, 50);
    assert!(
        text.contains("org.t.B000") && text.contains("org.t.B001"),
        "{text}"
    );
    assert!(
        !text.contains("1/2"),
        "unclipped list should not add a cue: {text}"
    );
    short.rootfs_dbus_selection = usize::MAX;
    assert!(rendered_text(&short, 80, 24).contains("org.t.B001"));
    for app in [&empty, &short] {
        for (width, height) in [(30, 10), (10, 3), (1, 1)] {
            let _ = rendered_text(app, width, height);
        }
    }
}
