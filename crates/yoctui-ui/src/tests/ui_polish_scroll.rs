use super::*;
use yoctui_app::{Input, MouseInput, MouseKind, mouse_action_for_app};
use yoctui_model::{Effect, WorkbenchAction as W, WorkbenchFocus, WorkbenchRequest};

#[derive(Clone, Copy, Debug)]
enum Browser {
    Layers,
    Build,
    Rootfs,
    Workbench,
}

fn browser_fixture(kind: Browser) -> App {
    let mut app = match kind {
        Browser::Rootfs => ux_rootfs_ui_app(),
        _ => App::new(30, 32_768),
    };
    app.preferences.mouse_enabled = true;
    app.preferences.symbols = SymbolPreference::Unicode;
    app.focus = FocusTarget::Workspace;
    let (label, root) = match kind {
        Browser::Layers => ("meta", "/fixture/meta"),
        Browser::Build => ("Build directory", "/fixture/build"),
        Browser::Rootfs => ("Rootfs: fixture", "/fixture/rootfs"),
        Browser::Workbench => ("Workbench", "/fixture/source"),
    };
    let mut tree = LayerBrowser::new(label.into(), root.into());
    for n in 0..200 {
        if !matches!(kind, Browser::Workbench) {
            tree.entries.push(LayerBrowserEntry {
                path: tree.root.join(format!("excluded-{n:03}")),
                ..Default::default()
            });
        }
        tree.entries.push(LayerBrowserEntry {
            path: tree.root.join(format!("entry-{n:03}")),
            ..Default::default()
        });
    }
    match kind {
        Browser::Workbench => {
            update(
                &mut app,
                Action::Workbench(W::OpenRecipe {
                    recipe: "fixture".into(),
                    root: root.into(),
                }),
            );
            update(&mut app, Action::Workbench(W::Focus(WorkbenchFocus::Files)));
            app.workbench.current_mut().unwrap().folders[0].browser = tree;
        }
        Browser::Rootfs => {
            app.images_view = ImagesView::RootfsFilesystem;
            if let RootfsCompositionState::Partial { composition, .. } = &mut app.rootfs_composition
            {
                composition.root_directory = Some(tree.root.clone());
            }
            app.layer_browser = Some(tree);
        }
        Browser::Layers | Browser::Build => {
            app.screen = Screen::Layers;
            app.layer_browser = Some(tree);
        }
    }
    if !matches!(kind, Browser::Workbench) {
        app.metadata_query = "entry-".into();
        app.layer_browser.as_mut().unwrap().selection = 1;
    }
    app
}

fn tree(app: &App) -> &LayerBrowser {
    if app.screen == Screen::Workbench {
        &app.workbench.current().unwrap().folders[0].browser
    } else {
        app.layer_browser.as_ref().unwrap()
    }
}

fn press(app: &mut App, key: Input) {
    let action = match app.screen {
        Screen::Workbench => yoctui_app::workbench_action(app, key),
        Screen::Images => yoctui_app::rootfs_browser_action(app, key),
        _ => yoctui_app::layer_tree_action(app.metadata_searching, key),
    }
    .unwrap();
    update(app, action);
}

fn draw(app: &App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| render(frame, app)).unwrap();
    terminal.backend().buffer().clone()
}

