use super::*;

#[test]
fn maps_recipe_and_task_filter_controls() {
    assert_eq!(
        key_action(Input::Char('R')),
        Some(Action::CycleLogRecipeFilter)
    );
    assert_eq!(
        key_action(Input::Char('T')),
        Some(Action::CycleLogTaskFilter)
    );
}
