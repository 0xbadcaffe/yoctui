fn raw_favorite_template_digest(command: &RawCommand) -> RawFavoriteTemplateDigest {
    let mut digest = Sha256::new();
    raw_digest_field(&mut digest, command.id.as_str().as_bytes());
    for parameter in &command.parameters {
        raw_digest_field(&mut digest, parameter.id.as_str().as_bytes());
        raw_digest_field(&mut digest, parameter.placeholder.as_bytes());
        raw_digest_field(
            &mut digest,
            match parameter.kind {
                RawParameterKind::Recipe => b"recipe",
                RawParameterKind::Image => b"image",
                RawParameterKind::Target => b"target",
                RawParameterKind::Task => b"task",
                RawParameterKind::UserInterface => b"user_interface",
                RawParameterKind::File => b"file",
                RawParameterKind::Integer => b"integer",
                RawParameterKind::Text => b"text",
                RawParameterKind::Multiconfig => b"multiconfig",
            },
        );
        raw_digest_field(
            &mut digest,
            match parameter.presence {
                RawParameterPresence::Required => b"required",
                RawParameterPresence::Optional => b"optional",
            },
        );
    }
    match &command.execution {
        RawExecutionPolicy::ReferenceOnly { .. } => raw_digest_field(&mut digest, b"reference"),
        RawExecutionPolicy::Executable { template } => {
            raw_digest_field(&mut digest, b"executable");
            raw_digest_field(&mut digest, template.executable.as_str().as_bytes());
            raw_digest_field(
                &mut digest,
                raw_interaction_name(template.interaction).as_bytes(),
            );
            raw_digest_field(&mut digest, raw_safety_name(template.safety).as_bytes());
            match &template.capabilities {
                RawCapabilityRequirement::All { capabilities } => {
                    raw_digest_field(&mut digest, b"all");
                    for capability in capabilities {
                        raw_digest_field(&mut digest, capability.to_string().as_bytes());
                    }
                }
                RawCapabilityRequirement::Any { capabilities } => {
                    raw_digest_field(&mut digest, b"any");
                    for capability in capabilities {
                        raw_digest_field(&mut digest, capability.to_string().as_bytes());
                    }
                }
            }
            for argument in &template.arguments {
                match argument {
                    RawArgument::Literal { value } => {
                        raw_digest_field(&mut digest, b"literal");
                        raw_digest_field(&mut digest, value.as_bytes());
                    }
                    RawArgument::Empty => raw_digest_field(&mut digest, b"empty"),
                    RawArgument::Parameter { parameter } => {
                        raw_digest_field(&mut digest, b"parameter");
                        raw_digest_field(&mut digest, parameter.as_str().as_bytes());
                    }
                    RawArgument::JoinedParameter { prefix, parameter } => {
                        raw_digest_field(&mut digest, b"joined");
                        raw_digest_field(&mut digest, prefix.as_bytes());
                        raw_digest_field(&mut digest, parameter.as_str().as_bytes());
                    }
                    RawArgument::Composed { segments } => {
                        raw_digest_field(&mut digest, b"composed");
                        for segment in segments {
                            match segment {
                                RawArgumentSegment::Literal { value } => {
                                    raw_digest_field(&mut digest, b"literal");
                                    raw_digest_field(&mut digest, value.as_bytes());
                                }
                                RawArgumentSegment::Parameter { parameter } => {
                                    raw_digest_field(&mut digest, b"parameter");
                                    raw_digest_field(&mut digest, parameter.as_str().as_bytes());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    RawFavoriteTemplateDigest(digest.finalize().into())
}

fn validate_raw_favorite_defaults(
    command: &RawCommand,
    defaults: &BTreeMap<RawParameterId, RawParameterValue>,
) -> Result<(), RawFavoriteError> {
    if defaults.len() > MAX_RAW_PARAMETERS {
        return Err(RawFavoriteError::TooManyDefaults);
    }
    for (parameter, value) in defaults {
        let definition = command
            .parameters
            .iter()
            .find(|definition| &definition.id == parameter)
            .ok_or_else(|| RawFavoriteError::InvalidDefault(parameter.clone()))?;
        definition.validate_value(value)?;
    }
    Ok(())
}

fn raw_favorite_aggregate_bytes(favorites: &[RawFavorite]) -> usize {
    favorites
        .iter()
        .map(|favorite| {
            2 + 32
                + 2
                + favorite.command.as_str().len()
                + favorite.name.len()
                + favorite
                    .parameter_defaults
                    .iter()
                    .map(|(parameter, value)| parameter.as_str().len() + value.argument().len() + 8)
                    .sum::<usize>()
                + favorite
                    .additional_arguments
                    .as_slice()
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
        })
        .sum()
}

pub fn validate_raw_favorites(favorites: &[RawFavorite]) -> Result<(), RawFavoriteError> {
    if favorites.len() > MAX_RAW_FAVORITES {
        return Err(RawFavoriteError::TooManyFavorites);
    }
    let mut commands = BTreeSet::new();
    for (index, favorite) in favorites.iter().enumerate() {
        favorite.validate()?;
        if favorite.order as usize != index {
            return Err(RawFavoriteError::InvalidOrder);
        }
        if !commands.insert(favorite.command.clone()) {
            return Err(RawFavoriteError::DuplicateCommand(favorite.command.clone()));
        }
    }
    if raw_favorite_aggregate_bytes(favorites) > MAX_RAW_FAVORITE_AGGREGATE_BYTES {
        return Err(RawFavoriteError::AggregateTooLarge);
    }
    Ok(())
}

fn normalize_raw_favorite_order(favorites: &mut [RawFavorite]) {
    for (index, favorite) in favorites.iter_mut().enumerate() {
        favorite.order = index as u16;
    }
}

pub fn add_raw_favorite(
    favorites: &mut Vec<RawFavorite>,
    mut favorite: RawFavorite,
) -> Result<(), RawFavoriteError> {
    favorite.validate()?;
    if favorites
        .iter()
        .any(|item| item.command == favorite.command)
    {
        return Err(RawFavoriteError::DuplicateCommand(favorite.command));
    }
    if favorites.len() == MAX_RAW_FAVORITES {
        return Err(RawFavoriteError::TooManyFavorites);
    }
    favorite.order = favorites.len() as u16;
    let mut next = favorites.clone();
    next.push(favorite);
    validate_raw_favorites(&next)?;
    *favorites = next;
    Ok(())
}

pub fn update_raw_favorite(
    favorites: &mut Vec<RawFavorite>,
    replacement: RawFavorite,
) -> Result<(), RawFavoriteError> {
    replacement.validate()?;
    let Some(index) = favorites
        .iter()
        .position(|item| item.command == replacement.command)
    else {
        return Err(RawFavoriteError::Unknown(replacement.command));
    };
    let mut next = favorites.clone();
    next[index] = RawFavorite {
        order: index as u16,
        ..replacement
    };
    validate_raw_favorites(&next)?;
    *favorites = next;
    Ok(())
}

pub fn remove_raw_favorite(
    favorites: &mut Vec<RawFavorite>,
    command: &RawCommandId,
) -> Result<RawFavorite, RawFavoriteError> {
    let Some(index) = favorites.iter().position(|item| &item.command == command) else {
        return Err(RawFavoriteError::Unknown(command.clone()));
    };
    let mut next = favorites.clone();
    let removed = next.remove(index);
    normalize_raw_favorite_order(&mut next);
    validate_raw_favorites(&next)?;
    *favorites = next;
    Ok(removed)
}

pub fn move_raw_favorite(
    favorites: &mut Vec<RawFavorite>,
    command: &RawCommandId,
    delta: isize,
) -> Result<(), RawFavoriteError> {
    let Some(index) = favorites.iter().position(|item| &item.command == command) else {
        return Err(RawFavoriteError::Unknown(command.clone()));
    };
    let mut next = favorites.clone();
    let destination = shifted_index(index, next.len(), delta);
    let favorite = next.remove(index);
    next.insert(destination, favorite);
    normalize_raw_favorite_order(&mut next);
    validate_raw_favorites(&next)?;
    *favorites = next;
    Ok(())
}
