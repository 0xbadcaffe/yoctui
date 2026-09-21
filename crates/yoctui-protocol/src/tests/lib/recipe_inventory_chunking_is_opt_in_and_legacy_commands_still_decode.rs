use super::*;

#[test]
fn recipe_inventory_chunking_is_opt_in_and_legacy_commands_still_decode() {
    let legacy: Command = serde_json::from_str(r#"{"type":"list_recipes","filter":null}"#).unwrap();
    assert!(matches!(
        legacy,
        Command::ListRecipes { chunked: false, .. }
    ));
    assert!(!serde_json::to_string(&legacy).unwrap().contains("chunked"));
    let chunked = Command::ListRecipes {
        filter: None,
        chunked: true,
    };
    assert!(
        serde_json::to_string(&chunked)
            .unwrap()
            .contains("\"chunked\":true")
    );
    let event = Event::RecipesChunk {
        offset: 0,
        total: 0,
        complete: true,
        recipes: vec![],
    };
    let encoded = serde_json::to_string(&event).unwrap();
    assert_eq!(serde_json::from_str::<Event>(&encoded).unwrap(), event);
}
