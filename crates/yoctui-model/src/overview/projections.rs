impl App {
    pub(crate) fn record_overview_image_size(&mut self, composition: &RootfsComposition) {
        let Some(packages) = composition.package_inventory() else {
            return;
        };
        let installed_bytes = packages.packages.iter().fold(0_u64, |total, package| {
            total.saturating_add(package.installed_size_bytes)
        });
        let snapshot = OverviewImageSizeSnapshot {
            image: composition.image.clone(),
            installed_bytes,
        };
        if self.overview_image_size_history.back() == Some(&snapshot) {
            return;
        }
        self.overview_image_size_history.push_back(snapshot);
        while self.overview_image_size_history.len() > MAX_OVERVIEW_IMAGE_SNAPSHOTS {
            self.overview_image_size_history.pop_front();
        }
    }

    pub fn overview_timeline(&self, now: SystemTime) -> Vec<OverviewTimelineRow> {
        let mut tasks = self.tasks.values().collect::<Vec<_>>();
        tasks.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        tasks.truncate(MAX_OVERVIEW_ROWS);
        let origin = tasks.iter().filter_map(|task| task.started).min();
        let by_id = tasks
            .iter()
            .map(|task| (task.id.0.clone(), *task))
            .collect::<BTreeMap<_, _>>();
        let mut longest = BTreeMap::<String, (u64, Option<String>)>::new();
        for task in &tasks {
            longest_path(
                &task.id.0,
                &by_id,
                now,
                &mut std::collections::BTreeSet::new(),
                &mut longest,
            );
        }
        let mut critical = std::collections::BTreeSet::new();
        let mut cursor = longest
            .iter()
            .max_by_key(|(id, value)| (value.0, std::cmp::Reverse(id.as_str())))
            .map(|(id, _)| id.clone());
        while let Some(id) = cursor {
            if !critical.insert(id.clone()) {
                break;
            }
            cursor = longest
                .get(&id)
                .and_then(|(_, predecessor)| predecessor.clone());
        }
        let mut rows = tasks
            .into_iter()
            .map(|task| OverviewTimelineRow {
                id: task.id.0.clone(),
                label: format!("{}:{}", task.recipe, task.task),
                state: task.state,
                start_millis: task
                    .started
                    .and_then(|started| {
                        origin.and_then(|origin| started.duration_since(origin).ok())
                    })
                    .map_or(0, |value| value.as_millis() as u64),
                duration_millis: task
                    .elapsed_at(now)
                    .map_or(0, |value| value.as_millis() as u64),
                critical: critical.contains(&task.id.0),
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| (row.start_millis, row.label.clone()));
        rows
    }

    pub fn overview_rebuild_causes(&self) -> Vec<OverviewEdgeRow> {
        let differences = match &self.signature_comparison {
            SignatureComparisonState::Available { differences, .. }
            | SignatureComparisonState::Partial { differences, .. } => differences,
            _ => return Vec::new(),
        };
        differences
            .iter()
            .take(MAX_OVERVIEW_ROWS)
            .map(|difference| OverviewEdgeRow {
                source: match difference.category {
                    SignatureDifferenceCategory::BaseHash => "signature".into(),
                    SignatureDifferenceCategory::ChangedValue => "variable".into(),
                    SignatureDifferenceCategory::Dependency => "dependency".into(),
                    SignatureDifferenceCategory::Unavailable => "unknown".into(),
                },
                relation: "changed".into(),
                target: difference.key.clone(),
            })
            .collect()
    }

    pub fn overview_cache(&self) -> OverviewCacheProjection {
        let mut projection = OverviewCacheProjection {
            sstate_hits: self.build.cache.setscene_completed,
            sstate_misses: self.build.cache.setscene_failed,
            fetch_completed: self.build.cache.fetch_completed,
            fetch_failed: self.build.cache.fetch_failed,
            sstate_dir: self.workspace.variables.get("SSTATE_DIR").cloned(),
            downloads_dir: self.workspace.variables.get("DL_DIR").cloned(),
            ..OverviewCacheProjection::default()
        };
        for task in self.tasks.values() {
            let is_sstate = task.task.ends_with("_setscene");
            let is_fetch = task.task == "do_fetch";
            if is_sstate && task.state == TaskState::Active {
                projection.sstate_active += 1;
            }
            if is_fetch && task.state == TaskState::Active {
                projection.fetch_active += 1;
            }
        }
        projection
    }

    pub fn overview_image_sizes(&self) -> Vec<OverviewSizedRow> {
        let composition = match &self.rootfs_composition {
            RootfsCompositionState::Available { composition, .. }
            | RootfsCompositionState::Partial { composition, .. } => composition,
            _ => return Vec::new(),
        };
        let Some(packages) = composition.package_inventory() else {
            return Vec::new();
        };
        let mut categories = BTreeMap::<String, u64>::new();
        for package in &packages.packages {
            *categories.entry(package.category.clone()).or_default() = categories
                .get(&package.category)
                .copied()
                .unwrap_or(0)
                .saturating_add(package.installed_size_bytes);
        }
        let mut rows = categories
            .into_iter()
            .map(|(label, bytes)| OverviewSizedRow { label, bytes })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| std::cmp::Reverse(row.bytes));
        rows.truncate(MAX_OVERVIEW_ROWS);
        rows
    }

    pub fn overview_image_size_delta(&self) -> Option<OverviewImageSizeDelta> {
        let composition = self.rootfs_composition.composition()?;
        let current = self.overview_image_size_history.back()?;
        if current.image != composition.image {
            return None;
        }
        let previous = self
            .overview_image_size_history
            .iter()
            .rev()
            .skip(1)
            .find(|snapshot| {
                snapshot.image.machine == current.image.machine
                    && snapshot.image.image == current.image.image
            })?;
        Some(OverviewImageSizeDelta {
            current_bytes: current.installed_bytes,
            previous_bytes: previous.installed_bytes,
            delta_bytes: i128::from(current.installed_bytes) - i128::from(previous.installed_bytes),
        })
    }

    pub fn overview_provenance(&self) -> Vec<OverviewEdgeRow> {
        let mut rows = Vec::new();
        for (variable, chain) in &self.workspace.variable_provenance_chain {
            let mut previous = variable.as_str();
            for source in chain {
                rows.push(OverviewEdgeRow {
                    source: previous.into(),
                    relation: "set by".into(),
                    target: source.clone(),
                });
                previous = source;
                if rows.len() == MAX_OVERVIEW_ROWS {
                    return rows;
                }
            }
        }
        rows
    }

    pub fn overview_package_topology(&self) -> Vec<OverviewEdgeRow> {
        let mut rows = Vec::new();
        for (identity, state) in &self.package_details {
            let (PackageDetailState::Available { detail, .. }
            | PackageDetailState::Partial { detail, .. }) = state
            else {
                continue;
            };
            if let PackageField::Available(dependencies) = &detail.runtime_dependencies {
                for dependency in dependencies {
                    rows.push(OverviewEdgeRow {
                        source: identity.name.clone(),
                        relation: "RDEPENDS".into(),
                        target: dependency.name.clone(),
                    });
                    if rows.len() == MAX_OVERVIEW_ROWS {
                        return rows;
                    }
                }
            }
        }
        rows.sort_by(|left, right| {
            (&left.source, &left.target).cmp(&(&right.source, &right.target))
        });
        rows
    }

    pub fn overview_supply_chain(&self) -> OverviewSupplyChainProjection {
        let mut projection = OverviewSupplyChainProjection::default();
        for report in self.security.inventory.reports().unwrap_or_default() {
            match report {
                SecurityReport::Cve(report) => {
                    projection.cve_reports += 1;
                    projection.vulnerable += report
                        .findings
                        .iter()
                        .filter(|finding| matches!(finding.status, crate::CveStatus::Vulnerable))
                        .count();
                }
                SecurityReport::Spdx(document) => {
                    projection.spdx_documents += 1;
                    projection.components += document.components.len();
                    projection
                        .limitations
                        .extend(document.limitations.iter().cloned());
                }
                SecurityReport::CycloneDx(document) => {
                    projection.cyclonedx_documents += 1;
                    projection.components += document.components.len();
                    projection
                        .limitations
                        .extend(document.limitations.iter().cloned());
                }
                SecurityReport::PackageManifest(document) => {
                    projection.manifest_documents += 1;
                    projection.components += document.components.len();
                    projection
                        .limitations
                        .extend(document.limitations.iter().cloned());
                }
            }
        }
        projection.limitations.sort();
        projection.limitations.dedup();
        projection.limitations.truncate(16);
        projection
    }
}
