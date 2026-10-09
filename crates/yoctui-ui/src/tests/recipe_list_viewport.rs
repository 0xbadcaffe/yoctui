use super::*;

fn recipes_app() -> App {
    let mut app = App::new(32, 8192);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Workspace;
    app.workspace.recipes = (0..100)
        .map(|index| yoctui_model::Recipe {
            name: format!("r{index:03}"),
            ..Default::default()
        })
        .collect();
    app
}

fn selected_row(app: &App, width: u16, height: u16) -> (usize, Vec<String>) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| package_render::recipes(frame, app, frame.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let rows = buffer
        .content
        .chunks(width as usize)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>();
    let palette = ThemePalette::for_app(app);
    let selected = format!("r{:03}", app.recipe_selection);
    let y = buffer
        .content
        .chunks(width as usize)
        .position(|row| {
            row.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains(&selected)
                && row
                    .iter()
                    .any(|cell| cell.bg == palette.selection_background)
        })
        .expect("selected recipe must be visible and highlighted");
    (y, rows)
}

#[test]
fn recipe_list_viewport_moves_highlight_down_then_changes_page_and_returns_up() {
    let mut app = recipes_app();
    let (first, _) = selected_row(&app, 240, 30);
    for index in 1..26 {
        app.recipe_selection = index;
        let (y, rows) = selected_row(&app, 240, 30);
        assert_eq!(y, first + index);
        assert!(
            rows[first].contains("r000"),
            "list scrolled before page edge"
        );
    }
    app.recipe_selection = 26;
    let (y, rows) = selected_row(&app, 240, 30);
    assert_eq!(y, first);
    assert!(rows[first].contains("r026"));
    app.recipe_selection = 25;
    assert_eq!(selected_row(&app, 240, 30).0, first + 25);
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        app.recipe_selection = 99;
        selected_row(&app, width, height);
    }
}

#[test]
fn recipe_list_viewport_uses_filtered_ordinal_not_inventory_index() {
    let mut app = recipes_app();
    for (index, recipe) in app.workspace.recipes.iter_mut().enumerate() {
        if index % 2 == 0 {
            recipe.version = Some("keep".into());
        }
    }
    app.metadata_query = "keep".into();
    let (first, _) = selected_row(&app, 240, 30);
    app.recipe_selection = 40;
    let (y, rows) = selected_row(&app, 240, 30);
    assert_eq!(y, first + 20);
    assert!(rows[first].contains("r000"));
    app.recipe_selection = 52;
    let (y, rows) = selected_row(&app, 240, 30);
    assert_eq!(y, first);
    assert!(rows[first].contains("r052"));
}
