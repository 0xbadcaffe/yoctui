//! Cache acceleration reuses the Maintenance Sstate workspace and native BitBake metadata.
use crate::*;
mod report;
mod settings;
pub use settings::{SstateCacheProfile, SstateCacheSettings};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SstateCacheState {
    pub dashboard: bool,
    pub daemon_supports_profiles: bool,
    pub scroll: usize,
    pub visible_rows: usize,
    pub review_scroll: usize,
    pub review_width: usize,
    pub review_visible_rows: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SstateCacheDialog {
    Configure {
        editor: PopupEditor,
        build_directory: PathBuf,
        validation: Option<String>,
    },
    Review(SstateCacheProfile),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SstateCacheAction {
    Open,
    Configure,
    Review,
    Save,
    Cancel,
    Disable,
    Tools,
    Scroll(isize),
    First,
    Last,
}

pub(crate) fn reduce(app: &mut App, action: SstateCacheAction) -> Option<Effect> {
    if action == SstateCacheAction::Open {
        if app.active_dialog().is_some() || app.onboarding.open {
            return None;
        }
        let effect = crate::update(app, Action::Open(Screen::Maintenance));
        app.maintenance.view = MaintenanceView::Sstate;
        app.sstate_cache.dashboard = true;
        return effect;
    }
    let cache_dialog = matches!(app.active_dialog(), Some(Dialog::SstateCache(_)));
    if app.screen != Screen::Maintenance
        || app.maintenance.view != MaintenanceView::Sstate
        || (!cache_dialog
            && (app.focus != FocusTarget::Workspace
                || app.active_dialog().is_some()
                || app.menu.is_open()
                || app.command_palette_open))
    {
        return None;
    }
    match action {
        SstateCacheAction::Configure => {
            let Some(build_directory) = app.workspace.build_dir.clone() else {
                app.notification =
                    Some("Connect to a build environment before configuring cache reuse".into());
                return None;
            };
            let settings = app
                .sstate_cache_profile()
                .map(|p| p.settings.clone())
                .unwrap_or_else(|| SstateCacheSettings {
                    hash_server: "auto".into(),
                    ..Default::default()
                });
            let mut editor = PopupEditor::new(settings.document());
            let _ = editor.select_toml_value("mirror_url");
            crate::open_dialog(
                app,
                Dialog::SstateCache(Box::new(SstateCacheDialog::Configure {
                    editor,
                    build_directory,
                    validation: None,
                })),
            );
        }
        SstateCacheAction::Review => {
            app.sstate_cache.review_scroll = 0;
            let Some(Dialog::SstateCache(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            if let SstateCacheDialog::Configure {
                editor,
                build_directory,
                ..
            } = *dialog
            {
                match SstateCacheSettings::parse(&editor.text) {
                    Ok(settings) => crate::replace_dialog(
                        app,
                        Dialog::SstateCache(Box::new(SstateCacheDialog::Review(
                            SstateCacheProfile {
                                build_directory,
                                settings,
                            },
                        ))),
                    ),
                    Err(error) => crate::replace_dialog(
                        app,
                        Dialog::SstateCache(Box::new(SstateCacheDialog::Configure {
                            editor,
                            build_directory,
                            validation: Some(error),
                        })),
                    ),
                }
            }
        }
        SstateCacheAction::Save => {
            let Some(Dialog::SstateCache(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            let SstateCacheDialog::Review(profile) = *dialog else {
                return None;
            };
            if app.workspace.build_dir.as_ref() != Some(&profile.build_directory) {
                app.notification = Some(
                    "Build environment changed; cancel and review cache settings again".into(),
                );
                return None;
            }
            if let Some(error) = app.sstate_cache_unavailable() {
                app.notification = Some(error);
                return None;
            }
            if let Err(error) = profile.validate() {
                app.notification = Some(error);
                return None;
            }
            let profiles = &mut app.preferences.sstate_cache_profiles;
            if let Some(p) = profiles
                .iter_mut()
                .find(|p| p.build_directory == profile.build_directory)
            {
                *p = profile;
            } else if profiles.len() < 16 {
                profiles.push(profile);
            } else {
                app.notification = Some(
                    "Sixteen cache profiles are already saved; disable an unused profile first"
                        .into(),
                );
                return None;
            }
            crate::close_dialog(app);
            app.settings_dirty = true;
            app.notification = Some(
                "Cache reuse saved for future confirmed builds in this build directory".into(),
            );
            return Some(Effect::PersistSettings);
        }
        SstateCacheAction::Cancel => {
            if cache_dialog {
                crate::close_dialog(app);
            }
        }
        SstateCacheAction::Disable => {
            app.preferences
                .sstate_cache_profiles
                .retain(|p| Some(&p.build_directory) != app.workspace.build_dir.as_ref());
            app.settings_dirty = true;
            app.notification = Some(
                "Future builds use existing cache configuration; cached files were preserved"
                    .into(),
            );
            return Some(Effect::PersistSettings);
        }
        SstateCacheAction::Tools => app.sstate_cache.dashboard = false,
        action @ (SstateCacheAction::Scroll(_)
        | SstateCacheAction::First
        | SstateCacheAction::Last) => {
            let review = if let Some(Dialog::SstateCache(dialog)) = app.active_dialog() {
                if let SstateCacheDialog::Review(profile) = dialog.as_ref() {
                    Some(
                        profile
                            .review_rows(app.sstate_cache.review_width.max(1))
                            .len(),
                    )
                } else {
                    None
                }
            } else {
                None
            };
            let (maximum, current) = if let Some(count) = review {
                (
                    count.saturating_sub(app.sstate_cache.review_visible_rows.max(1)),
                    &mut app.sstate_cache.review_scroll,
                )
            } else {
                (
                    app.sstate_cache_rows()
                        .len()
                        .saturating_sub(app.sstate_cache.visible_rows.max(1)),
                    &mut app.sstate_cache.scroll,
                )
            };
            *current = match action {
                SstateCacheAction::First => 0,
                SstateCacheAction::Last => maximum,
                SstateCacheAction::Scroll(delta) => {
                    crate::shifted_index(*current, delta, maximum + 1)
                }
                _ => unreachable!(),
            };
        }
        SstateCacheAction::Open => {}
    }
    None
}
impl App {
    pub fn sstate_cache_profile(&self) -> Option<&SstateCacheProfile> {
        self.preferences
            .sstate_cache_profiles
            .iter()
            .find(|p| Some(&p.build_directory) == self.workspace.build_dir.as_ref())
    }
    pub fn sstate_cache_unavailable(&self) -> Option<String> {
        if self.daemon.status != ClientReplicaStatus::Current
            || !self.build_environment.connected()
            || self.workspace.build_dir.is_none()
        {
            Some("Cache acceleration requires a connected daemon build environment".into())
        } else if !self.sstate_cache.daemon_supports_profiles {
            Some("Update/restart the daemon to enable reviewed sstate cache profiles".into())
        } else if !self.workspace.bitbake_version.as_deref().is_some_and(|v| {
            let mut parts = v.split('.');
            parts
                .next()
                .and_then(|v| v.parse::<u32>().ok())
                .zip(parts.next().and_then(|v| v.parse::<u32>().ok()))
                .is_some_and(|v| v >= (2, 0))
        }) {
            Some("Cache profiles require a reported BitBake version of 2.0 or newer".into())
        } else {
            None
        }
    }
    pub fn sstate_cache_build_summary(&self) -> String {
        self.sstate_cache_profile().map_or_else(
            || "Sstate: existing BitBake configuration".into(),
            |p| p.settings.summary(),
        )
    }
}
#[cfg(test)]
#[path = "tests/sstate_cache.rs"]
mod tests;