fn line(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn tree_cells(buffer: &Buffer) -> (u16, u16, u16, String) {
    for y in 0..buffer.area.height {
        let text = line(buffer, y);
        for title in ["Files ", "Explorer "] {
            if let Some((before, after)) = text.split_once(title)
                && let Some(range) = after
                    .split(|c: char| !c.is_ascii_digit() && c != '-' && c != '/')
                    .next()
                && range.contains('/')
                && before.ends_with('┌')
            {
                let left = before.chars().count() as u16;
                let right = (left..buffer.area.width)
                    .find(|x| buffer[(*x, y)].symbol() == "┐")
                    .unwrap();
                return (left, right, y, range.into());
            }
        }
    }
    panic!("file browser range missing from rendered frame")
}

fn thumb(buffer: &Buffer, right: u16, top: u16) -> (u16, u16) {
    let rows = (top + 1..buffer.area.height)
        .take_while(|y| buffer[(right, *y)].symbol() == "│" || buffer[(right, *y)].symbol() == "█")
        .filter(|y| buffer[(right, *y)].symbol() == "█")
        .collect::<Vec<_>>();
    (
        *rows.first().expect("scrollbar thumb"),
        *rows.last().unwrap(),
    )
}

fn assert_rendered_range(buffer: &Buffer, left: u16, right: u16, top: u16, range: &str) {
    let (visible, total) = range.split_once('/').unwrap();
    assert_eq!(total, "200");
    let (first, last) = visible.split_once('-').unwrap();
    let first: usize = first.parse().unwrap();
    let last: usize = last.parse().unwrap();
    let items: Vec<usize> = (top + 1..buffer.area.height)
        .take_while(|y| buffer[(right, *y)].symbol() != "┘")
        .filter_map(|y| {
            let row: String = (left..right).map(|x| buffer[(x, y)].symbol()).collect();
            row.split_once("entry-")?.1.get(..3)?.parse().ok()
        })
        .collect();
    assert_eq!(items, ((first - 1)..last).collect::<Vec<_>>());
}

#[test]
fn all_file_browsers_agree_on_visible_rows_scroll_and_resized_mouse_targets() {
    for kind in [
        Browser::Layers,
        Browser::Build,
        Browser::Rootfs,
        Browser::Workbench,
    ] {
        let mut app = browser_fixture(kind);
        for (width, height) in [(80, 24), (100, 30), (160, 48)] {
            press(&mut app, Input::Home);
            let initial = draw(&app, width, height);
            let (left, right, top, range) = tree_cells(&initial);
            assert!(
                range.starts_with("1-") && range.ends_with("/200"),
                "{kind:?} {range}"
            );
            assert_rendered_range(&initial, left, right, top, &range);
            let first_thumb = thumb(&initial, right, top);
            press(&mut app, Input::PageDown);
            let page = draw(&app, width, height);
            assert!(thumb(&page, right, top).0 >= first_thumb.0, "{kind:?}");
            press(&mut app, Input::End);
            let bottom = draw(&app, width, height);
            let (left_after, right_after, top_after, range) = tree_cells(&bottom);
            assert_eq!((left, right, top), (left_after, right_after, top_after));
            assert!(range.ends_with("-200/200"), "{kind:?} {range}");
            assert_rendered_range(&bottom, left, right, top, &range);
            let last_thumb = thumb(&bottom, right, top);
            assert!(last_thumb.0 >= thumb(&page, right, top).0);
            let x = left + 2;
            let last_row = (top + 1..height)
                .find(|row| {
                    (left..right)
                        .map(|column| bottom[(column, *row)].symbol())
                        .collect::<String>()
                        .contains("entry-199")
                })
                .unwrap();
            assert_eq!(last_thumb.1, last_row, "{kind:?} {width}x{height}");
            assert_eq!(
                tree(&app)
                    .selected_entry()
                    .unwrap()
                    .path
                    .file_name()
                    .unwrap(),
                "entry-199"
            );

            let pointer = |kind, row| MouseInput {
                kind,
                column: x,
                row,
            };
            for (event, expected) in [
                (MouseKind::ScrollUp, "entry-198"),
                (MouseKind::ScrollDown, "entry-199"),
            ] {
                let action = mouse_action_for_app(pointer(event, last_row), &app, width, height)
                    .expect("wheel routed to file tree");
                update(&mut app, action);
                assert_eq!(
                    tree(&app)
                        .selected_entry()
                        .unwrap()
                        .path
                        .file_name()
                        .unwrap(),
                    expected
                );
            }
            let action =
                mouse_action_for_app(pointer(MouseKind::Down, last_row - 1), &app, width, height)
                    .expect("click in resized file tree");
            update(&mut app, action);
            assert_eq!(
                tree(&app)
                    .selected_entry()
                    .unwrap()
                    .path
                    .file_name()
                    .unwrap(),
                "entry-198"
            );
            press(&mut app, Input::PageUp);
            assert_eq!(
                tree(&app)
                    .selected_entry()
                    .unwrap()
                    .path
                    .file_name()
                    .unwrap(),
                "entry-188"
            );
        }
    }
}

#[test]
fn file_browser_preview_and_inventory_refresh_keep_stable_selection() {
    for kind in [
        Browser::Layers,
        Browser::Build,
        Browser::Rootfs,
        Browser::Workbench,
    ] {
        let mut app = browser_fixture(kind);
        press(&mut app, Input::End);
        let old = tree(&app).selected_entry().unwrap().path.clone();
        let wide_range = tree_cells(&draw(&app, 160, 48)).3;
        if matches!(kind, Browser::Workbench) {
            let Some(Effect::Workbench(WorkbenchRequest::Read {
                session,
                root,
                path,
                generation,
            })) = update(&mut app, Action::Workbench(W::Open))
            else {
                panic!("no file-read request");
            };
            assert_eq!(path, old);
            update(
                &mut app,
                Action::Workbench(W::FileLoaded {
                    session,
                    root,
                    path,
                    generation,
                    result: Ok("retained preview".into()),
                }),
            );
            assert_eq!(
                app.workbench.current().unwrap().focus,
                WorkbenchFocus::Editor
            );
            assert!(rendered_text(&app, 160, 48).contains("retained preview"));
            update(&mut app, Action::Workbench(W::Cancel));
        } else {
            update(&mut app, Action::LayerBrowserExpand);
            assert!(tree(&app).preview_focused);
            update(
                &mut app,
                Action::LoadLayerBrowserPreview {
                    path: old.clone(),
                    content: "retained preview".into(),
                    kind: PreviewKind::Text,
                    truncated: false,
                },
            );
            assert!(rendered_text(&app, 160, 48).contains("retained preview"));
            update(
                &mut app,
                yoctui_app::file_browser_preview_action(Input::Left).unwrap(),
            );
            assert!(!tree(&app).preview_focused);
        }
        assert_eq!(tree_cells(&draw(&app, 160, 48)).3, wide_range);
        let _ = draw(&app, 80, 24);
        assert_eq!(tree(&app).selected_entry().unwrap().path, old);
        assert_eq!(tree_cells(&draw(&app, 160, 48)).3, wide_range);

        let mut entries = tree(&app).entries.clone();
        entries.insert(
            0,
            LayerBrowserEntry {
                path: tree(&app).root.join("entry-000-first"),
                ..Default::default()
            },
        );
        if matches!(kind, Browser::Workbench) {
            let Some(Effect::Workbench(WorkbenchRequest::Directory {
                session,
                root,
                directory,
                generation,
            })) = update(&mut app, Action::Workbench(W::Refresh))
            else {
                panic!("no refresh request");
            };
            update(
                &mut app,
                Action::Workbench(W::DirectoryLoaded {
                    session,
                    root,
                    directory,
                    generation,
                    result: Ok(entries),
                }),
            );
        } else {
            let root = tree(&app).root.clone();
            let layer = tree(&app).layer.clone();
            update(
                &mut app,
                Action::LoadLayerBrowserDirectory {
                    layer,
                    root: root.clone(),
                    directory: root,
                    entries,
                },
            );
        }
        assert_eq!(tree(&app).selected_entry().unwrap().path, old);
        assert!(
            tree_cells(&draw(&app, 160, 48)).3.ends_with("/201"),
            "{kind:?}"
        );
    }
}

#[test]
fn file_browser_ranges_for_empty_and_single_rows() {
    for kind in [
        Browser::Layers,
        Browser::Build,
        Browser::Rootfs,
        Browser::Workbench,
    ] {
        let mut app = browser_fixture(kind);
        if app.screen == Screen::Workbench {
            app.workbench.current_mut().unwrap().folders[0]
                .browser
                .entries
                .clear();
        } else {
            app.metadata_query = "nothing-matches".into();
        }
        assert_eq!(tree_cells(&draw(&app, 80, 24)).3, "0/0");
        if app.screen == Screen::Workbench {
            let browser = &mut app.workbench.current_mut().unwrap().folders[0].browser;
            browser.entries.push(LayerBrowserEntry {
                path: browser.root.join("single"),
                ..Default::default()
            });
        } else {
            app.metadata_query = "entry-199".into();
        }
        press(&mut app, Input::End);
        assert_eq!(tree_cells(&draw(&app, 80, 24)).3, "1-1/1");
    }
}

#[test]
fn narrow_workbench_keeps_active_folder_and_its_files_visible() {
    let mut app = browser_fixture(Browser::Workbench);
    let session = app.workbench.current_mut().unwrap();
    let active = session.folders.pop().unwrap();
    for index in 0..12 {
        let mut folder = active.clone();
        folder.browser = LayerBrowser::new(
            "Workbench".into(),
            format!("/fixture/folder-{index}").into(),
        );
        session.folders.push(folder);
    }
    session.folder = session.folders.len();
    session.folders.push(active);
    press(&mut app, Input::End);
    let buffer = draw(&app, 80, 24);
    assert!(tree_cells(&buffer).3.ends_with("-200/200"));
    assert!((0..24).any(|y| line(&buffer, y).contains("entry-199")));
}
