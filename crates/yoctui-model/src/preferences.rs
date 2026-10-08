use serde::{Deserialize, Serialize};

use crate::{AnimationSpeed, App, EffectiveKeymap, KeymapPreferences, Theme};

pub const WORKBENCH_PREFERENCES_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum UiDensity {
    #[default]
    Comfortable,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SymbolPreference {
    #[default]
    Unicode,
    Ascii,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ChartPreference {
    #[default]
    Automatic,
    AccessibleText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ImagePreviewPreference {
    #[default]
    MetadataOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalPrefixPreference {
    #[default]
    CtrlB,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkbenchPreferences {
    pub schema_version: u16,
    pub theme: Theme,
    pub animation_speed: AnimationSpeed,
    pub reduced_motion: bool,
    pub color_enabled: bool,
    pub density: UiDensity,
    /// Saved startup choice; the session toggle does not overwrite it.
    pub inspector_visible: bool,
    pub symbols: SymbolPreference,
    pub mouse_enabled: bool,
    pub footer_shortcuts: bool,
    pub log_wrap: bool,
    pub log_follow: bool,
    pub remember_pane_sizes: bool,
    pub charts: ChartPreference,
    pub image_previews: ImagePreviewPreference,
    pub terminal_prefix: TerminalPrefixPreference,
    pub keymap: KeymapPreferences,
}

impl Default for WorkbenchPreferences {
    fn default() -> Self {
        Self {
            schema_version: WORKBENCH_PREFERENCES_SCHEMA_VERSION,
            theme: Theme::default(),
            animation_speed: AnimationSpeed::default(),
            reduced_motion: false,
            color_enabled: true,
            density: UiDensity::default(),
            inspector_visible: false,
            symbols: SymbolPreference::default(),
            mouse_enabled: true,
            footer_shortcuts: true,
            log_wrap: false,
            log_follow: true,
            remember_pane_sizes: true,
            charts: ChartPreference::default(),
            image_previews: ImagePreviewPreference::default(),
            terminal_prefix: TerminalPrefixPreference::default(),
            keymap: KeymapPreferences::default(),
        }
    }
}

impl WorkbenchPreferences {
    pub fn reset_setting(&mut self, setting: Setting) {
        let defaults = Self::default();
        match setting {
            Setting::Theme => self.theme = defaults.theme,
            Setting::Density => self.density = defaults.density,
            Setting::Inspector => self.inspector_visible = defaults.inspector_visible,
            Setting::Symbols => self.symbols = defaults.symbols,
            Setting::AnimationSpeed => self.animation_speed = defaults.animation_speed,
            Setting::ReducedMotion => self.reduced_motion = defaults.reduced_motion,
            Setting::Color => self.color_enabled = defaults.color_enabled,
            Setting::Mouse => self.mouse_enabled = defaults.mouse_enabled,
            Setting::FooterShortcuts => self.footer_shortcuts = defaults.footer_shortcuts,
            Setting::LogWrap => self.log_wrap = defaults.log_wrap,
            Setting::LogFollow => self.log_follow = defaults.log_follow,
            Setting::RememberPaneSizes => self.remember_pane_sizes = defaults.remember_pane_sizes,
            Setting::Charts => self.charts = defaults.charts,
            Setting::ImagePreviews => self.image_previews = defaults.image_previews,
            Setting::TerminalPrefix => self.terminal_prefix = defaults.terminal_prefix,
            Setting::Keybindings => self.keymap = defaults.keymap,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != WORKBENCH_PREFERENCES_SCHEMA_VERSION {
            return Err(format!(
                "unsupported workbench preference schema {}; expected {}",
                self.schema_version, WORKBENCH_PREFERENCES_SCHEMA_VERSION
            ));
        }
        EffectiveKeymap::from_preferences(&self.keymap)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn migrate(self) -> Result<Self, String> {
        self.validate()?;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Theme,
    Density,
    Inspector,
    Symbols,
    AnimationSpeed,
    ReducedMotion,
    Color,
    Mouse,
    FooterShortcuts,
    LogWrap,
    LogFollow,
    RememberPaneSizes,
    Charts,
    ImagePreviews,
    TerminalPrefix,
    Keybindings,
}

pub const SETTINGS: [Setting; 16] = [
    Setting::Theme,
    Setting::Density,
    Setting::Inspector,
    Setting::Symbols,
    Setting::AnimationSpeed,
    Setting::ReducedMotion,
    Setting::Color,
    Setting::Mouse,
    Setting::FooterShortcuts,
    Setting::LogWrap,
    Setting::LogFollow,
    Setting::RememberPaneSizes,
    Setting::Charts,
    Setting::ImagePreviews,
    Setting::TerminalPrefix,
    Setting::Keybindings,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceRow {
    pub setting: Setting,
    pub label: &'static str,
    pub value: String,
    pub default_value: String,
    pub is_modified: bool,
    pub disabled_reason: Option<&'static str>,
}

impl PreferenceRow {
    pub fn enabled(&self) -> bool {
        self.disabled_reason.is_none()
    }
}

impl App {
    pub fn set_inspector_visible(&mut self, visible: bool) {
        self.inspector_visible = visible;
        if !visible {
            if self.focus == crate::FocusTarget::Inspector {
                self.focus = if crate::focus_target_is_relevant(self, crate::FocusTarget::Workspace)
                {
                    crate::FocusTarget::Workspace
                } else {
                    crate::FocusTarget::Navigator
                };
            }
            if self.zoomed_pane == Some(crate::FocusTarget::Inspector) {
                self.zoomed_pane = None;
            }
        }
    }

    pub fn effective_preferences(&self) -> WorkbenchPreferences {
        let mut preferences = self.preferences.clone();
        preferences.theme = self.theme;
        preferences.animation_speed = self.animation_speed;
        preferences.reduced_motion = self.reduced_motion;
        preferences.color_enabled = if self.color_forced_off {
            self.preferences.color_enabled
        } else {
            self.color_enabled
        };
        preferences.log_wrap = self.logs.wrap;
        preferences.log_follow = self.logs.follow;
        preferences.keymap = self.keymap_preferences.clone();
        preferences
    }

    pub fn install_preferences(&mut self, preferences: WorkbenchPreferences) -> Result<(), String> {
        preferences.validate()?;
        let effective = EffectiveKeymap::from_preferences(&preferences.keymap)
            .map_err(|error| error.to_string())?;
        self.theme = preferences.theme;
        self.animation_speed = preferences.animation_speed;
        self.reduced_motion = preferences.reduced_motion;
        if !self.color_forced_off {
            self.color_enabled = preferences.color_enabled;
        }
        self.logs.wrap = preferences.log_wrap;
        self.logs.follow = preferences.log_follow;
        self.logs.paused_len = (!preferences.log_follow).then_some(self.logs.entries.len());
        self.keymap_preferences = preferences.keymap.clone();
        self.effective_keymap = effective;
        self.set_inspector_visible(preferences.inspector_visible);
        self.preferences = preferences;
        self.keymap_chord.clear();
        Ok(())
    }

    pub fn preference_rows(&self) -> Vec<PreferenceRow> {
        let defaults = self.preference_rows_for(&WorkbenchPreferences::default(), false);
        let preferences = self.effective_preferences();
        let mut rows = self.preference_rows_for(&preferences, true);
        for (row, default) in rows.iter_mut().zip(defaults) {
            row.default_value = default.value.clone();
            row.is_modified = if row.setting == Setting::Keybindings {
                preferences.keymap != WorkbenchPreferences::default().keymap
            } else if row.setting == Setting::Color {
                preferences.color_enabled != WorkbenchPreferences::default().color_enabled
            } else {
                row.value != default.value
            };
        }
        rows
    }

    fn preference_rows_for(
        &self,
        preferences: &WorkbenchPreferences,
        launch_overrides: bool,
    ) -> Vec<PreferenceRow> {
        let custom = preferences.keymap.overrides.len();
        SETTINGS
            .into_iter()
            .map(|setting| {
                let (label, value, disabled_reason) = match setting {
                    Setting::Theme => ("Theme", preferences.theme.display_name().into(), None),
                    Setting::Density => {
                        ("Visual density", format!("{:?}", preferences.density), None)
                    }
                    Setting::Inspector => (
                        "Inspector at startup", preferences.inspector_visible.to_string(), None,
                    ),
                    Setting::Symbols => {
                        ("Symbols", format!("{:?}", preferences.symbols), None)
                    }
                    Setting::AnimationSpeed => (
                        "Animation speed",
                        format!("{:?}", preferences.animation_speed),
                        None,
                    ),
                    Setting::ReducedMotion => (
                        "Reduced motion",
                        preferences.reduced_motion.to_string(),
                        None,
                    ),
                    Setting::Color if launch_overrides && self.color_forced_off => (
                        "Color",
                        "false (--no-color launch override)".into(),
                        Some("Disabled by --no-color for this launch; the stored choice is preserved."),
                    ),
                    Setting::Color => {
                        ("Color", preferences.color_enabled.to_string(), None)
                    }
                    Setting::Mouse => (
                        "Mouse input",
                        preferences.mouse_enabled.to_string(),
                        None,
                    ),
                    Setting::FooterShortcuts => (
                        "Footer shortcuts",
                        preferences.footer_shortcuts.to_string(),
                        None,
                    ),
                    Setting::LogWrap => {
                        ("Log wrap", preferences.log_wrap.to_string(), None)
                    }
                    Setting::LogFollow => {
                        ("Log follow", preferences.log_follow.to_string(), None)
                    }
                    Setting::RememberPaneSizes => (
                        "Remember pane sizes",
                        preferences.remember_pane_sizes.to_string(),
                        None,
                    ),
                    Setting::Charts => {
                        ("Charts", format!("{:?}", preferences.charts), None)
                    }
                    Setting::ImagePreviews => (
                        "Image previews",
                        "MetadataOnly".into(),
                        Some("Raster preview is unavailable: deploy artifacts provide no safe raster/protocol authority."),
                    ),
                    Setting::TerminalPrefix => (
                        "Terminal prefix",
                        "Ctrl+B".into(),
                        Some("Ctrl+B is reserved to keep PTY input, workbench commands, and keymaps unambiguous."),
                    ),
                    Setting::Keybindings => (
                        "Keybindings",
                        format!(
                            "{} actions · {custom} custom",
                            crate::global_operator_action_definitions().len()
                        ),
                        None,
                    ),
                };
                PreferenceRow {
                    setting,
                    label,
                    value,
                    default_value: String::new(),
                    is_modified: false,
                    disabled_reason,
                }
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/preferences/mod.rs"]
mod tests;
