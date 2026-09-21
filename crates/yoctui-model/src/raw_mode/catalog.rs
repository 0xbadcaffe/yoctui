#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCatalog {
    pub version: u16,
    pub categories: Vec<RawCategory>,
    pub commands: Vec<RawCommand>,
}

impl RawCatalog {
    pub fn normalize(self) -> Result<Self, RawCatalogError> {
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), RawCatalogError> {
        if self.version == 0 {
            return Err(RawCatalogError::InvalidVersion);
        }
        if self.categories.is_empty() || self.categories.len() > MAX_RAW_CATEGORIES {
            return Err(RawCatalogError::InvalidCategoryCount(self.categories.len()));
        }
        if self.commands.is_empty() || self.commands.len() > MAX_RAW_COMMANDS {
            return Err(RawCatalogError::InvalidCommandCount(self.commands.len()));
        }

        let mut category_ids = BTreeSet::new();
        for category in &self.categories {
            validate_id_value("category", category.id.as_str())?;
            if !category_ids.insert(category.id.clone()) {
                return Err(RawCatalogError::DuplicateCategory(category.id.clone()));
            }
            if !valid_label(&category.label) || !valid_text(&category.reference_heading) {
                return Err(RawCatalogError::InvalidCategory(category.id.clone()));
            }
        }

        let mut command_ids = BTreeSet::new();
        let mut reference_ids = BTreeSet::new();
        for command in &self.commands {
            validate_id_value("command", command.id.as_str())?;
            if !command_ids.insert(command.id.clone()) {
                return Err(RawCatalogError::DuplicateCommand(command.id.clone()));
            }
            if !category_ids.contains(&command.category) {
                return Err(RawCatalogError::UnknownCategory {
                    command: command.id.clone(),
                    category: command.category.clone(),
                });
            }
            validate_command(command, &mut reference_ids)?;
        }
        Ok(())
    }

    pub fn category(&self, id: &RawCategoryId) -> Option<&RawCategory> {
        self.categories.iter().find(|category| &category.id == id)
    }

    /// Category order used by the Raw browser. Favorites is a pinned virtual
    /// collection; every reference-derived category retains catalog order.
    pub fn browser_categories(&self) -> Vec<&RawCategory> {
        self.categories
            .iter()
            .filter(|category| category.kind == RawCategoryKind::Favorites)
            .chain(
                self.categories
                    .iter()
                    .filter(|category| category.kind != RawCategoryKind::Favorites),
            )
            .collect()
    }

    pub fn command(&self, id: &RawCommandId) -> Option<&RawCommand> {
        self.commands.iter().find(|command| &command.id == id)
    }

    pub fn preview(
        &self,
        request: &RawPreviewRequest,
        authority: Option<&DaemonCompatibilitySnapshot>,
    ) -> Result<RawExecutionPreview, RawPreviewError> {
        self.validate()
            .map_err(|error| RawPreviewError::InvalidCatalog(error.to_string()))?;
        if request.catalog_version != self.version {
            return Err(RawPreviewError::StaleCatalog {
                current: self.version,
                received: request.catalog_version,
            });
        }
        let command = self
            .command(&request.command)
            .ok_or_else(|| RawPreviewError::UnknownCommand(request.command.clone()))?;
        let RawExecutionPolicy::Executable { template } = &command.execution else {
            return Err(RawPreviewError::ReferenceOnly(command.id.clone()));
        };
        let authority = authority.ok_or(RawPreviewError::MissingAuthority)?;
        if request.capability_generation != authority.snapshot.generation {
            return Err(RawPreviewError::StaleCapabilityGeneration {
                current: authority.snapshot.generation,
                received: request.capability_generation,
            });
        }
        let availability = command.availability(Some(authority));
        if !availability.is_enabled() {
            return Err(RawPreviewError::CapabilityUnavailable {
                state: availability.state,
                reasons: availability
                    .issues
                    .into_iter()
                    .map(|issue| issue.reason)
                    .collect(),
            });
        }
        let current_build_directory = authority
            .snapshot
            .environment
            .build_directory
            .value()
            .ok_or(RawPreviewError::MissingBuildDirectory)?;
        if current_build_directory != &request.build_directory {
            return Err(RawPreviewError::StaleBuildDirectory {
                current: current_build_directory.clone(),
                received: request.build_directory.clone(),
            });
        }
        validate_raw_preview_build_directory(current_build_directory)?;
        validate_raw_preview_parameters(command, &request.parameters)?;
        request
            .additional_arguments
            .validate()
            .map_err(RawPreviewError::InvalidAdditionalArguments)?;

        let mut arguments = Vec::new();
        let mut indexed_arguments = vec![RawPreviewArgument {
            index: 0,
            value: template.executable.as_str().into(),
            source: RawPreviewArgumentSource::Executable,
        }];
        for (template_index, argument) in template.arguments.iter().enumerate() {
            let Some(value) = render_raw_template_argument(argument, &request.parameters) else {
                continue;
            };
            push_raw_preview_argument(
                &mut arguments,
                &mut indexed_arguments,
                value,
                RawPreviewArgumentSource::Template {
                    index: template_index,
                },
            )?;
        }
        for (additional_index, value) in request.additional_arguments.as_slice().iter().enumerate()
        {
            push_raw_preview_argument(
                &mut arguments,
                &mut indexed_arguments,
                value.clone(),
                RawPreviewArgumentSource::Additional {
                    index: additional_index,
                },
            )?;
        }
        let limitations = availability
            .issues
            .iter()
            .flat_map(|issue| issue.limitations.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(RawExecutionPreview {
            catalog_version: self.version,
            command: command.id.clone(),
            executable: template.executable,
            arguments,
            indexed_arguments,
            capability_generation: authority.snapshot.generation,
            environment: authority.snapshot.environment.clone(),
            build_directory: current_build_directory.clone(),
            implementations: availability.implementations,
            capability_issues: availability.issues,
            interaction: template.interaction,
            safety: template.safety,
            limitations,
        })
    }
}
