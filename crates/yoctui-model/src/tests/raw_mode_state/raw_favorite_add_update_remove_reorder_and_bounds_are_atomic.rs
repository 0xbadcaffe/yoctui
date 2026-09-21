use super::*;

#[test]
fn raw_favorite_add_update_remove_reorder_and_bounds_are_atomic() {
    let catalog = catalog(1);
    let first = favorite_fixture(&catalog, "build.target", 99);
    let second = favorite_fixture(&catalog, "build.version", 99);
    let mut favorites = Vec::new();
    add_raw_favorite(&mut favorites, first.clone()).unwrap();
    add_raw_favorite(&mut favorites, second.clone()).unwrap();
    assert_eq!(favorites[0].order, 0);
    assert_eq!(favorites[1].order, 1);
    let before = favorites.clone();
    assert!(matches!(
        add_raw_favorite(&mut favorites, first),
        Err(RawFavoriteError::DuplicateCommand(_))
    ));
    assert_eq!(favorites, before);

    let mut renamed = favorites[0].clone();
    renamed.name = "Renamed favorite".into();
    update_raw_favorite(&mut favorites, renamed).unwrap();
    move_raw_favorite(&mut favorites, &command("build.target"), 1).unwrap();
    assert_eq!(favorites[1].name, "Renamed favorite");
    assert_eq!(favorites[0].order, 0);
    assert_eq!(favorites[1].order, 1);
    let removed = remove_raw_favorite(&mut favorites, &command("build.version")).unwrap();
    assert_eq!(removed.command, command("build.version"));
    assert_eq!(favorites[0].order, 0);

    let mut too_many = Vec::new();
    for index in 0..=MAX_RAW_FAVORITES {
        let mut item = second.clone();
        item.command = command(&format!("synthetic.{index}"));
        item.order = index as u16;
        too_many.push(item);
    }
    assert_eq!(
        validate_raw_favorites(&too_many),
        Err(RawFavoriteError::TooManyFavorites)
    );

    let long_arguments =
        RawAdditionalArguments::from_vec((0..16).map(|_| "x".repeat(512)).collect()).unwrap();
    let mut oversized = Vec::new();
    for index in 0..40 {
        let mut item = second.clone();
        item.command = command(&format!("large.{index}"));
        item.additional_arguments = long_arguments.clone();
        item.order = index;
        oversized.push(item);
    }
    assert_eq!(
        validate_raw_favorites(&oversized),
        Err(RawFavoriteError::AggregateTooLarge)
    );
}
