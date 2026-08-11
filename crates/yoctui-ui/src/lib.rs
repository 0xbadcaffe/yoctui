//! Rendering only; no backend parsing or mutation lives in widgets.
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, Wrap},
};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};
use yoctui_model::{
    App, BackgroundJobKind, BackgroundJobOutputSource, BackgroundJobStatus, BuildEnvironmentState,
    BuildStatus, ConfigCopyValue, DependencyEdgeKind, DependencyGraph, DependencyGraphState,
    DependencyNodeId, DependencyPathResult, DevtoolAction, DevtoolCapability, DevtoolGitState,
    DevtoolStatus, DevtoolStatusError, DevtoolWorkspace, Dialog, FocusTarget, GitFileState,
    ImageArtifactField, ImageArtifactInventoryState, LayerBrowser, LayerBrowserEntry,
    LayerInspectorMode, MaintenanceCapability, MaintenanceCapabilitySnapshot, MaintenanceDialog,
    MaintenanceIntegrationDiagnostics, MaintenanceIntegrationsSnapshot, MaintenanceOperation,
    MaintenanceOperationPreview, MaintenanceServiceDiagnostics, MaintenanceSessionStatus,
    MaintenanceTool, MaintenanceToolCapability, MaintenanceToolInterface, MaintenanceView,
    PackageDetailState, PackageField, PackageIdentity, PackageInventoryState, PreviewKind,
    QaCapability, QaCheckAvailability, QaCheckFamily, QaDialog, QaFindingStatus, QaLayerCapability,
    QaLayerRunCapability, QaOutputStream, QaReportFailureKind, QaReportInventoryState,
    QaSessionStatus, QaStatusFilter, QaView, QemuCapability, QemuDisplayMode, QemuLaunchDialog,
    QemuLaunchField, QemuLaunchPreview, QemuNetworkingMode, QemuSerialMode, QemuSessionId, Recipe,
    RecipeBuildStatus, RecipeEditor, RecipeIdentity, Screen, SdkArtifactInventoryState,
    SdkArtifactKind, SdkBuildAction, SdkKind, SdkNativeDialog, SdkNativeField, SdkNativeMode,
    SdkNativePreview, SdkOperation, SdkPublishDraft, SdkPublishPreview, SdkSessionId,
    SdkToolCapability, SecurityCapability, SecurityDialog, SecurityInventoryState,
    SecurityOperation, SecurityOutputStream, SecurityReport, SecurityScope, SecuritySessionStatus,
    SecurityView, Severity, SignatureComparisonState, SignatureDifferenceCategory,
    SignatureDumpState, SpdxArtifactKind, TaskFilterField, TaskRow, TaskState,
    TestComparisonCategory, TestComparisonState, TestExecutableCapability, TestJunitExportState,
    TestLaunchDialog, TestLaunchField, TestLaunchPreview, TestResultInventoryState,
    TestWorkspaceView, Theme, VariableIdentity, WicCapability, WicCompression, WicCreateDialog,
    WicCreateField, WicCreatePreview, WicDevice, WicDeviceInventoryState, WicDevicePickerDialog,
    WicKickstart, WicOperation, WicOutputInventoryState, WicSessionId, WicWritePhraseDialog,
    WicWritePreview, config_comparison, config_edit_disabled_reason, config_source_disabled_reason,
    format_duration, selected_config_copy_value,
};

fn matches_metadata(query: &str, values: &[&str]) -> bool {
    let query = query.to_lowercase();
    query.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(query.as_str()))
}

fn metadata_title(base: String, app: &App) -> String {
    if app.metadata_searching {
        format!("{base} | search: {}_", app.metadata_query)
    } else if app.metadata_query.is_empty() {
        base
    } else {
        format!("{base} | search: {}", app.metadata_query)
    }
}

fn timestamp_text(timestamp: SystemTime) -> String {
    timestamp.duration_since(UNIX_EPOCH).map_or_else(
        |_| "before Unix epoch".into(),
        |duration| format!("{}s since Unix epoch", duration.as_secs()),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThemePalette {
    foreground: Color,
    background: Color,
    border: Color,
    focused_border: Color,
    selection_foreground: Color,
    selection_background: Color,
    disabled: Color,
    info: Color,
    success: Color,
    warning: Color,
    error: Color,
    progress: Color,
    accent: Color,
    syntax_keyword: Color,
    syntax_name: Color,
    syntax_operator: Color,
    syntax_value: Color,
    syntax_comment: Color,
    attribute_only: bool,
}

impl ThemePalette {
    fn for_app(app: &App) -> Self {
        if !app.color_enabled {
            return Self::monochrome();
        }
        match app.theme {
            Theme::DarkPro => Self::packrat([
                (28, 28, 28),
                (36, 36, 36),
                (68, 68, 68),
                (0, 95, 95),
                (212, 212, 212),
                (154, 154, 154),
                (90, 90, 90),
                (95, 215, 215),
                (135, 215, 0),
                (215, 175, 0),
                (215, 95, 95),
                (175, 135, 215),
            ]),
            Theme::WhiteClassic => Self::packrat([
                (248, 248, 248),
                (240, 240, 240),
                (192, 192, 192),
                (178, 208, 232),
                (20, 20, 20),
                (74, 74, 74),
                (136, 136, 136),
                (0, 107, 107),
                (26, 122, 0),
                (139, 105, 20),
                (204, 34, 0),
                (123, 61, 160),
            ]),
            Theme::MatrixGreen => Self::packrat([
                (0, 0, 0),
                (6, 14, 6),
                (20, 60, 20),
                (0, 50, 0),
                (0, 204, 68),
                (0, 119, 34),
                (0, 51, 17),
                (0, 255, 136),
                (0, 221, 0),
                (136, 255, 0),
                (255, 51, 0),
                (136, 255, 136),
            ]),
            Theme::VscodeDark => Self::packrat([
                (30, 30, 30),
                (37, 37, 38),
                (68, 68, 68),
                (38, 79, 120),
                (212, 212, 212),
                (154, 154, 154),
                (106, 106, 106),
                (79, 193, 255),
                (106, 153, 85),
                (220, 220, 170),
                (244, 71, 71),
                (197, 134, 192),
            ]),
            Theme::VscodeLight => Self::packrat([
                (255, 255, 255),
                (243, 243, 243),
                (200, 200, 200),
                (173, 214, 255),
                (0, 0, 0),
                (68, 68, 68),
                (138, 138, 138),
                (0, 112, 193),
                (9, 134, 88),
                (120, 83, 0),
                (205, 49, 49),
                (175, 0, 219),
            ]),
            Theme::AccessibleDark => Self::packrat([
                (18, 18, 18),
                (28, 28, 30),
                (92, 92, 98),
                (45, 65, 72),
                (238, 238, 238),
                (188, 188, 192),
                (120, 120, 126),
                (86, 180, 233),
                (0, 158, 115),
                (240, 228, 66),
                (213, 94, 0),
                (204, 121, 167),
            ]),
            Theme::SoftLight => Self::packrat([
                (252, 252, 253),
                (241, 243, 245),
                (164, 170, 178),
                (198, 220, 232),
                (30, 34, 39),
                (78, 84, 92),
                (130, 136, 145),
                (0, 103, 148),
                (35, 120, 74),
                (139, 100, 0),
                (184, 44, 52),
                (128, 70, 160),
            ]),
            Theme::HighContrast => Self::packrat([
                (0, 0, 0),
                (12, 12, 12),
                (210, 210, 210),
                (70, 70, 70),
                (255, 255, 255),
                (220, 220, 220),
                (150, 150, 150),
                (0, 220, 255),
                (50, 255, 100),
                (255, 235, 40),
                (255, 70, 70),
                (255, 100, 235),
            ]),
            Theme::Monochrome => Self::monochrome(),
        }
    }

    fn packrat(colors: [(u8, u8, u8); 12]) -> Self {
        let [
            bg,
            _bg2,
            border,
            selected,
            fg,
            fg2,
            fg3,
            cyan,
            green,
            yellow,
            red,
            magenta,
        ] = colors;
        let rgb = |(r, g, b)| Color::Rgb(r, g, b);
        Self {
            foreground: rgb(fg),
            background: rgb(bg),
            border: rgb(border),
            focused_border: rgb(cyan),
            selection_foreground: rgb(fg),
            selection_background: rgb(selected),
            disabled: rgb(fg3),
            info: rgb(cyan),
            success: rgb(green),
            warning: rgb(yellow),
            error: rgb(red),
            progress: rgb(green),
            accent: rgb(magenta),
            syntax_keyword: rgb(cyan),
            syntax_name: rgb(yellow),
            syntax_operator: rgb(magenta),
            syntax_value: rgb(green),
            syntax_comment: rgb(fg2),
            attribute_only: false,
        }
    }

    fn monochrome() -> Self {
        Self {
            foreground: Color::Reset,
            background: Color::Reset,
            border: Color::Reset,
            focused_border: Color::Reset,
            selection_foreground: Color::Reset,
            selection_background: Color::Reset,
            disabled: Color::Reset,
            info: Color::Reset,
            success: Color::Reset,
            warning: Color::Reset,
            error: Color::Reset,
            progress: Color::Reset,
            accent: Color::Reset,
            syntax_keyword: Color::Reset,
            syntax_name: Color::Reset,
            syntax_operator: Color::Reset,
            syntax_value: Color::Reset,
            syntax_comment: Color::Reset,
            attribute_only: true,
        }
    }

    fn base(self) -> Style {
        Style::default().fg(self.foreground).bg(self.background)
    }

    fn focus(self) -> Style {
        let style = Style::default().fg(self.focused_border);
        if self.attribute_only {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    }

    fn selected(self) -> Style {
        if self.attribute_only {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
                .fg(self.selection_foreground)
                .bg(self.selection_background)
        }
    }

    fn role(self, color: Color, modifier: Modifier) -> Style {
        let style = Style::default().fg(color);
        if self.attribute_only {
            style.add_modifier(modifier)
        } else {
            style
        }
    }
}

fn selected_style(app: &App, active: bool) -> Style {
    if active {
        ThemePalette::for_app(app).selected()
    } else {
        Style::default()
    }
}
fn selected_log_style(app: &App, severity: Severity) -> Style {
    let palette = ThemePalette::for_app(app);
    let style = severity_style(app, severity);
    if palette.attribute_only {
        style.add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        style
            .bg(palette.selection_background)
            .add_modifier(Modifier::BOLD)
    }
}

fn severity_style(app: &App, severity: Severity) -> Style {
    let palette = ThemePalette::for_app(app);
    match severity {
        Severity::Trace => palette.role(palette.disabled, Modifier::DIM),
        Severity::Info => palette.role(palette.info, Modifier::ITALIC),
        Severity::Warning => palette.role(palette.warning, Modifier::BOLD),
        Severity::Error => palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED),
    }
}

fn build_status_style(app: &App) -> Style {
    let palette = ThemePalette::for_app(app);
    match app.build.status {
        yoctui_model::BuildStatus::Completed => palette.role(palette.success, Modifier::BOLD),
        yoctui_model::BuildStatus::Cancelled => palette.role(palette.warning, Modifier::BOLD),
        yoctui_model::BuildStatus::Failed => {
            palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
        }
        yoctui_model::BuildStatus::LoadingWorkspace
        | yoctui_model::BuildStatus::Parsing
        | yoctui_model::BuildStatus::Running
        | yoctui_model::BuildStatus::Cancelling => palette.role(palette.progress, Modifier::BOLD),
        yoctui_model::BuildStatus::Idle => palette.role(palette.disabled, Modifier::DIM),
    }
}

fn clear_popup(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);
    let palette = ThemePalette::for_app(app);
    frame.render_widget(
        Block::default()
            .style(palette.base())
            .border_style(palette.focus()),
        area,
    );
}

fn active_yocto(app: &App) -> String {
    let release = app
        .workspace
        .release
        .as_deref()
        .unwrap_or("unknown release");
    let location = app
        .workspace
        .source_dir
        .as_deref()
        .or(app.workspace.build_dir.as_deref())
        .map_or_else(
            || "workspace unavailable".into(),
            |path| path.display().to_string(),
        );
    format!("{release} @ {location}")
}

fn source_preview(content: &str, file_name: &str, app: &App) -> Text<'static> {
    let bitbake_source = ["bb", "bbappend", "inc", "conf", "wks", "wks.in"]
        .iter()
        .any(|extension| file_name.ends_with(&format!(".{extension}")));
    let markdown = file_name.ends_with(".md") || file_name.ends_with(".markdown");
    let palette = ThemePalette::for_app(app);
    if palette.attribute_only || (!bitbake_source && !markdown) {
        return Text::from(content.to_owned());
    }
    Text::from(
        content
            .lines()
            .map(|line| {
                if markdown {
                    let style = if line.starts_with('#') {
                        Style::default().fg(palette.syntax_keyword)
                    } else if line.starts_with("```") {
                        Style::default().fg(palette.syntax_operator)
                    } else {
                        Style::default()
                    };
                    return Line::from(Span::styled(line.to_owned(), style));
                }
                let (code, comment) = line
                    .split_once('#')
                    .map_or((line, None), |(code, comment)| (code, Some(comment)));
                let mut spans = Vec::new();
                let trimmed = code.trim_start();
                let indent_len = code.len().saturating_sub(trimmed.len());
                if indent_len > 0 {
                    spans.push(Span::raw(code[..indent_len].to_owned()));
                }
                if [
                    "inherit",
                    "require",
                    "include",
                    "export",
                    "addtask",
                    "deltask",
                    "part",
                    "partition",
                ]
                .iter()
                .any(|keyword| trimmed.starts_with(keyword))
                {
                    let keyword_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
                    spans.push(Span::styled(
                        trimmed[..keyword_end].to_owned(),
                        Style::default().fg(palette.syntax_keyword),
                    ));
                    spans.push(Span::raw(trimmed[keyword_end..].to_owned()));
                } else if let Some(equals) = trimmed.find('=') {
                    let lhs_end = trimmed[..equals]
                        .trim_end_matches([' ', '?', '+', ':'])
                        .len();
                    spans.push(Span::styled(
                        trimmed[..lhs_end].to_owned(),
                        Style::default().fg(palette.syntax_name),
                    ));
                    spans.push(Span::styled(
                        trimmed[lhs_end..=equals].to_owned(),
                        Style::default().fg(palette.syntax_operator),
                    ));
                    spans.push(Span::styled(
                        trimmed[equals + 1..].to_owned(),
                        Style::default().fg(palette.syntax_value),
                    ));
                } else {
                    spans.push(Span::raw(trimmed.to_owned()));
                }
                if let Some(comment) = comment {
                    spans.push(Span::styled(
                        format!("#{comment}"),
                        Style::default().fg(palette.syntax_comment),
                    ));
                }
                Line::from(spans)
            })
            .collect::<Vec<_>>(),
    )
}

fn numbered_source_preview(content: &str, file_name: &str, app: &App) -> Text<'static> {
    let palette = ThemePalette::for_app(app);
    let mut source = source_preview(content, file_name, app);
    for (index, line) in source.lines.iter_mut().enumerate() {
        line.spans.insert(
            0,
            Span::styled(
                format!("{:>4} │ ", index + 1),
                palette.role(palette.disabled, Modifier::DIM),
            ),
        );
    }
    source
}

fn task_activity(app: &App, task_progress: Option<u8>) -> &'static str {
    if task_progress.is_some() {
        return "";
    }
    if app.reduced_motion {
        return " active";
    }
    const FAST: [&str; 8] = [
        "▸▸▸▸▸▸▸▸",
        "▹▸▸▸▸▸▸▸",
        "▹▹▸▸▸▸▸▸",
        "▹▹▹▸▸▸▸▸",
        "▹▹▹▹▸▸▸▸",
        "▹▹▹▹▹▸▸▸",
        "▹▹▹▹▹▹▸▸",
        "▹▹▹▹▹▹▹▸",
    ];
    FAST[(app.animation_frame as usize
        / if app.animation_speed == yoctui_model::AnimationSpeed::Slow {
            3
        } else {
            1
        })
        % FAST.len()]
}

fn footer_shortcuts(app: &App) -> &'static str {
    if app.screen == Screen::Signatures {
        return "↑/↓ select | 1/2 sides | c compare | r refresh | e provider | Esc back/cancel";
    }
    if app.focus == FocusTarget::Navigator {
        return "j/k or ↑/↓ select | Enter open | Tab workspace | Shift+Tab inspector | q quit";
    }
    if app.focus == FocusTarget::Inspector {
        return "Tab navigator | Shift+Tab workspace | ↑/↓ scroll inspector | / search | q quit";
    }
    if app.layer_browser.is_some() {
        return "↑/↓ select | →/l expand | ←/h collapse | Enter open/toggle | e editor | r refresh | . hidden | / search | g Git | m metadata | d deps";
    }
    match app.screen {
        Screen::Dashboard => {
            "F5 build | Ctrl+P commands | Tab focus | ↑/↓ package progress | i image | ! shell | c cancel | r recipes | y layers | ? help | q quit"
        }
        Screen::Tasks => {
            "↑/↓ select | f state | F field | / edit filter | d duration | c cancel | Tab focus"
        }
        Screen::BuildHistory => "↑/↓ select | Esc dashboard | ? help | q quit",
        Screen::Dependencies => {
            "↑/↓ or j/k select | Enter recipe | o provider | L task log | r refresh | Tab focus | Esc dashboard"
        }
        Screen::Signatures => {
            "↑/↓ select | 1/2 sides | c compare | r refresh | e provider | Esc back/cancel"
        }
        Screen::LayerRelationships => "Esc dashboard | y layers | ? help | q quit",
        Screen::Recipes => {
            "↑/↓ select | Enter inspect | z task/Z signatures | e provider | o logs | p patches | b/f tasks | V CVE | X SPDX | d modify | u update | F finish | P deploy | D reset | / search"
        }
        Screen::Packages => {
            "↑/↓ select | Enter detail | / search | R refresh | D dep kind | [/] dep | d follow | u back | o recipe | e provider | c cancel"
        }
        Screen::Images => {
            "↑/↓ select | Q QEMU | W create Wic | D write device | x cancel | [/] output | O open output | / search | R refresh | c scan | b build | i image | o artifact | m manifest | l license | s SPDX | w Wic"
        }
        Screen::Sdk => {
            "↑/↓ select | i image | s standard | E extensible | t testsdk | T testsdkext | R refresh | P publish | n native | o open | c cancel"
        }
        Screen::Testing => {
            "Tab view | ↑/↓ select | Enter open | r run | i image | / search | I import | R refresh | c compare | J JUnit | o result | l log | x cancel"
        }
        Screen::Security => {
            "Tab view | ↑/↓ select | s scope | i image | / search | f status | V CVE check | M map | X SBOM | I import | R refresh | Enter details | o report | e recipe | v advisory | c cancel"
        }
        Screen::Qa => {
            "Tab view | ↑/↓ select | s scope | / search | f status | r run | I import | R refresh | Enter details | o report | e provider | l source | c cancel"
        }
        Screen::Layers => {
            "↑/↓ select | Enter browse | i image | R relationships | e in-TUI edit | o external editor | / search | Esc dashboard | ? help | q quit"
        }
        Screen::Configuration => {
            "↑/↓ select | Enter inspect | s scope | c compare | C copy effective | U copy unexpanded | o source | E edit | / search | x BBMASK | Esc dashboard | ? help | q quit"
        }
        Screen::Bbmask => {
            "e edit BBMASK | Enter preview/confirm | Esc cancel/dashboard | v configuration | ? help | q quit"
        }
        Screen::Maintenance => match app.maintenance.view {
            MaintenanceView::Sstate => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | S signatures | c check | d cleanup"
            }
            MaintenanceView::Services => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | S signatures | e PR export | m PR import"
            }
            MaintenanceView::Release => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | S signatures | l locked cache | h compare | a archive"
            }
            MaintenanceView::Integrations => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | S signatures | detection/inspection only"
            }
        },
        Screen::Logs => {
            "↑/↓ select | ←/→ horizontal | f follow | w wrap | s severity | R/T/B filters | / search | n/N match | o source | C copy"
        }
        Screen::Errors => {
            "↑/↓ select | Enter logs | o open source | Esc dashboard | ? help | q quit"
        }
        Screen::Help => "Esc dashboard | q quit",
        Screen::Settings => {
            "↑/↓ select | ←/→ change | r retry save | Ctrl+P commands | Tab focus | q quit"
        }
        Screen::BuildEnvironment => {
            "↑/↓ select | e edit profile | i initialize | V verify | Tab focus | q quit"
        }
    }
}

fn responsive_footer_shortcuts(app: &App, width: u16) -> &'static str {
    if app.screen == Screen::Images && width <= 90 {
        "↑↓ R refresh Q QEMU W create Wic D write x cancel [/] output O open output o artifact w Wic"
    } else if app.screen == Screen::Sdk && width <= 90 {
        "↑↓ i:image s/E:SDK t/T:test R:scan P:publish n:native o:open c:cancel"
    } else if app.screen == Screen::Testing && width <= 90 {
        "Tab:view ↑↓ Enter r:run i:image /:find I/R:results c:compare J:JUnit o/l:open x:cancel"
    } else if app.screen == Screen::Security && width <= 90 {
        "Tab:view ↑↓ s:scope i:image /:find f:status V:check M:map X:SBOM I/R:data Enter o/e/v:open c:cancel"
    } else if app.screen == Screen::Qa && width <= 90 {
        "Tab:view ↑↓ s:scope /:find f:status r:run I/R:data Enter o/e/l:open c:cancel"
    } else {
        footer_shortcuts(app)
    }
}

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let palette = ThemePalette::for_app(app);
    frame.render_widget(Block::default().style(palette.base()), area);
    if area.width < 80 || area.height < 24 {
        frame.render_widget(
            Paragraph::new(format!(
                "Yoctui needs at least 80x24.\nCurrent terminal: {}x{}.\nResize the terminal or press Q to quit.",
                area.width, area.height
            ))
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }
    let chunks = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    let elapsed = app
        .elapsed()
        .map(format_duration)
        .unwrap_or_else(|| "--:--:--".into());
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .map_or("unknown", String::as_str);
    let distro = app
        .workspace
        .variables
        .get("DISTRO")
        .map_or("unknown", String::as_str);
    let disk = app.host_telemetry.disk_available_bytes.map_or_else(
        || "Disk --".into(),
        |bytes| format!("Disk {}", format_bytes(bytes)),
    );
    frame.render_widget(
        Paragraph::new(format!(
            " Yoctui | {:?} | Yocto: {} | Target {} | MACHINE {} | DISTRO {}\n Status {:?} | Tasks {}/{} | Active {} | W {} | E {} | {} | CPU {} | {}",
            app.backend,
            active_yocto(app),
            app.build.target.as_deref().unwrap_or("not selected"),
            machine,
            distro,
            app.build.status,
            app.build.completed,
            app.build.total.map_or_else(|| "?".into(), |total| total.to_string()),
            app.tasks.len(),
            app.build.warnings,
            app.build.errors,
            elapsed,
            app.host_telemetry.cpu_utilization_percent.map_or_else(|| "CPU --".into(), |cpu| format!("CPU {cpu}%")),
            disk,
        ))
        .block(
            Block::default()
                .style(palette.base())
                .borders(Borders::ALL)
                .border_style(build_status_style(app)),
        ),
        chunks[0],
    );
    responsive_shell(frame, app, chunks[1], area.width);
    frame.render_widget(
        Paragraph::new(responsive_footer_shortcuts(app, area.width)).style(palette.focus()),
        chunks[2],
    );
    if app.command_palette_open {
        command_palette(frame, app, area);
    } else if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() {
        recipe_editor(frame, app, editor, area);
    } else if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog() {
        qemu_launch_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::QemuLaunchConfirmation(preview)) = app.active_dialog() {
        qemu_launch_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::QemuCancellationConfirmation(id)) = app.active_dialog() {
        qemu_cancellation_confirmation(frame, app, *id, area);
    } else if let Some(Dialog::WicCreateTomlEditor {
        content,
        editing,
        validation_error,
    }) = app.active_dialog()
    {
        let popup = Rect::new(
            area.width / 8,
            area.height / 6,
            area.width * 3 / 4,
            area.height * 2 / 3,
        );
        clear_popup(frame, app, popup);
        let error = validation_error
            .as_deref()
            .map_or(String::new(), |error| format!("\nValidation: {error}"));
        frame.render_widget(
            Paragraph::new(format!(
                "{}{}\n{}: i inserts, Enter previews, q closes.\nInsert: type, Backspace, Esc normal.{}",
                content,
                if *editing { "_" } else { "" },
                if *editing { "INSERT" } else { "NORMAL" },
                error,
            ))
            .block(Block::default().title(format!("Wic create.toml — {}", if *editing { "INSERT" } else { "NORMAL" })).borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::WicCreate(dialog)) = app.active_dialog() {
        wic_create_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::WicCreateConfirmation(preview)) = app.active_dialog() {
        wic_create_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::WicDevicePicker(dialog)) = app.active_dialog() {
        wic_device_picker(frame, app, dialog, area);
    } else if let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog() {
        wic_write_phrase_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::WicWriteConfirmation(preview)) = app.active_dialog() {
        wic_write_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::WicCancellationConfirmation {
        id,
        incomplete_device_warning,
    }) = app.active_dialog()
    {
        wic_cancellation_confirmation(frame, app, *id, *incomplete_device_warning, area);
    } else if let Some(Dialog::SdkBuildConfirmation(preview)) = app.active_dialog() {
        sdk_build_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::SdkPublishTomlEditor { content, editing }) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 6,
            area.width * 3 / 4,
            area.height * 2 / 3,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(Paragraph::new(format!("{}{}\n{}: i inserts, Enter previews, q closes.\nInsert: type, Backspace, Esc normal.", content, if *editing { "_" } else { "" }, if *editing { "INSERT" } else { "NORMAL" }))
            .block(Block::default().title(format!("SDK publish.toml — {}", if *editing { "INSERT" } else { "NORMAL" })).borders(Borders::ALL)).wrap(Wrap { trim: false }), popup);
    } else if let Some(Dialog::SdkPublish(draft)) = app.active_dialog() {
        sdk_publish_dialog(frame, app, draft, area);
    } else if let Some(Dialog::SdkPublishConfirmation(preview)) = app.active_dialog() {
        sdk_publish_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::SdkNativeTomlEditor { content, editing }) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 6,
            area.width * 3 / 4,
            area.height * 2 / 3,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(Paragraph::new(format!("{}{}\n{}: i inserts, Enter previews, q closes.\nInsert: type, Backspace, Esc normal.", content, if *editing { "_" } else { "" }, if *editing { "INSERT" } else { "NORMAL" }))
            .block(Block::default().title(format!("SDK native.toml — {}", if *editing { "INSERT" } else { "NORMAL" })).borders(Borders::ALL)).wrap(Wrap { trim: false }), popup);
    } else if let Some(Dialog::SdkNative(draft)) = app.active_dialog() {
        sdk_native_dialog(frame, app, draft, area);
    } else if let Some(Dialog::SdkNativeConfirmation(preview)) = app.active_dialog() {
        sdk_native_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::SdkCancellationConfirmation(id)) = app.active_dialog() {
        sdk_cancellation_confirmation(frame, app, *id, area);
    } else if let Some(Dialog::TestLaunchTomlEditor { content, editing }) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 6,
            area.width * 3 / 4,
            area.height * 2 / 3,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(Paragraph::new(format!("{}{}\n{}: i inserts, Enter previews, q closes.\nInsert: type, Backspace, Esc normal.", content, if *editing { "_" } else { "" }, if *editing { "INSERT" } else { "NORMAL" })).block(Block::default().title(format!("Test launch.toml — {}", if *editing { "INSERT" } else { "NORMAL" })).borders(Borders::ALL)).wrap(Wrap { trim: false }), popup);
    } else if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog() {
        test_launch_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::TestLaunchConfirmation(preview)) = app.active_dialog() {
        test_launch_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::TestCancellationConfirmation(id)) = app.active_dialog() {
        test_cancellation_confirmation(frame, app, *id, area);
    } else if let Some(Dialog::TestResultImportTomlEditor {
        content,
        editing,
        validation_error,
    }) = app.active_dialog()
    {
        let popup = Rect::new(
            area.width / 8,
            area.height / 6,
            area.width * 3 / 4,
            area.height * 2 / 3,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(Paragraph::new(format!("{}{}\n{}: i inserts, Enter imports, q closes.\nInsert: type, Backspace, Esc normal.{}", content, if *editing { "_" } else { "" }, if *editing { "INSERT" } else { "NORMAL" }, validation_error.as_deref().map_or(String::new(), |error| format!("\nValidation: {error}")))).block(Block::default().title(format!("Test result import.toml — {}", if *editing { "INSERT" } else { "NORMAL" })).borders(Borders::ALL)).wrap(Wrap { trim: false }), popup);
    } else if let Some(Dialog::TestResultImport(dialog)) = app.active_dialog() {
        test_result_import_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::TestComparisonTomlEditor {
        content,
        editing,
        validation_error,
    }) = app.active_dialog()
    {
        let popup = Rect::new(
            area.width / 8,
            area.height / 6,
            area.width * 3 / 4,
            area.height * 2 / 3,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(Paragraph::new(format!("{}{}\n{}: i inserts, Enter previews, q closes.\nInsert: type, Backspace, Esc normal.{}", content, if *editing { "_" } else { "" }, if *editing { "INSERT" } else { "NORMAL" }, validation_error.as_deref().map_or(String::new(), |error| format!("\nValidation: {error}")))).block(Block::default().title(format!("Test comparison.toml — {}", if *editing { "INSERT" } else { "NORMAL" })).borders(Borders::ALL)).wrap(Wrap { trim: false }), popup);
    } else if let Some(Dialog::TestComparison(picker)) = app.active_dialog() {
        test_comparison_dialog(frame, app, picker, area);
    } else if let Some(Dialog::TestComparisonConfirmation(preview)) = app.active_dialog() {
        test_comparison_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog() {
        test_junit_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::TestJunitExportConfirmation(preview)) = app.active_dialog() {
        test_junit_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::Security(dialog)) = app.active_dialog() {
        security_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::Qa(dialog)) = app.active_dialog() {
        qa_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::Maintenance(dialog)) = app.active_dialog() {
        maintenance_dialog(frame, app, dialog, area);
    } else if matches!(app.active_dialog(), Some(Dialog::BuildCompletion)) {
        build_completion_popup(frame, app, area);
    } else if matches!(app.active_dialog(), Some(Dialog::QuitConfirmation)) {
        let popup = Rect::new(area.width / 4, area.height / 3, area.width / 2, 3);
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new("Build is active. Press Y to quit UI, or Esc to continue.")
                .block(Block::default().title("Confirm quit").borders(Borders::ALL)),
            popup,
        )
    } else if let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 4,
            area.height / 4,
            area.width / 2,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Table::new(
                picker.tasks.iter().enumerate().map(|(index, task)| {
                    Row::new([task.as_str()]).style(selected_style(app, index == picker.selection))
                }),
                [Constraint::Min(1)],
            )
            .header(Row::new(["Authoritative signature tasks"]).style(Style::default().bold()))
            .block(
                Block::default()
                    .title(format!("Inspect signatures: {}", picker.recipe.name))
                    .borders(Borders::ALL),
            ),
            popup,
        );
    } else if let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 4,
            area.height / 4,
            area.width / 2,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Table::new(
                picker.tasks.iter().enumerate().map(|(index, task)| {
                    Row::new([task.as_str()]).style(selected_style(app, index == picker.selection))
                }),
                [Constraint::Min(1)],
            )
            .header(Row::new(["Authoritative BitBake tasks"]).style(Style::default().bold()))
            .block(
                Block::default()
                    .title(format!(
                        "{} task: {}",
                        if picker.force { "Force" } else { "Run" },
                        picker.recipe
                    ))
                    .borders(Borders::ALL),
            ),
            popup,
        );
    } else if let Some(Dialog::RecipeTaskLogPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 5,
            area.height / 4,
            area.width * 3 / 5,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Table::new(
                picker.logs.iter().enumerate().map(|(index, log)| {
                    Row::new([
                        Cell::from(log.task.as_str()),
                        Cell::from(format!("{:?}", log.state)),
                        Cell::from(log.path.display().to_string()),
                    ])
                    .style(selected_style(app, index == picker.selection))
                }),
                [
                    Constraint::Length(20),
                    Constraint::Length(12),
                    Constraint::Min(20),
                ],
            )
            .header(
                Row::new(["Task", "State", "Authoritative log path"])
                    .style(Style::default().bold()),
            )
            .block(
                Block::default()
                    .title(format!("{} retained task logs", picker.recipe))
                    .borders(Borders::ALL),
            ),
            popup,
        );
    } else if let Some(Dialog::ConfigEdit {
        identity: _,
        content,
        editing,
    }) = app.active_dialog()
    {
        let popup = Rect::new(
            area.width / 8,
            area.height / 6,
            area.width * 3 / 4,
            area.height * 2 / 3,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "{}{}\n{}: i inserts, Enter previews, q closes.\nInsert: type, Backspace, Esc normal.",
                content,
                if *editing { "_" } else { "" },
                if *editing { "INSERT" } else { "NORMAL" }
            ))
            .block(
                Block::default()
                    .title(format!("Configuration.toml — {}", if *editing { "INSERT" } else { "NORMAL" }))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::ConfigEditConfirmation(request)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 4,
            area.width * 3 / 4,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Destination:\n{}\n\nExact assignment:\n{}\n\nEnter confirms; Esc cancels.",
                request.destination.display(),
                request.assignment
            ))
            .block(
                Block::default()
                    .title("Preview configuration edit")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::ConfigComparison(comparison)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 5,
            area.width * 3 / 4,
            area.height * 3 / 5,
        );
        clear_popup(frame, app, popup);
        let field = |name: &str, value: &yoctui_model::ConfigComparisonField| {
            format!(
                "{name}: {:?}\n  global: {}\n  {}: {}",
                value.outcome,
                value.global.as_deref().unwrap_or("unavailable"),
                comparison.recipe,
                value.recipe.as_deref().unwrap_or("unavailable")
            )
        };
        frame.render_widget(
            Paragraph::new(format!(
                "Variable: {}\nGlobal vs recipe {}\n\n{}\n\n{}\n\nEnter or Esc closes.",
                comparison.variable,
                comparison.recipe,
                field("Effective", &comparison.effective),
                field("Unexpanded", &comparison.unexpanded),
            ))
            .block(
                Block::default()
                    .title("Configuration comparison")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::ConfigScopePicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 5,
            area.height / 4,
            area.width * 3 / 5,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Table::new(
                picker.scopes.iter().enumerate().map(|(index, scope)| {
                    Row::new([scope.as_deref().unwrap_or("(global)")])
                        .style(selected_style(app, index == picker.selection))
                }),
                [Constraint::Min(20)],
            )
            .header(Row::new(["Variable scope"]).style(Style::default().bold()))
            .block(
                Block::default()
                    .title(format!("{} scope", picker.variable))
                    .borders(Borders::ALL),
            ),
            popup,
        );
    } else if let Some(Dialog::ConfigSourcePicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 4,
            area.width * 3 / 4,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Table::new(
                picker.sources.iter().enumerate().map(|(index, source)| {
                    Row::new([
                        Cell::from(source.operation.as_str()),
                        Cell::from(source.path.display().to_string()),
                        Cell::from(
                            source
                                .line
                                .map_or_else(|| "—".into(), |line| line.to_string()),
                        ),
                    ])
                    .style(selected_style(app, index == picker.selection))
                }),
                [
                    Constraint::Length(12),
                    Constraint::Min(20),
                    Constraint::Length(7),
                ],
            )
            .header(
                Row::new(["Operation", "Authoritative source", "Line"])
                    .style(Style::default().bold()),
            )
            .block(
                Block::default()
                    .title(format!("{} defining sources", picker.identity.name))
                    .borders(Borders::ALL),
            ),
            popup,
        );
    } else if let Some(Dialog::RecipePatchPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 5,
            area.height / 4,
            area.width * 3 / 5,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Table::new(
                picker.patches.iter().enumerate().map(|(index, patch)| {
                    Row::new([patch.display().to_string()])
                        .style(selected_style(app, index == picker.selection))
                }),
                [Constraint::Min(20)],
            )
            .header(Row::new(["Authoritative local patch"]).style(Style::default().bold()))
            .block(
                Block::default()
                    .title(format!("{} patch review", picker.recipe))
                    .borders(Borders::ALL),
            ),
            popup,
        );
    } else if let Some(Dialog::RecipeTaskConfirmation(request)) = app.active_dialog() {
        let popup = Rect::new(area.width / 4, area.height / 3, area.width / 2, 5);
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `bitbake {}{} {}`?\n\nPress Enter to continue or Esc to cancel.",
                if request.force { "-f " } else { "" },
                request.targets.join(" "),
                request
                    .task
                    .as_deref()
                    .map_or(String::new(), |task| format!("-c {task}"))
            ))
            .block(
                Block::default()
                    .title("Confirm recipe task")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(Dialog::DevtoolModifyConfirmation(identity)) = app.active_dialog() {
        let width = area.width.saturating_sub(8).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(7) / 2,
            width,
            7,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool modify {}`?\n\nProvider: {}\n\nEnter continues; Esc cancels.",
                identity.name,
                identity.file.display()
            ))
            .block(
                Block::default()
                    .title("Confirm Devtool modify")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(Dialog::DevtoolResetConfirmation(plan)) = app.active_dialog() {
        let width = area.width.saturating_sub(8).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(9) / 2,
            width,
            9,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool reset {}`?\n\nProvider: {}\nWorkspace source to remove: {}\n\nThis removes the Devtool workspace. Enter continues; Esc cancels.",
                plan.identity.name,
                plan.identity.file.display(),
                plan.source_path.display()
            ))
            .block(
                Block::default()
                    .title("Confirm Devtool reset")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(Dialog::DevtoolUpdateConfirmation(identity)) = app.active_dialog() {
        let width = area.width.saturating_sub(8).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(7) / 2,
            width,
            7,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool update-recipe {}`?\n\nProvider: {}\n\nEnter continues; Esc cancels.",
                identity.name,
                identity.file.display()
            ))
            .block(
                Block::default()
                    .title("Confirm Devtool update-recipe")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(Dialog::DevtoolFinishConfirmation(plan)) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(9) / 2,
            width,
            9,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool finish {} {}`?\n\nProvider: {}\nConfigured layer: {}\nDestination: {}\n\nEnter continues; Esc cancels.",
                plan.identity.name,
                plan.layer.path.display(),
                plan.identity.file.display(),
                plan.layer.name,
                plan.layer.path.display()
            ))
            .block(
                Block::default()
                    .title("Confirm Devtool finish")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(Dialog::DevtoolDeployConfirmation(plan)) = app.active_dialog() {
        let width = area.width.saturating_sub(8).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(8) / 2,
            width,
            8,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool deploy-target {} {}`?\n\nProvider: {}\nTarget: {}\n\nEnter continues; Esc cancels.",
                plan.identity.name,
                plan.target,
                plan.identity.file.display(),
                plan.target
            ))
            .block(
                Block::default()
                    .title("Confirm Devtool deploy-target")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(Dialog::DevtoolDeploy(draft)) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(8) / 2,
            width,
            8,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Recipe: {}\nProvider: {}\nDeployment target: {}_\n\nEnter previews the command; Esc cancels.",
                draft.identity.name,
                draft.identity.file.display(),
                draft.target
            ))
            .block(
                Block::default()
                    .title("Devtool deploy target")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::DevtoolFinishPicker(picker)) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(44, 100);
        let height = (picker.layers.len() as u16)
            .saturating_add(5)
            .min(area.height.saturating_sub(4))
            .max(7);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(height) / 2,
            width,
            height,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Table::new(
                picker.layers.iter().enumerate().map(|(index, layer)| {
                    Row::new([layer.name.clone(), layer.path.display().to_string()])
                        .style(selected_style(app, index == picker.selection))
                }),
                [Constraint::Length(24), Constraint::Min(20)],
            )
            .header(
                Row::new(["Configured layer", "Absolute destination"])
                    .style(Style::default().bold()),
            )
            .block(
                Block::default()
                    .title(format!(
                        "Devtool finish {} — ↑/↓ select, Enter preview, Esc cancel",
                        picker.identity.name
                    ))
                    .borders(Borders::ALL),
            ),
            popup,
        );
    } else if let Some(Dialog::BbmaskConfirmation(value)) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(40, 96);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(7) / 2,
            width,
            7,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Append this exact assignment to $BUILDDIR/conf/local.conf:\n\n{}\n\nEnter writes and refreshes configuration; Esc cancels.",
                bbmask_assignment(value)
            ))
            .block(Block::default().title("Confirm BBMASK change").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::BbmaskEdit { content, editing }) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 6,
            area.width * 3 / 4,
            area.height * 2 / 3,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "{}{}\n{}: i inserts, Enter previews, q closes.\nInsert: type, Backspace, Esc normal.",
                content,
                if *editing { "_" } else { "" },
                if *editing { "INSERT" } else { "NORMAL" }
            ))
            .block(
                Block::default()
                    .title(format!("BBMASK.toml — {}", if *editing { "INSERT" } else { "NORMAL" }))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::ImagePicker(picker)) = app.active_dialog() {
        let width = area.width.saturating_sub(24).clamp(42, 90);
        let height = area.height.saturating_sub(8).clamp(10, 24);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            (area.height.saturating_sub(height)) / 2,
            width,
            height,
        );
        let machine = app
            .workspace
            .variables
            .get("MACHINE")
            .map_or("unknown", String::as_str);
        let images = picker
            .images
            .iter()
            .enumerate()
            .map(|(index, image)| {
                format!(
                    "{} {}",
                    if index == picker.selection { ">" } else { " " },
                    image
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Active MACHINE: {machine}\n\n{images}\n\nUp/Down select  Enter choose image  Esc cancel"
            ))
            .block(Block::default().title("Available image targets").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::BuildEnvironmentCloneEditor { content, editing }) =
        app.active_dialog()
    {
        build_environment_clone_editor(frame, app, content, *editing, area);
    } else if let Some(Dialog::BuildEnvironmentCloneReview(plan)) = app.active_dialog() {
        build_environment_clone_review(frame, app, plan, area);
    } else if let Some(Dialog::BuildEnvironmentEditor { content, editing }) = app.active_dialog() {
        build_environment_editor(frame, app, content, *editing, area);
    } else if let Some(Dialog::ThemePicker { selection }) = app.active_dialog() {
        theme_picker(frame, app, *selection, area);
    } else if matches!(app.active_dialog(), Some(Dialog::BuildOptions)) {
        let machine = app
            .workspace
            .variables
            .get("MACHINE")
            .map_or("unknown", String::as_str);
        let width = area.width.saturating_sub(12).clamp(38, 84);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(10) / 2,
            width,
            10,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Machine: {machine}\nCurrent image target: {}\n\nb  Build image\nc  Clean image\nm  Run menuconfig\ne  Enter a different image target\n\nEsc closes this menu.",
                app.build.target.as_deref().unwrap_or("not selected")
            ))
            .block(Block::default().title("Image build options").borders(Borders::ALL)),
            popup,
        );
    } else if let Some(Dialog::BuildTarget {
        content,
        task,
        editing,
    }) = app.active_dialog()
    {
        let width = area.width * 3 / 4;
        let popup = Rect::new(area.width / 8, area.height / 6, width, area.height * 2 / 3);
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "{}{}\ntask = \"{}\"\n\n{}: i inserts, Enter previews, q closes.\nInsert: type, Backspace, Esc normal.",
                content,
                if *editing { "_" } else { "" },
                task.as_deref().unwrap_or("default"),
                if *editing { "INSERT" } else { "NORMAL" }
            ))
            .block(Block::default().title(format!("Build target.toml — {}", if *editing { "INSERT" } else { "NORMAL" })).borders(Borders::ALL)),
            popup,
        );
    } else if let Some(notification) = app.notification.as_deref() {
        let palette = ThemePalette::for_app(app);
        let width = area.width.saturating_sub(8).clamp(24, 80);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(5) / 2,
            width,
            5,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!("{notification}\n\nPress Enter to dismiss."))
                .style(palette.role(palette.info, Modifier::BOLD))
                .block(
                    Block::default()
                        .title("Notice")
                        .borders(Borders::ALL)
                        .border_style(palette.role(palette.accent, Modifier::BOLD)),
                )
                .wrap(Wrap { trim: true }),
            popup,
        );
    }
}

fn theme_picker(frame: &mut Frame, app: &App, selection: usize, area: Rect) {
    let popup = Rect::new(
        area.width / 4,
        area.height / 5,
        area.width / 2,
        area.height * 3 / 5,
    );
    clear_popup(frame, app, popup);
    let rows = yoctui_model::THEMES
        .iter()
        .enumerate()
        .map(|(index, theme)| {
            Row::new([format!("{:?}", theme)]).style(selected_style(app, index == selection))
        });
    frame.render_widget(
        Table::new(rows, [Constraint::Min(1)])
            .block(
                Block::default()
                    .title("Theme — applies immediately")
                    .borders(Borders::ALL),
            )
            .footer(Row::new(["↑/↓ select  Enter apply  Esc close"])),
        popup,
    );
}

fn build_environment_editor(
    frame: &mut Frame,
    app: &App,
    content: &str,
    editing: bool,
    area: Rect,
) {
    let popup = Rect::new(
        area.width / 8,
        area.height / 8,
        area.width * 3 / 4,
        area.height * 3 / 4,
    );
    clear_popup(frame, app, popup);
    let mode = if editing { "INSERT" } else { "NORMAL" };
    frame.render_widget(
        Paragraph::new(content.to_owned())
            .block(
                Block::default()
                    .title(format!("Build environment.toml — {mode}"))
                    .borders(Borders::ALL),
            )
            .style(ThemePalette::for_app(app).base()),
        popup,
    );
    let footer = Rect::new(
        popup.x,
        popup.y + popup.height.saturating_sub(2),
        popup.width,
        2,
    );
    frame.render_widget(Paragraph::new("Normal: i insert, Enter apply, Esc/q close | Insert: type, Backspace, Enter apply, Esc normal").style(ThemePalette::for_app(app).focus()), footer);
}

fn build_environment_clone_editor(
    frame: &mut Frame,
    app: &App,
    content: &str,
    editing: bool,
    area: Rect,
) {
    let popup = Rect::new(
        area.width / 8,
        area.height / 8,
        area.width * 3 / 4,
        area.height * 3 / 4,
    );
    clear_popup(frame, app, popup);
    let mode = if editing { "INSERT" } else { "NORMAL" };
    frame.render_widget(
        Paragraph::new(content.to_owned())
            .block(
                Block::default()
                    .title(format!("Clone Poky.toml — {mode}"))
                    .borders(Borders::ALL),
            )
            .style(ThemePalette::for_app(app).base()),
        popup,
    );
}

fn build_environment_clone_review(
    frame: &mut Frame,
    app: &App,
    plan: &yoctui_model::BuildEnvironmentClonePlan,
    area: Rect,
) {
    let popup = Rect::new(
        area.width / 8,
        area.height / 4,
        area.width * 3 / 4,
        area.height / 2,
    );
    clear_popup(frame, app, popup);
    let clone = if plan.request.revision.is_some() {
        format!(
            "git clone --no-checkout {} {}",
            plan.request.repository,
            plan.request.destination.display()
        )
    } else {
        format!(
            "git clone {} {}",
            plan.request.repository,
            plan.request.destination.display()
        )
    };
    let checkout = plan
        .request
        .revision
        .as_ref()
        .map_or_else(String::new, |revision| {
            format!(
                "\ngit -C {} checkout {revision}",
                plan.request.destination.display()
            )
        });
    frame.render_widget(
        Paragraph::new(format!("Review before cloning:\n\n{clone}{checkout}\n\nBuild directory: {}\n\nEnter confirms this network operation. Esc cancels.", plan.build_dir.display()))
            .block(Block::default().title("Clone Poky review").borders(Borders::ALL))
            .style(ThemePalette::for_app(app).base())
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn command_palette(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(12).clamp(60, 100);
    let height = area.height.saturating_sub(4).clamp(16, 26);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    let commands = app.filtered_command_palette_commands();
    let visible_count = usize::from(height.saturating_sub(10)).max(1);
    let start = app
        .command_palette_selection
        .saturating_sub(visible_count.saturating_sub(1));
    let palette = ThemePalette::for_app(app);
    let mut lines = vec![
        Line::from(format!("Search: {}_", app.command_palette_query)),
        Line::from(""),
    ];
    if commands.is_empty() {
        lines.push(Line::styled(
            "No commands match this search.",
            palette.role(palette.disabled, Modifier::DIM),
        ));
    } else {
        lines.extend(
            commands
                .iter()
                .enumerate()
                .skip(start)
                .take(visible_count)
                .map(|(index, command)| {
                    let disabled = command.disabled_reason.is_some();
                    let marker = if index == app.command_palette_selection {
                        ">"
                    } else {
                        " "
                    };
                    let suffix = if disabled { " (unavailable)" } else { "" };
                    let style = if index == app.command_palette_selection {
                        selected_style(app, true)
                    } else if disabled {
                        palette.role(palette.disabled, Modifier::DIM)
                    } else {
                        Style::default()
                    };
                    Line::styled(
                        format!("{marker} {}  [{}]{suffix}", command.label, command.shortcut),
                        style,
                    )
                }),
        );
    }
    lines.push(Line::from(""));
    if let Some(command) = commands.get(app.command_palette_selection) {
        lines.push(Line::styled(
            command.description,
            palette.role(palette.info, Modifier::ITALIC),
        ));
        if let Some(reason) = command.disabled_reason {
            lines.push(Line::styled(
                format!("Unavailable: {reason}."),
                palette.role(palette.warning, Modifier::BOLD),
            ));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Type to search  ↑/↓ select  Enter run  Backspace edit  Esc cancel",
    ));

    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title("Command palette")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn responsive_shell(frame: &mut Frame, app: &App, area: Rect, terminal_width: u16) {
    if app.screen == Screen::Signatures {
        signatures_workspace(frame, app, area, terminal_width);
        return;
    }
    if terminal_width >= 130 {
        let panes = Layout::horizontal([
            Constraint::Length(22),
            Constraint::Percentage(43),
            Constraint::Min(28),
        ])
        .split(area);
        navigator(frame, app, panes[0]);
        workspace(frame, app, panes[1]);
        inspector(frame, app, panes[2]);
    } else if terminal_width >= 100 {
        let panes = Layout::horizontal([Constraint::Length(22), Constraint::Min(40)]).split(area);
        navigator(frame, app, panes[0]);
        workspace(frame, app, panes[1]);
        if app.focus == FocusTarget::Inspector {
            frame.render_widget(Clear, panes[1]);
            inspector(frame, app, panes[1]);
        }
    } else {
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
        pane_switcher(frame, app, rows[0]);
        match app.focus {
            FocusTarget::Navigator => navigator(frame, app, rows[1]),
            FocusTarget::Inspector => inspector(frame, app, rows[1]),
            FocusTarget::Workspace | FocusTarget::Dialog | FocusTarget::CommandPalette => {
                workspace(frame, app, rows[1]);
            }
        }
    }
}

fn pane_switcher(frame: &mut Frame, app: &App, area: Rect) {
    let label = |target: FocusTarget, name: &str| {
        if app.focus == target {
            format!("[{name}]")
        } else {
            name.to_owned()
        }
    };
    frame.render_widget(
        Paragraph::new(format!(
            "Panes: {}  {}  {}  Tab/Shift+Tab",
            label(FocusTarget::Navigator, "Navigator"),
            label(FocusTarget::Workspace, "Workspace"),
            label(FocusTarget::Inspector, "Inspector"),
        ))
        .style(ThemePalette::for_app(app).focus()),
        area,
    );
}

fn pane_block<'a>(app: &App, title: &'a str, focused: bool) -> Block<'a> {
    let palette = ThemePalette::for_app(app);
    let style = if focused {
        palette.focus()
    } else {
        Style::default().fg(palette.border)
    };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(palette.base())
        .border_style(style)
}

fn signatures_workspace(frame: &mut Frame, app: &App, area: Rect, terminal_width: u16) {
    if terminal_width >= 110 {
        let panes = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        signature_records(frame, app, panes[0]);
        signature_detail(frame, app, panes[1]);
    } else {
        let panes =
            Layout::vertical([Constraint::Percentage(44), Constraint::Percentage(56)]).split(area);
        signature_records(frame, app, panes[0]);
        signature_detail(frame, app, panes[1]);
    }
}

fn signature_target_label(app: &App) -> String {
    app.signature_dump.target().map_or_else(
        || "no target".into(),
        |target| format!("{}:{}", target.recipe, target.task),
    )
}

fn signature_comparison_sides(
    state: &SignatureComparisonState,
) -> (
    Option<&yoctui_model::SignatureIdentity>,
    Option<&yoctui_model::SignatureIdentity>,
) {
    match state {
        SignatureComparisonState::NotSelected => (None, None),
        SignatureComparisonState::Ready { left, right } => (left.as_ref(), right.as_ref()),
        SignatureComparisonState::Loading { request }
        | SignatureComparisonState::AvailableEmpty { request }
        | SignatureComparisonState::Available { request, .. }
        | SignatureComparisonState::Partial { request, .. }
        | SignatureComparisonState::Failed { request, .. } => {
            (Some(&request.left), Some(&request.right))
        }
    }
}

fn signature_records(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!("Signatures — {}", signature_target_label(app));
    let block = pane_block(app, &title, app.focus == FocusTarget::Workspace);
    let records = app.signature_dump.records();
    let text = match &app.signature_dump {
        SignatureDumpState::NotLoaded => {
            "Signatures have not been loaded.\n\nReturn to Recipes and press Z.".into()
        }
        SignatureDumpState::Loading { .. } => {
            "Loading authoritative signature artifacts…\n\nEsc requests cancellation.".into()
        }
        SignatureDumpState::AvailableEmpty { .. } => {
            "BitBake reported no signature artifacts for this recipe/task.".into()
        }
        SignatureDumpState::Failed { message, .. } => {
            format!("Signature dump failed:\n{message}\n\nr retries; Esc returns to Recipes.")
        }
        SignatureDumpState::Available { .. } | SignatureDumpState::Partial { .. } => {
            let (left, right) = signature_comparison_sides(&app.signature_comparison);
            let mut lines = records
                .unwrap_or_default()
                .iter()
                .map(|record| {
                    let selected = app.signature_selection.as_ref() == Some(&record.identity);
                    let side = match (
                        left == Some(&record.identity),
                        right == Some(&record.identity),
                    ) {
                        (true, true) => "12",
                        (true, false) => "1 ",
                        (false, true) => " 2",
                        (false, false) => "  ",
                    };
                    format!(
                        "{} [{}] {}\n    {}",
                        if selected { ">" } else { " " },
                        side,
                        record
                            .identity
                            .hash
                            .as_deref()
                            .unwrap_or("hash unavailable"),
                        record.identity.path.as_ref().map_or_else(
                            || "path unavailable".into(),
                            |path| path.display().to_string()
                        )
                    )
                })
                .collect::<Vec<_>>();
            if let SignatureDumpState::Partial { limitations, .. } = &app.signature_dump {
                lines.push(String::new());
                lines.push("Partial result:".into());
                lines.extend(limitations.iter().map(|value| format!("! {value}")));
            }
            lines.join("\n")
        }
    };
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

fn signature_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(signature_detail_text(app))
            .block(pane_block(
                app,
                "Selected record and comparison",
                app.focus == FocusTarget::Inspector,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn signature_detail_text(app: &App) -> String {
    let selected = app.signature_dump.records().and_then(|records| {
        app.signature_selection
            .as_ref()
            .and_then(|identity| records.iter().find(|record| &record.identity == identity))
    });
    let mut lines = Vec::new();
    if let Some(record) = selected {
        lines.extend([
            format!(
                "Hash: {}",
                record.identity.hash.as_deref().unwrap_or("unavailable")
            ),
            format!(
                "Base hash: {}",
                record.base_hash.as_deref().unwrap_or("unavailable")
            ),
            format!(
                "Task hash: {}",
                record.task_hash.as_deref().unwrap_or("unavailable")
            ),
            String::new(),
            format!("Variables ({})", record.variables.len()),
        ]);
        lines.extend(record.variables.iter().take(120).map(|value| {
            format!(
                "{} = {}",
                value.name,
                value.value.as_deref().unwrap_or("unavailable")
            )
        }));
        if record.variables.len() > 120 {
            lines.push(format!(
                "… {} more bounded variables",
                record.variables.len() - 120
            ));
        }
        lines.push(String::new());
        lines.push(format!("Task dependencies ({})", record.dependencies.len()));
        lines.extend(
            record
                .dependencies
                .iter()
                .take(80)
                .map(|dependency| format!("• {dependency}")),
        );
        if record.dependencies.len() > 80 {
            lines.push(format!(
                "… {} more bounded dependencies",
                record.dependencies.len() - 80
            ));
        }
    } else {
        lines.push("No current signature record is selected.".into());
    }
    lines.push(String::new());
    lines.push("Comparison".into());
    match &app.signature_comparison {
        SignatureComparisonState::NotSelected => {
            lines.push("Assign two records with 1 and 2.".into());
        }
        SignatureComparisonState::Ready { left, right } => {
            lines.push(format!(
                "1: {}\n2: {}",
                left.as_ref()
                    .and_then(|identity| identity.hash.as_deref())
                    .unwrap_or("not selected"),
                right
                    .as_ref()
                    .and_then(|identity| identity.hash.as_deref())
                    .unwrap_or("not selected")
            ));
        }
        SignatureComparisonState::Loading { .. } => {
            lines.push("Comparing authoritative signature artifacts…".into());
        }
        SignatureComparisonState::AvailableEmpty { .. } => {
            lines.push("No typed differences were found.".into());
        }
        SignatureComparisonState::Failed { message, .. } => {
            lines.push(format!("Comparison failed: {message}"));
        }
        SignatureComparisonState::Available { differences, .. }
        | SignatureComparisonState::Partial { differences, .. } => {
            lines.extend(differences.iter().take(160).map(|difference| {
                let category = match difference.category {
                    SignatureDifferenceCategory::BaseHash => "hash",
                    SignatureDifferenceCategory::ChangedValue => "value",
                    SignatureDifferenceCategory::Dependency => "dependency",
                    SignatureDifferenceCategory::Unavailable => "unavailable",
                };
                format!(
                    "[{category}] {}: {} → {}",
                    difference.key,
                    difference.left.as_deref().unwrap_or("unavailable"),
                    difference.right.as_deref().unwrap_or("unavailable")
                )
            }));
            if differences.len() > 160 {
                lines.push(format!(
                    "… {} more bounded differences",
                    differences.len() - 160
                ));
            }
            if let SignatureComparisonState::Partial { limitations, .. } = &app.signature_comparison
            {
                lines.push(String::new());
                lines.push("Partial comparison:".into());
                lines.extend(limitations.iter().map(|value| format!("! {value}")));
            }
        }
    }
    lines.join("\n")
}

fn navigator(frame: &mut Frame, app: &App, area: Rect) {
    let entries = [
        ("Dashboard", Screen::Dashboard),
        ("Layers", Screen::Layers),
        ("Recipes", Screen::Recipes),
        ("Packages", Screen::Packages),
        ("Images", Screen::Images),
        ("SDK", Screen::Sdk),
        ("Testing", Screen::Testing),
        ("Security", Screen::Security),
        ("QA", Screen::Qa),
        ("Tasks", Screen::Tasks),
        ("Logs", Screen::Logs),
        ("Errors", Screen::Errors),
        ("Configuration", Screen::Configuration),
        ("Dependencies", Screen::Dependencies),
        ("Devtool", Screen::Recipes),
        ("Maintenance", Screen::Maintenance),
        ("Build environment", Screen::BuildEnvironment),
        ("Settings", Screen::Settings),
    ];
    let text = entries
        .iter()
        .enumerate()
        .map(|(index, (name, _))| {
            format!(
                "{} {}",
                if index == app.navigator_selection {
                    "▶"
                } else {
                    " "
                },
                name
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(
        Paragraph::new(text).block(pane_block(
            app,
            "Navigator",
            app.focus == FocusTarget::Navigator,
        )),
        area,
    );
}

fn workspace(frame: &mut Frame, app: &App, area: Rect) {
    match app.screen {
        Screen::Dashboard => dashboard(frame, app, area),
        Screen::Tasks => tasks_workspace(frame, app, area),
        Screen::BuildHistory => build_history(frame, app, area),
        Screen::Dependencies => dependencies(frame, app, area),
        Screen::Signatures => signature_records(frame, app, area),
        Screen::LayerRelationships => layer_relationships(frame, app, area),
        Screen::Logs => logs(frame, app, area),
        Screen::Errors => errors(frame, app, area),
        Screen::Recipes => recipes(frame, app, area),
        Screen::Packages => packages_workspace(frame, app, area),
        Screen::Images => images_workspace(frame, app, area),
        Screen::Sdk => sdk_workspace(frame, app, area),
        Screen::Testing => testing_workspace(frame, app, area),
        Screen::Security => security_workspace(frame, app, area),
        Screen::Qa => qa_workspace(frame, app, area),
        Screen::Layers => {
            if let Some(browser) = app.layer_browser.as_ref() {
                layer_browser(frame, app, browser, area)
            } else {
                layers(frame, app, area)
            }
        }
        Screen::Configuration => config(frame, app, area),
        Screen::Bbmask => bbmask(frame, app, area),
        Screen::Maintenance => maintenance_workspace(frame, app, area),
        Screen::Help => help(frame, area),
        Screen::Settings => settings_workspace(frame, app, area),
        Screen::BuildEnvironment => build_environment_workspace(frame, app, area),
    }
}

fn inspector(frame: &mut Frame, app: &App, area: Rect) {
    let details = match app.screen {
        Screen::Recipes => app.workspace.recipes.get(app.recipe_selection).map_or_else(
            || "No recipe selected.".into(),
            |recipe| recipe_inspector(app, recipe),
        ),
        Screen::Layers => app.layer_browser.as_ref().map_or_else(
            || {
                app.workspace.layers.get(app.layer_selection).map_or_else(
                    || "No layer selected.".into(),
                    |layer| {
                        format!(
                            "Layer: {}\nPath: {}\nPriority: {}\n\nEnter browses this layer.",
                            layer.name,
                            layer.path.display(),
                            layer
                                .priority
                                .map_or_else(|| "unknown".into(), |value| value.to_string())
                        )
                    },
                )
            },
            |browser| {
                browser.selected_entry().map_or_else(
                    || format!("Layer: {}\n\nThis layer is empty.", browser.layer),
                    |entry| {
                        let detail = layer_entry_metadata(app, browser, entry);
                        let preview = match browser.preview_kind {
                            PreviewKind::Binary => "Binary preview unavailable.",
                            PreviewKind::Text if !browser.preview.is_empty() => {
                                "Text preview is visible in the workspace."
                            }
                            _ => "Preview unavailable.",
                        };
                        format!("{detail}\n\n{preview}")
                    },
                )
            },
        ),
        Screen::Configuration => config_inspector(app),
        Screen::Logs => app.logs.selected().map_or_else(
            || "No logs retained.".into(),
            |entry| {
                format!(
                    "Time: {}\nSeverity: {:?}\nBuild: {}\nRecipe: {}\nTask: {}\nSource: {}\nProtected: {}\n\n{}",
                    timestamp_text(entry.timestamp),
                    entry.severity,
                    entry.build.as_deref().unwrap_or("unavailable"),
                    entry.recipe.as_deref().unwrap_or("unavailable"),
                    entry.task.as_deref().unwrap_or("unavailable"),
                    entry
                        .path
                        .as_ref()
                        .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                    if entry.protected { "yes" } else { "no" },
                    entry.message,
                )
            },
        ),
        Screen::Errors => app
            .logs
            .diagnostics()
            .nth(app.error_selection)
            .map_or_else(
                || "No retained warnings or errors.".into(),
                |entry| diagnostic_detail(app, entry),
            ),
        Screen::Tasks => app.selected_task_row().map_or_else(
            || "No task selected.\n\nTask details appear as typed BitBake events arrive.".into(),
            |row| match row {
                TaskRow::WaitingSummary(count) => format!(
                    "Waiting tasks: {count}\n\nBitBake reported the overall task total, but individual queued-task metadata is not available yet."
                ),
                TaskRow::Task(task) => {
                    let now = SystemTime::now();
                    let elapsed = task
                        .elapsed_at(now)
                        .map(format_duration)
                        .unwrap_or_else(|| "unavailable".into());
                    let dependencies = if task.dependencies.is_empty() {
                        "unavailable".into()
                    } else {
                        task.dependencies
                            .iter()
                            .map(|dependency| dependency.0.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    let live_log = app
                        .logs
                        .entries
                        .iter()
                        .rev()
                        .find(|entry| {
                            entry.recipe.as_deref() == Some(task.recipe.as_str())
                                && entry.task.as_deref() == Some(task.task.as_str())
                        })
                        .map_or("unavailable", |entry| entry.message.as_str());
                    format!(
                        "Recipe: {}\nTask: {}\nState: {:?}\nProgress: {}\nWorker: {}\nPID: {}\nStarted: {}\nElapsed: {}\nDependencies: {}\nSource log: {}\nCancellation: {}\n\nLive log:\n{}",
                        task.recipe,
                        task.task,
                        task.state,
                        task.progress
                            .map_or_else(|| "unknown".into(), |value| format!("{value}%")),
                        task.worker.as_deref().unwrap_or("unavailable"),
                        task.pid.map_or_else(|| "unavailable".into(), |pid| pid.to_string()),
                        task.started
                            .map(timestamp_text)
                            .unwrap_or_else(|| "unavailable".into()),
                        elapsed,
                        dependencies,
                        task.log_path
                            .as_ref()
                            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                        task.cancellation.as_deref().unwrap_or("none"),
                        live_log,
                    )
                }
            },
        ),
        Screen::Dependencies => dependency_inspector(app),
        Screen::Signatures => signature_detail_text(app),
        Screen::Packages => package_inspector_text(app),
        Screen::Images => image_artifact_inspector_text(app),
        Screen::Sdk => sdk_inspector_text(app),
        Screen::Testing => testing_inspector_text(app),
        Screen::Security => security_inspector_text(app),
        Screen::Qa => qa_inspector_text(app),
        Screen::Maintenance => maintenance_inspector_text(app),
        _ => format!(
            "Target: {}\nStatus: {:?}\n\nSelect an item in the workspace to inspect its details.",
            app.build.target.as_deref().unwrap_or("not selected"),
            app.build.status
        ),
    };
    frame.render_widget(
        Paragraph::new(details)
            .block(pane_block(
                app,
                "Inspector",
                app.focus == FocusTarget::Inspector,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

#[allow(dead_code)]
fn build_progress_popup(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(14).clamp(50, 110);
    let height = area.height.saturating_sub(6).clamp(12, 28);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    let mut task_lines = app
        .tasks
        .values()
        .map(|task| {
            format!(
                "  {:<28} {:<18} {:>3}%",
                task.recipe,
                task.task,
                task.progress.unwrap_or(0)
            )
        })
        .collect::<Vec<_>>();
    task_lines.sort();
    if task_lines.is_empty() {
        task_lines.push("  Waiting for BitBake task events…".into());
    }
    let cpu = app
        .host_telemetry
        .cpu_utilization_percent
        .map_or_else(|| "sampling".into(), |value| format!("{value}%"));
    let disk = app
        .host_telemetry
        .disk_available_bytes
        .map_or_else(|| "unavailable".into(), format_bytes);
    let parse = match (app.build.parse_current, app.build.parse_total) {
        (Some(current), Some(total)) if total > 0 => format!(
            "{current}/{total} ({:.0}%)",
            current as f64 / total as f64 * 100.0
        ),
        _ => "not parsing".into(),
    };
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(format!(
            "Target: {}\nStatus: {:?}    Tasks: {} complete, {} active\nParse: {parse}    CPU: {cpu}    Free disk: {disk}\n\nActive recipe tasks:\n{}\n\nBitBake is running. c cancels the build.",
            app.build.target.as_deref().unwrap_or("unknown"),
            app.build.status,
            app.build.completed,
            app.tasks.len(),
            task_lines.join("\n"),
        ))
        .block(Block::default().title("Build progress").borders(Borders::ALL))
        .wrap(Wrap { trim: false }),
        popup,
    );
}

fn build_completion_popup(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(24).clamp(44, 90);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        area.height.saturating_sub(9) / 2,
        width,
        9,
    );
    let result = match app.build.status {
        yoctui_model::BuildStatus::Completed => "completed successfully",
        yoctui_model::BuildStatus::Cancelled => "was cancelled",
        _ => "failed",
    };
    let action = if app.build.status == yoctui_model::BuildStatus::Failed && app.build.errors > 0 {
        "Press Enter to investigate Errors; any other key returns to Yoctui."
    } else {
        "Press any key to return to Yoctui."
    };
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(format!(
            "Build {} for {}.\n\nTasks completed: {}\nWarnings: {}    Errors: {}    Exit code: {}\nElapsed: {}\n\n{}",
            result,
            app.build.target.as_deref().unwrap_or("unknown target"),
            app.build.completed,
            app.build.warnings,
            app.build.errors,
            app.build.exit_code.map_or_else(|| "unknown".into(), |code| code.to_string()),
            app.elapsed().map(format_duration).unwrap_or_else(|| "unknown".into()),
            action,
        ))
        .style(build_status_style(app))
        .block(Block::default().title("Build finished").borders(Borders::ALL))
        .wrap(Wrap { trim: true }),
        popup,
    );
}
fn recipe_editor(frame: &mut Frame, app: &App, editor: &RecipeEditor, area: Rect) {
    let width = area.width.saturating_sub(4).max(30);
    let height = area.height.saturating_sub(2).max(8);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    clear_popup(frame, app, popup);
    let columns =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)]).split(popup);
    let files = editor
        .files
        .iter()
        .enumerate()
        .map(|(index, path)| {
            format!(
                "{} {}",
                if index == editor.selection { ">" } else { " " },
                path.display()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(
        Paragraph::new(files)
            .block(
                Block::default()
                    .title(format!("Workspace file tree: {}", editor.recipe))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        columns[0],
    );
    let selected = editor
        .files
        .get(editor.selection)
        .map_or_else(|| "no file".into(), |path| path.display().to_string());
    let mode = if editor.editing {
        "editing"
    } else {
        "read-only"
    };
    let modified = if editor.dirty { " modified" } else { "" };
    let content = if editor.editing {
        format!("{}▏", editor.content)
    } else {
        editor.content.clone()
    };
    frame.render_widget(
        Paragraph::new(source_preview(&content, &selected, app))
            .block(
                Block::default()
                    .title(format!("{selected} ({mode}{modified})"))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        columns[1],
    );
    let footer = Rect::new(
        popup.x,
        popup.y.saturating_add(popup.height.saturating_sub(1)),
        popup.width,
        1,
    );
    frame.render_widget(
        Paragraph::new(
            "↑/↓ file  Enter/e edit  Ctrl+S save  Ctrl+B build recipe  Esc return to Yoctui",
        )
        .style(ThemePalette::for_app(app).role(ThemePalette::for_app(app).disabled, Modifier::DIM)),
        footer,
    );
}
fn dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let mut active = app.tasks.values().collect::<Vec<_>>();
    active.sort_by(|left, right| {
        (left.recipe.as_str(), left.task.as_str())
            .cmp(&(right.recipe.as_str(), right.task.as_str()))
    });
    let mut package_tasks = active.iter().map(|task| (*task, None)).collect::<Vec<_>>();
    package_tasks.extend(
        app.completed_tasks
            .iter()
            .rev()
            .map(|completed| (&completed.task, Some(completed.success))),
    );
    let recent = app
        .logs
        .entries
        .iter()
        .rev()
        .take(8)
        .rev()
        .map(|l| l.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let chunks =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).split(area);
    let parse_progress = app.build.parse_current.map_or_else(
        || "not parsing".into(),
        |current| {
            app.build
                .parse_total
                .map_or_else(|| current.to_string(), |total| format!("{current}/{total}"))
        },
    );
    let cpu_utilization = app
        .host_telemetry
        .cpu_utilization_percent
        .map_or_else(|| "sampling".into(), |percent| format!("{percent}%"));
    let disk_available = app
        .host_telemetry
        .disk_available_bytes
        .map_or_else(|| "unavailable".into(), format_bytes);
    let build_panels =
        Layout::vertical([Constraint::Length(13), Constraint::Min(3)]).split(chunks[0]);
    frame.render_widget(
        Paragraph::new(format!(
            "Target: {}\nBackend: {}\nStatus: {}\nExit code: {}\nParse progress: {}\nMachine: {}\nDistro: {}\nRelease: {}\nTasks: {}/{} (active: {})\nWarnings: {}  Errors: {}\nHost CPU: {}  Build disk free: {}",
            app.build.target.as_deref().unwrap_or("none"),
            app.backend,
            app.build.status,
            app.build.exit_code.map_or_else(|| "none".into(), |code| code.to_string()),
            parse_progress,
            app.workspace
                .variables
                .get("MACHINE")
                .map_or("unknown", String::as_str),
            app.workspace
                .variables
                .get("DISTRO")
                .map_or("unknown", String::as_str),
            app.workspace.release.as_deref().unwrap_or("unknown"),
            app.build.completed,
            app.build
                .total
                .map_or_else(|| "?".into(), |total| total.to_string()),
            app.tasks.len(),
            app.build.warnings,
            app.build.errors,
            cpu_utilization,
            disk_available,
        ))
        .block(Block::default().title("Build").borders(Borders::ALL)),
        build_panels[0],
    );
    let task_count = package_tasks.len();
    let start = app.task_progress_scroll.min(task_count.saturating_sub(1));
    let task_block = Block::default()
        .title(format!(
            "Package task progress (? = progress unknown; {} active, {} complete; use Up/Down to scroll)",
            active.len(),
            app.completed_tasks.len()
        ))
        .borders(Borders::ALL);
    let task_area = task_block.inner(build_panels[1]);
    frame.render_widget(task_block, build_panels[1]);
    if package_tasks.is_empty() {
        frame.render_widget(
            Paragraph::new("Waiting for BitBake task events."),
            task_area,
        );
    } else {
        let rows = Layout::vertical(
            package_tasks[start..]
                .iter()
                .take(task_area.height as usize)
                .map(|_| Constraint::Length(1))
                .collect::<Vec<_>>(),
        )
        .split(task_area);
        for ((task, completed), row) in package_tasks[start..]
            .iter()
            .take(rows.len())
            .zip(rows.iter().copied())
        {
            let progress = if completed.is_some() {
                100
            } else {
                task.progress.unwrap_or(0).min(100)
            };
            let palette = ThemePalette::for_app(app);
            let progress_style = if *completed == Some(false) {
                palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
            } else if progress >= 100 {
                palette.role(palette.success, Modifier::BOLD)
            } else if progress >= 75 {
                palette.role(palette.warning, Modifier::BOLD)
            } else {
                palette.role(palette.progress, Modifier::BOLD)
            };
            let label = if completed.is_some() {
                format!(
                    "{}:{} {progress}%{}",
                    task.recipe,
                    task.task,
                    match completed {
                        Some(true) => " complete",
                        Some(false) => " failed",
                        None => "",
                    }
                )
            } else if task.progress.is_some() {
                format!("{}:{} {progress}%", task.recipe, task.task)
            } else {
                format!(
                    "? {}:{}{}",
                    task.recipe,
                    task.task,
                    task_activity(app, None)
                )
            };
            frame.render_widget(
                Gauge::default()
                    .ratio(f64::from(progress) / 100.0)
                    .label(label)
                    .gauge_style(progress_style),
                row,
            );
        }
    }
    frame.render_widget(
        Paragraph::new(recent)
            .block(
                Block::default()
                    .title("Recent output")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    )
}

fn tasks_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let rows = app.visible_task_rows();
    let waiting = app.waiting_task_count();
    let active = app
        .tasks
        .values()
        .filter(|task| task.state == TaskState::Active)
        .count();
    let failed = app
        .completed_tasks
        .iter()
        .filter(|task| !task.success)
        .count();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(3),
    ])
    .split(area);
    let overall = Block::default()
        .title("Overall build progress")
        .borders(Borders::ALL);
    if let Some(total) = app.build.total.filter(|total| *total > 0) {
        let completed = app.build.completed.min(total);
        frame.render_widget(
            Gauge::default()
                .block(overall)
                .ratio(completed as f64 / total as f64)
                .label(format!(
                    "{}%  {completed}/{total} | active {active} | waiting {waiting} | failed {failed}",
                    completed.saturating_mul(100) / total
                ))
                .gauge_style(
                    ThemePalette::for_app(app)
                        .role(ThemePalette::for_app(app).progress, Modifier::BOLD),
                ),
            chunks[0],
        );
    } else {
        frame.render_widget(
            Paragraph::new(format!(
                "progress unknown | {} complete | {active} active | waiting unknown | {failed} failed",
                app.build.completed
            ))
            .block(overall),
            chunks[0],
        );
    }
    let duration = app
        .task_filters
        .minimum_duration
        .map_or_else(|| "all".into(), |value| format!("≥{}s", value.as_secs()));
    let active_field = match app.task_filter_field {
        TaskFilterField::Recipe => "recipe",
        TaskFilterField::Task => "task",
        TaskFilterField::Worker => "worker",
    };
    frame.render_widget(
        Paragraph::new(format!(
            "State {:?} | recipe '{}' | task '{}' | worker '{}' | duration {duration}\n{} field: {active_field}  F next field  / edit  f state  d duration",
            app.task_filters.state,
            app.task_filters.recipe,
            app.task_filters.task,
            app.task_filters.worker,
            if app.task_filter_editing { "Editing" } else { "Filter" },
        ))
        .block(Block::default().title("Filters").borders(Borders::ALL)),
        chunks[1],
    );
    let table_rows = rows.iter().enumerate().map(|(index, row)| {
        let values = match row {
            TaskRow::WaitingSummary(count) => vec![
                Cell::from("(queue)"),
                Cell::from(format!("{count} tasks")),
                Cell::from("--"),
                Cell::from("WAITING"),
                Cell::from("queued metadata unavailable"),
            ],
            TaskRow::Task(task) => vec![
                Cell::from(task.recipe.clone()),
                Cell::from(task.task.clone()),
                Cell::from(
                    task.elapsed_at(SystemTime::now())
                        .map(format_duration)
                        .unwrap_or_else(|| "--".into()),
                ),
                Cell::from(format!("{:?}", task.state).to_uppercase()),
                Cell::from(match (task.state, task.progress) {
                    (TaskState::Active, None) => {
                        format!("progress unknown{}", task_activity(app, None))
                    }
                    (_, Some(progress)) => format!("{progress}%"),
                    _ => "--".into(),
                }),
            ],
        };
        Row::new(values).style(selected_style(app, index == app.task_progress_scroll))
    });
    frame.render_widget(
        Table::new(
            table_rows,
            [
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Length(10),
                Constraint::Length(11),
                Constraint::Min(16),
            ],
        )
        .header(
            Row::new(["Recipe", "Task", "Elapsed", "State", "Progress"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!("Live Tasks ({} visible)", rows.len()))
                .borders(Borders::ALL)
                .style(ThemePalette::for_app(app).base()),
        ),
        chunks[2],
    );
}

fn qemu_popup_rect(area: Rect, preferred_width: u16, preferred_height: u16) -> Rect {
    let width = preferred_width.min(area.width.saturating_sub(2)).max(1);
    let height = preferred_height.min(area.height.saturating_sub(2)).max(1);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn sdk_kind_label(kind: SdkArtifactKind) -> &'static str {
    match kind {
        SdkArtifactKind::Installer => "installer",
        SdkArtifactKind::Checksum => "checksum",
        SdkArtifactKind::Manifest => "manifest",
        SdkArtifactKind::Other => "other",
    }
}

fn sdk_type_label(kind: Option<SdkKind>) -> &'static str {
    match kind {
        Some(SdkKind::Standard) => "standard",
        Some(SdkKind::Extensible) => "extensible",
        None => "unavailable",
    }
}

fn sdk_inventory_root(app: &App) -> String {
    let request_root = match &app.sdk_artifacts {
        SdkArtifactInventoryState::Loading { request }
        | SdkArtifactInventoryState::AvailableEmpty { request }
        | SdkArtifactInventoryState::Available { request, .. }
        | SdkArtifactInventoryState::Partial { request, .. }
        | SdkArtifactInventoryState::Failed { request, .. } => Some(&request.root),
        SdkArtifactInventoryState::NotLoaded => None,
    };
    request_root
        .map(|path| path.display().to_string())
        .or_else(|| app.workspace.variables.get("SDK_DEPLOY").cloned())
        .unwrap_or_else(|| "unavailable".into())
}

fn qa_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let active = |view| {
        if app.qa.view == view {
            palette.focus()
        } else {
            Style::default()
        }
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Recipe & Kernel ", active(QaView::RecipeKernel)),
            Span::raw(" | "),
            Span::styled(" Layer QA ", active(QaView::LayerQa)),
        ]),
        Line::from(format!(
            "Scope: {} | Filter: {}{}",
            qa_scope_label(app),
            qa_filter_label(app.qa.status_filter),
            if app.qa.searching {
                format!(" | Search: {}_", app.qa.query)
            } else if app.qa.query.is_empty() {
                String::new()
            } else {
                format!(" | Search: {}", app.qa.query)
            }
        )),
        Line::from(""),
    ];
    if app.qa.drilled {
        qa_finding_lines(app, &palette, &mut lines);
    } else {
        match app.qa.view {
            QaView::RecipeKernel => qa_check_lines(app, &palette, &mut lines),
            QaView::LayerQa => qa_layer_lines(app, &palette, &mut lines),
        }
    }
    qa_inventory_lines(app, &palette, &mut lines);
    qa_session_lines(app, &palette, &mut lines);
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(app, "QA", app.focus == FocusTarget::Workspace))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn qa_scope_label(app: &App) -> String {
    match app.qa.view {
        QaView::RecipeKernel => app.qa.scope.as_ref().map_or_else(
            || "unavailable".into(),
            |scope| format!("{} ({})", scope.recipe.name, scope.recipe.file.display()),
        ),
        QaView::LayerQa => app.qa.layer_selection.as_ref().map_or_else(
            || "unavailable".into(),
            |layer| format!("{} ({})", layer.name, layer.root.display()),
        ),
    }
}

fn qa_check_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    match &app.qa.capability {
        QaCapability::NotInspected => {
            lines.push(Line::from("QA capability is not inspected."));
            return;
        }
        QaCapability::Inspecting => {
            lines.push(Line::styled(
                "Inspecting recipe and kernel QA capability…",
                palette.info,
            ));
            return;
        }
        QaCapability::Failed(message) => {
            lines.push(Line::styled(
                format!("QA capability failed: {message}"),
                palette.error,
            ));
            return;
        }
        QaCapability::Partial { limitations, .. } => lines.push(Line::styled(
            format!("Partial capability: {}", limitations.join(" | ")),
            palette.warning,
        )),
        QaCapability::Available(_) => {}
    }
    lines.push(Line::from(
        "  Family                Exact task             Availability       Findings  Label",
    ));
    let checks = app.qa.visible_checks();
    for check in &checks {
        let selected = app.qa.check_selection.as_ref() == Some(&check.id);
        let findings = app.qa.findings_for_check(&check.id);
        let status = qa_worst_status(findings.iter().map(|finding| finding.status));
        let availability = match &check.availability {
            QaCheckAvailability::Available => "available",
            QaCheckAvailability::Disabled(_) => "disabled",
        };
        lines.push(
            Line::from(format!(
                "{} {:<21} {:<22} {:<18} {:<9} {}",
                if selected { "▶" } else { " " },
                qa_family_label(check.family),
                check.task.as_deref().unwrap_or("unavailable"),
                availability,
                qa_status_label(status),
                check.label,
            ))
            .style(if selected {
                palette.selected()
            } else {
                qa_status_style(palette, status)
            }),
        );
    }
    if checks.is_empty() {
        lines.push(Line::from(
            "No checks match the exact scope, status filter, and search.",
        ));
    }
}

fn qa_layer_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    match &app.qa.layer_capability {
        QaLayerCapability::NotInspected => {
            lines.push(Line::from("Layer-QA capability is not inspected."));
            return;
        }
        QaLayerCapability::Inspecting => {
            lines.push(Line::styled(
                "Inspecting configured layer-QA capability…",
                palette.info,
            ));
            return;
        }
        QaLayerCapability::Failed(message) => {
            lines.push(Line::styled(
                format!("Layer-QA capability failed: {message}"),
                palette.error,
            ));
            return;
        }
        QaLayerCapability::Partial { limitations, .. } => lines.push(Line::styled(
            format!("Partial capability: {}", limitations.join(" | ")),
            palette.warning,
        )),
        QaLayerCapability::Available(_) => {}
    }
    lines.push(Line::from(
        "  Layer                 Capability       Pass Warn Fail Skip Unknown  Exact root",
    ));
    let layers = app.qa.visible_layers();
    for layer in &layers {
        let selected = app.qa.layer_selection.as_ref() == Some(&layer.identity);
        let counts = app.qa.layer_finding_counts(&layer.identity);
        let capability = match layer.run {
            QaLayerRunCapability::Available { .. } => "available",
            QaLayerRunCapability::Disabled(_) => "disabled",
        };
        let status = qa_worst_status(
            app.qa
                .findings_for_layer(&layer.identity)
                .iter()
                .map(|finding| finding.status),
        );
        lines.push(
            Line::from(format!(
                "{} {:<21} {:<16} {:>4} {:>4} {:>4} {:>4} {:>7}  {}",
                if selected { "▶" } else { " " },
                layer.identity.name,
                capability,
                counts.passed,
                counts.warnings,
                counts.failed,
                counts.skipped,
                counts.unknown,
                layer.identity.root.display(),
            ))
            .style(if selected {
                palette.selected()
            } else {
                qa_status_style(palette, status)
            }),
        );
    }
    if layers.is_empty() {
        lines.push(Line::from(
            "No configured layers match the status filter and search.",
        ));
    }
}

fn qa_finding_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(
        "Findings (Esc returns) — Status     Severity      Rule / test                 Message",
    ));
    let findings = app.qa.visible_findings();
    for finding in &findings {
        let selected = app.qa.finding_selection.as_ref() == Some(&finding.identity);
        lines.push(
            Line::from(format!(
                "{} {:<10} {:<13} {:<27} {}",
                if selected { "▶" } else { " " },
                qa_status_label(Some(finding.status)),
                finding.severity.as_deref().unwrap_or("unavailable"),
                finding
                    .rule
                    .as_deref()
                    .or(finding.test_name.as_deref())
                    .unwrap_or("unavailable"),
                finding.message,
            ))
            .style(if selected {
                palette.selected()
            } else {
                qa_status_style(palette, Some(finding.status))
            }),
        );
    }
    if findings.is_empty() {
        lines.push(Line::from(
            "No findings match the active filter and search.",
        ));
    }
}

fn qa_inventory_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    match &app.qa.inventory {
        QaReportInventoryState::NotLoaded => lines.push(Line::from(
            "Reports not loaded. I imports; R refreshes exact paths.",
        )),
        QaReportInventoryState::Loading { request } => lines.push(Line::styled(
            format!(
                "Loading report generation {} from {} exact path(s)…",
                request.generation,
                request.paths.len()
            ),
            palette.info,
        )),
        QaReportInventoryState::AvailableEmpty { request } => lines.push(Line::from(format!(
            "Report generation {} available-empty: no reports or findings.",
            request.generation
        ))),
        QaReportInventoryState::Available { reports, .. } => lines.push(Line::from(format!(
            "{} exact report(s) available.",
            reports.len()
        ))),
        QaReportInventoryState::Partial {
            reports,
            limitations,
            ..
        } => lines.push(Line::styled(
            format!(
                "Partial: {} report(s) | {}",
                reports.len(),
                limitations.join(" | ")
            ),
            palette.warning,
        )),
        QaReportInventoryState::Failed { kind, message, .. } => lines.push(Line::styled(
            format!("Report acquisition {}: {message}", qa_failure_label(*kind)),
            palette.error,
        )),
        QaReportInventoryState::Cancelled { .. } => lines.push(Line::styled(
            "Report acquisition cancelled.",
            palette.warning,
        )),
        QaReportInventoryState::TimedOut { .. } => {
            lines.push(Line::styled("Report acquisition timed out.", palette.error))
        }
        QaReportInventoryState::Lost { message, .. } => lines.push(Line::styled(
            format!("Report worker lost: {message}"),
            palette.error,
        )),
    }
}

fn qa_session_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    let session = match app.qa.view {
        QaView::RecipeKernel => app.qa.sessions.back().map(|session| {
            (
                session.id.0,
                session.status,
                session.message.as_deref(),
                session
                    .output
                    .iter()
                    .map(|line| (line.stream, line.line.as_str(), line.truncated))
                    .collect::<Vec<_>>(),
            )
        }),
        QaView::LayerQa => app.qa.layer_sessions.back().map(|session| {
            (
                session.id.0,
                session.status,
                session.message.as_deref(),
                session
                    .output
                    .iter()
                    .map(|line| (line.stream, line.line.as_str(), line.truncated))
                    .collect::<Vec<_>>(),
            )
        }),
    };
    let Some((id, status, message, output)) = session else {
        return;
    };
    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!("Latest session {id}: {}", qa_session_status_label(status)),
        qa_session_style(palette, status),
    ));
    if let Some(message) = message {
        lines.push(Line::styled(
            format!("Session detail: {message}"),
            palette.warning,
        ));
    }
    for (stream, line, truncated) in output.iter().rev().take(4).rev() {
        lines.push(Line::styled(
            format!(
                "[{}] {}{}",
                match stream {
                    QaOutputStream::Stdout => "stdout",
                    QaOutputStream::Stderr => "stderr",
                },
                line,
                if *truncated { " [truncated]" } else { "" }
            ),
            if *stream == QaOutputStream::Stderr {
                security_warning_style(palette)
            } else {
                Style::default()
            },
        ));
    }
}

fn qa_inspector_text(app: &App) -> String {
    let mut lines = vec![format!(
        "View: {}\nExact scope: {}",
        match app.qa.view {
            QaView::RecipeKernel => "Recipe & Kernel",
            QaView::LayerQa => "Layer QA",
        },
        qa_scope_label(app)
    )];
    if let Some(finding) = app.qa.selected_finding() {
        lines.push(format!(
            "\nFinding {}\nStatus: {}\nSeverity: {}\nMessage: {}\nTask: {}\nTest: {}\nRule: {}\nSuggestion: {}\nSource: {}\nMetadata: {}",
            finding.identity.fingerprint,
            qa_status_label(Some(finding.status)),
            finding.severity.as_deref().unwrap_or("unavailable"),
            finding.message,
            finding.task.as_deref().unwrap_or("unavailable"),
            finding.test_name.as_deref().unwrap_or("unavailable"),
            finding.rule.as_deref().unwrap_or("unavailable"),
            finding.suggestion.as_deref().unwrap_or("unavailable"),
            finding.source.as_ref().map_or_else(
                || "unavailable".into(),
                |source| format!(
                    "{}:{}:{}",
                    source.path.display(),
                    source.line.map_or_else(|| "unavailable".into(), |line| line.to_string()),
                    source.column.map_or_else(|| "unavailable".into(), |column| column.to_string())
                )
            ),
            if finding.metadata.is_empty() {
                "unavailable".into()
            } else {
                finding.metadata.iter().map(|item| format!("{}={}", item.key, item.value)).collect::<Vec<_>>().join(", ")
            }
        ));
    } else if let Some(report) = app.qa.selected_report() {
        lines.push(format!(
            "\nReport\nPath: {}\nFormat: {:?}\nBytes: {}\nFingerprint: {}\nModified: {}\nFindings: {}\nLimitations: {}",
            report.identity.path.display(),
            report.identity.format,
            report.identity.byte_size,
            report.identity.fingerprint,
            timestamp_text(report.identity.modified_at),
            report.findings.len(),
            if report.limitations.is_empty() { "none".into() } else { report.limitations.join(" | ") },
        ));
    } else {
        match app.qa.view {
            QaView::RecipeKernel => {
                if let Some(check) = app.qa.selected_check() {
                    lines.push(format!(
                        "\nCheck {}\nFamily: {}\nTask: {}\nAvailability: {}\nProvider: {}\nReport roots: {}\nLimitations: {}",
                        check.id.0,
                        qa_family_label(check.family),
                        check.task.as_deref().unwrap_or("unavailable"),
                        match &check.availability {
                            QaCheckAvailability::Available => "available",
                            QaCheckAvailability::Disabled(reason) => reason,
                        },
                        check.scope.recipe.file.display(),
                        if check.report_roots.is_empty() { "unavailable".into() } else { check.report_roots.iter().map(|path| path.display().to_string()).collect::<Vec<_>>().join(", ") },
                        if check.limitations.is_empty() { "none".into() } else { check.limitations.join(" | ") },
                    ));
                } else {
                    lines.push("\nNo QA check selected.".into());
                }
            }
            QaView::LayerQa => {
                if let Some(layer) = app.qa.selected_layer() {
                    lines.push(format!(
                        "\nLayer {}\nRoot: {}\nCompatible series: {}\nCapability: {}\nLimitations: {}",
                        layer.identity.name,
                        layer.identity.root.display(),
                        if layer.compatible_series.is_empty() { "unavailable".into() } else { layer.compatible_series.join(", ") },
                        match &layer.run {
                            QaLayerRunCapability::Available { executable, arguments, report_roots } => format!("{} | argv [{}] | reports {}", executable.path.display(), arguments.join(", "), report_roots.len()),
                            QaLayerRunCapability::Disabled(reason) => format!("disabled: {reason}"),
                        },
                        if layer.limitations.is_empty() { "none".into() } else { layer.limitations.join(" | ") },
                    ));
                } else {
                    lines.push("\nNo configured layer selected.".into());
                }
            }
        }
    }
    lines.join("\n")
}

fn qa_dialog(frame: &mut Frame, app: &App, dialog: &QaDialog, area: Rect) {
    let (title, body) = match dialog {
        QaDialog::Operation(preview) => (
            "Confirm recipe/kernel QA",
            format!(
                "Operation {}\nCheck: {}\nRecipe: {}\nProvider: {}\n\nIndexed BitBake request:\n{}\n\nReport roots:\n{}\n\nEnter runs | Esc cancels",
                preview.id.0,
                preview.check.0,
                preview.scope.recipe.name,
                preview.scope.recipe.file.display(),
                preview.indexed_arguments.join("\n"),
                preview
                    .report_roots
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        ),
        QaDialog::LayerOperation(preview) => (
            "Confirm layer QA",
            format!(
                "Operation {}\nLayer: {} ({})\nExecutable: {}\n\nIndexed native vector:\n{}\n\nEnter runs | Esc cancels",
                preview.id.0,
                preview.layer.name,
                preview.layer.root.display(),
                preview.executable.path.display(),
                preview.indexed_arguments.join("\n")
            ),
        ),
        QaDialog::Import { input } => (
            "Import QA reports",
            format!("Exact absolute report or directory:\n{input}_\n\nEnter imports | Esc cancels"),
        ),
        QaDialog::Cancellation {
            session,
            background_job,
        } => (
            "Cancel managed QA",
            format!(
                "Cancel QA session {} attached to build job {}?\n\nEnter confirms | Esc keeps running",
                session.0, background_job.0
            ),
        ),
        QaDialog::LayerCancellation(session) => (
            "Cancel layer QA",
            format!(
                "Cancel exact layer-QA session {}?\n\nEnter confirms | Esc keeps running",
                session.0
            ),
        ),
    };
    let width = 78.min(area.width.saturating_sub(2));
    let height = 18.min(area.height.saturating_sub(2));
    let popup = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(body)
            .block(Block::default().title(title).borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn qa_family_label(family: QaCheckFamily) -> &'static str {
    match family {
        QaCheckFamily::KernelConfiguration => "kernel configuration",
        QaCheckFamily::UriFetch => "URI / fetch",
        QaCheckFamily::Patch => "patch",
        QaCheckFamily::License => "license",
        QaCheckFamily::RecipePackage => "recipe / package",
    }
}

fn qa_filter_label(filter: QaStatusFilter) -> &'static str {
    match filter {
        QaStatusFilter::All => "all",
        QaStatusFilter::Failed => "failed",
        QaStatusFilter::Warning => "warning",
        QaStatusFilter::Passed => "passed",
        QaStatusFilter::Skipped => "skipped",
        QaStatusFilter::Unknown => "unknown",
    }
}

fn qa_status_label(status: Option<QaFindingStatus>) -> &'static str {
    match status {
        Some(QaFindingStatus::Passed) => "passed",
        Some(QaFindingStatus::Warning) => "warning",
        Some(QaFindingStatus::Failed) => "failed",
        Some(QaFindingStatus::Skipped) => "skipped",
        Some(QaFindingStatus::Unknown) => "unknown",
        None => "unavailable",
    }
}

fn qa_worst_status(values: impl Iterator<Item = QaFindingStatus>) -> Option<QaFindingStatus> {
    values.max_by_key(|status| match status {
        QaFindingStatus::Failed => 5,
        QaFindingStatus::Warning => 4,
        QaFindingStatus::Unknown => 3,
        QaFindingStatus::Skipped => 2,
        QaFindingStatus::Passed => 1,
    })
}

fn qa_failure_label(kind: QaReportFailureKind) -> &'static str {
    match kind {
        QaReportFailureKind::Missing => "missing",
        QaReportFailureKind::PermissionDenied => "permission denied",
        QaReportFailureKind::Stale => "stale",
        QaReportFailureKind::Malformed => "malformed",
        QaReportFailureKind::Failed => "failed",
    }
}

fn qa_session_status_label(status: QaSessionStatus) -> &'static str {
    match status {
        QaSessionStatus::Starting => "starting",
        QaSessionStatus::Running => "running",
        QaSessionStatus::Cancelling => "cancelling",
        QaSessionStatus::Succeeded => "succeeded",
        QaSessionStatus::Failed => "failed",
        QaSessionStatus::Cancelled => "cancelled",
        QaSessionStatus::TimedOut => "timed out",
        QaSessionStatus::Lost => "lost",
    }
}

fn qa_status_style(palette: &ThemePalette, status: Option<QaFindingStatus>) -> Style {
    match status {
        Some(QaFindingStatus::Passed) => palette.role(palette.success, Modifier::BOLD),
        Some(QaFindingStatus::Warning | QaFindingStatus::Skipped) => {
            security_warning_style(palette)
        }
        Some(QaFindingStatus::Failed) => security_error_style(palette),
        Some(QaFindingStatus::Unknown) | None => Style::default(),
    }
}

fn qa_session_style(palette: &ThemePalette, status: QaSessionStatus) -> Style {
    match status {
        QaSessionStatus::Succeeded => palette.role(palette.success, Modifier::BOLD),
        QaSessionStatus::Failed | QaSessionStatus::TimedOut | QaSessionStatus::Lost => {
            security_error_style(palette)
        }
        QaSessionStatus::Cancelled | QaSessionStatus::Cancelling => security_warning_style(palette),
        QaSessionStatus::Starting | QaSessionStatus::Running => security_info_style(palette),
    }
}

fn security_scope_text(scope: Option<&SecurityScope>) -> String {
    match scope {
        Some(SecurityScope::Recipe(identity)) => {
            format!("recipe {} ({})", identity.name, identity.file.display())
        }
        Some(SecurityScope::Image {
            target,
            machine,
            distro,
        }) => format!("image {target} | MACHINE={machine} | DISTRO={distro}"),
        None => "unavailable".into(),
    }
}

fn security_capability_summary(capability: &SecurityCapability) -> String {
    match capability {
        SecurityCapability::NotInspected => {
            "not inspected; entering Security requests inspection".into()
        }
        SecurityCapability::Inspecting => "inspection in progress".into(),
        SecurityCapability::Failed(message) => format!("inspection failed: {message}"),
        SecurityCapability::Available(capability) => format!(
            "{} | build={} | CVE={} | recipe SBOM={} | image SBOM={} | mapper={}",
            capability
                .release
                .as_deref()
                .unwrap_or("release unavailable"),
            capability.build_directory.display(),
            capability.cve_task.as_deref().unwrap_or("unavailable"),
            capability
                .recipe_sbom_task
                .as_deref()
                .unwrap_or("unavailable"),
            capability
                .image_sbom_task
                .as_deref()
                .unwrap_or(if capability.image_build_emits_sbom {
                    "ordinary image build"
                } else {
                    "unavailable"
                }),
            capability.mapper.as_ref().map_or_else(
                || "unavailable".into(),
                |mapper| mapper.executable.display().to_string()
            ),
        ),
    }
}

fn security_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let active = |view| {
        if app.security.view == view {
            palette.focus()
        } else {
            Style::default()
        }
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" CVEs ", active(SecurityView::Cves)),
            Span::raw(" | "),
            Span::styled(" SBOM ", active(SecurityView::Sbom)),
        ]),
        Line::from(format!(
            "Scope: {}",
            security_scope_text(app.security.scope.as_ref())
        )),
        Line::from(format!(
            "Capability: {}",
            security_capability_summary(&app.security.capability)
        )),
    ];
    if app.security.searching {
        lines.push(Line::from(format!("Search: {}_", app.security.query)));
    } else if !app.security.query.is_empty() {
        lines.push(Line::from(format!("Search: {}", app.security.query)));
    }
    lines.push(Line::from(""));
    security_inventory_lines(app, &palette, &mut lines);
    security_session_lines(app, &palette, &mut lines);
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(
                app,
                "Security",
                app.focus == FocusTarget::Workspace,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn security_inventory_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    match &app.security.inventory {
        SecurityInventoryState::NotLoaded => lines.push(Line::from(
            "Reports are not loaded. Press I to import or R after capability discovery.",
        )),
        SecurityInventoryState::Loading { request } => lines.push(Line::styled(
            format!(
                "Loading report generation {} from {} exact path(s)…",
                request.generation,
                request.paths.len()
            ),
            security_info_style(palette),
        )),
        SecurityInventoryState::AvailableEmpty { request } => lines.push(Line::from(format!(
            "Report generation {} is available-empty: no reports or findings.",
            request.generation
        ))),
        SecurityInventoryState::Available { .. } | SecurityInventoryState::Partial { .. } => {
            match app.security.view {
                SecurityView::Cves => security_cve_lines(app, palette, lines),
                SecurityView::Sbom => security_sbom_lines(app, palette, lines),
            }
            if let SecurityInventoryState::Partial { limitations, .. } = &app.security.inventory {
                lines.push(Line::styled(
                    format!("Partial: {}", limitations.join(" | ")),
                    security_warning_style(palette),
                ));
            }
        }
        SecurityInventoryState::Failed { message, .. } => lines.push(Line::styled(
            format!("Security report acquisition failed: {message}"),
            security_error_style(palette),
        )),
        SecurityInventoryState::Cancelled { .. } => lines.push(Line::styled(
            "Security report acquisition cancelled.",
            security_warning_style(palette),
        )),
        SecurityInventoryState::TimedOut { .. } => lines.push(Line::styled(
            "Security report acquisition timed out.",
            security_error_style(palette),
        )),
        SecurityInventoryState::Lost { message, .. } => lines.push(Line::styled(
            format!("Security report worker lost: {message}"),
            security_error_style(palette),
        )),
    }
}

fn security_cve_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(format!(
        "Filter: {} | {} visible finding(s)",
        security_filter_label(app.security.cve_filter),
        app.security.visible_findings().len()
    )));
    lines.push(Line::from(
        "  CVE             Status        Recipe / package          Severity/score  Exact source",
    ));
    let findings = app.security.visible_findings();
    for finding in &findings {
        let selected = app.security.finding_selection.as_ref() == Some(&finding.identity);
        let source = cve_source_for_finding(app, &finding.identity).map_or_else(
            || "unavailable".into(),
            |report| report.identity.path.display().to_string(),
        );
        lines.push(
            Line::from(format!(
                "{} {:<15} {:<13} {:<25} {:<15} {}",
                if selected { "▶" } else { " " },
                finding.identity.cve,
                security_cve_status_label(finding.status),
                format!(
                    "{} / {}",
                    finding.identity.recipe,
                    finding.identity.package.as_deref().unwrap_or("—")
                ),
                format!(
                    "{}/{}",
                    finding.severity.as_deref().unwrap_or("—"),
                    finding.score.as_deref().unwrap_or("—")
                ),
                source,
            ))
            .style(if selected {
                palette.selected()
            } else {
                security_cve_status_style(palette, finding.status)
            }),
        );
    }
    if findings.is_empty() {
        lines.push(Line::from(
            "No findings match the active view, status filter, and search.",
        ));
    }
}

fn security_sbom_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    if app.security.drilled {
        let Some(SecurityReport::Spdx(document)) = app.security.selected_report() else {
            lines.push(Line::styled(
                "The selected SPDX document is no longer available.",
                security_warning_style(palette),
            ));
            return;
        };
        lines.push(Line::from(format!(
            "Document: {} | fingerprint {}",
            document.identity.path.display(),
            document.identity.fingerprint
        )));
        lines.push(Line::from(
            "  Component identity        Name                    Version       Supplier / license",
        ));
        let components = app.security.visible_components();
        for component in &components {
            let selected =
                app.security.component_selection.as_deref() == Some(component.identity.as_str());
            lines.push(
                Line::from(format!(
                    "{} {:<25} {:<23} {:<13} {} / {}",
                    if selected { "▶" } else { " " },
                    component.identity,
                    component.name,
                    component.version.as_deref().unwrap_or("—"),
                    component.supplier.as_deref().unwrap_or("—"),
                    component.license.as_deref().unwrap_or("—"),
                ))
                .style(if selected {
                    palette.selected()
                } else {
                    Style::default()
                }),
            );
        }
        if components.is_empty() {
            lines.push(Line::from(
                "No components match the active document and search.",
            ));
        }
        return;
    }
    lines.push(Line::from(
        "  Kind     SPDX version   Document                 Components  Exact artifact",
    ));
    let reports = app.security.visible_reports();
    for report in &reports {
        let SecurityReport::Spdx(document) = report else {
            continue;
        };
        let selected = app.security.report_selection.as_ref() == Some(&document.identity);
        lines.push(
            Line::from(format!(
                "{} {:<8} {:<14} {:<24} {:<11} {}",
                if selected { "▶" } else { " " },
                security_spdx_kind_label(document.kind),
                document.spdx_version.as_deref().unwrap_or("unavailable"),
                document.name.as_deref().unwrap_or("unavailable"),
                document.components.len(),
                document.identity.path.display(),
            ))
            .style(if selected {
                palette.selected()
            } else if document.limitations.is_empty() {
                Style::default()
            } else {
                security_warning_style(palette)
            }),
        );
    }
    if reports.is_empty() {
        lines.push(Line::from(
            "No SPDX documents match the active view and search.",
        ));
    }
}

fn security_session_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    let Some(session) = app.security.sessions.last() else {
        return;
    };
    let operation = security_operation_label(&session.preview.operation);
    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!(
            "Latest session {} | {operation} | {} | scope {}",
            session.preview.id.0,
            security_session_status_label(session.status),
            security_scope_text(Some(&session.preview.scope))
        ),
        security_session_style(palette, session.status),
    ));
    if let Some(background_job_id) = session.background_job_id
        && let Some(job) = app.background_jobs.get(background_job_id)
    {
        lines.push(Line::from(format!(
            "Managed build {:?} | warnings={} | errors={} | output dropped={}",
            job.status, job.warnings, job.errors, job.dropped_output_entries
        )));
    }
    if let Some(message) = &session.message {
        lines.push(Line::styled(
            format!("Session detail: {message}"),
            security_warning_style(palette),
        ));
    }
    for output in session.output.iter().rev().take(4).rev() {
        let stream = match output.stream {
            SecurityOutputStream::Stdout => "stdout",
            SecurityOutputStream::Stderr => "stderr",
        };
        let style = if output.stream == SecurityOutputStream::Stderr {
            security_warning_style(palette)
        } else {
            Style::default()
        };
        lines.push(Line::styled(
            format!(
                "[{stream}] {}{}",
                output.line,
                if output.truncated { " [truncated]" } else { "" }
            ),
            style,
        ));
    }
}

fn security_inspector_text(app: &App) -> String {
    let capability = security_capability_detail(&app.security.capability);
    let selected = match app.security.view {
        SecurityView::Cves => security_cve_inspector(app),
        SecurityView::Sbom => security_sbom_inspector(app),
    };
    let session = app.security.sessions.last().map_or_else(
        || "No Security operation has run.".into(),
        |session| {
            format!(
                "Session {}: {} ({})\nStarted: {}\nFinished: {}\nResult paths: {}\nRetained mapper output: {}",
                session.preview.id.0,
                security_operation_label(&session.preview.operation),
                security_session_status_label(session.status),
                timestamp_text(session.started_at),
                session
                    .finished_at
                    .map(timestamp_text)
                    .unwrap_or_else(|| "not finished".into()),
                session.result_paths.len(),
                session.output.len(),
            )
        },
    );
    format!("{selected}\n\n{capability}\n\n{session}")
}

fn security_capability_detail(capability: &SecurityCapability) -> String {
    match capability {
        SecurityCapability::Available(capability) => {
            let limitations = if capability.limitations.is_empty() {
                "none".into()
            } else {
                capability.limitations.join("\n")
            };
            format!(
                "Capability: available\nRelease: {}\nBuild: {}\nScope: {}\nCVE task: {}\nRecipe SBOM task: {}\nImage SBOM task: {}\nImage build emits SBOM: {}\nMapper: {}\nCVE roots: {}\nSBOM roots: {}\nLimitations:\n{}",
                capability.release.as_deref().unwrap_or("unavailable"),
                capability.build_directory.display(),
                security_scope_text(Some(&capability.scope)),
                capability.cve_task.as_deref().unwrap_or("unavailable"),
                capability
                    .recipe_sbom_task
                    .as_deref()
                    .unwrap_or("unavailable"),
                capability
                    .image_sbom_task
                    .as_deref()
                    .unwrap_or("unavailable"),
                capability.image_build_emits_sbom,
                capability.mapper.as_ref().map_or_else(
                    || "unavailable".into(),
                    |mapper| mapper.executable.display().to_string()
                ),
                display_security_paths(&capability.cve_roots),
                display_security_paths(&capability.sbom_roots),
                limitations,
            )
        }
        _ => format!("Capability: {}", security_capability_summary(capability)),
    }
}

fn security_cve_inspector(app: &App) -> String {
    let Some((report, finding)) = selected_security_finding(app) else {
        return "Select a typed CVE finding to inspect it.".into();
    };
    let mapping = display_security_metadata(&finding.mapping);
    let metadata = display_security_metadata(&report.metadata);
    let limitations = if report.limitations.is_empty() {
        "none".into()
    } else {
        report.limitations.join("\n")
    };
    format!(
        "Finding: {}\nStatus: {}\nRecipe: {}\nPackage: {}\nProduct: {}\nVersion: {}\nSeverity: {}\nScore: {}\nVector: {}\nAdvisory: {}\nSummary: {}\n\nExact report: {}\nFingerprint: {}\nBytes: {}\nModified: {}\nReport scope: {}\n\nPackage mapping:\n{}\n\nReport metadata:\n{}\n\nLimitations:\n{}",
        finding.identity.cve,
        security_cve_status_label(finding.status),
        finding.identity.recipe,
        finding.identity.package.as_deref().unwrap_or("unavailable"),
        finding.product.as_deref().unwrap_or("unavailable"),
        finding.version.as_deref().unwrap_or("unavailable"),
        finding.severity.as_deref().unwrap_or("unavailable"),
        finding.score.as_deref().unwrap_or("unavailable"),
        finding.vector.as_deref().unwrap_or("unavailable"),
        finding.advisory_url.as_deref().unwrap_or("unavailable"),
        finding.summary.as_deref().unwrap_or("unavailable"),
        report.identity.path.display(),
        report.identity.fingerprint,
        report.identity.byte_size,
        timestamp_text(report.identity.modified_at),
        security_scope_text(report.scope.as_ref()),
        mapping,
        metadata,
        limitations,
    )
}

fn security_sbom_inspector(app: &App) -> String {
    let Some(SecurityReport::Spdx(document)) = app.security.selected_report() else {
        return "Select an exact SPDX document or archive to inspect it.".into();
    };
    let creators = if document.creators.is_empty() {
        "unavailable".into()
    } else {
        document.creators.join("\n")
    };
    let checksums = display_security_metadata(&document.checksums);
    let limitations = if document.limitations.is_empty() {
        "none".into()
    } else {
        document.limitations.join("\n")
    };
    let component = app
        .security
        .visible_components()
        .into_iter()
        .find(|component| {
            app.security.component_selection.as_deref() == Some(component.identity.as_str())
        })
        .map_or_else(
            || "No component selected.".into(),
            |component| {
                format!(
                    "Component: {}\nName: {}\nVersion: {}\nSupplier: {}\nLicense: {}",
                    component.identity,
                    component.name,
                    component.version.as_deref().unwrap_or("unavailable"),
                    component.supplier.as_deref().unwrap_or("unavailable"),
                    component.license.as_deref().unwrap_or("unavailable"),
                )
            },
        );
    format!(
        "Exact artifact: {}\nFingerprint: {}\nBytes: {}\nModified: {}\nKind: {}\nScope: {}\nSPDX version: {}\nDocument: {}\nNamespace: {}\nData license: {}\nCreators:\n{}\nComponents: {}\nFiles: {}\nRelationships: {}\nChecksums:\n{}\n\n{}\n\nLimitations:\n{}",
        document.identity.path.display(),
        document.identity.fingerprint,
        document.identity.byte_size,
        timestamp_text(document.identity.modified_at),
        security_spdx_kind_label(document.kind),
        security_scope_text(document.scope.as_ref()),
        document.spdx_version.as_deref().unwrap_or("unavailable"),
        document.name.as_deref().unwrap_or("unavailable"),
        document.namespace.as_deref().unwrap_or("unavailable"),
        document.data_license.as_deref().unwrap_or("unavailable"),
        creators,
        document.components.len(),
        document
            .file_count
            .map_or_else(|| "unavailable".into(), |value| value.to_string()),
        document
            .relationship_count
            .map_or_else(|| "unavailable".into(), |value| value.to_string()),
        checksums,
        component,
        limitations,
    )
}

fn selected_security_finding(
    app: &App,
) -> Option<(&yoctui_model::CveReport, &yoctui_model::CveFinding)> {
    let identity = app.security.finding_selection.as_ref()?;
    app.security
        .inventory
        .reports()?
        .iter()
        .find_map(|report| match report {
            SecurityReport::Cve(report) => report
                .findings
                .iter()
                .find(|finding| &finding.identity == identity)
                .map(|finding| (report, finding)),
            SecurityReport::Spdx(_) => None,
        })
}

fn cve_source_for_finding<'a>(
    app: &'a App,
    identity: &yoctui_model::CveFindingIdentity,
) -> Option<&'a yoctui_model::CveReport> {
    app.security
        .inventory
        .reports()?
        .iter()
        .find_map(|report| match report {
            SecurityReport::Cve(report)
                if report
                    .findings
                    .iter()
                    .any(|finding| &finding.identity == identity) =>
            {
                Some(report)
            }
            _ => None,
        })
}

fn display_security_metadata(values: &[yoctui_model::SecurityMetadata]) -> String {
    if values.is_empty() {
        "unavailable".into()
    } else {
        values
            .iter()
            .map(|value| format!("{}={}", value.key, value.value))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn display_security_paths(paths: &[std::path::PathBuf]) -> String {
    if paths.is_empty() {
        "unavailable".into()
    } else {
        paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn security_dialog(frame: &mut Frame, app: &App, dialog: &SecurityDialog, area: Rect) {
    let import_field_width = usize::from(area.width.saturating_sub(6)).saturating_mul(2);
    let (title, text, height) = match dialog {
        SecurityDialog::Operation(preview) => {
            let roots = display_security_paths(&preview.report_roots);
            (
                format!("Confirm {}", security_operation_label(&preview.operation)),
                format!(
                    "Session: {}\nScope: {}\n\nExact indexed shell-free operation:\n{}\n\nAuthoritative report roots:\n{}\n\nEnter starts; Esc cancels.",
                    preview.id.0,
                    security_scope_text(Some(&preview.scope)),
                    preview.indexed_arguments.join("\n"),
                    roots,
                ),
                18,
            )
        }
        SecurityDialog::Import { input } => (
            "Import Security reports".into(),
            format!(
                "Normalized absolute CVE JSON/text, SPDX JSON/archive, or bounded directory:\n{}_\n\nOnly this exact canonical non-symlink path is scanned.\nEnter imports; Esc cancels.",
                bounded_security_field(input, import_field_width)
            ),
            11,
        ),
        SecurityDialog::Cancellation(id) => (
            "Confirm Security cancellation".into(),
            format!(
                "Cancel Security session {} only?\n\nEnter requests cancellation; Esc keeps it running.",
                id.0
            ),
            7,
        ),
    };
    let popup = qemu_popup_rect(area, 94, height);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().title(title).borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn bounded_security_field(value: &str, max_chars: usize) -> String {
    let mut characters = value.chars();
    let mut bounded = characters.by_ref().take(max_chars).collect::<String>();
    if characters.next().is_some() {
        bounded.push('…');
    }
    bounded
}

fn security_operation_label(operation: &SecurityOperation) -> &'static str {
    match operation {
        SecurityOperation::CveCheck(_) => "CVE check",
        SecurityOperation::SbomBuild(_) => "SBOM generation",
        SecurityOperation::PackageMap { .. } => "CVE package mapping",
    }
}

fn security_filter_label(filter: yoctui_model::CveStatusFilter) -> &'static str {
    match filter {
        yoctui_model::CveStatusFilter::All => "all",
        yoctui_model::CveStatusFilter::Vulnerable => "vulnerable",
        yoctui_model::CveStatusFilter::Patched => "patched",
        yoctui_model::CveStatusFilter::Ignored => "ignored",
        yoctui_model::CveStatusFilter::NotAffected => "not affected",
        yoctui_model::CveStatusFilter::Unknown => "unknown",
    }
}

fn security_cve_status_label(status: yoctui_model::CveStatus) -> &'static str {
    match status {
        yoctui_model::CveStatus::Vulnerable => "vulnerable",
        yoctui_model::CveStatus::Patched => "patched",
        yoctui_model::CveStatus::Ignored => "ignored",
        yoctui_model::CveStatus::NotAffected => "not affected",
        yoctui_model::CveStatus::Unknown => "unknown",
    }
}

fn security_spdx_kind_label(kind: SpdxArtifactKind) -> &'static str {
    match kind {
        SpdxArtifactKind::Json => "JSON",
        SpdxArtifactKind::Archive => "archive",
    }
}

fn security_session_status_label(status: SecuritySessionStatus) -> &'static str {
    match status {
        SecuritySessionStatus::Starting => "starting",
        SecuritySessionStatus::Running => "running",
        SecuritySessionStatus::Cancelling => "cancelling",
        SecuritySessionStatus::Succeeded => "succeeded",
        SecuritySessionStatus::Failed => "failed",
        SecuritySessionStatus::Cancelled => "cancelled",
        SecuritySessionStatus::TimedOut => "timed out",
        SecuritySessionStatus::Lost => "lost",
    }
}

fn security_info_style(palette: &ThemePalette) -> Style {
    palette.role(palette.info, Modifier::ITALIC)
}

fn security_warning_style(palette: &ThemePalette) -> Style {
    palette.role(palette.warning, Modifier::BOLD)
}

fn security_error_style(palette: &ThemePalette) -> Style {
    palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
}

fn security_cve_status_style(palette: &ThemePalette, status: yoctui_model::CveStatus) -> Style {
    match status {
        yoctui_model::CveStatus::Vulnerable => security_error_style(palette),
        yoctui_model::CveStatus::Patched | yoctui_model::CveStatus::NotAffected => {
            palette.role(palette.success, Modifier::BOLD)
        }
        yoctui_model::CveStatus::Ignored | yoctui_model::CveStatus::Unknown => {
            security_warning_style(palette)
        }
    }
}

fn security_session_style(palette: &ThemePalette, status: SecuritySessionStatus) -> Style {
    match status {
        SecuritySessionStatus::Succeeded => palette.role(palette.success, Modifier::BOLD),
        SecuritySessionStatus::Failed | SecuritySessionStatus::Lost => {
            security_error_style(palette)
        }
        SecuritySessionStatus::Cancelled | SecuritySessionStatus::TimedOut => {
            security_warning_style(palette)
        }
        SecuritySessionStatus::Starting
        | SecuritySessionStatus::Running
        | SecuritySessionStatus::Cancelling => palette.role(palette.progress, Modifier::BOLD),
    }
}

fn testing_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let active = |view| {
        if app.test_view == view {
            palette.focus()
        } else {
            Style::default()
        }
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Launches ", active(TestWorkspaceView::Launches)),
            Span::raw(" | "),
            Span::styled(" Results ", active(TestWorkspaceView::Results)),
            Span::raw(" | "),
            Span::styled(" Comparison ", active(TestWorkspaceView::Comparison)),
        ]),
        Line::from(format!(
            "MACHINE={} | DISTRO={} | image={} | resulttool={}",
            app.workspace
                .variables
                .get("MACHINE")
                .map_or("unavailable", String::as_str),
            app.workspace
                .variables
                .get("DISTRO")
                .map_or("unavailable", String::as_str),
            app.build.target.as_deref().unwrap_or("unavailable"),
            resulttool_capability_label(&app.result_tool_capability),
        )),
        Line::from(""),
    ];
    match app.test_view {
        TestWorkspaceView::Launches => testing_launch_lines(app, &palette, &mut lines),
        TestWorkspaceView::Results => testing_result_lines(app, &palette, &mut lines),
        TestWorkspaceView::Comparison => testing_comparison_lines(app, &palette, &mut lines),
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(
                app,
                "Testing",
                app.focus == FocusTarget::Workspace,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn testing_inspector_text(app: &App) -> String {
    match app.test_view {
        TestWorkspaceView::Launches => testing_launch_inspector(app),
        TestWorkspaceView::Results => testing_result_inspector(app),
        TestWorkspaceView::Comparison => testing_comparison_inspector(app),
    }
}

fn testing_launch_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(
        "  Family                  Authority / availability",
    ));
    for family in yoctui_model::TestFamily::ALL {
        let selected = family == app.test_family_selection;
        lines.push(
            Line::from(format!(
                "{} {:<23} {}",
                if selected { "▶" } else { " " },
                family.label(),
                test_family_capability(app, family),
            ))
            .style(if selected {
                palette.selected()
            } else {
                Style::default()
            }),
        );
    }
    lines.push(Line::from(""));
    match app.latest_test_session() {
        None => lines.push(Line::from("No Testing session has run.")),
        Some(session) => {
            let status = session
                .background_job_id
                .and_then(|id| app.background_jobs.get(id))
                .map_or_else(
                    || {
                        session.outcome.map_or_else(
                            || "awaiting runner attachment".into(),
                            |outcome| format!("{outcome:?}"),
                        )
                    },
                    |job| format!("{:?}", job.status),
                );
            lines.push(Line::from(format!(
                "Latest session {} | {} | {status} | exit={} | structured results={}",
                session.id.0,
                session.operation.family().label(),
                session
                    .exit_code
                    .map_or_else(|| "—".into(), |code| code.to_string()),
                session.result_paths.len(),
            )));
            if let Some(detail) = &session.error_detail {
                lines.push(Line::styled(
                    format!("Failure: {detail}"),
                    testing_error_style(palette),
                ));
            }
        }
    }
}

fn testing_result_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    if app.test_result_searching {
        lines.push(Line::from(format!("Search: {}_", app.test_result_query)));
    } else if !app.test_result_query.is_empty() {
        lines.push(Line::from(format!("Search: {}", app.test_result_query)));
    }
    if app.test_result_drilled {
        let Some(record) = app.selected_test_result() else {
            lines.push(Line::styled(
                "Selected result is no longer available.",
                testing_warning_style(palette),
            ));
            return;
        };
        lines.push(Line::from(format!(
            "Result {} | fingerprint {}",
            record.identity.path.display(),
            record.identity.fingerprint
        )));
        lines.push(Line::from("  Status    Duration     Exact suite / case"));
        for suite in &record.suites {
            lines.push(Line::styled(
                format!("  suite                  {}", suite.identity),
                testing_info_style(palette),
            ));
            for case in &suite.cases {
                let selected = app.test_case_selection.as_ref() == Some(&case.identity);
                lines.push(
                    Line::from(format!(
                        "{} {:<9} {:<12} {}/{}",
                        if selected { "▶" } else { " " },
                        format!("{:?}", case.outcome).to_ascii_lowercase(),
                        case.duration
                            .map(format_duration)
                            .unwrap_or_else(|| "—".into()),
                        case.identity.suite,
                        case.identity.case,
                    ))
                    .style(if selected {
                        palette.selected()
                    } else {
                        test_outcome_style(palette, case.outcome)
                    }),
                );
            }
        }
        return;
    }
    lines.push(Line::from(
        "  Family       Machine / image             P/F/S/E/U  Exact result",
    ));
    match &app.test_results {
        TestResultInventoryState::NotLoaded => lines.push(Line::from(
            "Results are not loaded. Press I to import an exact path.",
        )),
        TestResultInventoryState::Loading { request } => lines.push(Line::styled(
            format!("Loading result generation {}…", request.generation),
            testing_info_style(palette),
        )),
        TestResultInventoryState::AvailableEmpty { .. } => {
            lines.push(Line::from("No structured test results were found."))
        }
        TestResultInventoryState::Available { .. } | TestResultInventoryState::Partial { .. } => {
            for record in app.filtered_test_results() {
                let selected = app.test_result_selection.as_ref() == Some(&record.identity);
                let counts = record.counts();
                lines.push(
                    Line::from(format!(
                        "{} {:<12} {:<27} {}/{}/{}/{}/{}  {}",
                        if selected { "▶" } else { " " },
                        record.family.map_or("unknown", |family| family.label()),
                        format!(
                            "{} / {}",
                            record.machine.as_deref().unwrap_or("—"),
                            record.image.as_deref().unwrap_or("—")
                        ),
                        counts.passed,
                        counts.failed,
                        counts.skipped,
                        counts.errors,
                        counts.unknown,
                        record.identity.path.display(),
                    ))
                    .style(if selected {
                        palette.selected()
                    } else if counts.failed + counts.errors > 0 {
                        testing_error_style(palette)
                    } else {
                        Style::default()
                    }),
                );
            }
            if app.filtered_test_results().is_empty() {
                lines.push(Line::from("No results match the active search."));
            }
            if let TestResultInventoryState::Partial { limitations, .. } = &app.test_results {
                lines.push(Line::styled(
                    format!("Partial: {}", limitations.join(" | ")),
                    testing_warning_style(palette),
                ));
            }
        }
        TestResultInventoryState::Failed { message, .. } => lines.push(Line::styled(
            format!("Result import failed: {message}"),
            testing_error_style(palette),
        )),
        TestResultInventoryState::Cancelled { .. } => lines.push(Line::styled(
            "Result import cancelled.",
            testing_warning_style(palette),
        )),
        TestResultInventoryState::TimedOut { .. } => lines.push(Line::styled(
            "Result import timed out.",
            testing_error_style(palette),
        )),
        TestResultInventoryState::Lost { message, .. } => lines.push(Line::styled(
            format!("Result import worker lost: {message}"),
            testing_error_style(palette),
        )),
    }
}

fn testing_comparison_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    match &app.test_comparison {
        TestComparisonState::NotSelected => lines.push(Line::from(
            "No comparison selected. Press c to choose exact results.",
        )),
        TestComparisonState::Loading { request } => lines.push(Line::styled(
            format!(
                "Comparing {} → {}…",
                request.baseline.path.display(),
                request.candidate.path.display()
            ),
            testing_info_style(palette),
        )),
        TestComparisonState::Available { comparison, .. }
        | TestComparisonState::Partial { comparison, .. } => {
            let count = |category| {
                comparison
                    .transitions
                    .iter()
                    .filter(|transition| transition.category == category)
                    .count()
            };
            lines.push(Line::from(format!(
                "regressions={} | new failures={} | new passes={} | removed={} | other={}",
                count(TestComparisonCategory::Regression),
                count(TestComparisonCategory::NewFailure),
                count(TestComparisonCategory::NewPass),
                count(TestComparisonCategory::Removed),
                count(TestComparisonCategory::UnchangedOther),
            )));
            lines.push(Line::from(format!(
                "Baseline: {}",
                comparison.baseline.path.display()
            )));
            lines.push(Line::from(format!(
                "Candidate: {}",
                comparison.candidate.path.display()
            )));
            lines.push(Line::from(
                "  Category          Baseline → candidate  Exact case",
            ));
            for transition in &comparison.transitions {
                let selected = app.test_comparison_selection.as_ref() == Some(&transition.identity);
                lines.push(
                    Line::from(format!(
                        "{} {:<17} {:<9} → {:<9} {}/{}",
                        if selected { "▶" } else { " " },
                        comparison_category_label(transition.category),
                        transition.baseline.map_or_else(
                            || "absent".into(),
                            |value| { format!("{value:?}").to_ascii_lowercase() }
                        ),
                        transition.candidate.map_or_else(
                            || "absent".into(),
                            |value| { format!("{value:?}").to_ascii_lowercase() }
                        ),
                        transition.identity.suite,
                        transition.identity.case,
                    ))
                    .style(if selected {
                        palette.selected()
                    } else {
                        comparison_category_style(palette, transition.category)
                    }),
                );
            }
            if let TestComparisonState::Partial { limitations, .. } = &app.test_comparison {
                lines.push(Line::styled(
                    format!("Partial: {}", limitations.join(" | ")),
                    testing_warning_style(palette),
                ));
            }
        }
        TestComparisonState::Failed { message, .. } => lines.push(Line::styled(
            format!("Comparison failed: {message}"),
            testing_error_style(palette),
        )),
        TestComparisonState::Cancelled { .. } => lines.push(Line::styled(
            "Comparison cancelled.",
            testing_warning_style(palette),
        )),
        TestComparisonState::TimedOut { .. } => lines.push(Line::styled(
            "Comparison timed out.",
            testing_error_style(palette),
        )),
        TestComparisonState::Lost { message, .. } => lines.push(Line::styled(
            format!("Comparison worker lost: {message}"),
            testing_error_style(palette),
        )),
    }
}

fn testing_info_style(palette: &ThemePalette) -> Style {
    palette.role(palette.info, Modifier::ITALIC)
}

fn testing_warning_style(palette: &ThemePalette) -> Style {
    palette.role(palette.warning, Modifier::BOLD)
}

fn testing_error_style(palette: &ThemePalette) -> Style {
    palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
}

fn test_outcome_style(palette: &ThemePalette, outcome: yoctui_model::TestCaseOutcome) -> Style {
    match outcome {
        yoctui_model::TestCaseOutcome::Passed => palette.role(palette.success, Modifier::BOLD),
        yoctui_model::TestCaseOutcome::Skipped | yoctui_model::TestCaseOutcome::Unknown => {
            palette.role(palette.warning, Modifier::ITALIC)
        }
        yoctui_model::TestCaseOutcome::Failed | yoctui_model::TestCaseOutcome::Error => {
            testing_error_style(palette)
        }
    }
}

fn comparison_category_label(category: TestComparisonCategory) -> &'static str {
    match category {
        TestComparisonCategory::Regression => "regression",
        TestComparisonCategory::NewFailure => "new failure",
        TestComparisonCategory::NewPass => "new pass",
        TestComparisonCategory::Removed => "removed",
        TestComparisonCategory::UnchangedOther => "unchanged/other",
    }
}

fn comparison_category_style(palette: &ThemePalette, category: TestComparisonCategory) -> Style {
    match category {
        TestComparisonCategory::Regression | TestComparisonCategory::NewFailure => {
            testing_error_style(palette)
        }
        TestComparisonCategory::NewPass => palette.role(palette.success, Modifier::BOLD),
        TestComparisonCategory::Removed => testing_warning_style(palette),
        TestComparisonCategory::UnchangedOther => Style::default(),
    }
}

fn resulttool_capability_label(capability: &yoctui_model::ResultToolCapability) -> String {
    match capability {
        yoctui_model::ResultToolCapability::NotInspected => "pending".into(),
        yoctui_model::ResultToolCapability::Missing => "missing".into(),
        yoctui_model::ResultToolCapability::Available(path) => path.display().to_string(),
        yoctui_model::ResultToolCapability::Failed(message) => format!("failed: {message}"),
    }
}

fn test_family_capability(app: &App, family: yoctui_model::TestFamily) -> String {
    let executable = |capability: &TestExecutableCapability| match capability {
        TestExecutableCapability::NotInspected => "capability pending".into(),
        TestExecutableCapability::Missing => "executable missing".into(),
        TestExecutableCapability::Available(path) => path.display().to_string(),
        TestExecutableCapability::Failed(message) => format!("inspection failed: {message}"),
    };
    match family {
        yoctui_model::TestFamily::OeSelftest => executable(&app.test_capability.oe_selftest),
        yoctui_model::TestFamily::BitbakeSelftest => {
            executable(&app.test_capability.bitbake_selftest)
        }
        yoctui_model::TestFamily::Ptest => match &app.test_capability.ptest {
            yoctui_model::PtestCapability::NotInspected => "prerequisites pending".into(),
            yoctui_model::PtestCapability::Configured => {
                "Configured do_testimage (ptest suite)".into()
            }
            yoctui_model::PtestCapability::Unavailable(reason) => {
                format!("unavailable: {reason}")
            }
            yoctui_model::PtestCapability::Failed(message) => {
                format!("inspection failed: {message}")
            }
        },
        family => format!(
            "managed BitBake do_{}",
            family.task().unwrap_or("unavailable")
        ),
    }
}

fn testing_launch_inspector(app: &App) -> String {
    let family = app.test_family_selection;
    let latest = app.latest_test_session().map_or_else(
        || "No Testing session has run.".into(),
        |session| {
            let status = session
                .background_job_id
                .and_then(|id| app.background_jobs.get(id))
                .map_or_else(
                    || {
                        session.outcome.map_or_else(
                            || "awaiting runner attachment".into(),
                            |outcome| format!("{outcome:?}"),
                        )
                    },
                    |job| format!("{:?}", job.status),
                );
            format!(
                "Latest session: {}\nStatus: {status}\nExit: {}\nStructured results: {}\n{}",
                session.id.0,
                session
                    .exit_code
                    .map_or_else(|| "unavailable".into(), |code| code.to_string()),
                session.result_paths.len(),
                session
                    .error_detail
                    .as_deref()
                    .unwrap_or("No error detail.")
            )
        },
    );
    format!(
        "Family: {}\nAuthority: {}\nMACHINE: {}\nDISTRO: {}\nImage: {}\nTask: {}\n\n{}",
        family.label(),
        test_family_capability(app, family),
        app.workspace
            .variables
            .get("MACHINE")
            .map_or("unavailable", String::as_str),
        app.workspace
            .variables
            .get("DISTRO")
            .map_or("unavailable", String::as_str),
        app.build.target.as_deref().unwrap_or("unavailable"),
        family.task().map_or("selftest executable", |task| task),
        latest,
    )
}

fn testing_result_inspector(app: &App) -> String {
    let Some(record) = app.selected_test_result() else {
        return format!(
            "Resulttool: {}\n\nSelect an exact structured result.",
            resulttool_capability_label(&app.result_tool_capability)
        );
    };
    let counts = record.counts();
    let metadata = record
        .metadata
        .iter()
        .map(|entry| format!("{}={}", entry.key, entry.value))
        .collect::<Vec<_>>()
        .join("\n");
    let limitations = if record.limitations.is_empty() {
        "none".into()
    } else {
        record.limitations.join("\n")
    };
    let case = app.selected_test_case().map_or_else(String::new, |case| {
        let metadata = case
            .metadata
            .iter()
            .map(|entry| format!("{}={}", entry.key, entry.value))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "\n\nCase: {}/{}\nStatus: {:?}\nDuration: {}\nRelated log: {}\n{}",
            case.identity.suite,
            case.identity.case,
            case.outcome,
            case.duration
                .map(format_duration)
                .unwrap_or_else(|| "unavailable".into()),
            case.log_path
                .as_deref()
                .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
            metadata,
        )
    });
    format!(
        "Exact path:\n{}\nFingerprint: {}\nBytes: {}\nModified: {}\nFamily: {}\nMachine: {}\nImage: {}\nRevision: {}\nCounts P/F/S/E/U: {}/{}/{}/{}/{}\nDuration: {}\nOriginating session: {}\n\nMetadata:\n{}\n\nLimitations:\n{}{}",
        record.identity.path.display(),
        record.identity.fingerprint,
        record.identity.byte_size,
        timestamp_text(record.identity.modified_at),
        record.family.map_or("unknown", |family| family.label()),
        record.machine.as_deref().unwrap_or("unavailable"),
        record.image.as_deref().unwrap_or("unavailable"),
        record.revision.as_deref().unwrap_or("unavailable"),
        counts.passed,
        counts.failed,
        counts.skipped,
        counts.errors,
        counts.unknown,
        record
            .duration
            .map(format_duration)
            .unwrap_or_else(|| "unavailable".into()),
        record
            .originating_session
            .map_or_else(|| "unavailable".into(), |id| id.0.to_string()),
        if metadata.is_empty() {
            "unavailable"
        } else {
            &metadata
        },
        limitations,
        case,
    )
}

fn testing_comparison_inspector(app: &App) -> String {
    let export = match &app.test_junit_export {
        TestJunitExportState::NotStarted => "not started".into(),
        TestJunitExportState::Inspecting { destination, .. } => {
            format!("validating {}", destination.display())
        }
        TestJunitExportState::Ready(preview) => {
            format!("ready: {}", preview.request.destination.display())
        }
        TestJunitExportState::Running(request) => {
            format!("running: {}", request.destination.display())
        }
        TestJunitExportState::Succeeded(request) => {
            format!("succeeded: {}", request.destination.display())
        }
        TestJunitExportState::Failed { request, message } => {
            format!("failed {}: {message}", request.destination.display())
        }
        TestJunitExportState::Cancelled(request) => {
            format!("cancelled: {}", request.destination.display())
        }
        TestJunitExportState::TimedOut(request) => {
            format!("timed out: {}", request.destination.display())
        }
        TestJunitExportState::Lost { request, message } => {
            format!("lost {}: {message}", request.destination.display())
        }
    };
    app.selected_test_transition().map_or_else(
        || format!("Select an exact comparison transition.\n\nJUnit export: {export}"),
        |transition| {
            format!(
                "Case: {}/{}\nCategory: {}\nBaseline: {}\nCandidate: {}\nBaseline log: {}\nCandidate log: {}\n\nJUnit export: {}",
                transition.identity.suite,
                transition.identity.case,
                comparison_category_label(transition.category),
                transition
                    .baseline
                    .map_or_else(|| "absent".into(), |value| format!("{value:?}")),
                transition
                    .candidate
                    .map_or_else(|| "absent".into(), |value| format!("{value:?}")),
                transition.baseline_log.as_deref().map_or_else(
                    || "unavailable".into(),
                    |path| path.display().to_string()
                ),
                transition.candidate_log.as_deref().map_or_else(
                    || "unavailable".into(),
                    |path| path.display().to_string()
                ),
                export,
            )
        },
    )
}

fn testing_popup(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    text: String,
    preferred_height: u16,
) {
    let popup = qemu_popup_rect(area, 92, preferred_height);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().title(title).borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn test_launch_dialog(frame: &mut Frame, app: &App, dialog: &TestLaunchDialog, area: Rect) {
    let marker = |field| {
        if dialog.selected_field == Some(field) {
            "▶"
        } else {
            " "
        }
    };
    let editing = if dialog.editing { " [editing]" } else { "" };
    let text = format!(
        "Family: {}\nMACHINE: {}\nDISTRO: {}\nImage: {}\n\n{} Scope: {:?}\n{} Selector: {}{}\n{} Parallelism: {}{}\n{} Verbose: {}\n{} Skip network: {}\n\n{}\n↑/↓ field | ←/→ or Enter choice | Enter edit | p preview | Esc cancel",
        dialog.draft.family.label(),
        dialog.draft.machine,
        dialog.draft.distro,
        dialog.draft.image,
        marker(TestLaunchField::Scope),
        dialog.draft.scope,
        marker(TestLaunchField::Selector),
        if dialog.draft.selector.is_empty() {
            "(none)"
        } else {
            &dialog.draft.selector
        },
        if dialog.selected_field == Some(TestLaunchField::Selector) {
            editing
        } else {
            ""
        },
        marker(TestLaunchField::Parallelism),
        dialog.parallelism_input,
        if dialog.selected_field == Some(TestLaunchField::Parallelism) {
            editing
        } else {
            ""
        },
        marker(TestLaunchField::Verbose),
        dialog.draft.verbose,
        marker(TestLaunchField::SkipNetwork),
        dialog.draft.skip_network,
        dialog
            .validation_error
            .as_deref()
            .unwrap_or("Exact typed choices only."),
    );
    testing_popup(frame, app, area, "Testing launch", text, 19);
}

fn test_launch_confirmation(frame: &mut Frame, app: &App, preview: &TestLaunchPreview, area: Rect) {
    let text = match preview {
        TestLaunchPreview::Selftest(request) => format!(
            "Family: {}\nExact indexed shell-free argv:\n{}\nChild-only environment: {}\n\nEnter starts; Esc cancels.",
            request.family.label(),
            request
                .argv()
                .iter()
                .enumerate()
                .map(|(index, value)| format!("[{index}] {}", value.display()))
                .collect::<Vec<_>>()
                .join("\n"),
            if request.skip_network {
                "BB_SKIP_NETTESTS=yes"
            } else {
                "none"
            },
        ),
        TestLaunchPreview::Build {
            family,
            machine,
            distro,
            image,
            request,
        } => format!(
            "Family: {}\nMACHINE: {machine}\nDISTRO: {distro}\nImage: {image}\nExact managed BuildRequest:\ntargets={:?}\ntask={}\nforce={}\n\nEnter starts; Esc cancels.",
            family.label(),
            request.targets,
            request.task.as_deref().unwrap_or("none"),
            request.force,
        ),
    };
    testing_popup(frame, app, area, "Confirm Testing launch", text, 18);
}

fn test_cancellation_confirmation(
    frame: &mut Frame,
    app: &App,
    id: yoctui_model::TestSessionId,
    area: Rect,
) {
    testing_popup(
        frame,
        app,
        area,
        "Confirm Testing cancellation",
        format!(
            "Cancel Testing session {} only?\n\nEnter requests cancellation; Esc keeps it running.",
            id.0
        ),
        7,
    );
}

fn test_result_import_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &yoctui_model::TestResultImportDialog,
    area: Rect,
) {
    testing_popup(
        frame,
        app,
        area,
        "Import structured test results",
        format!(
            "Normalized absolute testresults.json file or retained directory:\n{}_\n\n{}\nEnter imports; Esc cancels.",
            dialog.input,
            dialog
                .validation_error
                .as_deref()
                .unwrap_or("Only the exact selected root is scanned within bounded limits."),
        ),
        10,
    );
}

fn test_comparison_dialog(
    frame: &mut Frame,
    app: &App,
    picker: &yoctui_model::TestComparisonPicker,
    area: Rect,
) {
    let rows = app
        .test_results
        .records()
        .iter()
        .map(|record| {
            format!(
                "{} {} [{}]",
                if picker.cursor.as_ref() == Some(&record.identity) {
                    "▶"
                } else {
                    " "
                },
                record.identity.path.display(),
                record.identity.fingerprint,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    testing_popup(
        frame,
        app,
        area,
        "Choose exact comparison inputs",
        format!(
            "Active field: {:?}\nBaseline: {}\nCandidate: {}\n\n{}\n\n{}\nTab field | ↑/↓ choose | Enter set | p preview | Esc cancel",
            picker.active_field,
            picker.baseline.as_ref().map_or_else(
                || "unavailable".into(),
                |value| value.path.display().to_string()
            ),
            picker.candidate.as_ref().map_or_else(
                || "unavailable".into(),
                |value| value.path.display().to_string()
            ),
            rows,
            picker
                .validation_error
                .as_deref()
                .unwrap_or("Baseline and candidate must be distinct."),
        ),
        20,
    );
}

fn test_comparison_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &yoctui_model::TestComparisonPreview,
    area: Rect,
) {
    testing_popup(
        frame,
        app,
        area,
        "Confirm result comparison",
        format!(
            "Baseline:\n{}\nfingerprint: {}\n\nCandidate:\n{}\nfingerprint: {}\n\nExact indexed shell-free argv:\n{}\n\nEnter compares; Esc cancels.",
            preview.request.baseline.path.display(),
            preview.request.baseline.fingerprint,
            preview.request.candidate.path.display(),
            preview.request.candidate.fingerprint,
            preview
                .argv
                .iter()
                .enumerate()
                .map(|(index, value)| format!("[{index}] {}", value.display()))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        22,
    );
}

fn test_junit_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &yoctui_model::TestJunitExportDialog,
    area: Rect,
) {
    testing_popup(
        frame,
        app,
        area,
        "JUnit export destination",
        format!(
            "Result:\n{}\nfingerprint: {}\n\nNew absolute .xml destination:\n{}_\n\n{}\nEnter validates; Esc cancels.",
            dialog.result.path.display(),
            dialog.result.fingerprint,
            dialog.destination_input,
            dialog.validation_error.as_deref().unwrap_or(
                "The destination must not exist and its canonical parent must remain unchanged."
            ),
        ),
        14,
    );
}

fn test_junit_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &yoctui_model::TestJunitExportPreview,
    area: Rect,
) {
    testing_popup(
        frame,
        app,
        area,
        "Confirm JUnit export",
        format!(
            "Result:\n{}\nfingerprint: {}\nDestination:\n{}\n\nExact indexed shell-free argv:\n{}\n\nThis never overwrites. Enter exports; Esc cancels.",
            preview.request.result.path.display(),
            preview.request.result.fingerprint,
            preview.request.destination.display(),
            preview
                .argv
                .iter()
                .enumerate()
                .map(|(index, value)| format!("[{index}] {}", value.display()))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        19,
    );
}

fn sdk_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .map_or("unavailable", String::as_str);
    let distro = app
        .workspace
        .variables
        .get("DISTRO")
        .map_or("unavailable", String::as_str);
    let target = app.build.target.as_deref().unwrap_or("not selected");
    let root = sdk_inventory_root(app);
    let mut lines = vec![
        Line::from(format!(
            "MACHINE {machine} | DISTRO {distro} | image {target}"
        )),
        Line::from(format!("SDK_DEPLOY {root}")),
    ];
    if app.sdk_artifact_searching {
        lines.push(Line::from(format!("Search: {}_", app.sdk_artifact_query)));
    } else if !app.sdk_artifact_query.is_empty() {
        lines.push(Line::from(format!("Search: {}", app.sdk_artifact_query)));
    }
    lines.push(Line::from(
        "  Kind       SDK type     Size       Modified     Published  Artifact",
    ));
    match &app.sdk_artifacts {
        SdkArtifactInventoryState::NotLoaded => {
            lines.push(Line::from(
                "SDK artifacts are not loaded. Press R to scan SDK_DEPLOY.",
            ));
        }
        SdkArtifactInventoryState::Loading { request } => {
            lines.push(Line::from(format!(
                "Loading SDK artifacts (generation {})…",
                request.generation
            )));
        }
        SdkArtifactInventoryState::AvailableEmpty { .. } => {
            lines.push(Line::from(
                "No SDK artifacts were found in the authoritative SDK_DEPLOY root.",
            ));
        }
        SdkArtifactInventoryState::Failed { message, .. } => {
            lines.push(Line::from(format!("SDK artifact scan failed: {message}")));
        }
        SdkArtifactInventoryState::Available { .. } | SdkArtifactInventoryState::Partial { .. } => {
            let artifacts = app.filtered_sdk_artifacts();
            if artifacts.is_empty() {
                lines.push(Line::from("No SDK artifacts match the active search."));
            } else {
                let selected_index = artifacts
                    .iter()
                    .position(|artifact| {
                        app.sdk_artifact_selection.as_ref() == Some(&artifact.identity)
                    })
                    .unwrap_or(0);
                let capacity = usize::from(area.height.saturating_sub(7)).max(1);
                let start = selected_index
                    .saturating_sub(capacity / 2)
                    .min(artifacts.len().saturating_sub(capacity));
                for artifact in artifacts.into_iter().skip(start).take(capacity) {
                    let selected = app.sdk_artifact_selection.as_ref() == Some(&artifact.identity);
                    let published = artifact.published.map_or("unavailable", |published| {
                        if published { "yes" } else { "no" }
                    });
                    let file = artifact.identity.path.file_name().map_or_else(
                        || "unavailable".into(),
                        |name| name.to_string_lossy().into_owned(),
                    );
                    lines.push(
                        Line::from(format!(
                            "{} {:<10} {:<12} {:<10} {:<12} {:<10} {}",
                            if selected { "▶" } else { " " },
                            sdk_kind_label(artifact.kind),
                            sdk_type_label(artifact.sdk_kind),
                            artifact.identity.size_bytes,
                            artifact.identity.modified_unix_seconds,
                            published,
                            file,
                        ))
                        .style(selected_style(app, selected)),
                    );
                }
            }
            if let SdkArtifactInventoryState::Partial { limitations, .. } = &app.sdk_artifacts {
                lines.push(Line::from(format!(
                    "Partial SDK inventory: {} limitation(s); see Inspector.",
                    limitations.len()
                )));
            }
        }
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(app, "SDK", app.focus == FocusTarget::Workspace))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn sdk_capability_text(app: &App) -> String {
    match &app.sdk_tool_capability {
        SdkToolCapability::NotInspected => "not inspected".into(),
        SdkToolCapability::Failed { message } => format!("inspection failed: {message}"),
        SdkToolCapability::Available {
            publish,
            find_sysroot,
            run_native,
        } => format!(
            "oe-publish-sdk: {}\noe-find-native-sysroot: {}\noe-run-native: {}",
            publish
                .as_ref()
                .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
            find_sysroot
                .as_ref()
                .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
            run_native
                .as_ref()
                .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        ),
    }
}

fn sdk_association_text(paths: &[std::path::PathBuf]) -> String {
    if paths.is_empty() {
        "none".into()
    } else {
        paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn background_status_label(status: BackgroundJobStatus) -> &'static str {
    match status {
        BackgroundJobStatus::Queued => "queued",
        BackgroundJobStatus::Starting => "starting",
        BackgroundJobStatus::Running => "running",
        BackgroundJobStatus::Cancelling => "cancelling",
        BackgroundJobStatus::Succeeded => "succeeded",
        BackgroundJobStatus::Failed => "failed",
        BackgroundJobStatus::Cancelled => "cancelled",
        BackgroundJobStatus::Lost => "lost",
    }
}

fn sdk_session_operation_text(operation: &SdkOperation) -> String {
    match operation {
        SdkOperation::Publish(request) => format!(
            "publish\nInstaller: {}\nDestination: {}",
            request.artifact.path.display(),
            request.destination.display()
        ),
        SdkOperation::Native(request) => format!(
            "{:?}\nWorkspace: {}\nRecipe: {}\nTool: {}\nArguments: {}",
            request.mode,
            request
                .extracted_root
                .as_ref()
                .map_or("active build", |path| path
                    .to_str()
                    .unwrap_or("unavailable")),
            request.recipe,
            request.tool.as_deref().unwrap_or("unavailable"),
            if request.arguments.is_empty() {
                "none".into()
            } else {
                request.arguments.join(" ")
            }
        ),
    }
}

fn sdk_session_text(app: &App) -> String {
    let Some(session) = app.latest_sdk_session() else {
        return "Managed SDK operation\nNo publication or native-tool operation has been started."
            .into();
    };
    let Some(job) = app.background_jobs.get(session.background_job_id) else {
        return format!(
            "Managed SDK operation {}\nLifecycle record unavailable.",
            session.id.0
        );
    };
    let mut retained = job.output.iter().rev().take(80).collect::<Vec<_>>();
    retained.reverse();
    let output = if retained.is_empty() {
        "none".into()
    } else {
        retained
            .into_iter()
            .map(|entry| {
                let source = match entry.source {
                    BackgroundJobOutputSource::Backend => "backend",
                    BackgroundJobOutputSource::Stdout => "stdout",
                    BackgroundJobOutputSource::Stderr => "stderr",
                };
                format!(
                    "[{source}] {}{}",
                    entry.message,
                    if entry.truncated { " [truncated]" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let result = job
        .result
        .as_ref()
        .map_or_else(|| "none".into(), |result| result.summary.clone());
    let result_artifacts = job.result.as_ref().map_or_else(
        || "none".into(),
        |result| {
            if result.artifacts.is_empty() {
                "none".into()
            } else {
                result
                    .artifacts
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        },
    );
    let error = job.error.as_ref().map_or_else(
        || session.error_detail.as_deref().unwrap_or("none").into(),
        |error| {
            error.detail.as_ref().map_or_else(
                || error.summary.clone(),
                |detail| format!("{}: {detail}", error.summary),
            )
        },
    );
    let history = app
        .sdk_sessions
        .iter()
        .rev()
        .take(8)
        .filter_map(|record| {
            app.background_jobs
                .get(record.background_job_id)
                .map(|job| {
                    format!(
                        "#{} {} {}",
                        record.id.0,
                        match &record.operation {
                            SdkOperation::Publish(_) => "publish",
                            SdkOperation::Native(_) => "native",
                        },
                        background_status_label(job.status)
                    )
                })
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Managed SDK operation {}\nStatus: {}\n{}\nQueued: {}\nStarted: {}\nFinished: {}\nExit code: {}\nResult: {}\nResult artifacts:\n{}\nError: {}\nRetained output: {} entries (showing latest {})\nDropped output: {} entries\nWarnings: {}  Errors: {}\n\nOutput:\n{}\n\nRecent SDK history ({} retained):\n{}",
        session.id.0,
        background_status_label(job.status),
        sdk_session_operation_text(&session.operation),
        timestamp_text(job.queued_at),
        job.started_at
            .map(timestamp_text)
            .unwrap_or_else(|| "unavailable".into()),
        job.finished_at
            .map(timestamp_text)
            .unwrap_or_else(|| "unavailable".into()),
        session
            .exit_code
            .map_or_else(|| "unavailable".into(), |code| code.to_string()),
        result,
        result_artifacts,
        error,
        job.output.len(),
        job.output.len().min(80),
        job.dropped_output_entries,
        job.warnings,
        job.errors,
        output,
        app.sdk_sessions.len(),
        if history.is_empty() { "none" } else { &history },
    )
}

fn sdk_inspector_text(app: &App) -> String {
    let limitations = match &app.sdk_artifacts {
        SdkArtifactInventoryState::Partial { limitations, .. } => limitations
            .iter()
            .map(|limitation| format!("! {limitation}"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => "none".into(),
    };
    let artifact = app.selected_sdk_artifact().map_or_else(
        || {
            "No SDK artifact selected.\nPress R to scan SDK_DEPLOY or adjust the active search."
                .into()
        },
        |artifact| {
            format!(
                "Path: {}\nKind: {}\nSDK type: {}\nMachine: {}\nHost tuple: {}\nTarget tuple: {}\nSize: {} bytes\nModified: {}s since Unix epoch\nPublished: {}\n\nChecksums:\n{}\n\nManifests:\n{}",
                artifact.identity.path.display(),
                sdk_kind_label(artifact.kind),
                sdk_type_label(artifact.sdk_kind),
                artifact.machine.as_deref().unwrap_or("unavailable"),
                artifact.host_tuple.as_deref().unwrap_or("unavailable"),
                artifact.target_tuple.as_deref().unwrap_or("unavailable"),
                artifact.identity.size_bytes,
                artifact.identity.modified_unix_seconds,
                artifact.published.map_or("unavailable", |published| if published {
                    "yes"
                } else {
                    "no"
                }),
                sdk_association_text(&artifact.checksums),
                sdk_association_text(&artifact.manifests),
            )
        },
    );
    format!(
        "SDK tool capability\n{}\n\n{}\n\nSelected artifact\n{}\n\nScan limitations\n{}",
        sdk_capability_text(app),
        sdk_session_text(app),
        artifact,
        limitations,
    )
}

fn sdk_popup(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    content: String,
    preferred_height: u16,
) {
    let popup = qemu_popup_rect(area, 92, preferred_height);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(content)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(ThemePalette::for_app(app).focus()),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn sdk_build_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &yoctui_model::SdkBuildPreview,
    area: Rect,
) {
    let action = match preview.action {
        SdkBuildAction::Populate(SdkKind::Standard) => "populate standard SDK",
        SdkBuildAction::Populate(SdkKind::Extensible) => "populate extensible SDK",
        SdkBuildAction::Test(SdkKind::Standard) => "run testsdk",
        SdkBuildAction::Test(SdkKind::Extensible) => "run testsdkext",
    };
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK build",
        format!(
            "Action: {action}\nMachine: {}\nDistro: {}\nImage target: {}\nBitBake task: {}\n\nEnter starts the managed BitBake build.\nEsc closes without starting.",
            preview.machine,
            preview.distro,
            preview.image,
            preview.request.task.as_deref().unwrap_or("unavailable"),
        ),
        12,
    );
}

fn sdk_publish_dialog(frame: &mut Frame, app: &App, draft: &SdkPublishDraft, area: Rect) {
    let installer = app.selected_sdk_artifact().map_or_else(
        || "unavailable".into(),
        |artifact| artifact.identity.path.display().to_string(),
    );
    let validation = app
        .sdk_tool_capability
        .publish_executable()
        .and_then(|executable| {
            app.selected_sdk_artifact()
                .ok_or("an installer selection is required")
                .and_then(|artifact| {
                    SdkPublishPreview::new(
                        executable,
                        artifact.identity.clone(),
                        std::path::PathBuf::from(&draft.destination),
                    )
                    .map(|_| ())
                })
        })
        .map_or_else(
            |message| format!("Validation: {message}"),
            |()| "Validation: ready for exact preview".into(),
        );
    sdk_popup(
        frame,
        app,
        area,
        "Publish SDK installer",
        format!(
            "Tool: {}\nInstaller [read-only]: {installer}\nDestination [editing]: {}_\n{validation}\n\nDestination must be an absolute canonical empty directory.\nEnter validates and opens the exact argument preview.\nEsc closes without publishing.",
            app.sdk_tool_capability
                .publish_executable()
                .map_or_else(|_| "unavailable".into(), |path| path.display().to_string()),
            draft.destination,
        ),
        13,
    );
}

fn indexed_path_vector(paths: &[std::path::PathBuf]) -> String {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| format!("[{index}] {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn sdk_publish_confirmation(frame: &mut Frame, app: &App, preview: &SdkPublishPreview, area: Rect) {
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK publication",
        format!(
            "Installer: {}\nDestination: {}\n\nExact indexed shell-free argument vector:\n{}\n\nNo overwrite policy is guessed.\nEnter starts the managed publication job.\nEsc closes without publishing.",
            preview.request.artifact.path.display(),
            preview.request.destination.display(),
            indexed_path_vector(&preview.argv),
        ),
        18,
    );
}

fn sdk_native_dialog(frame: &mut Frame, app: &App, dialog: &SdkNativeDialog, area: Rect) {
    let draft = &dialog.draft;
    let arguments = if dialog.arguments_input.is_empty() {
        "none".into()
    } else {
        dialog.arguments_input.clone()
    };
    let validation = app
        .sdk_tool_capability
        .executable_for(draft.mode)
        .and_then(|executable| {
            yoctui_model::SdkNativePreview::new(yoctui_model::SdkNativeRequest {
                executable,
                mode: draft.mode,
                extracted_root: (!draft.extracted_root.is_empty())
                    .then(|| std::path::PathBuf::from(&draft.extracted_root)),
                recipe: draft.recipe.clone(),
                tool: (draft.mode == SdkNativeMode::RunNative).then(|| draft.tool.clone()),
                arguments: draft.arguments.clone(),
            })
            .map(|_| ())
        })
        .map_or_else(
            |message| format!("Validation: {message}"),
            |()| "Validation: ready for exact preview".into(),
        );
    let row = |field: SdkNativeField, label: &str, value: &str| {
        let marker = if dialog.selected_field == field {
            "▶"
        } else {
            " "
        };
        let editing = if dialog.selected_field == field && dialog.editing {
            " [editing]"
        } else {
            ""
        };
        format!("{marker} {label}: {value}{editing}")
    };
    let validation = dialog
        .validation_error
        .as_ref()
        .map_or(validation, |message| format!("Validation: {message}"));
    sdk_popup(
        frame,
        app,
        area,
        "SDK native tool",
        format!(
            "{}\nExecutable: {}\n{}\n{}\n{}\n{}\n{validation}\n\n↑/↓ select · Enter edit/cycle · ←/→ mode · p preview · Esc close",
            row(SdkNativeField::Mode, "Mode", &format!("{:?}", draft.mode)),
            app.sdk_tool_capability
                .executable_for(draft.mode)
                .map_or_else(|_| "unavailable".into(), |path| path.display().to_string()),
            row(
                SdkNativeField::Workspace,
                "Workspace",
                if draft.extracted_root.is_empty() {
                    "active build"
                } else {
                    &draft.extracted_root
                },
            ),
            row(
                SdkNativeField::Recipe,
                "Recipe",
                if draft.recipe.is_empty() {
                    "unavailable"
                } else {
                    &draft.recipe
                },
            ),
            row(
                SdkNativeField::Tool,
                "Tool",
                if draft.mode == SdkNativeMode::FindSysroot {
                    "not applicable"
                } else if draft.tool.is_empty() {
                    "unavailable"
                } else {
                    &draft.tool
                },
            ),
            row(SdkNativeField::Arguments, "Arguments", &arguments),
        ),
        18,
    );
}

fn sdk_native_confirmation(frame: &mut Frame, app: &App, preview: &SdkNativePreview, area: Rect) {
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK native tool",
        format!(
            "Mode: {:?}\nWorkspace: {}\nRecipe: {}\nTool: {}\n\nExact indexed shell-free argument vector:\n{}\n\nEnvironment changes apply only to the managed child.\nEnter starts the operation.\nEsc closes without starting.",
            preview.request.mode,
            preview
                .request
                .extracted_root
                .as_ref()
                .map_or("active build".into(), |path| path.display().to_string()),
            preview.request.recipe,
            preview.request.tool.as_deref().unwrap_or("not applicable"),
            indexed_path_vector(&preview.argv),
        ),
        19,
    );
}

fn sdk_cancellation_confirmation(frame: &mut Frame, app: &App, id: SdkSessionId, area: Rect) {
    let operation = app
        .sdk_session(id)
        .map_or("unavailable", |session| match &session.operation {
            SdkOperation::Publish(_) => "publication",
            SdkOperation::Native(_) => "native tool",
        });
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK cancellation",
        format!(
            "Cancel managed SDK {operation} operation #{}?\n\nRetained output and the terminal cancellation result remain in SDK history.\nEnter requests cancellation.\nEsc keeps the operation running.",
            id.0
        ),
        9,
    );
}

fn qemu_launch_field_label(field: QemuLaunchField) -> &'static str {
    match field {
        QemuLaunchField::Machine => "Machine",
        QemuLaunchField::Image => "Image",
        QemuLaunchField::Kernel => "Kernel",
        QemuLaunchField::Rootfs => "Root filesystem",
        QemuLaunchField::Networking => "Networking",
        QemuLaunchField::Memory => "Memory MiB",
        QemuLaunchField::Display => "Display",
        QemuLaunchField::Serial => "Serial",
        QemuLaunchField::ExtraArguments => "Extra arguments",
    }
}

fn qemu_launch_field_value(dialog: &QemuLaunchDialog, field: QemuLaunchField) -> String {
    match field {
        QemuLaunchField::Machine => dialog.draft.machine.clone(),
        QemuLaunchField::Image => dialog.draft.image.path.display().to_string(),
        QemuLaunchField::Kernel => {
            if dialog.draft.kernel.is_empty() {
                "not set".into()
            } else {
                dialog.draft.kernel.clone()
            }
        }
        QemuLaunchField::Rootfs => {
            if dialog.draft.rootfs.is_empty() {
                "not set".into()
            } else {
                dialog.draft.rootfs.clone()
            }
        }
        QemuLaunchField::Networking => match dialog.draft.networking {
            QemuNetworkingMode::Slirp => "slirp",
            QemuNetworkingMode::Tap => "tap",
            QemuNetworkingMode::None => "none",
        }
        .into(),
        QemuLaunchField::Memory => dialog.draft.memory_mib.clone(),
        QemuLaunchField::Display => match dialog.draft.display {
            QemuDisplayMode::Graphical => "graphical",
            QemuDisplayMode::Nographic => "nographic",
        }
        .into(),
        QemuLaunchField::Serial => match dialog.draft.serial {
            QemuSerialMode::Stdio => "stdio",
            QemuSerialMode::Telnet => "telnet",
            QemuSerialMode::None => "none",
        }
        .into(),
        QemuLaunchField::ExtraArguments => {
            if dialog.draft.extra_arguments.is_empty() {
                "none".into()
            } else {
                dialog.draft.extra_arguments.clone()
            }
        }
    }
}

fn qemu_launch_dialog(frame: &mut Frame, app: &App, dialog: &QemuLaunchDialog, area: Rect) {
    let popup = qemu_popup_rect(area, 100, 20);
    clear_popup(frame, app, popup);
    let palette = ThemePalette::for_app(app);
    let fields = [
        QemuLaunchField::Machine,
        QemuLaunchField::Image,
        QemuLaunchField::Kernel,
        QemuLaunchField::Rootfs,
        QemuLaunchField::Networking,
        QemuLaunchField::Memory,
        QemuLaunchField::Display,
        QemuLaunchField::Serial,
        QemuLaunchField::ExtraArguments,
    ];
    let rows = fields.into_iter().map(|field| {
        let selected = dialog.selected_field == field;
        let suffix = if field.is_read_only() {
            " [read-only]"
        } else if selected && dialog.editing {
            " [editing]"
        } else {
            ""
        };
        Row::new([
            format!("{}{}", qemu_launch_field_label(field), suffix),
            qemu_launch_field_value(dialog, field),
        ])
        .style(selected_style(app, selected))
    });
    let mut title =
        String::from("Launch runqemu | ↑/↓ field ←/→ choice Enter edit p preview Esc close");
    if popup.width < 80 {
        title = "Launch runqemu | p preview | Esc close".into();
    }
    frame.render_widget(
        Table::new(rows, [Constraint::Length(23), Constraint::Min(1)])
            .header(Row::new(["Field", "Value"]).style(Style::default().bold()))
            .block(Block::default().title(title).borders(Borders::ALL)),
        popup,
    );
    if let Some(message) = &dialog.validation_error {
        let error = Rect::new(
            popup.x.saturating_add(1),
            popup.y + popup.height.saturating_sub(3),
            popup.width.saturating_sub(2),
            2.min(popup.height.saturating_sub(1)),
        );
        frame.render_widget(
            Paragraph::new(format!("Validation: {message}"))
                .style(palette.role(palette.error, Modifier::BOLD))
                .wrap(Wrap { trim: false }),
            error,
        );
    }
}

fn qemu_launch_confirmation(frame: &mut Frame, app: &App, preview: &QemuLaunchPreview, area: Rect) {
    let popup = qemu_popup_rect(area, 100, 18);
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from(format!("Machine: {}", preview.request.machine)),
        Line::from(format!("Image: {}", preview.request.image.image)),
        Line::from(format!(
            "Artifact: {}",
            preview.request.image.path.display()
        )),
        Line::from("Exact argument vector (one argument per line):"),
    ];
    lines.extend(
        preview
            .argv
            .iter()
            .enumerate()
            .map(|(index, argument)| Line::from(format!("[{index}] {}", argument.display()))),
    );
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Enter confirms launch. Esc closes without launch.",
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Confirm managed runqemu launch")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn qemu_cancellation_confirmation(frame: &mut Frame, app: &App, id: QemuSessionId, area: Rect) {
    let popup = qemu_popup_rect(area, 72, 7);
    clear_popup(frame, app, popup);
    let detail = app.qemu_session(id).map_or_else(
        || format!("Session {} is no longer available.", id.0),
        |session| {
            format!(
                "Cancel managed session {}?\nImage: {}\nArtifact: {}",
                id.0,
                session.request.image.image,
                session.request.image.path.display()
            )
        },
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nEnter confirms cancellation. Esc keeps it running."
        ))
        .block(
            Block::default()
                .title("Confirm runqemu cancellation")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        popup,
    );
}

fn wic_field_value(dialog: &WicCreateDialog, field: WicCreateField) -> String {
    match field {
        WicCreateField::Machine => dialog.draft.machine.clone(),
        WicCreateField::Image => dialog.draft.image.clone(),
        WicCreateField::Kickstart => dialog.draft.kickstart.name.clone(),
        WicCreateField::OutputDirectory => dialog.draft.output_directory.clone(),
        WicCreateField::GenerateBmap => if dialog.draft.generate_bmap {
            "yes"
        } else {
            "no"
        }
        .into(),
        WicCreateField::Compression => match dialog.draft.compression {
            WicCompression::None => "none",
            WicCompression::Gzip => "gzip",
            WicCompression::Bzip2 => "bzip2",
            WicCompression::Xz => "xz",
        }
        .into(),
    }
}

fn wic_partition_summary(kickstart: &WicKickstart) -> String {
    if kickstart.partitions.is_empty() {
        return "none reported".into();
    }
    kickstart
        .partitions
        .iter()
        .enumerate()
        .map(|(index, partition)| {
            format!(
                "{}: mount={} fs={} source={} size={} MiB align={} KiB",
                index + 1,
                partition.mount_point.as_deref().unwrap_or("unavailable"),
                partition.filesystem.as_deref().unwrap_or("unavailable"),
                partition.source_plugin.as_deref().unwrap_or("unavailable"),
                partition
                    .size_mib
                    .map_or_else(|| "dynamic".into(), |value| value.to_string()),
                partition
                    .alignment_kib
                    .map_or_else(|| "unavailable".into(), |value| value.to_string()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn wic_limitations(kickstart: &WicKickstart) -> String {
    if kickstart.limitations.is_empty() {
        "none".into()
    } else {
        kickstart.limitations.join("\n")
    }
}

fn wic_create_dialog(frame: &mut Frame, app: &App, dialog: &WicCreateDialog, area: Rect) {
    let popup = qemu_popup_rect(area, 100, 20);
    clear_popup(frame, app, popup);
    let fields = [
        WicCreateField::Machine,
        WicCreateField::Image,
        WicCreateField::Kickstart,
        WicCreateField::OutputDirectory,
        WicCreateField::GenerateBmap,
        WicCreateField::Compression,
    ];
    let labels = [
        "Machine",
        "Image",
        "Kickstart",
        "Output directory",
        "Generate bmap",
        "Compression",
    ];
    let rows = fields.into_iter().zip(labels).map(|(field, label)| {
        let selected = dialog.selected_field == field;
        let marker = if field.is_read_only() {
            " [read-only]"
        } else if selected && dialog.editing {
            " [editing]"
        } else {
            ""
        };
        Row::new([format!("{label}{marker}"), wic_field_value(dialog, field)])
            .style(selected_style(app, selected))
    });
    let title = if popup.width < 80 {
        "Create Wic | p preview | Esc close"
    } else {
        "Create Wic | ↑/↓ field ←/→ choice Enter edit p preview Esc close"
    };
    frame.render_widget(
        Table::new(rows, [Constraint::Length(27), Constraint::Min(1)])
            .header(Row::new(["Field", "Value"]).style(Style::default().bold()))
            .block(Block::default().title(title).borders(Borders::ALL)),
        popup,
    );
    if let Some(message) = &dialog.validation_error {
        let palette = ThemePalette::for_app(app);
        frame.render_widget(
            Paragraph::new(format!("Validation: {message}"))
                .style(palette.role(palette.error, Modifier::BOLD))
                .wrap(Wrap { trim: false }),
            Rect::new(
                popup.x.saturating_add(1),
                popup.y + popup.height.saturating_sub(3),
                popup.width.saturating_sub(2),
                2.min(popup.height.saturating_sub(1)),
            ),
        );
    }
}

fn wic_create_confirmation(frame: &mut Frame, app: &App, preview: &WicCreatePreview, area: Rect) {
    let popup = qemu_popup_rect(area, 100, area.height.saturating_sub(2).min(30));
    clear_popup(frame, app, popup);
    let partitions = wic_partition_summary(&preview.kickstart);
    let limitations = wic_limitations(&preview.kickstart);
    let argv = preview
        .argv
        .iter()
        .enumerate()
        .map(|(index, argument)| format!("[{index}]={}", argument.display()))
        .collect::<Vec<_>>()
        .join("  ");
    let source_line_count = preview.kickstart.source.lines().count();
    let source_limit = if popup.height <= 22 { 2 } else { 6 };
    let mut lines = vec![
        Line::from(format!(
            "Machine: {} | Image: {}",
            preview.request.machine, preview.request.image
        )),
        Line::from(format!(
            "Kickstart: {} | Output: {}",
            preview.request.kickstart.name,
            preview.request.output_directory.display()
        )),
        Line::from(""),
        Line::from(format!(
            "Kickstart source (showing {} of {} lines):",
            source_line_count.min(source_limit),
            source_line_count
        )),
    ];
    let mut source = source_preview(
        &preview
            .kickstart
            .source
            .lines()
            .take(source_limit)
            .collect::<Vec<_>>()
            .join("\n"),
        preview
            .kickstart
            .identity
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("kickstart.wks"),
        app,
    );
    lines.append(&mut source.lines);
    lines.extend([
        Line::from(""),
        Line::from(format!("Partitions: {partitions}")),
        Line::from(format!("Limitations: {limitations}")),
        Line::from(""),
        Line::from(format!("Exact argument vector: {argv}")),
        Line::from(""),
        Line::from("Enter confirms creation. Esc closes."),
    ]);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title("Confirm managed Wic creation")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn wic_device_lines(app: &App, devices: &[WicDevice]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for device in devices {
        let selected = app.wic_device_selection.as_ref() == Some(&device.identity);
        let style = selected_style(app, selected);
        let mounts = if device.descendant_mounts.is_empty() {
            "none".into()
        } else {
            device
                .descendant_mounts
                .iter()
                .map(|mount| mount.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        lines.push(
            Line::from(format!(
                "{} {} | {} | {} | major:minor {}",
                if selected { "▶" } else { " " },
                device.identity.path.display(),
                format_bytes(device.identity.size_bytes),
                device.identity.size_bytes,
                device.identity.major_minor,
            ))
            .style(style),
        );
        lines.push(
            Line::from(format!(
                "  model={} | serial={} | transport={} | removable={} | writable={} | read-only={} | mounts={}",
                device.identity.model.as_deref().unwrap_or("unavailable"),
                device.identity.serial.as_deref().unwrap_or("unavailable"),
                device.identity.transport.as_deref().unwrap_or("unavailable"),
                device.removable,
                device.writable,
                device.read_only,
                mounts,
            ))
            .style(style),
        );
        if let Some(reason) = &device.unavailable_reason {
            lines.push(Line::from(format!("  unavailable: {reason}")).style(style));
        }
    }
    lines
}

fn wic_device_picker(frame: &mut Frame, app: &App, dialog: &WicDevicePickerDialog, area: Rect) {
    let popup = qemu_popup_rect(area, 110, area.height.saturating_sub(2).min(30));
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from(format!(
            "Image: {} | {} bytes | modified {}s",
            dialog.request.image.path.display(),
            dialog.request.image.size_bytes,
            dialog.request.image.modified_unix_seconds,
        )),
        Line::from(
            "Only removable, writable whole devices without mounted descendants are eligible.",
        ),
        Line::from(""),
    ];
    match &app.wic_devices {
        WicDeviceInventoryState::Loading { request } if request == &dialog.request => {
            lines.push(Line::from("Discovering removable whole devices…"));
        }
        WicDeviceInventoryState::Available { request, devices } if request == &dialog.request => {
            if devices.is_empty() {
                lines.push(Line::from(
                    "No eligible removable whole devices were found.",
                ));
            } else {
                lines.extend(wic_device_lines(app, devices));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Discovery limitations: none"));
        }
        WicDeviceInventoryState::Partial {
            request,
            devices,
            limitations,
        } if request == &dialog.request => {
            if devices.is_empty() {
                lines.push(Line::from(
                    "No eligible removable whole devices were found.",
                ));
            } else {
                lines.extend(wic_device_lines(app, devices));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "Discovery limitations: {}",
                limitations.join("; ")
            )));
        }
        WicDeviceInventoryState::Failed { request, message } if request == &dialog.request => {
            lines.push(Line::from(format!("Device discovery failed: {message}")));
        }
        _ => lines.push(Line::from(
            "This device inventory is stale; close and start a new discovery.",
        )),
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "↑/↓ selects. Enter opens the required phrase dialog. Esc closes.",
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Select protected Wic write device")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn wic_write_phrase_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &WicWritePhraseDialog,
    area: Rect,
) {
    let popup = qemu_popup_rect(area, 100, 15);
    clear_popup(frame, app, popup);
    let expected = format!("WRITE {}", dialog.device.path.display());
    let mut lines = vec![
        Line::from(format!(
            "Image: {} | {} bytes",
            dialog.request.image.path.display(),
            dialog.request.image.size_bytes,
        )),
        Line::from(format!(
            "Device: {} | major:minor {} | {} bytes",
            dialog.device.path.display(),
            dialog.device.major_minor,
            dialog.device.size_bytes,
        )),
        Line::from(format!(
            "Model: {} | Serial: {} | Transport: {}",
            dialog.device.model.as_deref().unwrap_or("unavailable"),
            dialog.device.serial.as_deref().unwrap_or("unavailable"),
            dialog.device.transport.as_deref().unwrap_or("unavailable"),
        )),
        Line::from(""),
        Line::from(format!("Required phrase: {expected}")),
        Line::from(format!("Input: {}_", dialog.input)),
    ];
    if let Some(error) = &dialog.validation_error {
        let palette = ThemePalette::for_app(app);
        lines.push(
            Line::from(format!("Validation: {error}"))
                .style(palette.role(palette.error, Modifier::BOLD)),
        );
    }
    lines.extend([
        Line::from(""),
        Line::from(
            "The phrase alone does not write. Enter opens a separate exact command preview.",
        ),
        Line::from("Esc closes without writing."),
    ]);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Confirm protected Wic device identity")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn wic_write_confirmation(frame: &mut Frame, app: &App, preview: &WicWritePreview, area: Rect) {
    let popup = qemu_popup_rect(area, 110, area.height.saturating_sub(2).min(25));
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from("DESTRUCTIVE OPERATION: this overwrites the selected whole device."),
        Line::from(""),
        Line::from(format!(
            "Image: {} | {} bytes | modified {}s",
            preview.request.image.path.display(),
            preview.request.image.size_bytes,
            preview.request.image.modified_unix_seconds,
        )),
        Line::from(format!(
            "Device: {} | major:minor {} | {} bytes",
            preview.request.device.path.display(),
            preview.request.device.major_minor,
            preview.request.device.size_bytes,
        )),
        Line::from(format!(
            "Model: {} | Serial: {} | Transport: {}",
            preview
                .request
                .device
                .model
                .as_deref()
                .unwrap_or("unavailable"),
            preview
                .request
                .device
                .serial
                .as_deref()
                .unwrap_or("unavailable"),
            preview
                .request
                .device
                .transport
                .as_deref()
                .unwrap_or("unavailable"),
        )),
        Line::from(""),
        Line::from("Exact argument vector:"),
    ];
    lines.extend(
        preview
            .argv
            .iter()
            .enumerate()
            .map(|(index, argument)| Line::from(format!("[{index}]={}", argument.display()))),
    );
    lines.extend([
        Line::from(""),
        Line::from("Enter starts WRITE DEVICE. Esc closes without writing."),
    ]);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Final protected Wic device-write preview")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn wic_cancellation_confirmation(
    frame: &mut Frame,
    app: &App,
    id: WicSessionId,
    incomplete_device_warning: bool,
    area: Rect,
) {
    let popup = qemu_popup_rect(area, 84, 10);
    clear_popup(frame, app, popup);
    let detail = app.wic_session(id).map_or_else(
        || format!("Wic operation {} is unavailable.", id.0),
        |session| match &session.operation {
            WicOperation::Create(request) => format!(
                "Cancel Wic creation {}?\nImage: {}\nOutput: {}",
                id.0,
                request.image,
                request.output_directory.display()
            ),
            WicOperation::Write(request) => format!(
                "Cancel Wic device write {}?\nImage: {}\nDevice: {}",
                id.0,
                request.image.path.display(),
                request.device.path.display()
            ),
        },
    );
    let warning = if incomplete_device_warning {
        "\nWARNING: stopping a device write can leave the target incomplete and unusable."
    } else {
        ""
    };
    let title = if incomplete_device_warning {
        "Confirm Wic device-write cancellation"
    } else {
        "Confirm Wic cancellation"
    };
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}{warning}\n\nEnter confirms cancellation. Esc keeps it running."
        ))
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false }),
        popup,
    );
}

fn images_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let recipe_count = app
        .workspace
        .recipes
        .iter()
        .filter(|recipe| recipe.name.contains("image"))
        .count();
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .map_or("unavailable", String::as_str);
    let mut lines = vec![Line::from(format!(
        "MACHINE {machine} | build target {} | {recipe_count} image recipe target(s)",
        app.build.target.as_deref().unwrap_or("not selected")
    ))];
    let recipe_targets = app
        .workspace
        .recipes
        .iter()
        .filter(|recipe| recipe.name.contains("image"))
        .take(8)
        .map(|recipe| recipe.name.as_str())
        .collect::<Vec<_>>();
    lines.push(Line::from(format!(
        "Recipe targets: {}",
        if recipe_targets.is_empty() {
            "none discovered".into()
        } else {
            recipe_targets.join(", ")
        }
    )));
    if app.image_artifact_searching {
        lines.push(Line::from(format!("Search: {}_", app.image_artifact_query)));
    } else if !app.image_artifact_query.is_empty() {
        lines.push(Line::from(format!("Search: {}", app.image_artifact_query)));
    }
    lines.push(Line::from(
        "Image target                 Kind             Size       Timestamp    File",
    ));
    match &app.image_artifacts {
        ImageArtifactInventoryState::NotLoaded => {
            lines.push(Line::from("Artifacts not loaded. Press R to scan."));
        }
        ImageArtifactInventoryState::Loading { .. } => {
            lines.push(Line::from("Loading deployed image artifacts…"));
        }
        ImageArtifactInventoryState::AvailableEmpty { .. } => {
            lines.push(Line::from(
                "No deployed image artifacts were found in DEPLOY_DIR_IMAGE.",
            ));
        }
        ImageArtifactInventoryState::Failed { message, .. } => {
            lines.push(Line::from(format!("Artifact scan failed: {message}")));
        }
        ImageArtifactInventoryState::Available { .. }
        | ImageArtifactInventoryState::Partial { .. } => {
            let artifacts = app.filtered_image_artifacts();
            if artifacts.is_empty() {
                lines.push(Line::from("No artifacts match the active search."));
            }
            for artifact in artifacts {
                let selected = app.image_artifact_selection.as_ref() == Some(&artifact.identity);
                let size = artifact
                    .size_bytes
                    .available()
                    .map_or_else(|| "unavailable".into(), |size| size.to_string());
                let timestamp = artifact
                    .modified_unix_seconds
                    .available()
                    .map_or_else(|| "unavailable".into(), |value| value.to_string());
                let file = artifact
                    .identity
                    .path
                    .file_name()
                    .map_or_else(|| "unavailable".into(), |name| name.to_string_lossy());
                lines.push(
                    Line::from(format!(
                        "{:<28} {:<16} {:<10} {:<12} {}",
                        artifact.identity.image,
                        artifact.kind.label(),
                        size,
                        timestamp,
                        file
                    ))
                    .style(selected_style(app, selected)),
                );
            }
            if let ImageArtifactInventoryState::Partial { limitations, .. } = &app.image_artifacts {
                lines.push(Line::from(format!(
                    "Partial artifact inventory: {} limitation(s); inspect the selected row.",
                    limitations.len()
                )));
            }
        }
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(
                app,
                "Images",
                app.focus == FocusTarget::Workspace,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn image_artifact_inspector_text(app: &App) -> String {
    let paths = |field: &ImageArtifactField<Vec<std::path::PathBuf>>| {
        field.available().map_or_else(
            || "unavailable".into(),
            |paths| {
                if paths.is_empty() {
                    "none".into()
                } else {
                    paths
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            },
        )
    };
    let artifact_text = app.selected_image_artifact().map_or_else(
        || {
            "No deployed image artifact selected.\nUse i to select a buildable image recipe; press R to scan DEPLOY_DIR_IMAGE.".into()
        },
        |artifact| {
            let checksums = artifact.checksums.available().map_or_else(
                || "unavailable".into(),
                |checksums| {
                    if checksums.is_empty() {
                        "none".into()
                    } else {
                        checksums
                            .iter()
                            .map(|checksum| {
                                format!(
                                    "{} {} ({})",
                                    checksum.algorithm,
                                    checksum.digest,
                                    checksum.source.display()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                },
            );
            let deploy = app
                .image_artifacts
                .inventory()
                .and_then(|inventory| inventory.deploy_directory.available())
                .map_or_else(|| "unavailable".into(), |path| path.display().to_string());
            let limitations = match &app.image_artifacts {
                ImageArtifactInventoryState::Partial { limitations, .. } => limitations
                    .iter()
                    .map(|limitation| format!("! {limitation}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => "none".into(),
            };
            format!(
                "Machine: {}\nImage: {}\nKind: {}\nPath: {}\nDeploy directory: {}\nSize: {}\nTimestamp: {}\nLimitations:\n{}\n\nChecksums:\n{}\n\nManifests:\n{}\n\nLicenses:\n{}\n\nSPDX/SBOM:\n{}\n\nWic files:\n{}",
                artifact.identity.machine,
                artifact.identity.image,
                artifact.kind.label(),
                artifact.identity.path.display(),
                deploy,
                artifact
                    .size_bytes
                    .available()
                    .map_or_else(|| "unavailable".into(), |value| format!("{value} bytes")),
                artifact.modified_unix_seconds.available().map_or_else(
                    || "unavailable".into(),
                    |value| format!("{value}s since Unix epoch")
                ),
                limitations,
                checksums,
                paths(&artifact.manifests),
                paths(&artifact.licenses),
                paths(&artifact.spdx),
                paths(&artifact.wic_files),
            )
        },
    );
    format!(
        "runqemu capability\n{}\nLaunch: {}\n\n{}\n{}\nSelected artifact\n{artifact_text}",
        qemu_capability_text(app),
        app.qemu_launch_unavailable_reason()
            .unwrap_or_else(|| "ready for selected artifact (Q)".into()),
        qemu_session_text(app),
        wic_inspector_text(app),
    )
}

fn qemu_capability_text(app: &App) -> String {
    match &app.qemu_capability {
        QemuCapability::NotInspected => "not inspected".into(),
        QemuCapability::MissingTool => "missing runqemu executable".into(),
        QemuCapability::MissingCompatibleImage => "no compatible deployed image".into(),
        QemuCapability::Failed { message } => format!("inspection failed: {message}"),
        QemuCapability::Available {
            executable,
            compatible_images,
        } => format!(
            "available: {}\nCompatible images: {}",
            executable.display(),
            compatible_images.len()
        ),
    }
}

fn qemu_session_text(app: &App) -> String {
    let Some(session) = app.latest_qemu_session() else {
        return "Managed runqemu session\nNo session has been launched.".into();
    };
    let Some(job) = app.background_jobs.get(session.background_job_id) else {
        return format!(
            "Managed runqemu session {}\nLifecycle record unavailable.",
            session.id.0
        );
    };
    let status = match job.status {
        BackgroundJobStatus::Queued => "queued",
        BackgroundJobStatus::Starting => "starting",
        BackgroundJobStatus::Running => "running",
        BackgroundJobStatus::Cancelling => "cancelling",
        BackgroundJobStatus::Succeeded => "succeeded",
        BackgroundJobStatus::Failed => "failed",
        BackgroundJobStatus::Cancelled => "cancelled",
        BackgroundJobStatus::Lost => "lost",
    };
    let mut retained = job.output.iter().rev().take(80).collect::<Vec<_>>();
    retained.reverse();
    let output = if retained.is_empty() {
        "none".into()
    } else {
        retained
            .into_iter()
            .map(|entry| {
                let source = match entry.source {
                    yoctui_model::BackgroundJobOutputSource::Backend => "backend",
                    yoctui_model::BackgroundJobOutputSource::Stdout => "stdout",
                    yoctui_model::BackgroundJobOutputSource::Stderr => "stderr",
                };
                format!(
                    "[{source}] {}{}",
                    entry.message,
                    if entry.truncated { " [truncated]" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let result = job
        .result
        .as_ref()
        .map_or_else(|| "none".into(), |result| result.summary.clone());
    let error = job.error.as_ref().map_or_else(
        || session.error_detail.as_deref().unwrap_or("none").into(),
        |error| {
            error.detail.as_ref().map_or_else(
                || error.summary.clone(),
                |detail| format!("{}: {detail}", error.summary),
            )
        },
    );
    format!(
        "Managed runqemu session {}\nStatus: {}\nMachine: {}\nImage: {}\nArtifact: {}\nNetworking: {:?}\nDisplay: {:?}\nSerial: {:?}\nMemory: {} MiB\nQueued: {}\nStarted: {}\nFinished: {}\nExit code: {}\nResult: {}\nError: {}\nRetained output: {} entries (showing latest {})\nDropped output: {} entries\n\nOutput:\n{}",
        session.id.0,
        status,
        session.request.machine,
        session.request.image.image,
        session.request.image.path.display(),
        session.request.networking,
        session.request.display,
        session.request.serial,
        session.request.memory_mib,
        timestamp_text(job.queued_at),
        job.started_at
            .map(timestamp_text)
            .unwrap_or_else(|| "unavailable".into()),
        job.finished_at
            .map(timestamp_text)
            .unwrap_or_else(|| "unavailable".into()),
        session
            .exit_code
            .map_or_else(|| "unavailable".into(), |code| code.to_string()),
        result,
        error,
        job.output.len(),
        job.output.len().min(80),
        job.dropped_output_entries,
        output,
    )
}

fn wic_inspector_text(app: &App) -> String {
    let capability = match &app.wic_capability {
        WicCapability::NotInspected => "not inspected".into(),
        WicCapability::MissingTool => "missing wic executable".into(),
        WicCapability::MissingKickstarts { .. } => "no kickstarts available".into(),
        WicCapability::Failed { message } => format!("inspection failed: {message}"),
        WicCapability::Available {
            executable,
            kickstarts,
            image_targets,
        } => format!(
            "available: {}\nKickstarts: {}\nImages: {}",
            executable.display(),
            kickstarts.len(),
            image_targets.len()
        ),
    };
    let readiness = app
        .wic_create_unavailable_reason()
        .unwrap_or_else(|| "ready for selected image (W)".into());
    let write_readiness = app.wic_device_write_unavailable_reason().map_or_else(
        || {
            app.selected_wic_write_image().map_or_else(
                |message| message,
                |image| {
                    format!(
                        "ready for protected write of {} ({} bytes) (D)",
                        image.path.display(),
                        image.size_bytes
                    )
                },
            )
        },
        |message| format!("disabled: {message}"),
    );
    if app.latest_wic_session().is_none()
        && matches!(app.wic_outputs, WicOutputInventoryState::NotLoaded)
        && matches!(app.wic_devices, WicDeviceInventoryState::NotLoaded)
        && !matches!(app.wic_capability, WicCapability::Available { .. })
    {
        return format!(
            "Wic: {capability} | Create disabled | Device write: {write_readiness} | Outputs and protected devices not loaded"
        );
    }
    let selected_kickstart = {
        let selected_identity = match app.active_dialog() {
            Some(Dialog::WicCreate(dialog)) => Some(&dialog.draft.kickstart),
            Some(Dialog::WicCreateConfirmation(preview)) => Some(&preview.request.kickstart),
            _ => app
                .latest_wic_session()
                .and_then(|session| match &session.operation {
                    WicOperation::Create(request) => Some(&request.kickstart),
                    WicOperation::Write(_) => None,
                }),
        };
        match &app.wic_capability {
            WicCapability::Available { kickstarts, .. } => selected_identity
                .and_then(|identity| {
                    kickstarts
                        .iter()
                        .find(|kickstart| kickstart.identity == *identity)
                })
                .or_else(|| kickstarts.first()),
            _ => None,
        }
    };
    let session = app.latest_wic_session().map_or_else(
        || "Managed Wic operation\nNo operation has been started.".into(),
        |session| {
            let Some(job) = app.background_jobs.get(session.background_job_id) else {
                return format!("Managed Wic operation {}\nLifecycle unavailable.", session.id.0);
            };
            let request = match &session.operation {
                WicOperation::Create(request) => format!(
                    "create image={} kickstart={} output={}",
                    request.image,
                    request.kickstart.name,
                    request.output_directory.display()
                ),
                WicOperation::Write(request) => format!(
                    "write\nimage={} ({} bytes, modified {}s)\ndevice={} major:minor={} capacity={} bytes model={} serial={} transport={}",
                    request.image.path.display(),
                    request.image.size_bytes,
                    request.image.modified_unix_seconds,
                    request.device.path.display(),
                    request.device.major_minor,
                    request.device.size_bytes,
                    request.device.model.as_deref().unwrap_or("unavailable"),
                    request.device.serial.as_deref().unwrap_or("unavailable"),
                    request.device.transport.as_deref().unwrap_or("unavailable"),
                ),
            };
            let output = job
                .output
                .iter()
                .rev()
                .take(40)
                .map(|entry| {
                    let source = match entry.source {
                        yoctui_model::BackgroundJobOutputSource::Backend => "backend",
                        yoctui_model::BackgroundJobOutputSource::Stdout => "stdout",
                        yoctui_model::BackgroundJobOutputSource::Stderr => "stderr",
                    };
                    format!(
                        "[{source}] {}{}",
                        entry.message,
                        if entry.truncated { " [truncated]" } else { "" }
                    )
                })
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            let result = job
                .result
                .as_ref()
                .map_or_else(|| "none".into(), |result| result.summary.clone());
            let error = job.error.as_ref().map_or_else(
                || session.error_detail.as_deref().unwrap_or("none").into(),
                |error| {
                    error.detail.as_ref().map_or_else(
                        || error.summary.clone(),
                        |detail| format!("{}: {detail}", error.summary),
                    )
                },
            );
            format!(
                "Managed Wic operation {}\nStatus: {}\nRequest: {}\nQueued: {}\nStarted: {}\nFinished: {}\nExit: {}\nResult: {}\nError: {}\nRetained output: {} entries / {} bytes (showing latest {})\nDropped output: {} entries\nWarnings: {} | Errors: {}\nHost telemetry: CPU {} | Disk available {}\nOutput:\n{}",
                session.id.0,
                match job.status {
                    BackgroundJobStatus::Queued => "queued",
                    BackgroundJobStatus::Starting => "starting",
                    BackgroundJobStatus::Running => "running",
                    BackgroundJobStatus::Cancelling => "cancelling",
                    BackgroundJobStatus::Succeeded => "succeeded",
                    BackgroundJobStatus::Failed => "failed",
                    BackgroundJobStatus::Cancelled => "cancelled",
                    BackgroundJobStatus::Lost => "lost",
                },
                request,
                timestamp_text(job.queued_at),
                job.started_at
                    .map(timestamp_text)
                    .unwrap_or_else(|| "unavailable".into()),
                job.finished_at
                    .map(timestamp_text)
                    .unwrap_or_else(|| "unavailable".into()),
                session
                    .exit_code
                    .map_or_else(|| "unavailable".into(), |value| value.to_string()),
                result,
                error,
                job.output.len(),
                job.retained_output_bytes,
                job.output.len().min(40),
                job.dropped_output_entries,
                job.warnings,
                job.errors,
                app.host_telemetry
                    .cpu_utilization_percent
                    .map_or_else(|| "unavailable".into(), |value| format!("{value}%")),
                app.host_telemetry
                    .disk_available_bytes
                    .map_or_else(|| "unavailable".into(), format_bytes),
                if output.is_empty() { "none" } else { &output },
            )
        },
    );
    let outputs = match &app.wic_outputs {
        WicOutputInventoryState::NotLoaded => "not loaded".into(),
        WicOutputInventoryState::Loading { request } => format!(
            "loading generation {} beneath {}",
            request.generation,
            request.output_directory.display()
        ),
        WicOutputInventoryState::Failed { request, message } => format!(
            "failed generation {} beneath {}: {message}",
            request.generation,
            request.output_directory.display()
        ),
        WicOutputInventoryState::Available { request, outputs }
        | WicOutputInventoryState::Partial {
            request, outputs, ..
        } => {
            let rows = if outputs.is_empty() {
                "none generated".into()
            } else {
                outputs
                    .iter()
                    .map(|output| {
                        let selected = app.wic_output_selection.as_ref() == Some(&output.identity);
                        format!(
                            "{} {:?} {} ({} bytes, {}s)",
                            if selected { "▶" } else { " " },
                            output.kind,
                            output.identity.path.display(),
                            output.identity.size_bytes,
                            output.identity.modified_unix_seconds,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            format!(
                "generation {} beneath {}\n{}",
                request.generation,
                request.output_directory.display(),
                rows
            )
        }
    };
    let limitations = match &app.wic_outputs {
        WicOutputInventoryState::Partial { limitations, .. } => limitations.join("\n"),
        _ => "none".into(),
    };
    let devices = match &app.wic_devices {
        WicDeviceInventoryState::NotLoaded => "not loaded".into(),
        WicDeviceInventoryState::Loading { request } => format!(
            "loading generation {} for {}",
            request.generation,
            request.image.path.display()
        ),
        WicDeviceInventoryState::Failed { request, message } => format!(
            "failed generation {} for {}: {message}",
            request.generation,
            request.image.path.display()
        ),
        WicDeviceInventoryState::Available { request, devices }
        | WicDeviceInventoryState::Partial {
            request, devices, ..
        } => {
            let rows = if devices.is_empty() {
                "no eligible removable whole devices".into()
            } else {
                devices
                    .iter()
                    .map(|device| {
                        let selected =
                            app.wic_device_selection.as_ref() == Some(&device.identity);
                        let mounts = if device.descendant_mounts.is_empty() {
                            "none".into()
                        } else {
                            device
                                .descendant_mounts
                                .iter()
                                .map(|mount| mount.display().to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        };
                        format!(
                            "{} {} major:minor={} capacity={} bytes model={} serial={} transport={} removable={} writable={} read-only={} mounts={} unavailable={}",
                            if selected { "▶" } else { " " },
                            device.identity.path.display(),
                            device.identity.major_minor,
                            device.identity.size_bytes,
                            device.identity.model.as_deref().unwrap_or("unavailable"),
                            device.identity.serial.as_deref().unwrap_or("unavailable"),
                            device.identity.transport.as_deref().unwrap_or("unavailable"),
                            device.removable,
                            device.writable,
                            device.read_only,
                            mounts,
                            device.unavailable_reason.as_deref().unwrap_or("none"),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            format!(
                "generation {} for {}\n{}",
                request.generation,
                request.image.path.display(),
                rows
            )
        }
    };
    let device_limitations = match &app.wic_devices {
        WicDeviceInventoryState::Partial { limitations, .. } => limitations.join("\n"),
        _ => "none".into(),
    };
    let kickstart = selected_kickstart.map_or_else(
        || "Selected kickstart\nunavailable".into(),
        |kickstart| {
            format!(
                "Selected kickstart\nName: {}\nPath: {}\nSource (bounded to {} bytes):\n{}\nPartitions:\n{}\nLimitations:\n{}",
                kickstart.identity.name,
                kickstart
                    .identity
                    .path
                    .as_ref()
                    .map_or_else(|| "canned name".into(), |path| path.display().to_string()),
                yoctui_model::MAX_WIC_SOURCE_BYTES,
                if kickstart.source.is_empty() {
                    "empty"
                } else {
                    &kickstart.source
                },
                wic_partition_summary(kickstart),
                wic_limitations(kickstart),
            )
        },
    );
    format!(
        "Wic capability\n{capability}\nCreate: {readiness}\nDevice write: {write_readiness}\n\n{session}\n\nGenerated outputs\n{outputs}\nLimitations: {limitations}\n\nProtected device inventory\n{devices}\nLimitations: {device_limitations}\n\n{kickstart}"
    )
}

fn settings_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let rows = vec![
        ("Theme", format!("{:?}", app.theme)),
        ("Animation speed", format!("{:?}", app.animation_speed)),
        ("Reduced motion", app.reduced_motion.to_string()),
        ("Color", app.color_enabled.to_string()),
        ("Log wrap", app.logs.wrap.to_string()),
        ("Log follow", app.logs.follow.to_string()),
    ];
    let chunks = Layout::vertical([Constraint::Min(8), Constraint::Length(4)]).split(area);
    frame.render_widget(
        Table::new(
            rows.into_iter().enumerate().map(|(index, (name, value))| {
                Row::new([name.to_owned(), value])
                    .style(selected_style(app, index == app.settings_selection))
            }),
            [Constraint::Percentage(55), Constraint::Percentage(45)],
        )
        .header(
            Row::new(["Setting", "Active value"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(if app.settings_dirty {
                    "Settings (not saved)"
                } else {
                    "Settings"
                })
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(
            "↑/↓ or j/k select  ←/→ or Enter change  r retry unsaved changes\nBuild environment: configure a Poky source/build profile, then press V to verify BitBake.\nBuild actions stay disabled until the connection is verified.",
        )
        .block(
            Block::default()
                .title("Settings controls")
                .borders(Borders::ALL)
                .style(ThemePalette::for_app(app).base()),
        )
        .wrap(Wrap { trim: true }),
        chunks[1],
    );
}

fn build_environment_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let status = match &app.build_environment {
        BuildEnvironmentState::Unconfigured => "not configured".to_owned(),
        BuildEnvironmentState::Configured(profile) => format!(
            "configured\nbuild: {}\nsource: {}\nscript: {}",
            profile.build_dir.display(),
            profile.source_dir.display(),
            profile.init_script.display()
        ),
        BuildEnvironmentState::Verifying { profile, .. } => {
            format!("verifying BitBake\nbuild: {}", profile.build_dir.display())
        }
        BuildEnvironmentState::Connected(profile) => format!(
            "connected\nbuild: {}\nsource: {}",
            profile.build_dir.display(),
            profile.source_dir.display()
        ),
        BuildEnvironmentState::Failed { profile, message } => {
            format!("failed: {message}\nbuild: {}", profile.build_dir.display())
        }
    };
    let draft = app.build_environment_draft.as_ref().map(|draft| format!(
        "\n\nEdit profile (field: {:?})\nsource: {}\nbuild: {}\nscript: {}\n\nType path, ↑/↓ select field, s save, Esc cancel.",
        draft.field, draft.source, draft.build, draft.script
    )).unwrap_or_default();
    let images = if app.build_environment.connected() && !app.available_images.is_empty() {
        format!("\n\navailable images:\n{}", app.available_images.join("\n"))
    } else {
        "\n\navailable images: locked until BitBake verification succeeds.".into()
    };
    let text = format!(
        "Build environment\n\n{status}{draft}{images}\n\nChoose e to edit, c to clone Poky, or V to verify BitBake."
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Build environment")
                    .borders(Borders::ALL)
                    .style(ThemePalette::for_app(app).base()),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn build_history(frame: &mut Frame, app: &App, area: Rect) {
    let records = app.build_history.iter().rev().collect::<Vec<_>>();
    let selected = records.get(app.build_history_selection).copied();
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(7)]).split(area);
    frame.render_widget(
        Table::new(
            records.iter().enumerate().map(|(index, record)| {
                Row::new(vec![
                    Cell::from(record.target.as_deref().unwrap_or("unknown")),
                    Cell::from(if record.success { "success" } else { "failed" }),
                    Cell::from(
                        record
                            .exit_code
                            .map_or_else(|| "--".into(), |code| code.to_string()),
                    ),
                    Cell::from(
                        record
                            .elapsed
                            .map_or_else(|| "--:--:--".into(), format_duration),
                    ),
                    Cell::from(record.completed_tasks.to_string()),
                ])
                .style(selected_style(app, index == app.build_history_selection))
            }),
            [
                Constraint::Percentage(35),
                Constraint::Percentage(16),
                Constraint::Percentage(12),
                Constraint::Percentage(18),
                Constraint::Percentage(19),
            ],
        )
        .header(
            Row::new(["Target", "Result", "Exit", "Elapsed", "Tasks"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Build history ({} retained; newest first)",
                    records.len()
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let detail = selected.map_or_else(
        || "No completed builds are retained in this session.".into(),
        |record| {
            format!(
                "Target: {}\nResult: {}\nWarnings: {}  Errors: {}\nCompleted package tasks: {}",
                record.target.as_deref().unwrap_or("unknown"),
                if record.success { "success" } else { "failed" },
                record.warnings,
                record.errors,
                record.completed_tasks,
            )
        },
    );
    frame.render_widget(
        Paragraph::new(detail).block(
            Block::default()
                .title("Selected build")
                .borders(Borders::ALL),
        ),
        chunks[1],
    );
}

fn dependency_identity_text(identity: &DependencyNodeId) -> String {
    match identity {
        DependencyNodeId::Recipe(recipe) => recipe.clone(),
        DependencyNodeId::Task { recipe, task } => format!("{recipe}:{task}"),
    }
}

fn dependency_kind_text(identity: &DependencyNodeId) -> &'static str {
    match identity {
        DependencyNodeId::Recipe(_) => "recipe",
        DependencyNodeId::Task { .. } => "task",
    }
}

fn dependency_edge_kind_text(kind: DependencyEdgeKind) -> &'static str {
    match kind {
        DependencyEdgeKind::Build => "build",
        DependencyEdgeKind::Runtime => "runtime",
        DependencyEdgeKind::Task => "task",
    }
}

fn dependency_edge_context(
    graph: &DependencyGraph,
    selected: &DependencyNodeId,
    incoming: bool,
) -> String {
    let edges = if incoming {
        graph.incoming(selected)
    } else {
        graph.outgoing(selected)
    };
    if edges.is_empty() {
        return "none reported".into();
    }
    let total = edges.len();
    let mut values = edges
        .into_iter()
        .take(8)
        .map(|edge| {
            let identity = if incoming { &edge.from } else { &edge.to };
            format!(
                "{}: {}",
                dependency_edge_kind_text(edge.kind),
                dependency_identity_text(identity)
            )
        })
        .collect::<Vec<_>>();
    if total > values.len() {
        values.push(format!("… {} more", total - values.len()));
    }
    values.join("\n")
}

fn dependency_why_built(graph: &DependencyGraph, selected: &DependencyNodeId) -> String {
    match graph.why_built(selected, 64, 4_096) {
        DependencyPathResult::Found(path) if path.len() == 1 => "root selected".into(),
        DependencyPathResult::Found(path) => {
            let mut text = dependency_identity_text(&path[0]);
            for pair in path.windows(2) {
                let kind = graph
                    .edges
                    .iter()
                    .find(|edge| edge.from == pair[0] && edge.to == pair[1])
                    .map_or("unknown", |edge| dependency_edge_kind_text(edge.kind));
                text.push_str(&format!(
                    "\n  --{kind}--> {}",
                    dependency_identity_text(&pair[1])
                ));
            }
            text
        }
        DependencyPathResult::Unreachable => "unreachable from root".into(),
        DependencyPathResult::LimitReached => "path limit reached".into(),
    }
}

fn dependency_inspector(app: &App) -> String {
    match &app.dependency_graph {
        DependencyGraphState::NotLoaded => {
            "Dependency graph: not loaded\n\nSelect a recipe in Recipes and press g.".into()
        }
        DependencyGraphState::Loading { root } => format!(
            "Dependency graph: loading\nRoot: {}\n\nNo stale graph is shown while the authoritative query runs.",
            dependency_identity_text(root)
        ),
        DependencyGraphState::AvailableEmpty { root } => format!(
            "Root: {}\nState: available-empty\n\nNo dependency edges reported.",
            dependency_identity_text(root)
        ),
        DependencyGraphState::Failed { root, message } => format!(
            "Root: {}\nState: failed\n\n{message}\n\nNo stale graph is presented as current.",
            dependency_identity_text(root)
        ),
        DependencyGraphState::Available(graph) | DependencyGraphState::Partial { graph, .. } => {
            let selected = app
                .dependency_graph_selection
                .as_ref()
                .and_then(|identity| graph.nodes.iter().find(|node| &node.id == identity));
            let Some(node) = selected else {
                return format!(
                    "Root: {}\n\nNo dependency node is selected.",
                    dependency_identity_text(&graph.root)
                );
            };
            let limitations = match &app.dependency_graph {
                DependencyGraphState::Partial { limitations, .. } if !limitations.is_empty() => {
                    limitations
                        .iter()
                        .map(|value| format!("- {value}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                _ => "none".into(),
            };
            format!(
                "Root: {}\nSelected: {} ({})\nProvider: {}\nTask log: {}\n\nReverse / incoming:\n{}\n\nDependencies / outgoing:\n{}\n\nWhy built:\n{}\n\nLimitations:\n{}",
                dependency_identity_text(&graph.root),
                dependency_identity_text(&node.id),
                dependency_kind_text(&node.id),
                node.provider
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                node.log
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                dependency_edge_context(graph, &node.id, true),
                dependency_edge_context(graph, &node.id, false),
                dependency_why_built(graph, &node.id),
                limitations,
            )
        }
    }
}

fn dependencies(frame: &mut Frame, app: &App, area: Rect) {
    let (graph, partial) = match &app.dependency_graph {
        DependencyGraphState::NotLoaded => {
            frame.render_widget(
                Paragraph::new(
                    "Dependency graph is not loaded.\n\nSelect a recipe in Recipes and press g.",
                )
                .block(
                    Block::default()
                        .title("Dependency graph · not loaded")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::Loading { root } => {
            frame.render_widget(
                Paragraph::new(format!(
                    "Loading authoritative dependency graph for {}…\n\nStale rows are hidden.",
                    dependency_identity_text(root)
                ))
                .block(
                    Block::default()
                        .title("Dependency graph · loading")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::AvailableEmpty { root } => {
            frame.render_widget(
                Paragraph::new(format!(
                    "Root: {}\n\nNo dependency edges reported.",
                    dependency_identity_text(root)
                ))
                .block(
                    Block::default()
                        .title("Dependency graph · available-empty")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::Failed { root, message } => {
            frame.render_widget(
                Paragraph::new(format!(
                    "Root: {}\n\n{message}\n\nNo stale graph is presented as current.",
                    dependency_identity_text(root)
                ))
                .block(
                    Block::default()
                        .title("Dependency graph · failed")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::Available(graph) => (graph, false),
        DependencyGraphState::Partial { graph, .. } => (graph, true),
    };

    let mut counts: HashMap<&DependencyNodeId, (usize, usize)> = HashMap::new();
    for edge in &graph.edges {
        counts.entry(&edge.from).or_default().1 += 1;
        counts.entry(&edge.to).or_default().0 += 1;
    }
    let selected_index = app
        .dependency_graph_selection
        .as_ref()
        .and_then(|selected| graph.nodes.iter().position(|node| &node.id == selected))
        .unwrap_or(0);
    let capacity = area.height.saturating_sub(3).max(1) as usize;
    let start = selected_index.saturating_add(1).saturating_sub(capacity);
    let end = graph.nodes.len().min(start.saturating_add(capacity));
    let rows = graph.nodes[start..end].iter().map(|node| {
        let (incoming, outgoing) = counts.get(&node.id).copied().unwrap_or_default();
        Row::new(vec![
            Cell::from(dependency_kind_text(&node.id)),
            Cell::from(dependency_identity_text(&node.id)),
            Cell::from(incoming.to_string()),
            Cell::from(outgoing.to_string()),
        ])
        .style(selected_style(
            app,
            app.dependency_graph_selection.as_ref() == Some(&node.id),
        ))
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Min(18),
                Constraint::Length(4),
                Constraint::Length(4),
            ],
        )
        .header(
            Row::new(["Kind", "Identity", "In", "Out"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Dependency graph: {} · {} nodes · {} edges{}",
                    dependency_identity_text(&graph.root),
                    graph.nodes.len(),
                    graph.edges.len(),
                    if partial { " · partial" } else { "" }
                ))
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn layer_relationships(frame: &mut Frame, app: &App, area: Rect) {
    let text = app.layer_relationships.as_ref().map_or_else(
        || "No layer relationship data is loaded. Open Layers and press i.".into(),
        |relationships| relationships.layers.iter().map(|layer| format!(
            "{} (priority: {})\n  compatible: {}\n  depends: {}\n  overlays: {}\n  appends: {}",
            layer.name, layer.priority.map_or_else(|| "unknown".into(), |value| value.to_string()),
            list_or_none(&layer.compatible), list_or_none(&layer.depends), list_or_none(&layer.overlays), list_or_none(&layer.appends)
        )).collect::<Vec<_>>().join("\n\n"),
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Layer relationships (server supplied)")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".into()
    } else {
        values.join(", ")
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
fn logs(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(6), Constraint::Min(3)]).split(area);
    let log_area = chunks[1];
    let all_visible = app
        .logs
        .filtered()
        .filter(|l| {
            app.screen != Screen::Errors
                || matches!(l.severity, Severity::Warning | Severity::Error)
        })
        .collect::<Vec<_>>();
    let height = log_area.height.saturating_sub(3) as usize;
    let selection = app.logs.selection.min(all_visible.len().saturating_sub(1));
    let end = selection
        .saturating_add(1)
        .max(height)
        .min(all_visible.len());
    let start = end.saturating_sub(height);
    let visible = &all_visible[start..end];
    let mode = format!(
        "{} | {} | {}",
        if app.logs.follow {
            "following"
        } else {
            "paused"
        },
        if app.logs.wrap {
            "wrapped"
        } else {
            "unwrapped"
        },
        app.logs
            .filter
            .map_or_else(|| "all".into(), |severity| format!("{severity:?}"))
            + &format!(
                " | recipe: {} | task: {}",
                app.logs.recipe_filter.as_deref().unwrap_or("all"),
                app.logs.task_filter.as_deref().unwrap_or("all")
            )
            + &format!(
                " | build: {}",
                app.logs.build_filter.as_deref().unwrap_or("all")
            )
    );
    let pressure = if app.logs.dropped > 0 || app.logs.coalesced > 0 {
        format!(
            "{} evicted [W {} E {}], {} coalesced; retained {}/{} bytes",
            app.logs.dropped,
            app.logs.dropped_warnings,
            app.logs.dropped_errors,
            app.logs.coalesced,
            app.logs.retained_bytes,
            app.logs.max_bytes
        )
    } else {
        format!(
            "No eviction or coalescing; retained {}/{} bytes",
            app.logs.retained_bytes, app.logs.max_bytes
        )
    };
    let search = if app.logs.searching {
        format!("search: {}_", app.logs.query)
    } else if app.logs.query.is_empty() {
        "search: none".into()
    } else {
        format!("search: {}", app.logs.query)
    };
    let search = app
        .logs
        .match_position()
        .map_or(search.clone(), |(current, count)| {
            format!("{search} | selected {current}/{count}")
        });
    frame.render_widget(
        Paragraph::new(format!("{mode}\n{pressure}\n{search}"))
            .block(Block::default().title("Log status").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        chunks[0],
    );
    if app.logs.wrap {
        let mut lines = Vec::new();
        for (offset, log) in visible.iter().enumerate() {
            let selected = start + offset == selection;
            for (line_index, message) in log.message.lines().enumerate() {
                let prefix = if line_index == 0 {
                    format!(
                        "{} {:?} {} ",
                        if selected { "▶" } else { " " },
                        log.severity,
                        log.recipe.as_deref().unwrap_or("")
                    )
                } else {
                    "    ".into()
                };
                let style = if selected {
                    selected_log_style(app, log.severity)
                } else {
                    severity_style(app, log.severity)
                };
                lines.push(Line::styled(format!("{prefix}{message}"), style));
            }
        }
        let text = Text::from(lines);
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().title("Logs").borders(Borders::ALL))
                .wrap(Wrap { trim: false }),
            log_area,
        );
        return;
    }
    let rows = visible.iter().enumerate().map(|(offset, l)| {
        let selected = start + offset == selection;
        Row::new(vec![
            Cell::from(format!("{:?}", l.severity)),
            Cell::from(l.recipe.as_deref().unwrap_or("")),
            Cell::from(l.task.as_deref().unwrap_or("")),
            Cell::from(
                l.message
                    .chars()
                    .skip(app.logs.horizontal_offset)
                    .collect::<String>(),
            ),
        ])
        .style(if selected {
            selected_log_style(app, l.severity)
        } else {
            severity_style(app, l.severity)
        })
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(9),
                Constraint::Length(16),
                Constraint::Length(18),
                Constraint::Min(10),
            ],
        )
        .header(
            Row::new(["Level", "Recipe", "Task", "Message"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().title("Logs").borders(Borders::ALL)),
        log_area,
    )
}
fn errors(frame: &mut Frame, app: &App, area: Rect) {
    let errors = app.logs.diagnostics().collect::<Vec<_>>();
    let selected = errors.get(app.error_selection).copied();
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(12)]).split(area);
    let height = chunks[0].height.saturating_sub(3) as usize;
    let selection = app.error_selection.min(errors.len().saturating_sub(1));
    let end = selection.saturating_add(1).max(height).min(errors.len());
    let start = end.saturating_sub(height);
    let rows = errors[start..end].iter().enumerate().map(|(offset, log)| {
        let index = start + offset;
        let diagnostic = log.diagnostic.as_ref();
        Row::new(vec![
            Cell::from(timestamp_text(log.timestamp)),
            Cell::from(format!("{:?}", log.severity)),
            Cell::from(log.recipe.as_deref().unwrap_or("")),
            Cell::from(log.task.as_deref().unwrap_or("")),
            Cell::from(diagnostic.map_or("", |value| value.summary.as_str())),
            Cell::from(log.build.as_deref().unwrap_or("")),
        ])
        .style(if index == selection {
            selected_log_style(app, log.severity)
        } else {
            severity_style(app, log.severity)
        })
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(14),
                Constraint::Length(9),
                Constraint::Length(14),
                Constraint::Length(16),
                Constraint::Min(18),
                Constraint::Length(20),
            ],
        )
        .header(
            Row::new(["Time", "Severity", "Recipe", "Task", "Summary", "Build"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Errors and warnings ({} retained; {} warning / {} error records evicted)",
                    errors.len(),
                    app.logs.dropped_warnings,
                    app.logs.dropped_errors,
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let detail = selected.map_or_else(
        || "No retained warnings or errors.".into(),
        |log| diagnostic_detail(app, log),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nEnter jumps to matching logs.  o opens the selected source log."
        ))
        .block(
            Block::default()
                .title("Selected diagnostic")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        chunks[1],
    );
}

fn diagnostic_detail(app: &App, log: &yoctui_model::LogEntry) -> String {
    let diagnostic = log.diagnostic.as_ref();
    let metadata = diagnostic.map_or_else(
        || "unavailable".into(),
        |value| {
            value
                .event_metadata
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join(", ")
        },
    );
    let suggestions = diagnostic.map_or_else(
        || "No suggested actions are available.".into(),
        |value| value.suggestions.join("\n- "),
    );
    let related = app
        .logs
        .diagnostics()
        .filter(|candidate| {
            candidate.id != log.id
                && ((log.recipe.is_some() && candidate.recipe == log.recipe)
                    || (log.task.is_some() && candidate.task == log.task)
                    || (log.build.is_some() && candidate.build == log.build))
        })
        .filter_map(|candidate| candidate.diagnostic.as_ref())
        .map(|value| value.summary.as_str())
        .take(3)
        .collect::<Vec<_>>();
    format!(
        "Category: {}\nSummary: {}\nTime: {}\nBuild: {}\nRecipe: {}  Task: {}\nSource log: {}\nEvent metadata: {}\n\nFull message:\n{}\n\nSuggested actions:\n- {}\n\nRelated diagnostics:\n{}",
        diagnostic.map_or("unavailable", |value| value.category.as_str()),
        diagnostic.map_or("unavailable", |value| value.summary.as_str()),
        timestamp_text(log.timestamp),
        log.build.as_deref().unwrap_or("unavailable"),
        log.recipe.as_deref().unwrap_or("unavailable"),
        log.task.as_deref().unwrap_or("unavailable"),
        log.path
            .as_ref()
            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        metadata,
        log.message,
        suggestions,
        if related.is_empty() {
            "none".into()
        } else {
            related.join("\n")
        },
    )
}
fn recipe_build_state(app: &App, recipe: &str) -> String {
    if let Some(task) = app.tasks.values().find(|task| task.recipe == recipe) {
        return format!("{:?}", task.state).to_ascii_lowercase();
    }
    if let Some(task) = app
        .completed_tasks
        .iter()
        .find(|task| task.task.recipe == recipe)
    {
        return format!("{:?}", task.task.state).to_ascii_lowercase();
    }
    if app.build.target.as_deref() == Some(recipe) {
        return match app.build.status {
            BuildStatus::Idle | BuildStatus::LoadingWorkspace => "idle",
            BuildStatus::Parsing => "parsing",
            BuildStatus::Running => "running",
            BuildStatus::Cancelling => "cancelling",
            BuildStatus::Completed => "succeeded",
            BuildStatus::Cancelled => "cancelled",
            BuildStatus::Failed => "failed",
        }
        .into();
    }
    app.recipe_metadata
        .get(recipe)
        .and_then(|metadata| metadata.build_status)
        .map_or_else(
            || "unavailable".into(),
            |status| {
                match status {
                    RecipeBuildStatus::Idle => "idle",
                    RecipeBuildStatus::Queued => "queued",
                    RecipeBuildStatus::Running => "running",
                    RecipeBuildStatus::Succeeded => "succeeded",
                    RecipeBuildStatus::Failed => "failed",
                    RecipeBuildStatus::Cancelled => "cancelled",
                }
                .into()
            },
        )
}

fn recipe_workspace_state(app: &App, recipe: &Recipe) -> String {
    let identity = recipe.file.as_ref().and_then(|file| {
        file.is_absolute().then(|| RecipeIdentity {
            name: recipe.name.clone(),
            file: file.clone(),
        })
    });
    let Some(identity) = identity else {
        return "unavailable (absolute provider path not reported)".into();
    };
    if app.devtool_status_loading.contains(&identity) {
        return "loading authoritative status…".into();
    }
    let Some(status) = app.devtool_statuses.get(&identity) else {
        return "not inspected; press t (or Enter)".into();
    };
    if status.capability == DevtoolCapability::MissingExecutable {
        return "Devtool executable missing; all actions disabled".into();
    }
    if let Some(error) = &status.error {
        return match error {
            DevtoolStatusError::InvalidRecipeIdentity => {
                "invalid recipe identity; all actions disabled".into()
            }
            DevtoolStatusError::DevtoolFailed { exit_code, message } => format!(
                "status failed (exit {}): {message}; all actions disabled",
                exit_code.map_or_else(|| "unavailable".into(), |code| code.to_string())
            ),
            DevtoolStatusError::MalformedOutput { line } => {
                format!("malformed Devtool output: {line}; all actions disabled")
            }
        };
    }
    let state = match &status.workspace {
        DevtoolWorkspace::NotMember => "not in workspace".into(),
        DevtoolWorkspace::MissingDirectory { source_path } => {
            format!("workspace source missing: {}", source_path.display())
        }
        DevtoolWorkspace::Present {
            source_path,
            recipe_file,
        } => {
            let git = match &status.git {
                DevtoolGitState::Available {
                    branch,
                    head,
                    modified,
                    untracked,
                    conflicted,
                } => format!(
                    "Git branch {}, head {}, {}, modified {modified}, untracked {untracked}, conflicted {conflicted}",
                    branch.as_deref().unwrap_or("detached"),
                    head.as_deref().unwrap_or("initial"),
                    if modified + untracked + conflicted == 0 {
                        "clean"
                    } else {
                        "dirty"
                    }
                ),
                DevtoolGitState::MissingExecutable => "Git executable missing".into(),
                DevtoolGitState::NotRepository => "source is not a Git repository".into(),
                DevtoolGitState::Failed { exit_code, message } => format!(
                    "Git status failed (exit {}): {message}",
                    exit_code.map_or_else(|| "unavailable".into(), |code| code.to_string())
                ),
                DevtoolGitState::Malformed { message } => {
                    format!("malformed Git status: {message}")
                }
                DevtoolGitState::NotApplicable => "Git status not applicable".into(),
            };
            format!(
                "member at {}\nWorkspace recipe: {}\n{git}",
                source_path.display(),
                recipe_file
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string())
            )
        }
    };
    format!("{state}\n{}", devtool_action_status(status))
}

fn devtool_action_status(status: &DevtoolStatus) -> String {
    [
        ("d modify/edit", DevtoolAction::ModifyOrEdit),
        ("u update", DevtoolAction::UpdateRecipe),
        ("F finish", DevtoolAction::Finish),
        ("P deploy", DevtoolAction::Deploy),
        ("D reset", DevtoolAction::Reset),
    ]
    .into_iter()
    .map(|(label, action)| {
        status.disabled_reason(action).map_or_else(
            || format!("{label}: enabled"),
            |reason| format!("{label}: disabled ({reason})"),
        )
    })
    .collect::<Vec<_>>()
    .join(" | ")
}

fn recipe_values(label: &str, values: Option<&Vec<String>>) -> String {
    let value = values.map_or_else(
        || "unavailable".into(),
        |values| {
            if values.is_empty() {
                "none".into()
            } else {
                values.join(", ")
            }
        },
    );
    format!("{label}: {value}")
}

fn recipe_paths(label: &str, values: Option<&Vec<std::path::PathBuf>>) -> String {
    let value = values.map_or_else(
        || "unavailable".into(),
        |values| {
            if values.is_empty() {
                "none".into()
            } else {
                values
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n  ")
            }
        },
    );
    format!("{label}: {value}")
}

fn recipe_inspector(app: &App, recipe: &Recipe) -> String {
    let load_state = if app.recipe_metadata_loading.contains(&recipe.name) {
        "loading selected recipe metadata…".into()
    } else if let Some(error) = app.recipe_metadata_errors.get(&recipe.name) {
        format!("metadata unavailable: {error}")
    } else if app.recipe_metadata.contains_key(&recipe.name) {
        "metadata loaded".into()
    } else {
        "not loaded; press Enter to inspect".into()
    };
    let metadata = app.recipe_metadata.get(&recipe.name);
    let dependencies = app
        .dependencies
        .as_ref()
        .filter(|dependencies| dependencies.recipe == recipe.name);
    let active_tasks = app
        .tasks
        .values()
        .filter(|task| task.recipe == recipe.name)
        .map(|task| format!("{} ({:?})", task.task, task.state))
        .collect::<Vec<_>>();
    let reported_tasks = metadata.and_then(|metadata| metadata.tasks.as_ref());
    let standard_tasks = [
        "clean",
        "cleansstate",
        "devshell",
        "menuconfig",
        "diffconfig",
        "diffsigs",
    ];
    let (enabled, disabled): (Vec<_>, Vec<_>) = standard_tasks.into_iter().partition(|task| {
        reported_tasks.is_some_and(|tasks| {
            let canonical = format!("do_{task}");
            tasks
                .iter()
                .any(|candidate| candidate == task || candidate == &canonical)
        })
    });
    let task_capabilities = if reported_tasks.is_none() {
        "Task actions unavailable until metadata is loaded.".into()
    } else {
        format!(
            "Task actions enabled: {}\nTask actions unavailable: {}",
            if enabled.is_empty() {
                "none".into()
            } else {
                enabled.join(", ")
            },
            if disabled.is_empty() {
                "none".into()
            } else {
                disabled.join(", ")
            }
        )
    };
    let retained_log_count = app
        .tasks
        .values()
        .chain(app.completed_tasks.iter().map(|completed| &completed.task))
        .filter(|task| task.recipe == recipe.name && task.log_path.is_some())
        .count();
    let patch_capability = match metadata.and_then(|value| value.patches.as_ref()) {
        None => "unavailable until metadata is loaded".into(),
        Some(patches) if patches.is_empty() => "unavailable (BitBake reported none)".into(),
        Some(patches) => {
            let local = patches
                .iter()
                .filter(|patch| std::path::Path::new(patch).is_absolute())
                .count();
            if local == 0 {
                "unavailable (remote or unresolved paths)".into()
            } else {
                format!("enabled ({local} local)")
            }
        }
    };
    let navigation_capabilities = format!(
        "Navigation: provider {}; task logs {}; patches {patch_capability}\nDevtool availability is authoritative above; press t to refresh.",
        if recipe.file.is_some() {
            "enabled"
        } else {
            "unavailable (provider path not reported)"
        },
        if retained_log_count == 0 {
            "unavailable (no retained path)".into()
        } else {
            format!("enabled ({retained_log_count})")
        },
    );
    let supports_task = |task: &str| {
        reported_tasks.is_some_and(|tasks| {
            let canonical = format!("do_{task}");
            tasks
                .iter()
                .any(|candidate| candidate == task || candidate == &canonical)
        })
    };
    let qa_capabilities = if reported_tasks.is_none() {
        "QA actions unavailable until authoritative task metadata is loaded.".into()
    } else {
        format!(
            "QA actions: CVE check {}; SPDX generation {}.",
            if supports_task("cve_check") {
                "enabled"
            } else {
                "unavailable (do_cve_check not reported)"
            },
            if supports_task("create_spdx") {
                "enabled"
            } else {
                "unavailable (do_create_spdx not reported)"
            }
        )
    };
    let latest_qa_job = app.background_jobs.jobs.iter().rev().find(|job| {
        job.context.recipe.as_deref() == Some(recipe.name.as_str())
            && matches!(
                job.kind,
                BackgroundJobKind::CveCheck | BackgroundJobKind::Spdx
            )
    });
    let qa_job = latest_qa_job.map_or_else(
        || "Latest QA job: not run.".into(),
        |job| {
            let artifacts = if job
                .result
                .as_ref()
                .is_some_and(|result| !result.artifacts.is_empty())
            {
                job.result
                    .as_ref()
                    .unwrap()
                    .artifacts
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                "none reported".into()
            };
            let detail = job
                .error
                .as_ref()
                .map(|error| error.summary.as_str())
                .or_else(|| job.result.as_ref().map(|result| result.summary.as_str()))
                .or_else(|| job.output.back().map(|entry| entry.message.as_str()))
                .unwrap_or("no retained result yet");
            format!(
                "Latest QA job: {} [{:?}], progress {:?}, warnings {}, errors {}.\nQA result: {detail}; artifacts: {artifacts}.",
                job.title, job.status, job.progress, job.warnings, job.errors
            )
        },
    );
    let devtool_job = app
        .background_jobs
        .jobs
        .iter()
        .rev()
        .find(|job| {
            job.kind == BackgroundJobKind::Devtool
                && job.context.recipe.as_deref() == Some(recipe.name.as_str())
        })
        .map_or_else(
            || "Latest Devtool job: not run.".into(),
            |job| {
                let output = job.output.back().map_or_else(
                    || "no retained output".into(),
                    |entry| {
                        format!(
                            "{:?}: {}{}",
                            entry.source,
                            entry.message,
                            if entry.truncated { " [truncated]" } else { "" }
                        )
                    },
                );
                let outcome = job
                    .result
                    .as_ref()
                    .map(|result| result.summary.as_str())
                    .or_else(|| job.error.as_ref().map(|error| error.summary.as_str()))
                    .unwrap_or("in progress");
                format!(
                    "Latest Devtool job: {} [{:?}].\nDevtool output: {output}; outcome: {outcome}.",
                    job.title, job.status
                )
            },
        );
    let tasks = if active_tasks.is_empty() {
        recipe_values("Tasks", reported_tasks)
    } else {
        format!("Active tasks: {}", active_tasks.join(", "))
    };
    format!(
        "Recipe: {}\nResolved version: {}\nPreferred version: {}\nProvider layer: {}\nProvider file: {}\nAppends: {}\nWorkspace/Devtool: {}\nBuild: {}\nDetail: {load_state}\n{task_capabilities}\n{navigation_capabilities}\n{qa_capabilities}\n{qa_job}\n{devtool_job}\n\nDependencies: {}\nRuntime dependencies: {}\nReverse dependencies: unavailable\n{tasks}\n{}\n{}\n{}\n{}",
        recipe.name,
        recipe.version.as_deref().unwrap_or("unavailable"),
        recipe.preferred_version.as_deref().unwrap_or("unavailable"),
        recipe.layer.as_deref().unwrap_or("unavailable"),
        recipe
            .file
            .as_ref()
            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        recipe
            .append_count
            .map_or_else(|| "unavailable".into(), |count| count.to_string()),
        recipe_workspace_state(app, recipe),
        recipe_build_state(app, &recipe.name),
        dependencies.map_or_else(
            || "unavailable; press g to query".into(),
            |value| if value.build.is_empty() {
                "none".into()
            } else {
                value.build.join(", ")
            }
        ),
        dependencies.map_or_else(
            || "unavailable".into(),
            |value| if value.runtime.is_empty() {
                "none".into()
            } else {
                value.runtime.join(", ")
            }
        ),
        recipe_paths(
            "Metadata sources",
            metadata.and_then(|value| value.sources.as_ref())
        ),
        recipe_values("Patches", metadata.and_then(|value| value.patches.as_ref())),
        recipe_values(
            "Package outputs",
            metadata.and_then(|value| value.packages.as_ref())
        ),
        recipe_values("History", metadata.and_then(|value| value.history.as_ref())),
    )
}

fn packages_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let title = if app.package_searching {
        format!("Packages | search: {}_", app.package_query)
    } else if app.package_query.is_empty() {
        "Packages".into()
    } else {
        format!("Packages | search: {}", app.package_query)
    };
    let block = pane_block(app, &title, app.focus == FocusTarget::Workspace);
    match &app.package_inventory {
        PackageInventoryState::NotLoaded => frame.render_widget(
            Paragraph::new(
                "Package data has not been loaded.\n\nEnter this workspace or press R to query generated pkgdata.",
            )
            .block(block)
            .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::Loading { .. } => frame.render_widget(
            Paragraph::new(
                "Loading authoritative package inventory…\n\nThe workspace remains responsive. Press c to cancel.",
            )
            .block(block)
            .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::AvailableEmpty { .. } => frame.render_widget(
            Paragraph::new("No built runtime packages were reported by oe-pkgdata-util.")
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::Failed { message, .. } => frame.render_widget(
            Paragraph::new(format!(
                "Package inventory failed.\n\n{message}\n\nIf generated pkgdata is missing, build a target through do_package and press R."
            ))
            .block(block)
            .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::Available { .. } | PackageInventoryState::Partial { .. } => {
            let limitations = match &app.package_inventory {
                PackageInventoryState::Partial { limitations, .. } => limitations.as_slice(),
                _ => &[],
            };
            let rows_area = if limitations.is_empty() || area.height < 10 {
                area
            } else {
                Layout::vertical([Constraint::Min(4), Constraint::Length(4)]).split(area)[0]
            };
            let rows = app
                .filtered_packages()
                .into_iter()
                .map(|package| {
                    let selected = app.package_selection.as_ref() == Some(&package.identity);
                    let style = if selected {
                        palette.selected()
                    } else {
                        palette.base()
                    };
                    let name = package.identity.name.clone();
                    let recipe = package_field_text(&package.recipe);
                    let version = package_field_text(&package.version);
                    let size = package
                        .installed_size_bytes
                        .available()
                        .map_or_else(|| "unavailable".into(), |value| format_bytes(*value));
                    let license = package_field_text(&package.license);
                    Row::new(vec![name, recipe, version, size, license]).style(style)
                })
                .collect::<Vec<_>>();
            let widths = if area.width >= 68 {
                vec![
                    Constraint::Percentage(27),
                    Constraint::Percentage(22),
                    Constraint::Percentage(18),
                    Constraint::Percentage(14),
                    Constraint::Percentage(19),
                ]
            } else if area.width >= 44 {
                vec![
                    Constraint::Percentage(42),
                    Constraint::Percentage(33),
                    Constraint::Percentage(25),
                    Constraint::Length(0),
                    Constraint::Length(0),
                ]
            } else {
                vec![
                    Constraint::Percentage(100),
                    Constraint::Length(0),
                    Constraint::Length(0),
                    Constraint::Length(0),
                    Constraint::Length(0),
                ]
            };
            let header = Row::new(vec!["Package", "Recipe", "Version", "Size", "License"])
                .style(palette.role(palette.accent, Modifier::BOLD));
            frame.render_widget(
                Table::new(rows, widths)
                    .header(header)
                    .block(block)
                    .column_spacing(1),
                rows_area,
            );
            if !limitations.is_empty() && area.height >= 10 {
                let split =
                    Layout::vertical([Constraint::Min(4), Constraint::Length(4)]).split(area);
                frame.render_widget(
                    Paragraph::new(
                        limitations
                            .iter()
                            .take(2)
                            .map(|value| format!("! {value}"))
                            .collect::<Vec<_>>()
                            .join("\n"),
                    )
                    .style(palette.role(palette.warning, Modifier::BOLD))
                    .block(Block::default().title("Partial result").borders(Borders::ALL))
                    .wrap(Wrap { trim: true }),
                    split[1],
                );
            }
        }
    }
}

fn package_field_text(field: &PackageField<String>) -> String {
    match field {
        PackageField::Available(value) if value.is_empty() => "empty".into(),
        PackageField::Available(value) => value.clone(),
        PackageField::Unavailable => "unavailable".into(),
    }
}

fn package_path_field_text(field: &PackageField<std::path::PathBuf>) -> String {
    match field {
        PackageField::Available(value) => value.display().to_string(),
        PackageField::Unavailable => "unavailable".into(),
    }
}

fn package_identity_list(
    field: &PackageField<Vec<PackageIdentity>>,
    selected: Option<&PackageIdentity>,
) -> Vec<String> {
    match field {
        PackageField::Unavailable => vec!["  unavailable".into()],
        PackageField::Available(values) if values.is_empty() => vec!["  empty".into()],
        PackageField::Available(values) => values
            .iter()
            .map(|identity| {
                format!(
                    "{} {}",
                    if selected == Some(identity) {
                        "▶"
                    } else {
                        " "
                    },
                    identity.name
                )
            })
            .collect(),
    }
}

fn package_path_list(field: &PackageField<Vec<std::path::PathBuf>>) -> Vec<String> {
    match field {
        PackageField::Unavailable => vec!["  unavailable".into()],
        PackageField::Available(values) if values.is_empty() => vec!["  empty".into()],
        PackageField::Available(values) => values
            .iter()
            .take(64)
            .map(|path| format!("  {}", path.display()))
            .collect(),
    }
}

fn package_membership_text(field: &PackageField<Vec<String>>) -> String {
    match field {
        PackageField::Unavailable => "unavailable".into(),
        PackageField::Available(values) if values.is_empty() => "empty".into(),
        PackageField::Available(values) => values.join(", "),
    }
}

fn package_inspector_text(app: &App) -> String {
    let Some(package) = app.selected_package() else {
        return match &app.package_inventory {
            PackageInventoryState::Loading { .. } => {
                "Package inventory is loading.\n\nNo package is selected yet.".into()
            }
            PackageInventoryState::Failed { message, .. } => {
                format!("Package inventory failed.\n\n{message}")
            }
            PackageInventoryState::AvailableEmpty { .. } => {
                "The authoritative package inventory is empty.".into()
            }
            _ => "Select a package to inspect its typed metadata.".into(),
        };
    };
    let mut lines = vec![
        format!("Package: {}", package.identity.name),
        format!("Recipe: {}", package_field_text(&package.recipe)),
        format!("Provider: {}", package_path_field_text(&package.provider)),
        format!("Version: {}", package_field_text(&package.version)),
        format!(
            "Installed size: {}",
            package
                .installed_size_bytes
                .available()
                .map_or_else(|| "unavailable".into(), |value| format_bytes(*value))
        ),
        format!("License: {}", package_field_text(&package.license)),
        format!(
            "Image membership: {}",
            package_membership_text(&package.image_membership)
        ),
        String::new(),
    ];
    match app.selected_package_detail() {
        None | Some(PackageDetailState::NotLoaded) => {
            lines.push("Detail: not loaded (press Enter)".into());
        }
        Some(PackageDetailState::Loading { .. }) => {
            lines.push("Detail: loading… (press c to cancel)".into());
        }
        Some(PackageDetailState::Failed { message, .. }) => {
            lines.push(format!("Detail: failed\n{message}"));
        }
        Some(PackageDetailState::AvailableEmpty { .. }) => {
            lines.push("Detail: available-empty".into());
            lines.push("Files: empty".into());
            lines.push("Runtime dependencies: empty".into());
            lines.push("Reverse dependencies: empty".into());
        }
        Some(
            PackageDetailState::Available { detail, .. }
            | PackageDetailState::Partial { detail, .. },
        ) => {
            lines.push("Files:".into());
            lines.extend(package_path_list(&detail.files));
            lines.push(String::new());
            lines.push(format!(
                "{} runtime dependencies:",
                if app.package_dependency_reverse {
                    " "
                } else {
                    "▶"
                }
            ));
            lines.extend(package_identity_list(
                &detail.runtime_dependencies,
                (!app.package_dependency_reverse)
                    .then(|| app.selected_package_dependency())
                    .flatten(),
            ));
            lines.push(String::new());
            lines.push(format!(
                "{} reverse dependencies:",
                if app.package_dependency_reverse {
                    "▶"
                } else {
                    " "
                }
            ));
            lines.extend(package_identity_list(
                &detail.reverse_dependencies,
                app.package_dependency_reverse
                    .then(|| app.selected_package_dependency())
                    .flatten(),
            ));
            if let Some(PackageDetailState::Partial { limitations, .. }) =
                app.selected_package_detail()
            {
                lines.push(String::new());
                lines.push("Partial detail:".into());
                lines.extend(limitations.iter().map(|value| format!("! {value}")));
            }
        }
    }
    if !app.package_navigation.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "Navigation history: {} item(s); press u to return",
            app.package_navigation.len()
        ));
    }
    lines.join("\n")
}

fn recipes(frame: &mut Frame, app: &App, area: Rect) {
    let recipes = app
        .workspace
        .recipes
        .iter()
        .enumerate()
        .filter(|(_, recipe)| {
            matches_metadata(
                &app.metadata_query,
                &[
                    recipe.name.as_str(),
                    recipe.version.as_deref().unwrap_or(""),
                    recipe.preferred_version.as_deref().unwrap_or(""),
                    recipe.layer.as_deref().unwrap_or(""),
                    recipe
                        .file
                        .as_ref()
                        .and_then(|path| path.to_str())
                        .unwrap_or(""),
                ],
            )
        })
        .collect::<Vec<_>>();
    let recipe_count = recipes.len();
    let selected = app.workspace.recipes.get(app.recipe_selection);
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(12)]).split(area);
    frame.render_widget(
        Table::new(
            recipes.into_iter().map(|(index, recipe)| {
                Row::new(vec![
                    Cell::from(recipe.name.as_str()),
                    Cell::from(recipe.version.as_deref().unwrap_or("?")),
                    Cell::from(recipe.preferred_version.as_deref().unwrap_or("?")),
                    Cell::from(recipe.layer.as_deref().unwrap_or("?")),
                    Cell::from(
                        recipe
                            .append_count
                            .map_or_else(|| "?".into(), |count| count.to_string()),
                    ),
                    Cell::from(recipe_workspace_state(app, recipe)),
                    Cell::from(recipe_build_state(app, &recipe.name)),
                ])
                .style(selected_style(app, index == app.recipe_selection))
            }),
            [
                Constraint::Min(12),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(4),
                Constraint::Length(11),
                Constraint::Length(10),
            ],
        )
        .header(
            Row::new([
                "Recipe",
                "Resolved",
                "Preferred",
                "Layer",
                "App",
                "Workspace",
                "Build",
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(metadata_title(
                    format!(
                        "Recipes (shown: {} of {})",
                        recipe_count,
                        app.workspace.recipes.len()
                    ),
                    app,
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let detail = selected.map_or_else(
        || "No recipes supplied by the backend.".into(),
        |recipe| recipe_inspector(app, recipe),
    );
    frame.render_widget(
        Paragraph::new(detail)
            .block(
                Block::default()
                    .title("Selected recipe Inspector")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}
fn git_state_text(state: GitFileState) -> &'static str {
    match state {
        GitFileState::Clean => "clean",
        GitFileState::Modified => "modified",
        GitFileState::Untracked => "untracked",
        GitFileState::Ignored => "ignored/generated",
        GitFileState::Unavailable => "Git unavailable",
    }
}

fn layer_relationship<'a>(
    app: &'a App,
    layer: &str,
) -> Option<&'a yoctui_model::LayerRelationship> {
    app.layer_relationships
        .as_ref()?
        .layers
        .iter()
        .find(|relationship| relationship.name == layer)
}

fn active_build_layer(app: &App, layer: &str) -> bool {
    app.build
        .target
        .as_ref()
        .and_then(|target| {
            app.workspace
                .recipes
                .iter()
                .find(|recipe| &recipe.name == target)
        })
        .and_then(|recipe| recipe.layer.as_deref())
        .is_some_and(|recipe_layer| recipe_layer == layer)
        || app.tasks.values().any(|task| {
            app.workspace
                .recipes
                .iter()
                .find(|recipe| recipe.name == task.recipe)
                .and_then(|recipe| recipe.layer.as_deref())
                .is_some_and(|recipe_layer| recipe_layer == layer)
        })
}

fn layer_entry_metadata(app: &App, browser: &LayerBrowser, entry: &LayerBrowserEntry) -> String {
    let relationship = layer_relationship(app, &browser.layer);
    let size = if entry.is_dir {
        "directory".into()
    } else {
        entry
            .size
            .map_or_else(|| "unavailable".into(), |size| format!("{size} bytes"))
    };
    let modified = entry
        .modified
        .map_or_else(|| "unavailable".into(), timestamp_text);
    let compatibility = relationship.map_or_else(
        || "unavailable".into(),
        |value| {
            if value.compatible.is_empty() {
                "not reported".into()
            } else {
                value.compatible.join(", ")
            }
        },
    );
    format!(
        "Path: {}\nType/size: {size}\nModified: {modified}\nGit: {}\nLayer: {}\nCompatibility: {compatibility}",
        entry.path.display(),
        git_state_text(entry.git),
        browser.layer
    )
}

fn layer_inspector_text(app: &App, browser: &LayerBrowser) -> Text<'static> {
    let Some(entry) = browser.selected_entry() else {
        return Text::from("This layer is empty.");
    };
    let metadata = layer_entry_metadata(app, browser, entry);
    let relationship = layer_relationship(app, &browser.layer);
    match browser.inspector_mode {
        LayerInspectorMode::Git => Text::from(format!(
            "{metadata}\n\nGit state: {}\nGit status is detected per loaded subtree; missing Git is non-fatal.",
            git_state_text(entry.git)
        )),
        LayerInspectorMode::Metadata => Text::from(metadata),
        LayerInspectorMode::Dependencies => Text::from(format!(
            "{metadata}\n\nDepends: {}\nOverlays: {}\nAppends: {}",
            relationship.map_or("unavailable".into(), |value| {
                if value.depends.is_empty() {
                    "none reported".into()
                } else {
                    value.depends.join(", ")
                }
            }),
            relationship.map_or("unavailable".into(), |value| {
                if value.overlays.is_empty() {
                    "none reported".into()
                } else {
                    value.overlays.join(", ")
                }
            }),
            relationship.map_or("unavailable".into(), |value| {
                if value.appends.is_empty() {
                    "none reported".into()
                } else {
                    value.appends.join(", ")
                }
            })
        )),
        LayerInspectorMode::Preview if entry.is_dir => Text::from(format!(
            "{metadata}\n\nDirectory contents are loaded only when expanded."
        )),
        LayerInspectorMode::Preview => match browser.preview_kind {
            PreviewKind::Binary => Text::from(format!(
                "{metadata}\n\nBinary preview unavailable.{}",
                if browser.preview_truncated {
                    "\nPreview exceeds the 64 KiB bound."
                } else {
                    ""
                }
            )),
            PreviewKind::Unavailable => Text::from(format!(
                "{metadata}\n\nPreview unavailable or still loading."
            )),
            PreviewKind::Text => {
                let file_name = entry
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                let mut preview = numbered_source_preview(&browser.preview, file_name, app);
                if browser.preview_truncated {
                    preview.lines.insert(
                        0,
                        Line::from("[preview truncated at 64 KiB]").style(
                            ThemePalette::for_app(app)
                                .role(ThemePalette::for_app(app).warning, Modifier::BOLD),
                        ),
                    );
                }
                preview
            }
        },
    }
}

fn layer_browser(frame: &mut Frame, app: &App, browser: &LayerBrowser, area: Rect) {
    let chunks =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).split(area);
    let layer_height = app.workspace.layers.len().saturating_add(2).min(8) as u16;
    let left =
        Layout::vertical([Constraint::Length(layer_height), Constraint::Min(3)]).split(chunks[0]);
    let palette = ThemePalette::for_app(app);
    let configured = app
        .workspace
        .layers
        .iter()
        .filter(|layer| {
            matches_metadata(
                &app.metadata_query,
                &[layer.name.as_str(), layer.path.to_str().unwrap_or("")],
            )
        })
        .map(|layer| {
            let relationship = layer_relationship(app, &layer.name);
            let compatibility = relationship.map_or("?", |value| {
                if value.compatible.is_empty() {
                    "-"
                } else {
                    "yes"
                }
            });
            let active = active_build_layer(app, &layer.name);
            let style = if layer.name == browser.layer {
                palette.selected()
            } else if active {
                palette.role(palette.success, Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new([
                if active {
                    format!("● {}", layer.name)
                } else {
                    format!("  {}", layer.name)
                },
                layer
                    .priority
                    .map_or_else(|| "?".into(), |priority| priority.to_string()),
                compatibility.into(),
            ])
            .style(style)
        });
    frame.render_widget(
        Table::new(
            configured,
            [
                Constraint::Min(8),
                Constraint::Length(4),
                Constraint::Length(6),
            ],
        )
        .header(Row::new(["Configured layers", "Pri", "Compat"]).style(Style::default().bold()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(palette.base())
                .border_style(palette.focus()),
        ),
        left[0],
    );

    let query = app.metadata_query.to_ascii_lowercase();
    let entries = browser.entries.iter().enumerate().filter(|(_, entry)| {
        query.is_empty()
            || entry
                .path
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains(&query)
    });
    frame.render_widget(
        Table::new(
            entries.map(|(index, entry)| {
                let name = entry.path.file_name().map_or_else(
                    || entry.path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                );
                let marker = if entry.is_dir {
                    if browser.expanded.contains(&entry.path) {
                        "▾"
                    } else {
                        "▸"
                    }
                } else {
                    " "
                };
                let git = match entry.git {
                    GitFileState::Modified => " M",
                    GitFileState::Untracked => " ?",
                    GitFileState::Ignored => " I",
                    GitFileState::Clean => "  ",
                    GitFileState::Unavailable => " -",
                };
                let indent = "  ".repeat(entry.depth);
                Row::new([format!(
                    "{indent}{marker} {name}{}{git}",
                    if entry.is_dir { "/" } else { "" }
                )])
                .style(selected_style(app, index == browser.selection))
            }),
            [Constraint::Min(1)],
        )
        .block(
            Block::default()
                .title(metadata_title(
                    format!(
                        "{} tree | hidden {}",
                        browser.layer,
                        if browser.show_hidden { "on" } else { "off" }
                    ),
                    app,
                ))
                .borders(Borders::ALL),
        ),
        left[1],
    );

    let mode = match browser.inspector_mode {
        LayerInspectorMode::Preview => "Preview",
        LayerInspectorMode::Git => "Git",
        LayerInspectorMode::Metadata => "Metadata",
        LayerInspectorMode::Dependencies => "Dependencies",
    };
    frame.render_widget(
        Paragraph::new(layer_inspector_text(app, browser))
            .block(
                Block::default()
                    .title(format!("Inspector [{mode}]"))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}

fn layers(frame: &mut Frame, app: &App, area: Rect) {
    let layers = app
        .workspace
        .layers
        .iter()
        .filter(|layer| {
            matches_metadata(
                &app.metadata_query,
                &[layer.name.as_str(), layer.path.to_str().unwrap_or("")],
            )
        })
        .collect::<Vec<_>>();
    let selected = layers.get(app.layer_selection).copied();
    let recipes = selected.map_or_else(Vec::new, |layer| {
        let mut recipes = app
            .workspace
            .recipes
            .iter()
            .filter(|recipe| recipe.layer.as_deref() == Some(layer.name.as_str()))
            .collect::<Vec<_>>();
        recipes.sort_by(|left, right| left.name.cmp(&right.name));
        recipes
    });
    let chunks =
        Layout::horizontal([Constraint::Percentage(48), Constraint::Percentage(52)]).split(area);
    frame.render_widget(
        Table::new(
            layers.into_iter().enumerate().map(|(index, layer)| {
                Row::new(vec![
                    Cell::from(format!("▸ {}", layer.name)),
                    Cell::from(layer.path.display().to_string()),
                    Cell::from(
                        layer
                            .priority
                            .map_or_else(String::new, |priority| priority.to_string()),
                    ),
                ])
                .style({
                    let mut style = selected_style(app, index == app.layer_selection);
                    let palette = ThemePalette::for_app(app);
                    if index != app.layer_selection {
                        style = style.fg(palette.success);
                    }
                    if palette.attribute_only {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    style
                })
            }),
            [
                Constraint::Percentage(32),
                Constraint::Percentage(53),
                Constraint::Percentage(15),
            ],
        )
        .header(
            Row::new(["Layer", "Path", "Priority"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(metadata_title(
                    format!(
                        "Active layer tree (shown: {} of {})",
                        app.workspace
                            .layers
                            .iter()
                            .filter(|layer| matches_metadata(
                                &app.metadata_query,
                                &[layer.name.as_str(), layer.path.to_str().unwrap_or("")]
                            ))
                            .count(),
                        app.workspace.layers.len()
                    ),
                    app,
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    frame.render_widget(
        Table::new(
            recipes.iter().map(|recipe| {
                Row::new(vec![
                    Cell::from(recipe.name.as_str()),
                    Cell::from(recipe.version.as_deref().unwrap_or("")),
                ])
            }),
            [Constraint::Percentage(68), Constraint::Percentage(32)],
        )
        .header(
            Row::new(["Recipe in selected layer", "Version"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(selected.map_or_else(
                    || "Layer recipes".into(),
                    |layer| format!("Recipes: {} ({})", layer.name, recipes.len()),
                ))
                .borders(Borders::ALL),
        ),
        chunks[1],
    );
}
fn config_variables(app: &App) -> Vec<(&String, &String)> {
    let mut variables = app.workspace.variables.iter().collect::<Vec<_>>();
    variables.sort_by_key(|(name, _)| *name);
    variables.retain(|(name, value)| {
        matches_metadata(&app.metadata_query, &[name.as_str(), value.as_str()])
    });
    variables
}

fn config_inspector(app: &App) -> String {
    let variables = config_variables(app);
    let Some((name, summary_value)) = variables.get(app.config_selection).copied() else {
        let state = if app.workspace.variables.is_empty() {
            "No configuration variables supplied by the backend."
        } else {
            "No configuration variables match the active search."
        };
        return format!("{state}\n\n{}", config_copy_status(app));
    };
    let identity = VariableIdentity {
        name: name.clone(),
        recipe: app.config_scope.clone(),
    };
    if app.variable_detail_loading.contains(&identity) {
        return format!(
            "Variable: {name}\nEffective summary: {summary_value}\n\nLoading authoritative detail…\n\n{}",
            config_copy_status(app)
        );
    }
    if let Some(error) = app.variable_detail_errors.get(&identity) {
        return format!(
            "Variable: {name}\nEffective summary: {summary_value}\n\nDetail unavailable: {error}\nPress Enter to retry.\n\n{}",
            config_copy_status(app)
        );
    }
    let Some(detail) = app.variable_details.get(&identity) else {
        return format!(
            "Variable: {name}\nEffective summary: {summary_value}\nScope: {}\n\nDetail not loaded; press Enter to inspect.\n\n{}",
            app.config_scope.as_deref().unwrap_or("global"),
            config_copy_status(app)
        );
    };
    let operations = if detail.operations.is_empty() {
        "none reported".into()
    } else {
        detail
            .operations
            .iter()
            .map(|operation| {
                let source = operation.file.as_ref().map_or_else(
                    || "source unavailable".into(),
                    |file| {
                        operation.line.map_or_else(
                            || file.display().to_string(),
                            |line| format!("{}:{line}", file.display()),
                        )
                    },
                );
                format!(
                    "{} @ {}{}",
                    operation.operation,
                    source,
                    operation
                        .value
                        .as_ref()
                        .map_or_else(String::new, |value| format!(" = {value}"))
                )
            })
            .collect::<Vec<_>>()
            .join("\n  ")
    };
    format!(
        "Variable: {}\nScope: {}\nEffective value: {}\nUnexpanded value: {}\n{}\nProvenance: {}\nActive overrides: {}\nOperations:\n  {}",
        detail.identity.name,
        detail
            .identity
            .recipe
            .as_deref()
            .map_or("global", |recipe| recipe),
        detail.effective_value.as_deref().unwrap_or("unavailable"),
        detail.unexpanded_value.as_deref().unwrap_or("unavailable"),
        config_copy_status(app),
        detail.provenance.as_deref().unwrap_or("unavailable"),
        if detail.active_overrides.is_empty() {
            "none reported".into()
        } else {
            detail.active_overrides.join(", ")
        },
        operations,
    )
}

fn config_copy_status(app: &App) -> String {
    let copy = [
        ("C effective", ConfigCopyValue::Effective),
        ("U unexpanded", ConfigCopyValue::Unexpanded),
    ]
    .into_iter()
    .map(|(label, value)| {
        selected_config_copy_value(app, value).map_or_else(
            |reason| format!("{label}: disabled ({reason})"),
            |_| format!("{label}: enabled"),
        )
    })
    .collect::<Vec<_>>()
    .join(" | ");
    let source = config_source_disabled_reason(app).map_or_else(
        || "o source: enabled".into(),
        |reason| format!("o source: disabled ({reason})"),
    );
    let scope = if app.workspace.recipes.is_empty() {
        "s scope: global only (no recipes reported)".into()
    } else {
        format!(
            "s scope: enabled ({} recipes; active {})",
            app.workspace.recipes.len(),
            app.config_scope.as_deref().unwrap_or("global")
        )
    };
    let compare = config_comparison(app).map_or_else(
        |reason| format!("c compare: disabled ({reason})"),
        |_| "c compare: enabled".into(),
    );
    let edit = if config_edit_disabled_reason(app).is_none() {
        "E edit: enabled"
    } else {
        "E edit: disabled"
    };
    format!("{scope} | {edit}\n{compare}\n{source}\n{copy}")
}

fn config(frame: &mut Frame, app: &App, area: Rect) {
    let variables = config_variables(app);
    let variable_count = variables.len();
    let chunks = Layout::vertical([Constraint::Percentage(32), Constraint::Min(5)]).split(area);
    frame.render_widget(
        Table::new(
            variables
                .into_iter()
                .enumerate()
                .map(|(index, (name, value))| {
                    Row::new(vec![Cell::from(name.as_str()), Cell::from(value.as_str())])
                        .style(selected_style(app, index == app.config_selection))
                }),
            [Constraint::Percentage(35), Constraint::Percentage(65)],
        )
        .header(
            Row::new(["Variable", "Effective value"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(metadata_title(
                    format!(
                        "Effective configuration (shown: {} of {}, read-only)",
                        variable_count,
                        app.workspace.variables.len()
                    ),
                    app,
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let detail = config_inspector(app);
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nEnter refreshes detail; o opens provenance when available."
        ))
        .block(
            Block::default()
                .title("Selected variable")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        chunks[1],
    );
}
fn bbmask(frame: &mut Frame, app: &App, area: Rect) {
    let value = app.workspace.variables.get("BBMASK").map_or(
        "(BBMASK is not set in the effective configuration)",
        String::as_str,
    );
    let provenance = app
        .workspace
        .variable_provenance
        .get("BBMASK")
        .map_or("backend did not provide source provenance", String::as_str);
    let patterns = value
        .split_whitespace()
        .enumerate()
        .map(|(index, pattern)| format!("{:>3}. {pattern}", index + 1))
        .collect::<Vec<_>>();
    let pattern_text = if patterns.is_empty() {
        "No masked recipe patterns are active.".into()
    } else {
        patterns.join("\n")
    };
    frame.render_widget(
        Paragraph::new(format!(
            "Effective BBMASK patterns:\n{pattern_text}\n\nProvenance: {provenance}\n\ne edits the value; Yoctui previews the exact local.conf assignment and requires confirmation."
        ))
        .block(Block::default().title("Effective BBMASK").borders(Borders::ALL))
        .wrap(Wrap { trim: false }),
        area,
    );
}

fn maintenance_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let mut lines = vec![Line::from(
        [
            MaintenanceView::Sstate,
            MaintenanceView::Services,
            MaintenanceView::Release,
            MaintenanceView::Integrations,
        ]
        .into_iter()
        .flat_map(|view| {
            let style = if app.maintenance.view == view {
                palette.role(palette.accent, Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                palette.role(palette.disabled, Modifier::DIM)
            };
            [
                Span::styled(format!(" {} ", maintenance_view_label(view)), style),
                Span::raw(" "),
            ]
        })
        .collect::<Vec<_>>(),
    )];
    lines.push(Line::from(""));
    match &app.maintenance.capability {
        MaintenanceCapability::NotInspected => lines.push(Line::styled(
            "Capability not inspected; press r to inspect.",
            palette.role(palette.disabled, Modifier::DIM),
        )),
        MaintenanceCapability::Loading(request) => lines.push(Line::styled(
            format!("Inspecting capability (request {request})…"),
            palette.role(palette.info, Modifier::BOLD),
        )),
        MaintenanceCapability::Available { snapshot, .. } => {
            maintenance_capability_lines(app, snapshot, &mut lines, palette)
        }
        MaintenanceCapability::Partial {
            snapshot,
            limitations,
            ..
        } => {
            lines.push(Line::styled(
                format!("Partial capability: {} limitation(s)", limitations.len()),
                palette.role(palette.warning, Modifier::BOLD),
            ));
            maintenance_capability_lines(app, snapshot, &mut lines, palette);
        }
        MaintenanceCapability::Failed { message, .. } => lines.push(Line::styled(
            format!("Capability inspection failed: {message}"),
            palette.role(palette.error, Modifier::BOLD),
        )),
    }
    if app.maintenance.view == MaintenanceView::Services {
        lines.push(Line::from(""));
        lines.push(Line::styled("Service diagnostics", palette.focus()));
        match &app.maintenance.services {
            MaintenanceServiceDiagnostics::NotInspected => lines.push(Line::raw("not inspected")),
            MaintenanceServiceDiagnostics::Loading(request) => {
                lines.push(Line::raw(format!("loading request {request}")))
            }
            MaintenanceServiceDiagnostics::Available { services, .. }
            | MaintenanceServiceDiagnostics::Partial { services, .. } => {
                for service in services {
                    lines.push(Line::styled(
                        format!(
                            "  {:?}: {:?} ({} endpoint(s), {} process(es))",
                            service.kind,
                            service.state,
                            service.endpoints.len(),
                            service.process_evidence.len()
                        ),
                        service_state_style(app, service.state),
                    ));
                }
            }
            MaintenanceServiceDiagnostics::Failed { message, .. } => lines.push(Line::styled(
                format!("failed: {message}"),
                palette.role(palette.error, Modifier::BOLD),
            )),
        }
    }
    if app.maintenance.view == MaintenanceView::Integrations {
        lines.push(Line::from(""));
        lines.push(Line::styled("Integration readiness", palette.focus()));
        match &app.maintenance.integrations {
            MaintenanceIntegrationDiagnostics::NotInspected => {
                lines.push(Line::raw("not inspected"))
            }
            MaintenanceIntegrationDiagnostics::Loading(request) => {
                lines.push(Line::raw(format!("loading request {request}")))
            }
            MaintenanceIntegrationDiagnostics::Available { snapshot, .. }
            | MaintenanceIntegrationDiagnostics::Partial { snapshot, .. } => {
                for (label, state) in maintenance_integration_rows(snapshot) {
                    lines.push(Line::styled(
                        format!("  {label}: {state:?}"),
                        optional_state_style(app, state),
                    ));
                }
            }
            MaintenanceIntegrationDiagnostics::Failed { message, .. } => lines.push(Line::styled(
                format!("failed: {message}"),
                palette.role(palette.error, Modifier::BOLD),
            )),
        }
    }
    if let Some(session) = app.maintenance.sessions.back() {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            format!(
                "Session {}: {:?}  exit {}  dropped {}",
                session.id.0,
                session.status,
                session
                    .exit_code
                    .map_or_else(|| "--".into(), |value| value.to_string()),
                session.dropped_lines,
            ),
            maintenance_session_style(app, session.status),
        ));
        for output in session.output.iter().rev().take(3).rev() {
            lines.push(Line::raw(format!("  {:?}: {}", output.stream, output.text)));
        }
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(pane_block(
                app,
                &format!(
                    "Maintenance · {}",
                    maintenance_view_label(app.maintenance.view)
                ),
                app.focus == FocusTarget::Workspace,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn maintenance_capability_lines(
    app: &App,
    snapshot: &MaintenanceCapabilitySnapshot,
    lines: &mut Vec<Line<'static>>,
    palette: ThemePalette,
) {
    for (index, tool) in maintenance_tools_for_view(app.maintenance.view)
        .iter()
        .enumerate()
    {
        let (text, style) = match snapshot.capability(*tool) {
            Some(MaintenanceToolCapability::Available {
                executable,
                interface,
                ..
            }) => (
                format!(
                    "{}  available ({})  {}",
                    maintenance_tool_label(*tool),
                    maintenance_interface_label(*interface),
                    executable.path.display()
                ),
                palette.role(palette.success, Modifier::BOLD),
            ),
            Some(MaintenanceToolCapability::Unavailable { reason, .. }) => (
                format!("{}  unavailable: {reason}", maintenance_tool_label(*tool)),
                palette.role(palette.disabled, Modifier::DIM),
            ),
            None => (
                format!(
                    "{}  unavailable: capability not reported",
                    maintenance_tool_label(*tool)
                ),
                palette.role(palette.disabled, Modifier::DIM),
            ),
        };
        let selected = index == app.maintenance.selection();
        lines.push(Line::styled(
            format!("{} {text}", if selected { "▶" } else { " " }),
            if selected {
                selected_style(app, true)
            } else {
                style
            },
        ));
    }
}

fn maintenance_inspector_text(app: &App) -> String {
    let mut sections = vec![format!(
        "View: {}",
        maintenance_view_label(app.maintenance.view)
    )];
    match &app.maintenance.capability {
        MaintenanceCapability::NotInspected => sections.push("Capability: not inspected".into()),
        MaintenanceCapability::Loading(request) => {
            sections.push(format!("Capability: loading request {request}"));
        }
        MaintenanceCapability::Failed { request, message } => {
            sections.push(format!("Capability request {request} failed: {message}"));
        }
        MaintenanceCapability::Available { request, snapshot }
        | MaintenanceCapability::Partial {
            request, snapshot, ..
        } => {
            sections.push(format!("Capability request: {request}"));
            sections.push(maintenance_metadata_text(snapshot));
            if let Some(tool) =
                maintenance_tools_for_view(app.maintenance.view).get(app.maintenance.selection())
            {
                sections.push(maintenance_tool_detail(snapshot, *tool));
            }
            if !snapshot.limitations.is_empty() {
                sections.push(format!(
                    "Capability limitations:\n- {}",
                    snapshot.limitations.join("\n- ")
                ));
            }
        }
    }
    if app.maintenance.view == MaintenanceView::Services {
        sections.push(maintenance_services_text(&app.maintenance.services));
    }
    if app.maintenance.view == MaintenanceView::Integrations {
        sections.push(maintenance_integrations_text(&app.maintenance.integrations));
    }
    if let Some(preview) = app.maintenance.pending.as_ref() {
        sections.push(maintenance_preview_text(preview));
    }
    if let Some(session) = app.maintenance.sessions.back() {
        let output = session
            .output
            .iter()
            .map(|line| format!("{:?}: {}", line.stream, line.text))
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!(
            "Latest session {}\nOperation: {}\nStatus: {:?}\nStarted: {}\nFinished: {}\nExit: {}\nDropped lines: {}\nMessage: {}\nOutput:\n{}",
            session.id.0,
            maintenance_operation_label(&session.preview.operation),
            session.status,
            session
                .started_at
                .map(timestamp_text)
                .unwrap_or_else(|| "not started".into()),
            session
                .finished_at
                .map(timestamp_text)
                .unwrap_or_else(|| "not finished".into()),
            session
                .exit_code
                .map_or_else(|| "unavailable".into(), |value| value.to_string()),
            session.dropped_lines,
            session.message.as_deref().unwrap_or("none"),
            if output.is_empty() { "none" } else { &output },
        ));
    }
    if let Some(evidence) = app.maintenance.selected_evidence() {
        sections.push(format!(
            "Selected evidence\nLabel: {}\nPath: {}\nBytes: {}\nModified: {}",
            evidence.label,
            evidence.identity.path.display(),
            evidence.identity.byte_size,
            timestamp_text(evidence.identity.modified_at),
        ));
    } else {
        sections.push("Evidence: none".into());
    }
    sections.join("\n\n")
}

fn maintenance_metadata_text(snapshot: &MaintenanceCapabilitySnapshot) -> String {
    let metadata = &snapshot.metadata;
    let path = |value: Option<&std::path::PathBuf>| {
        value.map_or_else(|| "unavailable".into(), |path| path.display().to_string())
    };
    let stamps = if metadata.stamps_dirs.is_empty() {
        "unavailable".into()
    } else {
        metadata
            .stamps_dirs
            .iter()
            .map(|value| value.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "Metadata\nBuild: {}\nSstate: {}\nTmp: {}\nStamps: {}\nBuild history: {}\nPR service: {}\nHash service: {}\nHash upstream: {}\nSignature handler: {}\nNative LSB: {}\nMachine: {}\nDistro: {}",
        path(metadata.build_dir.as_ref()),
        path(metadata.sstate_dir.as_ref()),
        path(metadata.tmp_dir.as_ref()),
        stamps,
        path(metadata.buildhistory_dir.as_ref()),
        metadata.prserv_host.as_deref().unwrap_or("unavailable"),
        metadata.hashserve.as_deref().unwrap_or("unavailable"),
        metadata
            .hashserve_upstream
            .as_deref()
            .unwrap_or("unavailable"),
        metadata
            .signature_handler
            .as_deref()
            .unwrap_or("unavailable"),
        metadata.native_lsb.as_deref().unwrap_or("unavailable"),
        metadata.machine.as_deref().unwrap_or("unavailable"),
        metadata.distro.as_deref().unwrap_or("unavailable"),
    )
}

fn maintenance_tool_detail(
    snapshot: &MaintenanceCapabilitySnapshot,
    tool: MaintenanceTool,
) -> String {
    match snapshot.capability(tool) {
        Some(MaintenanceToolCapability::Available {
            executable,
            interface,
            ..
        }) => format!(
            "Selected capability\nTool: {}\nState: available\nInterface: {}\nExecutable: {}\nBytes: {}\nModified: {}",
            maintenance_tool_label(tool),
            maintenance_interface_label(*interface),
            executable.path.display(),
            executable.byte_size,
            timestamp_text(executable.modified_at),
        ),
        Some(MaintenanceToolCapability::Unavailable { reason, .. }) => format!(
            "Selected capability\nTool: {}\nState: unavailable\nReason: {reason}",
            maintenance_tool_label(tool)
        ),
        None => format!(
            "Selected capability\nTool: {}\nState: unavailable\nReason: capability not reported",
            maintenance_tool_label(tool)
        ),
    }
}

fn maintenance_services_text(state: &MaintenanceServiceDiagnostics) -> String {
    match state {
        MaintenanceServiceDiagnostics::NotInspected => "Service diagnostics: not inspected".into(),
        MaintenanceServiceDiagnostics::Loading(request) => {
            format!("Service diagnostics: loading request {request}")
        }
        MaintenanceServiceDiagnostics::Failed { request, message } => {
            format!("Service diagnostics request {request} failed: {message}")
        }
        MaintenanceServiceDiagnostics::Available { request, services } => {
            maintenance_service_records(*request, services, &[])
        }
        MaintenanceServiceDiagnostics::Partial {
            request,
            services,
            limitations,
        } => maintenance_service_records(*request, services, limitations),
    }
}

fn maintenance_service_records(
    request: u64,
    services: &[yoctui_model::ServiceDiagnostic],
    limitations: &[String],
) -> String {
    let details = services
        .iter()
        .map(|service| {
            let endpoints = service
                .endpoints
                .iter()
                .map(|endpoint| {
                    format!(
                        "  {:?} {} [{:?}, {:?}]{}",
                        endpoint.role,
                        endpoint.value,
                        endpoint.location,
                        endpoint.reachability,
                        endpoint
                            .limitation
                            .as_deref()
                            .map_or_else(String::new, |value| format!(" — {value}")),
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let processes = service
                .process_evidence
                .iter()
                .map(|process| {
                    format!(
                        "  PID {} {} (observational)",
                        process.pid, process.executable
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{:?}: {:?}\nEndpoints:\n{}\nProcesses:\n{}\nLimitations:\n- {}",
                service.kind,
                service.state,
                if endpoints.is_empty() {
                    "  none"
                } else {
                    &endpoints
                },
                if processes.is_empty() {
                    "  none"
                } else {
                    &processes
                },
                if service.limitations.is_empty() {
                    "none".into()
                } else {
                    service.limitations.join("\n- ")
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "Service diagnostics request {request}\n{details}\nInspection limitations:\n- {}",
        if limitations.is_empty() {
            "none".into()
        } else {
            limitations.join("\n- ")
        }
    )
}

fn maintenance_integrations_text(state: &MaintenanceIntegrationDiagnostics) -> String {
    match state {
        MaintenanceIntegrationDiagnostics::NotInspected => {
            "Integration details: not inspected".into()
        }
        MaintenanceIntegrationDiagnostics::Loading(request) => {
            format!("Integration details: loading request {request}")
        }
        MaintenanceIntegrationDiagnostics::Failed { request, message } => {
            format!("Integration request {request} failed: {message}")
        }
        MaintenanceIntegrationDiagnostics::Available { request, snapshot }
        | MaintenanceIntegrationDiagnostics::Partial {
            request, snapshot, ..
        } => {
            let limitations = snapshot
                .limitations
                .iter()
                .chain(snapshot.pull_request.limitations.iter())
                .chain(snapshot.error_report.limitations.iter())
                .chain(snapshot.repo_manifest.limitations.iter())
                .chain(snapshot.toaster.limitations.iter())
                .cloned()
                .collect::<Vec<_>>();
            format!(
                "Integration request {request}\nPull request: {:?}\n  create: {}\n  send: {}\n  worktree: {}\n  HEAD: {}\nError report: {:?}\n  helper: {}\n  candidate: {}\nRepo manifest: {:?}\n  repo: {}\n  workspace: {}\n  manifest: {}\nToaster: {:?}\n  executable: {}\n  configurations: {}\n  observed processes: {}\n  process evidence is observational only",
                snapshot.pull_request.state,
                optional_file_path(snapshot.pull_request.create_helper.as_ref()),
                optional_file_path(snapshot.pull_request.send_helper.as_ref()),
                snapshot.pull_request.worktree.as_ref().map_or_else(
                    || "unavailable".into(),
                    |value| value.root.path.display().to_string()
                ),
                snapshot.pull_request.worktree.as_ref().map_or_else(
                    || "unavailable".into(),
                    |value| value.head.path.display().to_string()
                ),
                snapshot.error_report.state,
                optional_file_path(snapshot.error_report.helper.as_ref()),
                optional_file_path(snapshot.error_report.candidate_report.as_ref()),
                snapshot.repo_manifest.state,
                optional_file_path(snapshot.repo_manifest.repo_executable.as_ref()),
                snapshot.repo_manifest.workspace.as_ref().map_or_else(
                    || "unavailable".into(),
                    |value| value.path.display().to_string()
                ),
                optional_file_path(snapshot.repo_manifest.manifest.as_ref()),
                snapshot.toaster.state,
                optional_file_path(snapshot.toaster.executable.as_ref()),
                if snapshot.toaster.configurations.is_empty() {
                    "none".into()
                } else {
                    snapshot
                        .toaster
                        .configurations
                        .iter()
                        .map(|value| value.path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                if snapshot.toaster.observed_processes.is_empty() {
                    "none".into()
                } else {
                    snapshot
                        .toaster
                        .observed_processes
                        .iter()
                        .map(|value| format!("{}:{}", value.pid, value.executable))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            ) + &format!(
                "\nLimitations:\n- {}",
                if limitations.is_empty() {
                    "none".into()
                } else {
                    limitations.join("\n- ")
                }
            )
        }
    }
}

fn optional_file_path(identity: Option<&yoctui_model::MaintenanceFileIdentity>) -> String {
    identity.map_or_else(
        || "unavailable".into(),
        |value| value.path.display().to_string(),
    )
}

fn maintenance_preview_text(preview: &MaintenanceOperationPreview) -> String {
    format!(
        "Pending preview {}\nOperation: {}\nDestructive: {}\nNetwork: {}\nIndexed native vector:\n{}\nLimitations:\n- {}",
        preview.id,
        maintenance_operation_label(&preview.operation),
        preview.operation.destructive(),
        preview.operation.network_side_effect(),
        indexed_arguments(&preview.arguments),
        if preview.limitations.is_empty() {
            "none".into()
        } else {
            preview.limitations.join("\n- ")
        },
    )
}

fn maintenance_tools_for_view(view: MaintenanceView) -> &'static [MaintenanceTool] {
    match view {
        MaintenanceView::Sstate => &[
            MaintenanceTool::OeCheckSstate,
            MaintenanceTool::SstateCacheManagement,
        ],
        MaintenanceView::Services => &[MaintenanceTool::PrServiceTool],
        MaintenanceView::Release => &[
            MaintenanceTool::LockedSignatureCache,
            MaintenanceTool::BuildHistoryDiff,
            MaintenanceTool::BuildCompare,
            MaintenanceTool::GitArchive,
        ],
        MaintenanceView::Integrations => &[
            MaintenanceTool::CreatePullRequest,
            MaintenanceTool::SendPullRequest,
            MaintenanceTool::SendErrorReport,
            MaintenanceTool::Toaster,
        ],
    }
}

fn maintenance_view_label(view: MaintenanceView) -> &'static str {
    match view {
        MaintenanceView::Sstate => "Sstate",
        MaintenanceView::Services => "Services",
        MaintenanceView::Release => "Release",
        MaintenanceView::Integrations => "Integrations",
    }
}

fn maintenance_tool_label(tool: MaintenanceTool) -> &'static str {
    match tool {
        MaintenanceTool::OeCheckSstate => "oe-check-sstate",
        MaintenanceTool::SstateCacheManagement => "sstate cache management",
        MaintenanceTool::PrServiceTool => "bitbake-prserv-tool",
        MaintenanceTool::LockedSignatureCache => "gen-lockedsig-cache",
        MaintenanceTool::BuildHistoryDiff => "buildhistory-diff",
        MaintenanceTool::BuildCompare => "build-compare",
        MaintenanceTool::GitArchive => "oe-git-archive",
        MaintenanceTool::CreatePullRequest => "create-pull-request",
        MaintenanceTool::SendPullRequest => "send-pull-request",
        MaintenanceTool::SendErrorReport => "send-error-report",
        MaintenanceTool::Toaster => "Toaster",
    }
}

fn maintenance_interface_label(interface: MaintenanceToolInterface) -> &'static str {
    match interface {
        MaintenanceToolInterface::Native => "native",
        MaintenanceToolInterface::SstatePython => "current Python",
        MaintenanceToolInterface::SstateLegacyShell => "legacy shell",
        MaintenanceToolInterface::DetectionOnly => "detection only",
    }
}

fn maintenance_integration_rows(
    snapshot: &MaintenanceIntegrationsSnapshot,
) -> [(&'static str, yoctui_model::OptionalIntegrationState); 4] {
    [
        ("Pull request", snapshot.pull_request.state),
        ("Error report", snapshot.error_report.state),
        ("Repo manifest", snapshot.repo_manifest.state),
        ("Toaster", snapshot.toaster.state),
    ]
}

fn service_state_style(app: &App, state: yoctui_model::ServiceState) -> Style {
    let palette = ThemePalette::for_app(app);
    match state {
        yoctui_model::ServiceState::Reachable => palette.role(palette.success, Modifier::BOLD),
        yoctui_model::ServiceState::Unreachable => palette.role(palette.error, Modifier::BOLD),
        yoctui_model::ServiceState::Partial => palette.role(palette.warning, Modifier::BOLD),
        yoctui_model::ServiceState::Configured => palette.role(palette.info, Modifier::BOLD),
        yoctui_model::ServiceState::Disabled | yoctui_model::ServiceState::Unavailable => {
            palette.role(palette.disabled, Modifier::DIM)
        }
    }
}

fn optional_state_style(app: &App, state: yoctui_model::OptionalIntegrationState) -> Style {
    let palette = ThemePalette::for_app(app);
    match state {
        yoctui_model::OptionalIntegrationState::Available => {
            palette.role(palette.success, Modifier::BOLD)
        }
        yoctui_model::OptionalIntegrationState::Partial => {
            palette.role(palette.warning, Modifier::BOLD)
        }
        yoctui_model::OptionalIntegrationState::Unavailable => {
            palette.role(palette.disabled, Modifier::DIM)
        }
    }
}

fn maintenance_session_style(app: &App, status: MaintenanceSessionStatus) -> Style {
    let palette = ThemePalette::for_app(app);
    match status {
        MaintenanceSessionStatus::Succeeded => palette.role(palette.success, Modifier::BOLD),
        MaintenanceSessionStatus::Failed
        | MaintenanceSessionStatus::TimedOut
        | MaintenanceSessionStatus::Lost => palette.role(palette.error, Modifier::BOLD),
        MaintenanceSessionStatus::Cancelled => palette.role(palette.warning, Modifier::BOLD),
        MaintenanceSessionStatus::Queued
        | MaintenanceSessionStatus::Running
        | MaintenanceSessionStatus::Cancelling => palette.role(palette.info, Modifier::BOLD),
    }
}

fn indexed_arguments(arguments: &[String]) -> String {
    arguments
        .iter()
        .enumerate()
        .map(|(index, argument)| format!("[{index}] {argument}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn maintenance_dialog(frame: &mut Frame, app: &App, dialog: &MaintenanceDialog, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let (title, body, style) = match dialog {
        MaintenanceDialog::ReadinessForm(draft) => (
            "Sstate readiness check",
            format!(
                "{} Targets: {}\n{} Mode: {:?}\n{} Output: {}\n{} Log: {}\n{} Timeout seconds: {}\n\n{}\n\nTab/Shift+Tab field | Space/←/→ mode | Enter preview | Esc cancel",
                if draft.field == yoctui_model::MaintenanceReadinessField::Targets { ">" } else { " " },
                if draft.targets.is_empty() { "<required>" } else { &draft.targets },
                if draft.field == yoctui_model::MaintenanceReadinessField::Mode { ">" } else { " " },
                draft.mode,
                if draft.field == yoctui_model::MaintenanceReadinessField::Output { ">" } else { " " },
                if draft.output.is_empty() { "<none>" } else { &draft.output },
                if draft.field == yoctui_model::MaintenanceReadinessField::Log { ">" } else { " " },
                if draft.log.is_empty() { "<none>" } else { &draft.log },
                if draft.field == yoctui_model::MaintenanceReadinessField::Timeout { ">" } else { " " },
                if draft.timeout.is_empty() { "<required>" } else { &draft.timeout },
                draft.validation.as_deref().unwrap_or("No command runs until the exact adapter preview is confirmed."),
            ),
            if draft.validation.is_some() {
                palette.role(palette.error, Modifier::BOLD)
            } else {
                palette.role(palette.info, Modifier::BOLD)
            },
        ),
        MaintenanceDialog::CleanupForm(draft) => (
            "Protected sstate cleanup preview",
            format!(
                "Cache: {}\nStamps:\n- {}\n\n{} [{}] duplicates\n{} [{}] orphans\n{} [{}] unreferenced by stamps\n{} Jobs: {}\n\n{}\n\nTab/Shift+Tab field | Space toggle | Enter discover candidates | Esc cancel",
                draft.cache_dir.display(),
                if draft.stamps_dirs.is_empty() { "none".into() } else { draft.stamps_dirs.iter().take(3).map(|path| path.display().to_string()).collect::<Vec<_>>().join("\n- ") },
                if draft.field == yoctui_model::MaintenanceCleanupField::Duplicates { ">" } else { " " },
                if draft.duplicates { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceCleanupField::Orphans { ">" } else { " " },
                if draft.orphans { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceCleanupField::UnreferencedByStamps { ">" } else { " " },
                if draft.unreferenced_by_stamps { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceCleanupField::Jobs { ">" } else { " " },
                if draft.jobs.is_empty() { "<required>" } else { &draft.jobs },
                draft.validation.as_deref().unwrap_or("Candidate discovery is read-only. Deletion still requires the exact phrase and a second confirmation."),
            ),
            if draft.validation.is_some() {
                palette.role(palette.error, Modifier::BOLD)
            } else {
                palette.role(palette.warning, Modifier::BOLD)
            },
        ),
        MaintenanceDialog::PrServiceForm(draft) => (
            match draft.operation {
                yoctui_model::PrServiceOperation::Export => "PR service export",
                yoctui_model::PrServiceOperation::Import => "PR service import",
            },
            format!(
                "Operation: {:?}\nFile: {}\nBuild directory: {}\nConfigured endpoint: {}\n\nThe native helper may stop a memory-resident BitBake server and invalidate BitBake cache records.{}\n\n{}\n\nType canonical .conf/.inc path | Enter preview | Esc cancel",
                draft.operation,
                if draft.file.is_empty() { "<required>" } else { &draft.file },
                draft.build_dir.display(),
                draft.endpoint,
                if draft.operation == yoctui_model::PrServiceOperation::Import {
                    "\nImport changes PR service data."
                } else {
                    "\nExport may replace the exact destination."
                },
                draft.validation.as_deref().unwrap_or("No helper runs until the exact adapter preview is confirmed."),
            ),
            if draft.validation.is_some() {
                palette.role(palette.error, Modifier::BOLD)
            } else if draft.operation == yoctui_model::PrServiceOperation::Import {
                palette.role(palette.warning, Modifier::BOLD)
            } else {
                palette.role(palette.info, Modifier::BOLD)
            },
        ),
        MaintenanceDialog::LockedCacheForm(draft) => (
            "Locked-signature cache",
            format!(
                "{} Locked signatures: {}\n{} Input cache: {}\n{} Output cache: {}\n  Native LSB (read-only): {}\n{} Filter: {}\n\nMatching files beneath the exact output cache may be replaced. The adapter preview and a separate destructive confirmation are still required.\n\n{}\n\nTab/Shift+Tab field | Type/Backspace edit | Enter preview | Esc cancel",
                if draft.field == yoctui_model::MaintenanceLockedCacheField::LockedSignatures { ">" } else { " " },
                if draft.locked_signatures.is_empty() { "<required>" } else { &draft.locked_signatures },
                if draft.field == yoctui_model::MaintenanceLockedCacheField::InputCache { ">" } else { " " },
                if draft.input_cache.is_empty() { "<required>" } else { &draft.input_cache },
                if draft.field == yoctui_model::MaintenanceLockedCacheField::OutputCache { ">" } else { " " },
                if draft.output_cache.is_empty() { "<required>" } else { &draft.output_cache },
                draft.native_lsb,
                if draft.field == yoctui_model::MaintenanceLockedCacheField::Filter { ">" } else { " " },
                if draft.filter.is_empty() { "<none>" } else { &draft.filter },
                draft.validation.as_deref().unwrap_or("No generator runs until the exact adapter preview is confirmed."),
            ),
            palette.role(
                if draft.validation.is_some() {
                    palette.error
                } else {
                    palette.warning
                },
                Modifier::BOLD,
            ),
        ),
        MaintenanceDialog::BuildHistoryForm(draft) => (
            "Build-history comparison",
            format!(
                "Repository (read-only): {}\n{} From revision: {}\n{} To revision: {}\n{} [{}] report version\n{} [{}] report all\n{} [{}] signatures\n{} [{}] signature diff\n{} Exclude paths: {}\n{} [{}] no colour\n\nComparison uses buildhistory-diff only; build-compare is a separate unsupported capability. Output is bounded session evidence.\n\n{}\n\nTab/Shift+Tab field | Space/←/→ toggle | Enter preview | Esc cancel",
                draft.repository.display(),
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::FromRevision { ">" } else { " " },
                if draft.from_revision.is_empty() { "<none>" } else { &draft.from_revision },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::ToRevision { ">" } else { " " },
                if draft.to_revision.is_empty() { "<none>" } else { &draft.to_revision },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::ReportVersion { ">" } else { " " },
                if draft.report_version { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::ReportAll { ">" } else { " " },
                if draft.report_all { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::Signatures { ">" } else { " " },
                if draft.signatures { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::SignatureDiff { ">" } else { " " },
                if draft.signature_diff { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::ExcludePaths { ">" } else { " " },
                if draft.exclude_paths.is_empty() { "<none>" } else { &draft.exclude_paths },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::NoColour { ">" } else { " " },
                if draft.no_colour { "x" } else { " " },
                draft.validation.as_deref().unwrap_or("No comparison runs until the exact adapter preview is confirmed."),
            ),
            palette.role(
                if draft.validation.is_some() {
                    palette.error
                } else {
                    palette.info
                },
                Modifier::BOLD,
            ),
        ),
        MaintenanceDialog::GitArchiveForm(draft) => (
            "Git release archive",
            format!(
                "{} Data directory: {}\n{} Git directory: {}\n{} [{}] create  {} [{}] bare  {} [{}] create tag\n{} Branch: {}\n{} Tag: {}\n{} Commit subject: {}\n{} Commit body: {}\n{} Tag subject: {}\n{} Tag body: {}\n{} Exclusions: {}\n{} Notes: {}\n{} Push remote: {}\n\nLocal archive creation runs first. A configured push is deferred and requires a second network confirmation after local success. Repository creation, tag replacement, and tracked-output overwrite risks remain visible in the adapter preview.\n\n{}\n\nTab/Shift+Tab field | Space/←/→ toggle | Enter preview | Esc cancel",
                if draft.field == yoctui_model::MaintenanceGitArchiveField::DataDir { ">" } else { " " },
                if draft.data_dir.is_empty() { "<required>" } else { &draft.data_dir },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::GitDir { ">" } else { " " },
                if draft.git_dir.is_empty() { "<required>" } else { &draft.git_dir },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::Create { ">" } else { " " },
                if draft.create { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::Bare { ">" } else { " " },
                if draft.bare { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::CreateTag { ">" } else { " " },
                if draft.create_tag { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::BranchName { ">" } else { " " },
                draft.branch_name,
                if draft.field == yoctui_model::MaintenanceGitArchiveField::TagName { ">" } else { " " },
                if draft.tag_name.is_empty() { "<none>" } else { &draft.tag_name },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::CommitSubject { ">" } else { " " },
                draft.commit_subject,
                if draft.field == yoctui_model::MaintenanceGitArchiveField::CommitBody { ">" } else { " " },
                if draft.commit_body.is_empty() { "<none>" } else { &draft.commit_body },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::TagSubject { ">" } else { " " },
                draft.tag_subject,
                if draft.field == yoctui_model::MaintenanceGitArchiveField::TagBody { ">" } else { " " },
                if draft.tag_body.is_empty() { "<none>" } else { &draft.tag_body },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::Exclusions { ">" } else { " " },
                if draft.exclusions.is_empty() { "<none>" } else { &draft.exclusions },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::Notes { ">" } else { " " },
                if draft.notes.is_empty() { "<none>" } else { &draft.notes },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::PushRemote { ">" } else { " " },
                if draft.push_remote.is_empty() { "<local only>" } else { &draft.push_remote },
                draft.validation.as_deref().unwrap_or("No archive or network operation runs until the exact preview is confirmed."),
            ),
            palette.role(
                if draft.validation.is_some() {
                    palette.error
                } else if !draft.push_remote.is_empty() {
                    palette.warning
                } else {
                    palette.info
                },
                Modifier::BOLD,
            ),
        ),
        MaintenanceDialog::Confirm(preview) => (
            if preview.operation.destructive() {
                "Confirm destructive Maintenance operation"
            } else {
                "Confirm Maintenance operation"
            },
            format!(
                "Operation {}\nKind: {}\nDestructive: {}\nNetwork: {}\n\nIndexed native vector:\n{}\n\nLimitations:\n- {}\n\nEnter confirms | Esc cancels",
                preview.id,
                maintenance_operation_label(&preview.operation),
                preview.operation.destructive(),
                preview.operation.network_side_effect(),
                indexed_arguments(&preview.arguments),
                if preview.limitations.is_empty() {
                    "none".into()
                } else {
                    preview.limitations.join("\n- ")
                },
            ),
            if preview.operation.destructive() {
                palette.role(palette.warning, Modifier::BOLD)
            } else {
                palette.role(palette.info, Modifier::BOLD)
            },
        ),
        MaintenanceDialog::CleanupPhrase { preview, input } => (
            "Confirm protected sstate cleanup",
            format!(
                "Type exactly:\n{}\n\n{input}_\n\nTyping alone cannot delete files.\nEnter continues | Esc cancels",
                preview.operation.cleanup_phrase().unwrap_or_default()
            ),
            palette.role(palette.error, Modifier::BOLD),
        ),
        MaintenanceDialog::ConfirmNetworkPush(preview) => (
            "Confirm network push",
            format!(
                "Operation {} requests a separately confirmed remote push.\n\n{}\n\nEnter confirms | Esc cancels",
                preview.id,
                indexed_arguments(&preview.arguments)
            ),
            palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED),
        ),
        MaintenanceDialog::ConfirmCancellation(id) => (
            "Cancel Maintenance operation",
            format!(
                "Cancel exact session {}?\nA cleanup may leave a partially cleaned cache.\n\nEnter confirms | Esc keeps running",
                id.0
            ),
            palette.role(palette.warning, Modifier::BOLD),
        ),
    };
    let width = 78.min(area.width.saturating_sub(2));
    let preferred_height = if matches!(dialog, MaintenanceDialog::GitArchiveForm(_)) {
        22
    } else {
        18
    };
    let height = preferred_height.min(area.height.saturating_sub(2));
    let popup = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(body)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(style),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn maintenance_operation_label(operation: &MaintenanceOperation) -> &'static str {
    match operation {
        MaintenanceOperation::SstateReadiness(_) => "sstate readiness",
        MaintenanceOperation::SstateCleanup(_) => "sstate cleanup",
        MaintenanceOperation::PrService(_) => "PR service",
        MaintenanceOperation::LockedSignatureCache(_) => "locked signature cache",
        MaintenanceOperation::BuildHistoryComparison(_) => "build-history comparison",
        MaintenanceOperation::BuildCompare(_) => "build compare",
        MaintenanceOperation::GitArchive(_) => "Git archive",
    }
}
fn bbmask_assignment(value: &str) -> String {
    format!(
        "BBMASK = \"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    )
}
fn help(frame: &mut Frame, area: Rect) {
    frame.render_widget(Paragraph::new("B Image build options for the effective MACHINE; b build, c clean, m menuconfig, e choose target\n! Open an inherited Yocto shell; exit returns to Yoctui\nb Choose target and start build; h build history; Dashboard Up/Down scrolls observed package task progress\nc Cancel active build\nl Logs   f toggle follow   w toggle wrapping   s cycle severity\nR cycle recipe filter   T cycle task filter   n/N previous/next match\ne Errors   o open selected source log, layer directory, or config provenance\nr Recipes: z confirmed diffsigs task, Z signature inspection, e provider, o logs, p patches, b/f tasks, V CVE, X SPDX, d modify, u update, F finish, P deploy, D reset\ny Layers: e in-TUI edit, o external editor   v Configuration   x effective BBMASK, e edit with preview\n/ Search recipes, layers, or configuration   Esc Dashboard   q Quit\n\nSignatures: Up/Down select, 1/2 choose sides, c compare, r refresh, e provider, Esc back/cancel.\nCVE/SPDX, cleansstate, forced tasks, Devtool reset/update-recipe/finish/deploy, BBMASK changes, and quitting an active build require confirmation.").block(Block::default().title("Help").borders(Borders::ALL)),area)
}
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use std::path::PathBuf;
    use yoctui_model::{Action, BuildRequest, update};

    fn rendered_text(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn security_report_identity(
        path: &str,
        fingerprint: &str,
    ) -> yoctui_model::SecurityReportIdentity {
        yoctui_model::SecurityReportIdentity::new(
            PathBuf::from(path),
            512,
            SystemTime::UNIX_EPOCH,
            fingerprint.into(),
        )
        .unwrap()
    }

    fn security_workflow_ui_app() -> App {
        let scope = SecurityScope::Recipe(RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox_1.36.bb".into(),
        });
        let cve_identity =
            security_report_identity("/build/tmp/log/cve/busybox.cve.json", "cvefingerprint");
        let finding_identity = yoctui_model::CveFindingIdentity::new(
            "CVE-2026-0001".into(),
            "busybox".into(),
            Some("busybox".into()),
        )
        .unwrap();
        let cve = yoctui_model::CveReport {
            identity: cve_identity.clone(),
            scope: Some(scope.clone()),
            findings: vec![yoctui_model::CveFinding {
                identity: finding_identity.clone(),
                status: yoctui_model::CveStatus::Vulnerable,
                product: Some("busybox".into()),
                version: Some("1.36".into()),
                severity: Some("HIGH".into()),
                score: Some("8.1".into()),
                vector: Some("CVSS:3.1/AV:N".into()),
                advisory_url: Some("https://example.invalid/CVE-2026-0001".into()),
                summary: Some("A bounded vulnerability summary".into()),
                mapping: vec![
                    yoctui_model::SecurityMetadata::new(
                        "upstream-product".into(),
                        "busybox".into(),
                    )
                    .unwrap(),
                ],
            }],
            metadata: vec![
                yoctui_model::SecurityMetadata::new("source".into(), "cve-check".into()).unwrap(),
            ],
            limitations: vec!["one unknown status was preserved".into()],
        };
        let spdx_identity =
            security_report_identity("/build/tmp/deploy/spdx/image.spdx.json", "spdxfingerprint");
        let spdx = yoctui_model::SpdxDocument {
            identity: spdx_identity.clone(),
            scope: Some(SecurityScope::Image {
                target: "core-image-minimal".into(),
                machine: "qemux86-64".into(),
                distro: "poky".into(),
            }),
            kind: SpdxArtifactKind::Json,
            spdx_version: Some("SPDX-2.3".into()),
            name: Some("core-image-minimal".into()),
            namespace: Some("https://example.invalid/spdx/image".into()),
            data_license: Some("CC0-1.0".into()),
            creators: vec!["Tool: bitbake".into()],
            components: vec![yoctui_model::SpdxComponent {
                identity: "SPDXRef-Package-busybox".into(),
                name: "busybox".into(),
                version: Some("1.36".into()),
                supplier: Some("Organization: Yocto".into()),
                license: Some("GPL-2.0-only".into()),
            }],
            file_count: Some(42),
            relationship_count: Some(7),
            checksums: vec![
                yoctui_model::SecurityMetadata::new("SHA256".into(), "abcd1234".into()).unwrap(),
            ],
            limitations: vec!["external references unavailable".into()],
        };
        let request = yoctui_model::SecurityReportRequest::new(
            1,
            vec![cve_identity.path.clone(), spdx_identity.path.clone()],
        )
        .unwrap();
        let capability = yoctui_model::SecurityCapabilitySnapshot::new(
            Some("6.0".into()),
            "/build".into(),
            scope.clone(),
            vec![scope.clone()],
            Some("cve_check".into()),
            Some("create_recipe_sbom".into()),
            None,
            false,
            Some(yoctui_model::SecurityMapperCapability {
                executable: "/workspace/scripts/cve-check-map-pkgs".into(),
                arguments: vec!["/build/tmp/log/cve".into()],
            }),
            vec!["/build/tmp/log/cve".into()],
            vec!["/build/tmp/deploy/spdx".into()],
            vec!["image SBOM task unavailable for recipe scope".into()],
        )
        .unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Security;
        app.focus = FocusTarget::Workspace;
        app.security.scope = Some(scope);
        app.security.capability = SecurityCapability::Available(Box::new(capability));
        app.security.inventory = SecurityInventoryState::Partial {
            request,
            reports: vec![SecurityReport::Cve(cve), SecurityReport::Spdx(spdx)],
            limitations: vec!["one malformed report was skipped".into()],
        };
        app.security.report_selection = Some(cve_identity);
        app.security.finding_selection = Some(finding_identity);
        app
    }

    fn security_session(status: SecuritySessionStatus) -> yoctui_model::SecuritySession {
        let preview = yoctui_model::SecurityOperationPreview {
            id: yoctui_model::SecuritySessionId(9),
            scope: SecurityScope::Image {
                target: "core-image-minimal".into(),
                machine: "qemux86-64".into(),
                distro: "poky".into(),
            },
            operation: SecurityOperation::PackageMap {
                executable: "/workspace/scripts/cve-check-map-pkgs".into(),
                arguments: vec!["/build/tmp/log/cve".into()],
            },
            indexed_arguments: vec![
                "0: /workspace/scripts/cve-check-map-pkgs".into(),
                "1: /build/tmp/log/cve".into(),
            ],
            report_roots: vec!["/build/tmp/log/cve".into()],
        };
        yoctui_model::SecuritySession {
            preview,
            status,
            background_job_id: None,
            started_at: SystemTime::UNIX_EPOCH,
            finished_at: status.is_terminal().then_some(SystemTime::UNIX_EPOCH),
            message: matches!(
                status,
                SecuritySessionStatus::Failed
                    | SecuritySessionStatus::Lost
                    | SecuritySessionStatus::TimedOut
            )
            .then(|| format!("{} detail", security_session_status_label(status))),
            result_paths: (status == SecuritySessionStatus::Succeeded)
                .then(|| PathBuf::from("/build/tmp/log/cve/busybox.cve.json"))
                .into_iter()
                .collect(),
            output: vec![
                yoctui_model::SecurityOutputLine {
                    stream: SecurityOutputStream::Stdout,
                    line: "mapped busybox -> busybox".into(),
                    truncated: false,
                },
                yoctui_model::SecurityOutputLine {
                    stream: SecurityOutputStream::Stderr,
                    line: "bounded mapper warning".into(),
                    truncated: true,
                },
            ],
        }
    }

    #[test]
    fn security_workflow_renders_cve_identity_capability_partial_and_themes_responsively() {
        let mut app = security_workflow_ui_app();
        for (width, height, theme, color) in [
            (80, 24, Theme::Monochrome, false),
            (100, 30, Theme::WhiteClassic, true),
            (130, 30, Theme::MatrixGreen, true),
            (160, 40, Theme::HighContrast, true),
            (160, 40, Theme::DarkPro, true),
        ] {
            app.theme = theme;
            app.color_enabled = color;
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Security"), "{output}");
            assert!(output.contains("CVEs"), "{output}");
            assert!(output.contains("CVE-2026-0001"), "{output}");
            assert!(output.contains("vulnerable"), "{output}");
            assert!(output.contains("one malformed report"), "{output}");
        }

        app.focus = FocusTarget::Inspector;
        let inspector = rendered_text(&app, 160, 40);
        assert!(inspector.contains("Exact report"), "{inspector}");
        assert!(inspector.contains("cvefingerprint"), "{inspector}");
        assert!(
            inspector.contains("upstream-product=busybox"),
            "{inspector}"
        );
        assert!(inspector.contains("CVSS:3.1/AV:N"), "{inspector}");

        app.focus = FocusTarget::Workspace;
        app.color_enabled = false;
        let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(
            terminal.backend().buffer().content.iter().any(|cell| {
                cell.symbol() == "▶" && cell.modifier.contains(Modifier::REVERSED)
            }),
            "no-color selected finding must use reverse video"
        );
    }

    #[test]
    fn security_workflow_renders_sbom_document_component_drill_and_limitations() {
        let mut app = security_workflow_ui_app();
        let spdx_identity = app
            .security
            .inventory
            .reports()
            .unwrap()
            .iter()
            .find_map(|report| match report {
                SecurityReport::Spdx(document) => Some(document.identity.clone()),
                _ => None,
            })
            .unwrap();
        app.security.view = SecurityView::Sbom;
        app.security.report_selection = Some(spdx_identity);
        for (width, height) in [(80, 24), (100, 30), (130, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("SBOM"), "{output}");
            assert!(output.contains("SPDX-2.3"), "{output}");
            assert!(output.contains("core-image-minimal"), "{output}");
        }

        app.security.drilled = true;
        app.security.component_selection = Some("SPDXRef-Package-busybox".into());
        let drilled = rendered_text(&app, 80, 24);
        assert!(drilled.contains("SPDXRef-Package-busybox"), "{drilled}");
        assert!(drilled.contains("GPL-2.0-only"), "{drilled}");

        app.focus = FocusTarget::Inspector;
        let inspector = rendered_text(&app, 160, 40);
        assert!(inspector.contains("https://example.invalid/spdx/image"));
        assert!(inspector.contains("SHA256=abcd1234"));
        assert!(inspector.contains("external references unavailable"));
        assert!(inspector.contains("Organization: Yocto"));
    }

    #[test]
    fn security_workflow_renders_every_inventory_and_capability_state() {
        let mut app = security_workflow_ui_app();
        let request = app.security.inventory.request().unwrap().clone();
        for (state, expected) in [
            (SecurityInventoryState::NotLoaded, "Reports are not loaded"),
            (
                SecurityInventoryState::Loading {
                    request: request.clone(),
                },
                "Loading report generation",
            ),
            (
                SecurityInventoryState::AvailableEmpty {
                    request: request.clone(),
                },
                "available-empty",
            ),
            (
                SecurityInventoryState::Failed {
                    request: request.clone(),
                    message: "permission denied".into(),
                },
                "acquisition failed",
            ),
            (
                SecurityInventoryState::Cancelled {
                    request: request.clone(),
                },
                "acquisition cancelled",
            ),
            (
                SecurityInventoryState::TimedOut {
                    request: request.clone(),
                },
                "acquisition timed out",
            ),
            (
                SecurityInventoryState::Lost {
                    request,
                    message: "worker channel closed".into(),
                },
                "worker lost",
            ),
        ] {
            app.security.inventory = state;
            let output = rendered_text(&app, 80, 24);
            assert!(output.contains(expected), "{expected}: {output}");
        }

        app.security.inventory = SecurityInventoryState::NotLoaded;
        for (capability, expected) in [
            (SecurityCapability::NotInspected, "not inspected"),
            (SecurityCapability::Inspecting, "inspection in progress"),
            (
                SecurityCapability::Failed("missing class".into()),
                "inspection failed",
            ),
        ] {
            app.security.capability = capability;
            let output = rendered_text(&app, 80, 24);
            assert!(output.contains(expected), "{expected}: {output}");
        }
    }

    #[test]
    fn security_workflow_renders_mapper_session_terminal_outcomes_and_bounded_output() {
        let mut app = security_workflow_ui_app();
        let request = app.security.inventory.request().unwrap().clone();
        app.security.inventory = SecurityInventoryState::AvailableEmpty { request };
        for status in [
            SecuritySessionStatus::Starting,
            SecuritySessionStatus::Running,
            SecuritySessionStatus::Cancelling,
            SecuritySessionStatus::Succeeded,
            SecuritySessionStatus::Failed,
            SecuritySessionStatus::Cancelled,
            SecuritySessionStatus::TimedOut,
            SecuritySessionStatus::Lost,
        ] {
            app.security.sessions = vec![security_session(status)];
            let output = rendered_text(&app, 100, 30);
            assert!(
                output.contains(security_session_status_label(status)),
                "{status:?}: {output}"
            );
            assert!(output.contains("mapped busybox"), "{output}");
            assert!(output.contains("[truncated]"), "{output}");
        }
    }

    #[test]
    fn security_workflow_dialogs_render_exact_previews_at_all_breakpoints() {
        let mut app = security_workflow_ui_app();
        app.focus = FocusTarget::Dialog;
        let session = security_session(SecuritySessionStatus::Starting);
        app.dialogs
            .push_front(Dialog::Security(SecurityDialog::Operation(session.preview)));
        for (width, height) in [(80, 24), (100, 30), (130, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Confirm CVE package mapping"), "{output}");
            assert!(output.contains("Exact indexed shell-free"), "{output}");
            assert!(output.contains("1: /build/tmp/log/cve"), "{output}");
        }

        app.dialogs.clear();
        app.dialogs
            .push_front(Dialog::Security(SecurityDialog::Import {
                input: format!("/reports/{}", "long-path-".repeat(80)),
            }));
        let import = rendered_text(&app, 80, 24);
        assert!(import.contains("Import Security reports"), "{import}");
        assert!(import.contains("canonical non-symlink"), "{import}");

        app.dialogs.clear();
        app.dialogs
            .push_front(Dialog::Security(SecurityDialog::Cancellation(
                yoctui_model::SecuritySessionId(9),
            )));
        let cancellation = rendered_text(&app, 80, 24);
        assert!(
            cancellation.contains("Confirm Security cancellation"),
            "{cancellation}"
        );
        assert!(cancellation.contains("session 9 only"), "{cancellation}");
    }

    #[test]
    fn theme_palettes_define_distinct_semantic_roles() {
        for theme in [
            Theme::DarkPro,
            Theme::WhiteClassic,
            Theme::MatrixGreen,
            Theme::HighContrast,
            Theme::Monochrome,
        ] {
            let mut app = App::new(10, 1_000);
            app.theme = theme;
            let palette = ThemePalette::for_app(&app);
            let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
            terminal.draw(|frame| render(frame, &app)).unwrap();
            assert!(
                terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .any(|cell| cell.bg == palette.background)
            );
            if theme == Theme::Monochrome {
                assert!(palette.attribute_only);
                assert!(palette.focus().add_modifier.contains(Modifier::BOLD));
                assert!(palette.selected().add_modifier.contains(Modifier::REVERSED));
            } else {
                assert!(!palette.attribute_only);
                assert_ne!(palette.focused_border, palette.border);
                assert_ne!(palette.selection_background, palette.background);
                assert_ne!(palette.error, palette.success);
                assert_ne!(palette.warning, palette.info);
            }
        }
    }

    #[test]
    fn theme_no_color_uses_attributes_for_focus_selection_and_severity() {
        let mut app = App::new(10, 1_000);
        app.theme = Theme::WhiteClassic;
        app.color_enabled = false;
        let palette = ThemePalette::for_app(&app);

        assert!(palette.attribute_only);
        assert_eq!(palette.foreground, Color::Reset);
        assert!(palette.focus().add_modifier.contains(Modifier::BOLD));
        assert!(
            selected_style(&app, true)
                .add_modifier
                .contains(Modifier::REVERSED)
        );
        assert!(
            severity_style(&app, Severity::Error)
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
        assert_ne!(
            severity_style(&app, Severity::Warning).add_modifier,
            severity_style(&app, Severity::Trace).add_modifier
        );

        app.focus = FocusTarget::Navigator;
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(terminal.backend().buffer().content.iter().any(|cell| {
            cell.modifier.contains(Modifier::REVERSED) || cell.modifier.contains(Modifier::BOLD)
        }));
    }

    #[test]
    fn theme_light_shell_and_dialog_apply_the_semantic_background() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.theme = Theme::WhiteClassic;
        app.dialogs.push_back(Dialog::BuildOptions);
        terminal.draw(|frame| render(frame, &app)).unwrap();

        assert_eq!(
            ThemePalette::for_app(&app).background,
            Color::Rgb(248, 248, 248)
        );
        assert_eq!(ThemePalette::for_app(&app).info, Color::Rgb(0, 107, 107));
    }

    #[test]
    fn theme_picker_renders_named_choices_and_immediate_apply_hint() {
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::ThemePicker { selection: 1 });
        let output = rendered_text(&app, 100, 30);
        assert!(output.contains("Theme — applies immediately"));
        assert!(output.contains("WhiteClassic"));
        assert!(output.contains("Enter apply"));
    }

    #[test]
    fn theme_progress_and_log_severity_use_semantic_roles() {
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.theme = Theme::HighContrast;
        app.tasks.insert(
            yoctui_model::TaskId("busybox:do_compile".into()),
            yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(50),
                ..yoctui_model::TaskInfo::default()
            },
        );
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.fg == Color::Rgb(0, 220, 255))
        );

        app.screen = Screen::Logs;
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "compile failed".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: None,
            protected: false,
            diagnostic: None,
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.fg == Color::Rgb(255, 70, 70))
        );
    }

    #[test]
    fn settings_workspace_renders_typed_rows_and_controls_on_narrow_terminals() {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Settings;
        app.settings_selection = 5;
        app.settings_dirty = true;
        app.theme = Theme::MatrixGreen;
        app.animation_speed = yoctui_model::AnimationSpeed::Slow;
        app.reduced_motion = true;
        app.logs.follow = false;

        let output = rendered_text(&app, 100, 30);
        assert!(output.contains("Settings (not saved)"));
        assert!(output.contains("Theme"));
        assert!(output.contains("MatrixGreen"));
        assert!(output.contains("Animation speed"));
        assert!(output.contains("Reduced motion"));
        assert!(output.contains("Log wrap"));
        assert!(output.contains("Log follow"));
        assert!(output.contains("select"));
        assert!(output.contains("change"));
    }

    #[test]
    fn build_environment_workspace_renders_disconnected_state_and_unlock_rule() {
        let app = App::new_unconfigured(10, 1_000);
        let output = rendered_text(&app, 100, 30);
        assert!(output.contains("Build environment"));
        assert!(output.contains("not configured"));
        assert!(output.contains("available images"));
        assert!(output.contains("verification"));
    }

    #[test]
    fn build_environment_form_renders_selected_typed_fields() {
        let mut app = App::new_unconfigured(10, 1_000);
        let _ = update(&mut app, Action::BeginBuildEnvironmentEdit);
        let output = rendered_text(&app, 120, 36);
        assert!(output.contains("Edit profile"));
        assert!(output.contains("source:"));
        assert!(output.contains("script:"));
    }

    #[test]
    fn build_environment_editor_renders_large_vi_style_popup() {
        let mut app = App::new_unconfigured(10, 1_000);
        app.dialogs.push_back(Dialog::BuildEnvironmentEditor {
            content: "source = \"/home/poky\"\nbuild = \"/home/build\"".into(),
            editing: false,
        });
        let output = rendered_text(&app, 120, 40);
        assert!(output.contains("Build environment.toml"));
        assert!(output.contains("NORMAL"));
        assert!(output.contains("/home/poky"));
    }

    #[test]
    fn animation_fast_slow_and_reduced_motion_have_deterministic_cadence() {
        let mut fast = App::new(10, 1_000);
        fast.animation_frame = 0;
        let first = task_activity(&fast, None);
        fast.animation_frame = 1;
        assert_ne!(task_activity(&fast, None), first);

        let mut slow = App::new(10, 1_000);
        slow.animation_speed = yoctui_model::AnimationSpeed::Slow;
        slow.animation_frame = 0;
        let first = task_activity(&slow, None);
        slow.animation_frame = 1;
        assert_eq!(task_activity(&slow, None), first);
        slow.animation_frame = 3;
        assert_ne!(task_activity(&slow, None), first);

        slow.reduced_motion = true;
        assert_eq!(task_activity(&slow, None), " active");
        assert_eq!(task_activity(&slow, Some(0)), "");
    }

    #[test]
    fn animation_unknown_progress_never_fabricates_a_percentage() {
        for (theme, color_enabled) in [
            (Theme::DarkPro, true),
            (Theme::WhiteClassic, true),
            (Theme::MatrixGreen, true),
            (Theme::HighContrast, true),
            (Theme::Monochrome, true),
            (Theme::DarkPro, false),
        ] {
            let mut app = App::new(10, 1_000);
            app.theme = theme;
            app.color_enabled = color_enabled;
            app.tasks.insert(
                yoctui_model::TaskId("busybox:do_compile".into()),
                yoctui_model::TaskInfo {
                    id: yoctui_model::TaskId("busybox:do_compile".into()),
                    recipe: "busybox".into(),
                    task: "do_compile".into(),
                    progress: None,
                    ..yoctui_model::TaskInfo::default()
                },
            );
            let output = rendered_text(&app, 300, 30);
            assert!(output.contains("progress unknown"), "{output}");
            assert!(!output.contains("busybox:do_compile 0%"));
            let _ = rendered_text(&app, 80, 24);
        }
    }

    #[test]
    fn images_workspace_renders_typed_artifacts_inspector_and_responsive_modes() {
        let mut app = App::new(20, 20_000);
        app.screen = Screen::Images;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.build.target = Some("core-image-minimal".into());
        let request = yoctui_model::ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        };
        let path = std::path::PathBuf::from("/deploy/qemux86-64/core-image-minimal-qemux86-64.wic");
        let artifact = yoctui_model::ImageArtifact {
            identity: yoctui_model::ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: "core-image-minimal".into(),
                path: path.clone(),
            },
            kind: yoctui_model::ImageArtifactKind::Wic,
            size_bytes: ImageArtifactField::Available(8192),
            modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
            checksums: ImageArtifactField::Available(vec![yoctui_model::ImageChecksum {
                algorithm: "sha256".into(),
                digest: "abcdef".into(),
                source: "/deploy/qemux86-64/image.sha256".into(),
            }]),
            manifests: ImageArtifactField::Available(vec![
                "/deploy/qemux86-64/image.manifest".into(),
            ]),
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Available(vec!["/deploy/qemux86-64/image.spdx.json".into()]),
            wic_files: ImageArtifactField::Available(vec![path]),
        };
        app.image_artifact_selection = Some(artifact.identity.clone());
        app.image_artifacts = ImageArtifactInventoryState::Partial {
            request,
            inventory: yoctui_model::ImageArtifactInventory {
                machine: "qemux86-64".into(),
                deploy_directory: ImageArtifactField::Available("/deploy/qemux86-64".into()),
                artifacts: vec![artifact],
            },
            limitations: vec!["one symlink was not followed".into()],
        };

        for (width, theme, color) in [
            (180, Theme::DarkPro, true),
            (120, Theme::WhiteClassic, true),
            (80, Theme::Monochrome, false),
        ] {
            app.theme = theme;
            app.color_enabled = color;
            let output = rendered_text(&app, width, 32);
            assert!(output.contains("core-image-minimal"), "{output}");
            assert!(output.contains("wic"), "{output}");
            assert!(output.contains("refresh"), "{output}");
        }
        let wide = rendered_text(&app, 180, 40);
        assert!(wide.contains("Deploy directory"), "{wide}");
        assert!(wide.contains("sha256"), "{wide}");
        assert!(wide.contains("SPDX/SBOM"), "{wide}");
        assert!(wide.contains("one symlink was not followed"), "{wide}");
    }

    #[test]
    fn images_workspace_renders_loading_empty_failure_and_search_empty_states() {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Images;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let request = yoctui_model::ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        };
        app.image_artifacts = ImageArtifactInventoryState::Loading {
            request: request.clone(),
        };
        assert!(rendered_text(&app, 100, 25).contains("Loading deployed image artifacts"));
        app.image_artifacts = ImageArtifactInventoryState::AvailableEmpty {
            request: request.clone(),
            inventory: yoctui_model::ImageArtifactInventory {
                machine: "qemux86-64".into(),
                deploy_directory: ImageArtifactField::Available("/deploy/qemux86-64".into()),
                artifacts: Vec::new(),
            },
        };
        assert!(rendered_text(&app, 100, 25).contains("No deployed image artifacts"));
        app.image_artifacts = ImageArtifactInventoryState::Failed {
            request,
            message: "DEPLOY_DIR_IMAGE is unavailable".into(),
        };
        assert!(rendered_text(&app, 100, 25).contains("Artifact scan failed"));
    }

    fn sdk_workflow_ui_app() -> App {
        let mut app = App::new(20, 20_000);
        app.screen = Screen::Sdk;
        app.focus = FocusTarget::Workspace;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace
            .variables
            .insert("DISTRO".into(), "poky".into());
        app.workspace
            .variables
            .insert("SDK_DEPLOY".into(), "/deploy/sdk".into());
        app.build.target = Some("core-image-minimal".into());
        let installer = yoctui_model::SdkArtifact {
            identity: yoctui_model::SdkArtifactIdentity {
                path: "/deploy/sdk/poky-core-image-minimal-toolchain.sh".into(),
                size_bytes: 8_192,
                modified_unix_seconds: 1_700_000_000,
            },
            kind: SdkArtifactKind::Installer,
            sdk_kind: Some(SdkKind::Standard),
            machine: Some("qemux86-64".into()),
            host_tuple: Some("x86_64-pokysdk-linux".into()),
            target_tuple: Some("x86_64-poky-linux".into()),
            checksums: vec!["/deploy/sdk/poky-core-image-minimal-toolchain.sh.sha256".into()],
            manifests: vec!["/deploy/sdk/poky-core-image-minimal-toolchain.target.manifest".into()],
            published: None,
        };
        app.sdk_artifact_selection = Some(installer.identity.clone());
        app.sdk_artifacts = SdkArtifactInventoryState::Available {
            request: yoctui_model::SdkArtifactInventoryRequest {
                generation: 1,
                root: "/deploy/sdk".into(),
                machine: "qemux86-64".into(),
            },
            artifacts: vec![installer],
        };
        app.sdk_tool_capability = SdkToolCapability::Available {
            publish: Some("/workspace/scripts/oe-publish-sdk".into()),
            find_sysroot: Some("/workspace/scripts/oe-find-native-sysroot".into()),
            run_native: Some("/workspace/scripts/oe-run-native".into()),
        };
        app
    }

    fn sdk_workflow_running_ui_app() -> (App, SdkSessionId) {
        let mut app = sdk_workflow_ui_app();
        let _ = update(&mut app, Action::BeginSelectedSdkPublish);
        if let Some(Dialog::SdkPublishTomlEditor { content, .. }) = app.active_dialog_mut() {
            *content = "destination = \"/srv/sdk-publish\"\n".into();
        }
        let _ = update(&mut app, Action::PreviewSdkPublish);
        let Some(yoctui_model::Effect::StartSdkSession { id, .. }) =
            update(&mut app, Action::ConfirmSdkPublish)
        else {
            panic!("expected managed SDK session");
        };
        let _ = update(
            &mut app,
            Action::SdkSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::SdkSessionRunning { id });
        app.focus = FocusTarget::Inspector;
        (app, id)
    }

    #[test]
    fn sdk_workflow_renders_every_inventory_state_and_responsive_selection() {
        let mut app = sdk_workflow_ui_app();
        let request = yoctui_model::SdkArtifactInventoryRequest {
            generation: 2,
            root: "/deploy/sdk".into(),
            machine: "qemux86-64".into(),
        };

        app.sdk_artifacts = SdkArtifactInventoryState::NotLoaded;
        assert!(rendered_text(&app, 100, 25).contains("not loaded"));
        app.sdk_artifacts = SdkArtifactInventoryState::Loading {
            request: request.clone(),
        };
        assert!(rendered_text(&app, 100, 25).contains("generation 2"));
        app.sdk_artifacts = SdkArtifactInventoryState::AvailableEmpty {
            request: request.clone(),
        };
        assert!(rendered_text(&app, 100, 25).contains("No SDK artifacts were found"));
        app.sdk_artifacts = SdkArtifactInventoryState::Failed {
            request: request.clone(),
            message: "permission denied".into(),
        };
        assert!(rendered_text(&app, 100, 25).contains("permission denied"));

        app = sdk_workflow_ui_app();
        app.sdk_artifact_query = "missing".into();
        assert!(rendered_text(&app, 100, 25).contains("No SDK artifacts match"));
        app.sdk_artifact_query.clear();
        app.sdk_artifact_searching = true;
        let artifact = app.selected_sdk_artifact().unwrap().clone();
        app.sdk_artifacts = SdkArtifactInventoryState::Partial {
            request,
            artifacts: vec![artifact],
            limitations: vec!["one SDK symlink was not followed".into()],
        };
        app.focus = FocusTarget::Inspector;
        for (width, height, theme, color) in [
            (80, 24, Theme::Monochrome, false),
            (100, 30, Theme::WhiteClassic, true),
            (160, 40, Theme::MatrixGreen, true),
        ] {
            app.theme = theme;
            app.color_enabled = color;
            let output = rendered_text(&app, width, height);
            assert!(output.contains("SDK"), "{output}");
            if width == 160 {
                assert!(output.contains("Host tuple"), "{output}");
                assert!(output.contains("x86_64-pokysdk-linux"), "{output}");
                assert!(output.contains("Published: unavailable"), "{output}");
                assert!(
                    output.contains("one SDK symlink was not followed"),
                    "{output}"
                );
            }
        }
        for (theme, color) in [
            (Theme::DarkPro, true),
            (Theme::WhiteClassic, true),
            (Theme::MatrixGreen, true),
            (Theme::HighContrast, true),
            (Theme::Monochrome, true),
            (Theme::DarkPro, false),
        ] {
            app.theme = theme;
            app.color_enabled = color;
            let output = rendered_text(&app, 160, 40);
            assert!(output.contains("SDK tool capability"), "{output}");
            assert!(output.contains("Selected artifact"), "{output}");
        }
        app.focus = FocusTarget::Workspace;
        app.color_enabled = false;
        let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.modifier.contains(Modifier::REVERSED))
        );
        let narrow = rendered_text(&app, 80, 24);
        assert!(narrow.contains("s/E:SDK"), "{narrow}");
        assert!(narrow.contains("c:cancel"), "{narrow}");
    }

    #[test]
    fn sdk_workflow_renders_lifecycle_output_and_every_terminal_outcome() {
        let (mut running, id) = sdk_workflow_running_ui_app();
        let _ = update(
            &mut running,
            Action::AppendSdkSessionOutput {
                id,
                stream: yoctui_model::SdkOutputStream::Stderr,
                line: "publication warning".into(),
                truncated: true,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        );
        let output = rendered_text(&running, 180, 44);
        assert!(output.contains("Status: running"), "{output}");
        assert!(
            output.contains("[stderr] publication warning [truncated]"),
            "{output}"
        );

        let mut succeeded = running.clone();
        let _ = update(
            &mut succeeded,
            Action::CompleteSdkSession {
                id,
                exit_code: 0,
                artifacts: vec!["/srv/sdk-publish/toolchain.sh".into()],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&succeeded, 180, 44).contains("Status: succeeded"));

        let (mut failed, failed_id) = sdk_workflow_running_ui_app();
        let _ = update(
            &mut failed,
            Action::FailSdkSession {
                id: failed_id,
                message: "destination denied".into(),
                exit_code: Some(7),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let output = rendered_text(&failed, 180, 44);
        assert!(output.contains("Status: failed"), "{output}");
        assert!(output.contains("destination denied"), "{output}");

        let (mut lost, lost_id) = sdk_workflow_running_ui_app();
        let _ = update(
            &mut lost,
            Action::LoseSdkSession {
                id: lost_id,
                message: "runner channel lost".into(),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&lost, 180, 44).contains("Status: lost"));

        let (mut cancelled, cancelled_id) = sdk_workflow_running_ui_app();
        let _ = update(&mut cancelled, Action::BeginActiveSdkSessionCancellation);
        let _ = update(&mut cancelled, Action::ConfirmSdkSessionCancellation);
        let _ = update(
            &mut cancelled,
            Action::CancelSdkSession {
                id: cancelled_id,
                exit_code: Some(130),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&cancelled, 180, 44).contains("Status: cancelled"));
    }

    #[test]
    fn sdk_workflow_renders_all_dialogs_at_responsive_boundaries() {
        let mut app = sdk_workflow_ui_app();
        let _ = update(
            &mut app,
            Action::BeginSdkBuild(SdkBuildAction::Populate(SdkKind::Extensible)),
        );
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Confirm SDK build"), "{output}");
            assert!(output.contains("populate_sdk_ext"), "{output}");
        }

        app.dialogs.clear();
        app.focus = FocusTarget::Workspace;
        let _ = update(&mut app, Action::BeginSelectedSdkPublish);
        assert_eq!(app.focus, FocusTarget::Dialog);
        if let Some(Dialog::SdkPublishTomlEditor { content, .. }) = app.active_dialog_mut() {
            *content = format!(
                "destination = \"/srv/{}\"\n",
                "long-destination-".repeat(80)
            );
        }
        assert!(rendered_text(&app, 80, 24).contains("SDK publish.toml"));
        if let Some(Dialog::SdkPublishTomlEditor { content, .. }) = app.active_dialog_mut() {
            *content = "destination = \"/srv/sdk-publish\"\n".into();
        }
        let _ = update(&mut app, Action::PreviewSdkPublish);
        let publish = rendered_text(&app, 80, 24);
        assert!(publish.contains("Confirm SDK publication"), "{publish}");
        assert!(publish.contains("[0]"), "{publish}");

        app.dialogs.clear();
        app.focus = FocusTarget::Dialog;
        let native_draft = yoctui_model::SdkNativeDraft {
            mode: SdkNativeMode::RunNative,
            extracted_root: format!("/opt/{}", "sdk-root-".repeat(80)),
            recipe: "cmake-native".into(),
            tool: "cmake".into(),
            arguments: vec!["--version".into(), "--trace".into()],
        };
        app.dialogs
            .push_front(Dialog::SdkNative(SdkNativeDialog::new(native_draft)));
        let native = rendered_text(&app, 80, 24);
        assert!(native.contains("SDK native tool"), "{native}");
        assert!(native.contains("▶ Mode"), "{native}");
        let _ = update(&mut app, Action::SelectSdkNativeField { delta: 2 });
        let _ = update(&mut app, Action::ActivateSdkNativeField);
        let editing = rendered_text(&app, 80, 24);
        assert!(editing.contains("[editing]"), "{editing}");

        let preview = yoctui_model::SdkNativePreview::new(yoctui_model::SdkNativeRequest {
            executable: "/workspace/scripts/oe-run-native".into(),
            mode: SdkNativeMode::RunNative,
            extracted_root: Some("/opt/extracted-sdk".into()),
            recipe: "cmake-native".into(),
            tool: Some("cmake".into()),
            arguments: vec!["--version".into(), "x".repeat(1024)],
        })
        .unwrap();
        app.dialogs.clear();
        app.dialogs
            .push_front(Dialog::SdkNativeConfirmation(preview));
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Confirm SDK native tool"), "{output}");
            assert!(output.contains("Exact indexed"), "{output}");
        }

        let (mut running, id) = sdk_workflow_running_ui_app();
        running
            .dialogs
            .push_front(Dialog::SdkCancellationConfirmation(id));
        running.focus = FocusTarget::Dialog;
        let output = rendered_text(&running, 80, 24);
        assert!(output.contains("Confirm SDK cancellation"), "{output}");
        assert!(output.contains("Enter requests cancellation"), "{output}");
    }

    #[test]
    fn test_workflow_screen_renders_identity_capability_and_selection_responsively() {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Testing;
        app.focus = FocusTarget::Workspace;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace
            .variables
            .insert("DISTRO".into(), "poky".into());
        app.build.target = Some("core-image-minimal".into());
        app.test_capability = yoctui_model::TestCapability {
            oe_selftest: yoctui_model::TestExecutableCapability::Available(
                "/workspace/oe-selftest".into(),
            ),
            bitbake_selftest: yoctui_model::TestExecutableCapability::Missing,
            ptest: yoctui_model::PtestCapability::Configured,
        };

        for (width, height, theme, color) in [
            (80, 24, Theme::Monochrome, false),
            (100, 30, Theme::WhiteClassic, true),
            (160, 40, Theme::HighContrast, true),
        ] {
            app.theme = theme;
            app.color_enabled = color;
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Testing"), "{output}");
            assert!(output.contains("qemux86-64"), "{output}");
            assert!(output.contains("core-image-minimal"), "{output}");
            assert!(output.contains("OE selftest"), "{output}");
        }

        app.test_family_selection = yoctui_model::TestFamily::Ptest;
        app.focus = FocusTarget::Inspector;
        let output = rendered_text(&app, 160, 40);
        assert!(output.contains("Package tests"), "{output}");
        assert!(output.contains("Configured"), "{output}");
    }

    fn test_workflow_result(
        suffix: &str,
        outcome: yoctui_model::TestCaseOutcome,
    ) -> yoctui_model::TestResultRecord {
        let path = PathBuf::from(format!("/results/{suffix}/testresults.json"));
        let identity = yoctui_model::TestResultIdentity::new(
            path,
            128,
            SystemTime::UNIX_EPOCH,
            format!("{suffix}fingerprint"),
        )
        .unwrap();
        let case_identity =
            yoctui_model::TestCaseIdentity::new("runtime".into(), "Case.test_one".into()).unwrap();
        let (case, _) = yoctui_model::TestCaseRecord::new(
            case_identity,
            outcome,
            Some(std::time::Duration::from_millis(1250)),
            vec![yoctui_model::TestMetadata::new("result".into(), suffix.into()).unwrap()],
            Some(PathBuf::from(format!("/logs/{suffix}.log"))),
        )
        .unwrap();
        let (suite, _) =
            yoctui_model::TestSuiteRecord::new("runtime".into(), None, Vec::new(), vec![case])
                .unwrap();
        yoctui_model::TestResultRecord::new(
            identity,
            Some(yoctui_model::TestFamily::TestImage),
            Some("qemux86-64".into()),
            Some("core-image-minimal".into()),
            Some("revision-1".into()),
            Some(std::time::Duration::from_secs(2)),
            vec![yoctui_model::TestMetadata::new("DISTRO".into(), "poky".into()).unwrap()],
            vec![suite],
            Some(yoctui_model::TestSessionId(3)),
            vec!["fixture limitation".into()],
        )
        .0
    }

    fn test_workflow_results_app() -> (
        App,
        yoctui_model::TestResultRecord,
        yoctui_model::TestResultRecord,
    ) {
        let baseline = test_workflow_result("baseline", yoctui_model::TestCaseOutcome::Passed);
        let candidate = test_workflow_result("candidate", yoctui_model::TestCaseOutcome::Failed);
        let request = yoctui_model::TestResultImportRequest::new(
            1,
            vec![
                baseline.identity.path.clone(),
                candidate.identity.path.clone(),
            ],
        )
        .unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Testing;
        app.focus = FocusTarget::Workspace;
        app.test_view = TestWorkspaceView::Results;
        app.result_tool_capability =
            yoctui_model::ResultToolCapability::Available("/workspace/resulttool".into());
        app.test_result_selection = Some(candidate.identity.clone());
        app.test_results = TestResultInventoryState::Partial {
            request,
            records: vec![baseline.clone(), candidate.clone()],
            limitations: vec!["one malformed result was skipped".into()],
        };
        (app, baseline, candidate)
    }

    #[test]
    fn test_workflow_results_render_inventory_drill_partial_and_terminal_states() {
        let (mut app, _baseline, candidate) = test_workflow_results_app();
        for (width, height, theme, color) in [
            (80, 24, Theme::Monochrome, false),
            (100, 30, Theme::WhiteClassic, true),
            (160, 40, Theme::HighContrast, true),
        ] {
            app.theme = theme;
            app.color_enabled = color;
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Results"), "{output}");
            assert!(output.contains("candidate"), "{output}");
            assert!(output.contains("Partial"), "{output}");
        }

        app.test_result_drilled = true;
        app.test_case_selection = Some(candidate.suites[0].cases[0].identity.clone());
        app.focus = FocusTarget::Inspector;
        let output = rendered_text(&app, 160, 40);
        assert!(output.contains("Case.test_one"), "{output}");
        assert!(output.contains("Failed"), "{output}");
        assert!(output.contains("/logs/candidate.log"), "{output}");
        assert!(output.contains("fixture limitation"), "{output}");

        let request = app.test_results.request().unwrap().clone();
        for (state, expected) in [
            (
                TestResultInventoryState::AvailableEmpty {
                    request: request.clone(),
                },
                "No structured test results",
            ),
            (
                TestResultInventoryState::Failed {
                    request: request.clone(),
                    message: "invalid JSON".into(),
                },
                "Result import failed",
            ),
            (
                TestResultInventoryState::Cancelled {
                    request: request.clone(),
                },
                "Result import cancelled",
            ),
            (
                TestResultInventoryState::TimedOut {
                    request: request.clone(),
                },
                "Result import timed out",
            ),
            (
                TestResultInventoryState::Lost {
                    request,
                    message: "worker closed".into(),
                },
                "worker lost",
            ),
        ] {
            app.test_result_drilled = false;
            app.focus = FocusTarget::Workspace;
            app.test_results = state;
            let output = rendered_text(&app, 80, 24);
            assert!(output.contains(expected), "{expected}: {output}");
        }
    }

    #[test]
    fn test_workflow_comparison_renders_categories_limitations_and_outcomes() {
        let (mut app, baseline, candidate) = test_workflow_results_app();
        let request = yoctui_model::TestComparisonRequest::new(
            2,
            baseline.identity.clone(),
            candidate.identity.clone(),
        )
        .unwrap();
        let comparison = yoctui_model::TestComparison::between(&baseline, &candidate).unwrap();
        app.test_view = TestWorkspaceView::Comparison;
        app.test_comparison_selection = Some(comparison.transitions[0].identity.clone());
        app.test_comparison = TestComparisonState::Partial {
            request: request.clone(),
            comparison,
            limitations: vec!["resulttool detail unavailable".into()],
        };
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("regression"), "{output}");
            assert!(output.contains("resulttool detail unavailable"), "{output}");
        }
        app.focus = FocusTarget::Inspector;
        let output = rendered_text(&app, 160, 40);
        assert!(output.contains("Baseline log"), "{output}");
        assert!(output.contains("JUnit export"), "{output}");

        for (state, expected) in [
            (
                TestComparisonState::Failed {
                    request: request.clone(),
                    message: "nonzero".into(),
                },
                "Comparison failed",
            ),
            (
                TestComparisonState::Cancelled {
                    request: request.clone(),
                },
                "Comparison cancelled",
            ),
            (
                TestComparisonState::TimedOut {
                    request: request.clone(),
                },
                "Comparison timed out",
            ),
            (
                TestComparisonState::Lost {
                    request,
                    message: "worker closed".into(),
                },
                "worker lost",
            ),
        ] {
            app.focus = FocusTarget::Workspace;
            app.test_comparison = state;
            let output = rendered_text(&app, 80, 24);
            assert!(output.contains(expected), "{expected}: {output}");
        }
    }

    #[test]
    fn test_workflow_lifecycle_and_junit_outcomes_remain_visibly_distinct() {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Testing;
        app.focus = FocusTarget::Workspace;
        for (index, outcome) in [
            yoctui_model::TestSessionOutcome::Succeeded,
            yoctui_model::TestSessionOutcome::Failed,
            yoctui_model::TestSessionOutcome::Cancelled,
            yoctui_model::TestSessionOutcome::TimedOut,
            yoctui_model::TestSessionOutcome::Lost,
        ]
        .into_iter()
        .enumerate()
        {
            app.test_sessions.clear();
            app.test_sessions.push_back(yoctui_model::TestSession {
                id: yoctui_model::TestSessionId(index as u64 + 1),
                background_job_id: None,
                operation: yoctui_model::TestOperation::Build {
                    family: yoctui_model::TestFamily::TestImage,
                    request: BuildRequest {
                        targets: vec!["core-image-minimal".into()],
                        task: Some("testimage".into()),
                        force: false,
                    },
                },
                exit_code: (outcome == yoctui_model::TestSessionOutcome::Failed).then_some(3),
                result_paths: if outcome == yoctui_model::TestSessionOutcome::Succeeded {
                    vec!["/results/testresults.json".into()]
                } else {
                    Vec::new()
                },
                error_detail: (outcome != yoctui_model::TestSessionOutcome::Succeeded)
                    .then(|| format!("{outcome:?} detail")),
                outcome: Some(outcome),
            });
            let output = rendered_text(&app, 100, 30);
            assert!(output.contains(&format!("{outcome:?}")), "{output}");
        }

        let (mut app, _baseline, candidate) = test_workflow_results_app();
        app.test_view = TestWorkspaceView::Comparison;
        app.focus = FocusTarget::Inspector;
        let request = yoctui_model::TestJunitExportRequest {
            generation: 7,
            result: candidate.identity.clone(),
            destination: "/exports/results.xml".into(),
        };
        let preview = yoctui_model::TestJunitExportPreview::new(
            "/workspace/resulttool".into(),
            request.clone(),
        )
        .unwrap();
        let states = [
            (
                TestJunitExportState::Inspecting {
                    result: candidate.identity,
                    destination: request.destination.clone(),
                },
                "validating",
            ),
            (TestJunitExportState::Ready(preview), "ready"),
            (TestJunitExportState::Running(request.clone()), "running"),
            (
                TestJunitExportState::Succeeded(request.clone()),
                "succeeded",
            ),
            (
                TestJunitExportState::Failed {
                    request: request.clone(),
                    message: "nonzero".into(),
                },
                "failed",
            ),
            (
                TestJunitExportState::Cancelled(request.clone()),
                "cancelled",
            ),
            (TestJunitExportState::TimedOut(request.clone()), "timed out"),
            (
                TestJunitExportState::Lost {
                    request,
                    message: "worker closed".into(),
                },
                "lost",
            ),
        ];
        for (state, expected) in states {
            app.test_junit_export = state;
            let output = rendered_text(&app, 160, 40);
            assert!(output.contains(expected), "{expected}: {output}");
        }
    }

    #[test]
    fn test_workflow_dialogs_render_exact_previews_at_responsive_boundaries() {
        let (mut app, baseline, candidate) = test_workflow_results_app();
        app.focus = FocusTarget::Dialog;
        app.dialogs.push_front(Dialog::TestLaunchTomlEditor {
            content: "family = \"OE selftest\"\nmachine = \"qemux86-64\"\ndistro = \"poky\"\nimage = \"core-image-minimal\"\nscope = \"all\"\nselector = \"\"\nparallelism = 1\nverbose = false\nskip_network = false\n".into(),
            editing: false,
        });
        assert!(
            rendered_text(&app, 80, 24).contains("Test launch.toml"),
            "{}",
            rendered_text(&app, 80, 24)
        );

        app.dialogs.clear();
        let request = yoctui_model::TestSelftestRequest::new(
            "/workspace/oe-selftest".into(),
            yoctui_model::TestFamily::OeSelftest,
            Some("tinfoil.Case.test_one".into()),
            4,
            false,
            false,
        )
        .unwrap();
        app.dialogs.push_front(Dialog::TestLaunchConfirmation(
            yoctui_model::TestLaunchPreview::Selftest(request),
        ));
        let output = rendered_text(&app, 100, 30);
        assert!(output.contains("Confirm Testing launch"), "{output}");
        assert!(output.contains("[0] /workspace/oe-selftest"), "{output}");

        app.dialogs.clear();
        app.dialogs.push_front(Dialog::TestCancellationConfirmation(
            yoctui_model::TestSessionId(9),
        ));
        assert!(rendered_text(&app, 80, 24).contains("Confirm Testing cancellation"));

        app.dialogs.clear();
        let import = yoctui_model::TestResultImportDialog {
            input: "/results/testresults.json".into(),
            ..Default::default()
        };
        app.dialogs.push_front(Dialog::TestResultImport(import));
        assert!(rendered_text(&app, 80, 24).contains("Import structured test results"));

        app.dialogs.clear();
        let records = app.test_results.records().to_vec();
        app.dialogs.push_front(Dialog::TestComparison(
            yoctui_model::TestComparisonPicker::new(Some(baseline.identity.clone()), &records),
        ));
        assert!(rendered_text(&app, 100, 30).contains("Choose exact comparison inputs"));

        app.dialogs.clear();
        let comparison_request = yoctui_model::TestComparisonRequest::new(
            2,
            baseline.identity.clone(),
            candidate.identity.clone(),
        )
        .unwrap();
        app.dialogs.push_front(Dialog::TestComparisonConfirmation(
            yoctui_model::TestComparisonPreview::new(
                "/workspace/resulttool".into(),
                comparison_request,
            )
            .unwrap(),
        ));
        let output = rendered_text(&app, 160, 40);
        assert!(output.contains("Confirm result comparison"), "{output}");
        assert!(output.contains("[1] regression-file"), "{output}");

        app.dialogs.clear();
        let mut junit = yoctui_model::TestJunitExportDialog::new(candidate.identity.clone());
        junit.destination_input = "/exports/results.xml".into();
        app.dialogs.push_front(Dialog::TestJunitExport(junit));
        assert!(rendered_text(&app, 80, 24).contains("JUnit export destination"));

        app.dialogs.clear();
        let export = yoctui_model::TestJunitExportRequest {
            generation: 3,
            result: candidate.identity,
            destination: "/exports/results.xml".into(),
        };
        app.dialogs.push_front(Dialog::TestJunitExportConfirmation(
            yoctui_model::TestJunitExportPreview::new("/workspace/resulttool".into(), export)
                .unwrap(),
        ));
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Confirm JUnit export"), "{output}");
            assert!(output.contains("never overwrites"), "{output}");
        }
    }

    fn qemu_workspace_app() -> App {
        let mut app = App::new(20, 20_000);
        app.screen = Screen::Images;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let identity = yoctui_model::ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: "/deploy/qemux86-64/core-image-minimal.wic".into(),
        };
        let artifact = yoctui_model::ImageArtifact {
            identity: identity.clone(),
            kind: yoctui_model::ImageArtifactKind::Wic,
            size_bytes: ImageArtifactField::Available(8_192),
            modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
            checksums: ImageArtifactField::Unavailable,
            manifests: ImageArtifactField::Unavailable,
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Unavailable,
            wic_files: ImageArtifactField::Available(vec![identity.path.clone()]),
        };
        app.image_artifact_selection = Some(identity.clone());
        app.image_artifacts = ImageArtifactInventoryState::Available {
            request: yoctui_model::ImageArtifactRequest {
                generation: 1,
                machine: "qemux86-64".into(),
            },
            inventory: yoctui_model::ImageArtifactInventory {
                machine: "qemux86-64".into(),
                deploy_directory: ImageArtifactField::Available("/deploy/qemux86-64".into()),
                artifacts: vec![artifact],
            },
        };
        app.qemu_capability = QemuCapability::Available {
            executable: "/opt/poky/scripts/runqemu".into(),
            compatible_images: vec![identity],
        };
        app
    }

    fn qemu_running_workspace_app() -> (App, QemuSessionId) {
        let mut app = qemu_workspace_app();
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedQemuLaunch);
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewQemuLaunch);
        let Some(yoctui_model::Effect::StartQemuSession { id, .. }) =
            yoctui_model::update(&mut app, yoctui_model::Action::ConfirmQemuLaunch)
        else {
            panic!("expected session");
        };
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::QemuSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::QemuSessionRunning { id });
        app.focus = FocusTarget::Inspector;
        (app, id)
    }

    #[test]
    fn qemu_workspace_renders_capability_dialogs_session_and_responsive_states() {
        let mut app = qemu_workspace_app();
        app.focus = FocusTarget::Inspector;
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            if width == 160 {
                assert!(output.contains("runqemu capability"), "{output}");
                assert!(output.contains("ready for selected artifact"), "{output}");
            }
        }
        assert!(rendered_text(&app, 70, 20).contains("needs at least 80x24"));

        let artifact = app.selected_image_artifact().unwrap().clone();
        let mut launch = QemuLaunchDialog::new(yoctui_model::QemuLaunchDraft::for_artifact(
            artifact.identity,
            artifact.kind,
        ));
        launch.selected_field = QemuLaunchField::Kernel;
        launch.editing = true;
        launch.draft.kernel = "relative/kernel".into();
        launch.validation_error = Some("kernel path must be absolute".into());
        app.dialogs.push_front(Dialog::QemuLaunch(launch.clone()));
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Launch runqemu"), "{output}");
            assert!(output.contains("Kernel"), "{output}");
            assert!(output.contains("editing"), "{output}");
            assert!(output.contains("Validation"), "{output}");
        }

        app.dialogs.clear();
        let preview = launch.draft.preview(&app.qemu_capability).unwrap_err();
        assert!(preview.contains("normalized absolute"));
        launch.draft.kernel.clear();
        let preview = launch.draft.preview(&app.qemu_capability).unwrap();
        app.dialogs
            .push_front(Dialog::QemuLaunchConfirmation(preview));
        let confirmation = rendered_text(&app, 100, 30);
        assert!(
            confirmation.contains("Exact argument vector"),
            "{confirmation}"
        );
        assert!(confirmation.contains("qemumemory=1024"), "{confirmation}");

        app.dialogs.clear();
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedQemuLaunch);
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewQemuLaunch);
        let Some(yoctui_model::Effect::StartQemuSession { id, .. }) =
            yoctui_model::update(&mut app, yoctui_model::Action::ConfirmQemuLaunch)
        else {
            panic!("expected session");
        };
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::QemuSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::QemuSessionRunning { id });
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::AppendQemuSessionOutput {
                id,
                stream: yoctui_model::QemuOutputStream::Stderr,
                line: "guest warning".into(),
                truncated: true,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        );
        app.focus = FocusTarget::Inspector;
        let running = rendered_text(&app, 160, 40);
        assert!(running.contains("Status: running"), "{running}");
        assert!(
            running.contains("[stderr] guest warning [truncated]"),
            "{running}"
        );

        app.dialogs
            .push_front(Dialog::QemuCancellationConfirmation(id));
        let cancellation = rendered_text(&app, 80, 24);
        assert!(
            cancellation.contains("Confirm runqemu cancellation"),
            "{cancellation}"
        );
        app.dialogs.clear();
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::FailQemuSession {
                id,
                message: "display unavailable".into(),
                exit_code: Some(1),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let failed = rendered_text(&app, 160, 40);
        assert!(failed.contains("Status: failed"), "{failed}");
        assert!(failed.contains("display unavailable"), "{failed}");

        for capability in [
            QemuCapability::MissingTool,
            QemuCapability::MissingCompatibleImage,
            QemuCapability::Failed {
                message: "inspection denied".into(),
            },
        ] {
            app.qemu_capability = capability;
            let output = rendered_text(&app, 160, 40);
            assert!(output.contains("runqemu capability"), "{output}");
        }
    }

    #[test]
    fn qemu_workspace_renders_each_terminal_session_outcome() {
        let (mut succeeded, succeeded_id) = qemu_running_workspace_app();
        let _ = yoctui_model::update(
            &mut succeeded,
            yoctui_model::Action::CompleteQemuSession {
                id: succeeded_id,
                exit_code: 0,
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&succeeded, 160, 40).contains("Status: succeeded"));

        let (mut failed, failed_id) = qemu_running_workspace_app();
        let _ = yoctui_model::update(
            &mut failed,
            yoctui_model::Action::FailQemuSession {
                id: failed_id,
                message: "failed display".into(),
                exit_code: Some(1),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&failed, 160, 40).contains("Status: failed"));

        let (mut lost, lost_id) = qemu_running_workspace_app();
        let _ = yoctui_model::update(
            &mut lost,
            yoctui_model::Action::LoseQemuSession {
                id: lost_id,
                message: "runner lost".into(),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&lost, 160, 40).contains("Status: lost"));

        let (mut cancelled, cancelled_id) = qemu_running_workspace_app();
        let _ = yoctui_model::update(
            &mut cancelled,
            yoctui_model::Action::BeginQemuSessionCancellation { id: cancelled_id },
        );
        let _ = yoctui_model::update(
            &mut cancelled,
            yoctui_model::Action::ConfirmQemuSessionCancellation,
        );
        let _ = yoctui_model::update(
            &mut cancelled,
            yoctui_model::Action::CancelQemuSession {
                id: cancelled_id,
                exit_code: Some(130),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&cancelled, 160, 40).contains("Status: cancelled"));
    }

    fn wic_workspace_app() -> App {
        let mut app = qemu_workspace_app();
        app.wic_capability = WicCapability::Available {
            executable: "/opt/poky/scripts/wic".into(),
            kickstarts: vec![yoctui_model::WicKickstart {
                identity: yoctui_model::WicKickstartIdentity {
                    name: "directdisk".into(),
                    path: Some("/layers/meta/wic/directdisk.wks".into()),
                },
                source: "part / --source=rootfs --fstype=ext4 --size=64".into(),
                partitions: vec![yoctui_model::WicPartitionSummary {
                    mount_point: Some("/".into()),
                    filesystem: Some("ext4".into()),
                    source_plugin: Some("rootfs".into()),
                    size_mib: Some(64),
                    alignment_kib: None,
                }],
                limitations: vec!["dynamic boot size".into()],
            }],
            image_targets: vec!["core-image-minimal".into()],
        };
        app
    }

    fn wic_running_workspace_app() -> (App, WicSessionId) {
        let mut app = wic_workspace_app();
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedWicCreate);
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicCreate);
        let Some(yoctui_model::Effect::StartWicSession { id, .. }) =
            yoctui_model::update(&mut app, yoctui_model::Action::ConfirmWicCreate)
        else {
            panic!("expected Wic session");
        };
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::WicSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::WicSessionRunning { id });
        (app, id)
    }

    #[test]
    fn wic_workspace_renders_capability_dialogs_jobs_outputs_and_responsive_states() {
        let mut app = wic_workspace_app();
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            if width == 160 {
                assert!(output.contains("Wic capability"), "{output}");
                assert!(output.contains("ready for selected image"), "{output}");
            }
        }
        assert!(rendered_text(&app, 70, 20).contains("needs at least 80x24"));

        let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedWicCreate);
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Wic create.toml"), "{output}");
            assert!(output.contains("machine = \"qemux86-64\""), "{output}");
        }
        assert_eq!(app.focus, FocusTarget::Dialog);
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::ToggleWicCreateTomlEditor);
        assert!(rendered_text(&app, 80, 24).contains("INSERT"));
        if let Some(Dialog::WicCreateTomlEditor { content, .. }) = app.active_dialog_mut() {
            *content = "machine = \"qemux86-64\"\nimage = \"core-image-minimal\"\nkickstart = \"directdisk\"\noutput_directory = \"relative-output\"\ngenerate_bmap = true\ncompression = \"none\"\n".into();
        }
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicCreate);
        assert!(rendered_text(&app, 80, 24).contains("Validation:"));
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::DismissNotification);
        if let Some(Dialog::WicCreateTomlEditor { content, .. }) = app.active_dialog_mut() {
            *content = "machine = \"qemux86-64\"\nimage = \"core-image-minimal\"\nkickstart = \"directdisk\"\noutput_directory = \"/deploy/qemux86-64\"\ngenerate_bmap = true\ncompression = \"none\"\n".into();
        }
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicCreate);
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let confirmation = rendered_text(&app, width, height);
            assert!(
                confirmation.contains("Exact argument vector"),
                "{confirmation}"
            );
            assert!(confirmation.contains("Partitions"), "{confirmation}");
            assert!(
                confirmation.contains("part / --source=rootfs"),
                "{confirmation}"
            );
        }
        let Some(yoctui_model::Effect::StartWicSession { id, .. }) =
            yoctui_model::update(&mut app, yoctui_model::Action::ConfirmWicCreate)
        else {
            panic!("Wic start");
        };
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::WicSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::WicSessionRunning { id });
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::AppendWicSessionOutput {
                id,
                stream: yoctui_model::WicOutputStream::Stderr,
                line: "creation warning".into(),
                truncated: true,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        );
        app.dialogs.push_front(Dialog::WicCancellationConfirmation {
            id,
            incomplete_device_warning: false,
        });
        assert!(rendered_text(&app, 80, 24).contains("Confirm Wic cancellation"));
        app.dialogs.clear();
        let output = yoctui_model::WicOutput {
            identity: yoctui_model::WicOutputIdentity {
                path: "/deploy/qemux86-64/core-image-minimal.wic".into(),
                size_bytes: 4096,
                modified_unix_seconds: 1,
            },
            kind: yoctui_model::WicOutputKind::Wic,
        };
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::CompleteWicSession {
                id,
                exit_code: 0,
                outputs: vec![output],
                limitations: vec!["one dynamic field".into()],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let rendered = rendered_text(&app, 160, 40);
        assert!(rendered.contains("Status: succeeded"), "{rendered}");
        assert!(rendered.contains("core-image-minimal.wic"), "{rendered}");
        assert!(
            rendered.contains("creation warning [truncated]"),
            "{rendered}"
        );

        for capability in [
            WicCapability::MissingTool,
            WicCapability::MissingKickstarts {
                executable: "/usr/bin/wic".into(),
            },
            WicCapability::Failed {
                message: "inspection denied".into(),
            },
        ] {
            app.wic_capability = capability;
            let rendered = rendered_text(&app, 160, 40);
            assert!(rendered.contains("Wic capability"));
        }
    }

    #[test]
    fn wic_workspace_renders_all_capability_inventory_and_terminal_states() {
        let mut app = wic_workspace_app();
        for (capability, expected) in [
            (WicCapability::NotInspected, "not inspected"),
            (WicCapability::MissingTool, "missing wic executable"),
            (
                WicCapability::MissingKickstarts {
                    executable: "/usr/bin/wic".into(),
                },
                "no kickstarts available",
            ),
            (
                WicCapability::Failed {
                    message: "permission denied".into(),
                },
                "inspection failed: permission denied",
            ),
        ] {
            app.wic_capability = capability;
            let rendered = rendered_text(&app, 160, 40);
            assert!(rendered.contains(expected), "{rendered}");
        }

        let request = yoctui_model::WicOutputInventoryRequest {
            generation: 7,
            output_directory: "/deploy/qemux86-64".into(),
        };
        for (inventory, expected) in [
            (WicOutputInventoryState::NotLoaded, "not loaded"),
            (
                WicOutputInventoryState::Loading {
                    request: request.clone(),
                },
                "loading generation 7",
            ),
            (
                WicOutputInventoryState::Failed {
                    request: request.clone(),
                    message: "scan denied".into(),
                },
                "failed generation 7",
            ),
            (
                WicOutputInventoryState::Available {
                    request: request.clone(),
                    outputs: Vec::new(),
                },
                "none generated",
            ),
            (
                WicOutputInventoryState::Partial {
                    request,
                    outputs: Vec::new(),
                    limitations: vec!["symlink skipped".into()],
                },
                "symlink skipped",
            ),
        ] {
            app.wic_outputs = inventory;
            let rendered = rendered_text(&app, 160, 40);
            assert!(rendered.contains(expected), "{rendered}");
        }

        let output = yoctui_model::WicOutput {
            identity: yoctui_model::WicOutputIdentity {
                path: "/deploy/qemux86-64/selected.direct".into(),
                size_bytes: 8_192,
                modified_unix_seconds: 9,
            },
            kind: yoctui_model::WicOutputKind::Direct,
        };
        app.wic_output_selection = Some(output.identity.clone());
        app.wic_outputs = WicOutputInventoryState::Available {
            request: yoctui_model::WicOutputInventoryRequest {
                generation: 8,
                output_directory: "/deploy/qemux86-64".into(),
            },
            outputs: vec![output],
        };
        let rendered = rendered_text(&app, 160, 40);
        assert!(
            rendered.contains("▶ Direct /deploy/qemux86-64/selected.direct (8192 bytes, 9s)"),
            "{rendered}"
        );

        let mut lifecycle = wic_workspace_app();
        let _ = yoctui_model::update(&mut lifecycle, yoctui_model::Action::BeginSelectedWicCreate);
        let _ = yoctui_model::update(&mut lifecycle, yoctui_model::Action::PreviewWicCreate);
        let Some(yoctui_model::Effect::StartWicSession {
            id: lifecycle_id, ..
        }) = yoctui_model::update(&mut lifecycle, yoctui_model::Action::ConfirmWicCreate)
        else {
            panic!("expected Wic session");
        };
        assert!(rendered_text(&lifecycle, 160, 40).contains("Status: queued"));
        let _ = yoctui_model::update(
            &mut lifecycle,
            yoctui_model::Action::WicSessionStarting {
                id: lifecycle_id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&lifecycle, 160, 40).contains("Status: starting"));
        let _ = yoctui_model::update(
            &mut lifecycle,
            yoctui_model::Action::WicSessionRunning { id: lifecycle_id },
        );
        assert!(rendered_text(&lifecycle, 160, 40).contains("Status: running"));
        let _ = yoctui_model::update(
            &mut lifecycle,
            yoctui_model::Action::BeginActiveWicSessionCancellation,
        );
        let _ = yoctui_model::update(
            &mut lifecycle,
            yoctui_model::Action::ConfirmWicSessionCancellation {
                id: lifecycle_id,
                acknowledge_incomplete_device: false,
            },
        );
        assert!(rendered_text(&lifecycle, 160, 40).contains("Status: cancelling"));

        let (mut succeeded, succeeded_id) = wic_running_workspace_app();
        let _ = yoctui_model::update(
            &mut succeeded,
            yoctui_model::Action::CompleteWicSession {
                id: succeeded_id,
                exit_code: 0,
                outputs: Vec::new(),
                limitations: Vec::new(),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&succeeded, 160, 40).contains("Status: succeeded"));

        let (mut failed, failed_id) = wic_running_workspace_app();
        let _ = yoctui_model::update(
            &mut failed,
            yoctui_model::Action::FailWicSession {
                id: failed_id,
                message: "creator failed".into(),
                exit_code: Some(2),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let rendered = rendered_text(&failed, 160, 40);
        assert!(rendered.contains("Status: failed"), "{rendered}");
        assert!(rendered.contains("creator failed"), "{rendered}");

        let (mut lost, lost_id) = wic_running_workspace_app();
        let _ = yoctui_model::update(
            &mut lost,
            yoctui_model::Action::LoseWicSession {
                id: lost_id,
                message: "creator disappeared".into(),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&lost, 160, 40).contains("Status: lost"));

        let (mut cancelled, cancelled_id) = wic_running_workspace_app();
        let _ = yoctui_model::update(
            &mut cancelled,
            yoctui_model::Action::BeginActiveWicSessionCancellation,
        );
        let _ = yoctui_model::update(
            &mut cancelled,
            yoctui_model::Action::ConfirmWicSessionCancellation {
                id: cancelled_id,
                acknowledge_incomplete_device: false,
            },
        );
        let _ = yoctui_model::update(
            &mut cancelled,
            yoctui_model::Action::CancelWicSession {
                id: cancelled_id,
                exit_code: Some(130),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&cancelled, 160, 40).contains("Status: cancelled"));
    }

    #[test]
    fn wic_device_write_renders_protected_dialogs_inventory_history_and_footer() {
        let mut app = wic_workspace_app();
        let Some(yoctui_model::Effect::GetWicDevices(request)) =
            yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedWicDeviceWrite)
        else {
            panic!("expected protected device discovery");
        };
        let loading = rendered_text(&app, 80, 24);
        assert!(
            loading.contains("Select protected Wic write device"),
            "{loading}"
        );
        assert!(loading.contains("Discovering removable"), "{loading}");

        let mut empty = app.clone();
        empty.wic_devices = WicDeviceInventoryState::Available {
            request: request.clone(),
            devices: Vec::new(),
        };
        let rendered = rendered_text(&empty, 100, 30);
        assert!(
            rendered.contains("No eligible removable whole devices"),
            "{rendered}"
        );

        let mut failed = app.clone();
        failed.wic_devices = WicDeviceInventoryState::Failed {
            request: request.clone(),
            message: "lsblk permission denied".into(),
        };
        let rendered = rendered_text(&failed, 100, 30);
        assert!(
            rendered.contains("Device discovery failed: lsblk permission denied"),
            "{rendered}"
        );

        let device = WicDevice {
            identity: yoctui_model::WicDeviceIdentity {
                path: "/dev/sdz".into(),
                major_minor: "8:240".into(),
                size_bytes: 16_384,
                model: Some("Protected USB".into()),
                serial: Some("SERIAL-123".into()),
                transport: Some("usb".into()),
            },
            removable: true,
            writable: true,
            read_only: false,
            descendant_mounts: Vec::new(),
            unavailable_reason: None,
        };
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::WicDeviceInventoryLoaded {
                request: request.clone(),
                devices: vec![device],
                limitations: vec!["one udev property was unavailable".into()],
            },
        );
        for (theme, color_enabled) in [
            (Theme::DarkPro, true),
            (Theme::WhiteClassic, true),
            (Theme::MatrixGreen, true),
            (Theme::HighContrast, true),
            (Theme::Monochrome, false),
        ] {
            app.theme = theme;
            app.color_enabled = color_enabled;
            for (width, height) in [(80, 24), (110, 30), (160, 40)] {
                let rendered = rendered_text(&app, width, height);
                assert!(rendered.contains("/dev/sdz"), "{rendered}");
                assert!(rendered.contains("8:240"), "{rendered}");
                assert!(rendered.contains("Protected USB"), "{rendered}");
                assert!(rendered.contains("SERIAL-123"), "{rendered}");
                assert!(rendered.contains("transport=usb"), "{rendered}");
                assert!(rendered.contains("removable=true"), "{rendered}");
                assert!(
                    rendered.contains("one udev property was unavailable"),
                    "{rendered}"
                );
            }
        }

        let _ = yoctui_model::update(&mut app, yoctui_model::Action::ConfirmWicDeviceSelection);
        let phrase = rendered_text(&app, 80, 24);
        assert!(
            phrase.contains("Required phrase: WRITE /dev/sdz"),
            "{phrase}"
        );
        assert!(phrase.contains("phrase alone does not write"), "{phrase}");
        for character in "WRONG".chars() {
            let _ = yoctui_model::update(
                &mut app,
                yoctui_model::Action::AppendWicWritePhrase(character),
            );
        }
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicDeviceWrite);
        assert!(rendered_text(&app, 100, 30).contains("Validation:"));
        for _ in 0.."WRONG".len() {
            let _ = yoctui_model::update(&mut app, yoctui_model::Action::BackspaceWicWritePhrase);
        }
        for character in "WRITE /dev/sdz".chars() {
            let _ = yoctui_model::update(
                &mut app,
                yoctui_model::Action::AppendWicWritePhrase(character),
            );
        }
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicDeviceWrite);
        for (width, height) in [(80, 24), (110, 30), (160, 40)] {
            let preview = rendered_text(&app, width, height);
            assert!(preview.contains("DESTRUCTIVE OPERATION"), "{preview}");
            assert!(preview.contains("Exact argument vector"), "{preview}");
            assert!(preview.contains("[1]=write"), "{preview}");
            assert!(preview.contains("[2]=/deploy/qemux86-64"), "{preview}");
            assert!(preview.contains("[3]=/dev/sdz"), "{preview}");
        }

        let Some(yoctui_model::Effect::StartWicSession { id, .. }) =
            yoctui_model::update(&mut app, yoctui_model::Action::ConfirmWicDeviceWrite)
        else {
            panic!("expected managed Wic device write");
        };
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::WicSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::WicSessionRunning { id });
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::AppendWicSessionOutput {
                id,
                stream: yoctui_model::WicOutputStream::Stderr,
                line: "write progress".into(),
                truncated: true,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        );
        app.host_telemetry = yoctui_model::HostTelemetry {
            cpu_utilization_percent: Some(42),
            disk_available_bytes: Some(1_048_576),
        };
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::BeginActiveWicSessionCancellation,
        );
        let warning = rendered_text(&app, 80, 24);
        assert!(
            warning.contains("Confirm Wic device-write cancellation"),
            "{warning}"
        );
        assert!(warning.contains("incomplete"), "{warning}");
        assert!(warning.contains("unusable"), "{warning}");
        app.dialogs.clear();
        let inspector = wic_inspector_text(&app);
        assert!(inspector.contains("write\nimage=/deploy/qemux86-64"));
        assert!(inspector.contains("device=/dev/sdz major:minor=8:240"));
        assert!(inspector.contains("Host telemetry: CPU 42%"));
        assert!(inspector.contains("write progress [truncated]"));
        assert!(inspector.contains("Dropped output: 0 entries"));

        let footer = footer_shortcuts(&app);
        assert!(footer.contains("D write device"), "{footer}");
        assert!(
            responsive_footer_shortcuts(&app, 80).contains("D write"),
            "{}",
            responsive_footer_shortcuts(&app, 80)
        );
    }

    #[test]
    fn wic_workspace_handles_long_source_themes_and_exact_footer_hints() {
        let mut app = wic_workspace_app();
        if let WicCapability::Available { kickstarts, .. } = &mut app.wic_capability {
            kickstarts[0].source = (0..200)
                .map(|index| format!("part /p{index} --source=rootfs # row {index}"))
                .collect::<Vec<_>>()
                .join("\n");
        }
        for theme in [
            Theme::DarkPro,
            Theme::WhiteClassic,
            Theme::MatrixGreen,
            Theme::HighContrast,
            Theme::Monochrome,
        ] {
            app.theme = theme;
            let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedWicCreate);
            let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicCreate);
            let rendered = rendered_text(&app, 80, 24);
            assert!(
                rendered.contains("Confirm managed Wic creation"),
                "{rendered}"
            );
            let _ = yoctui_model::update(&mut app, yoctui_model::Action::CancelWicCreatePreview);
        }
        let footer = footer_shortcuts(&app);
        for expected in [
            "Q QEMU",
            "W create Wic",
            "D write device",
            "x cancel",
            "[/] output",
            "O open output",
            "w Wic",
        ] {
            assert!(footer.contains(expected), "{footer}");
        }
        app.theme = Theme::DarkPro;
        let preview = source_preview(
            "part / --source=rootfs # root partition",
            "directdisk.wks",
            &app,
        );
        assert_ne!(preview.lines[0].spans[0].style, Style::default());
    }

    #[test]
    fn animation_is_absent_from_determinate_and_terminal_rows() {
        let mut app = App::new(10, 1_000);
        app.tasks.insert(
            yoctui_model::TaskId("busybox:do_compile".into()),
            yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(42),
                ..yoctui_model::TaskInfo::default()
            },
        );
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("base-files:do_install".into()),
                recipe: "base-files".into(),
                task: "do_install".into(),
                progress: None,
                ..yoctui_model::TaskInfo::default()
            },
            success: true,
        });
        let output = rendered_text(&app, 300, 30);
        assert!(output.contains("busybox:do_compile 42%"));
        assert!(
            output.contains("base-files:do_install 100% complete"),
            "{output}"
        );
        assert!(!output.contains("base-files:do_install▸"));
        assert!(!output.contains("base-files:do_install active"));
    }

    #[test]
    fn renders_small_terminal() {
        let mut terminal = Terminal::new(TestBackend::new(62, 18)).unwrap();
        terminal.draw(|f| render(f, &App::new(1, 1))).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|c| c.symbol() == "Y")
        );
    }
    #[test]
    fn persistent_shell_degrades_across_supported_terminal_widths() {
        for (width, height, expected) in [
            (140, 30, "Inspector"),
            (100, 24, "Navigator"),
            (80, 24, "Build"),
        ] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| render(frame, &App::new(10, 1_000)))
                .unwrap();
            let output = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(
                output.contains(expected),
                "{width}x{height} should show {expected}"
            );
        }
    }
    #[test]
    fn responsive_shell_uses_semantic_content_at_every_breakpoint() {
        let mut app = App::new(10, 1_000);

        let wide = rendered_text(&app, 130, 24);
        assert!(wide.contains("Navigator"));
        assert!(wide.contains("Build"));
        assert!(wide.contains("Inspector"));

        let medium = rendered_text(&app, 129, 24);
        assert!(medium.contains("Navigator"));
        assert!(medium.contains("Build"));
        assert!(!medium.contains("Inspector"));

        app.focus = FocusTarget::Inspector;
        let medium_inspector = rendered_text(&app, 100, 24);
        assert!(medium_inspector.contains("Navigator"));
        assert!(medium_inspector.contains("Inspector"));

        app.focus = FocusTarget::Workspace;
        let narrow_workspace = rendered_text(&app, 99, 24);
        assert!(narrow_workspace.contains("Panes: Navigator  [Workspace]  Inspector"));
        assert!(narrow_workspace.contains("Build"));

        app.focus = FocusTarget::Navigator;
        let narrow_navigator = rendered_text(&app, 80, 24);
        assert!(narrow_navigator.contains("Panes: [Navigator]  Workspace  Inspector"));
        assert!(narrow_navigator.contains("Dashboard"));

        app.focus = FocusTarget::Inspector;
        let narrow_inspector = rendered_text(&app, 80, 24);
        assert!(narrow_inspector.contains("Panes: Navigator  Workspace  [Inspector]"));
        assert!(narrow_inspector.contains("Select an item in the workspace"));

        let too_small = rendered_text(&app, 79, 23);
        assert!(too_small.contains("Yoctui needs at least 80x24"));
        assert!(too_small.contains("Current terminal: 79x23"));
    }
    #[test]
    fn responsive_resize_preserves_the_selected_pane() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;
        let mut terminal = Terminal::new(TestBackend::new(130, 24)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();

        terminal.backend_mut().resize(100, 24);
        terminal.autoresize().unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let medium = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(medium.contains("Inspector"));

        terminal.backend_mut().resize(80, 24);
        terminal.autoresize().unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let narrow = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(narrow.contains("[Inspector]"));
        assert_eq!(app.focus, FocusTarget::Inspector);
    }
    #[test]
    fn responsive_all_screens_and_dialogs_render_at_boundary_sizes() {
        let screens = [
            Screen::Dashboard,
            Screen::Tasks,
            Screen::BuildHistory,
            Screen::Dependencies,
            Screen::LayerRelationships,
            Screen::Recipes,
            Screen::Images,
            Screen::Sdk,
            Screen::Testing,
            Screen::Security,
            Screen::Layers,
            Screen::Configuration,
            Screen::Bbmask,
            Screen::Maintenance,
            Screen::Logs,
            Screen::Errors,
            Screen::Help,
            Screen::Settings,
        ];
        for screen in screens {
            for (width, height) in [(130, 24), (129, 24), (100, 24), (99, 24), (80, 24)] {
                let mut app = App::new(10, 1_000);
                app.screen = screen;
                let _ = rendered_text(&app, width, height);
            }
        }

        let mut build_options = App::new(10, 1_000);
        build_options.dialogs.push_back(Dialog::BuildOptions);
        build_options.focus = FocusTarget::Dialog;
        let _ = rendered_text(&build_options, 80, 24);

        let mut palette = App::new(10, 1_000);
        palette.command_palette_open = true;
        palette.focus = FocusTarget::CommandPalette;
        let _ = rendered_text(&palette, 80, 24);

        let mut confirmation = App::new(10, 1_000);
        confirmation
            .dialogs
            .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["base-files".into()],
                task: Some("listtasks".into()),
                force: false,
            }));
        confirmation.focus = FocusTarget::Dialog;
        let _ = rendered_text(&confirmation, 80, 24);
    }
    #[test]
    fn dialog_families_render_on_narrow_supported_terminals() {
        let dialogs = vec![
            (Dialog::BuildOptions, "Image build options"),
            (Dialog::BuildCompletion, "Build finished"),
            (
                Dialog::BuildTarget {
                    content: "target = \"busybox\"\n".into(),
                    task: None,
                    editing: false,
                },
                "Build target",
            ),
            (
                Dialog::ImagePicker(yoctui_model::ImagePicker {
                    images: vec!["core-image-minimal".into()],
                    selection: 0,
                }),
                "Available image targets",
            ),
            (
                Dialog::RecipeTaskConfirmation(BuildRequest {
                    targets: vec!["busybox".into()],
                    task: None,
                    force: false,
                }),
                "Confirm recipe task",
            ),
            (
                Dialog::DevtoolModifyConfirmation(yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                }),
                "Confirm Devtool modify",
            ),
            (
                Dialog::DevtoolResetConfirmation(yoctui_model::DevtoolResetPlan {
                    identity: yoctui_model::RecipeIdentity {
                        name: "busybox".into(),
                        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                    },
                    source_path: "/build/workspace/sources/busybox".into(),
                }),
                "Confirm Devtool reset",
            ),
            (
                Dialog::DevtoolUpdateConfirmation(yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                }),
                "Confirm Devtool update-recipe",
            ),
            (
                Dialog::DevtoolFinishPicker(yoctui_model::DevtoolFinishPicker {
                    identity: yoctui_model::RecipeIdentity {
                        name: "busybox".into(),
                        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                    },
                    layers: vec![yoctui_model::Layer {
                        name: "meta".into(),
                        path: "/layers/meta".into(),
                        priority: Some(5),
                    }],
                    selection: 0,
                }),
                "Devtool finish busybox",
            ),
            (
                Dialog::DevtoolFinishConfirmation(yoctui_model::DevtoolFinishPlan {
                    identity: yoctui_model::RecipeIdentity {
                        name: "busybox".into(),
                        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                    },
                    layer: yoctui_model::Layer {
                        name: "meta".into(),
                        path: "/layers/meta".into(),
                        priority: Some(5),
                    },
                }),
                "Confirm Devtool finish",
            ),
            (
                Dialog::DevtoolDeploy(yoctui_model::DevtoolDeployDraft {
                    identity: yoctui_model::RecipeIdentity {
                        name: "busybox".into(),
                        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                    },
                    target: "qemu".into(),
                }),
                "Devtool deploy target",
            ),
            (
                Dialog::DevtoolDeployConfirmation(yoctui_model::DevtoolDeployPlan {
                    identity: yoctui_model::RecipeIdentity {
                        name: "busybox".into(),
                        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                    },
                    target: "qemu".into(),
                }),
                "Confirm Devtool deploy-target",
            ),
            (
                Dialog::BbmaskEdit {
                    content: "bbmask = \"meta-old/.*\"\n".into(),
                    editing: false,
                },
                "BBMASK.toml",
            ),
            (
                Dialog::BbmaskConfirmation("meta-old/.*".into()),
                "Confirm BBMASK change",
            ),
            (
                Dialog::RecipeEditor(RecipeEditor {
                    recipe: "busybox".into(),
                    root: "/workspace/busybox".into(),
                    files: vec!["main.c".into()],
                    selection: 0,
                    content: "int main() {}".into(),
                    editing: false,
                    dirty: false,
                }),
                "Workspace file tree",
            ),
            (Dialog::QuitConfirmation, "Confirm quit"),
        ];

        for (dialog, title) in dialogs {
            let mut app = App::new(10, 1_000);
            app.build.status = yoctui_model::BuildStatus::Completed;
            app.dialogs.push_back(dialog);
            let output = rendered_text(&app, 80, 24);
            assert!(output.contains(title), "missing {title} in narrow dialog");
        }
    }
    #[test]
    fn command_palette_renders_search_results_and_disabled_explanations() {
        let mut app = App::new(10, 1_000);
        app.command_palette_open = true;
        app.command_palette_query = "build".into();
        let output = rendered_text(&app, 100, 25);
        assert!(output.contains("Search: build_"));
        assert!(output.contains("Build image"));
        assert!(output.contains("unavailable"));
        assert!(output.contains("Load a Yocto workspace first"));
        assert!(output.contains("Type to search"));

        app.command_palette_query = "nothing matches this".into();
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("No commands match"));
    }
    #[test]
    fn command_palette_selection_description_and_shortcut_render_in_all_themes() {
        for theme in [
            Theme::DarkPro,
            Theme::WhiteClassic,
            Theme::MatrixGreen,
            Theme::HighContrast,
            Theme::Monochrome,
        ] {
            let mut app = App::new(10, 1_000);
            app.theme = theme;
            app.command_palette_open = true;
            app.command_palette_query = "Open Settings".into();
            let output = rendered_text(&app, 80, 24);
            assert!(output.contains("Open Settings"));
            assert!(output.contains("persistent visual"));
            assert!(output.contains("[none]"));
        }
    }
    #[test]
    fn dialog_focus_is_trapped_then_visibly_restored_to_inspector() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;
        let _ = update(&mut app, Action::OpenBuildOptions);

        let dialog = rendered_text(&app, 100, 24);
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert!(dialog.contains("Image build options"));
        assert!(!dialog.contains("Panes:"));

        let _ = update(&mut app, Action::CloseBuildOptions);
        let restored = rendered_text(&app, 100, 24);
        assert_eq!(app.focus, FocusTarget::Inspector);
        assert!(restored.contains("Inspector"));
        assert!(!restored.contains("Image build options"));
    }
    #[test]
    fn formats_error_timestamp_without_panicking() {
        assert_eq!(timestamp_text(UNIX_EPOCH), "0s since Unix epoch");
    }
    #[test]
    fn no_color_selection_uses_reverse_video() {
        let mut app = App::new(10, 1_000);
        app.color_enabled = false;
        assert!(
            selected_style(&app, true)
                .add_modifier
                .contains(Modifier::REVERSED)
        );
        assert_eq!(selected_style(&app, true).bg, None);
    }

    #[test]
    fn renders_notification() {
        let mut app = App::new(1, 1);
        app.notification = Some("Backend unavailable".into());
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();
        let screen = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(screen.contains("Backend unavailable"));
    }
    #[test]
    fn dashboard_renders_backend_and_build_metrics() {
        let mut terminal = Terminal::new(TestBackend::new(160, 32)).unwrap();
        let mut app = App::new(10, 1_000);
        app.backend = "bridge".into();
        app.build.completed = 3;
        app.build.total = Some(7);
        app.build.warnings = 2;
        app.build.errors = 1;
        app.workspace.release = Some("kirkstone".into());
        app.workspace.source_dir = Some("/src/poky".into());
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Backend: bridge"));
        assert!(output.contains("Tasks: 3/7"));
        assert!(output.contains("Warnings: 2  Errors: 1"));
        assert!(output.contains("Yocto: kirkstone @ /src/poky"));
    }
    #[test]
    fn bbmask_footer_shows_its_edit_shortcut() {
        let mut terminal = Terminal::new(TestBackend::new(300, 40)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Bbmask;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("e edit BBMASK"));
    }
    #[test]
    fn dashboard_renders_host_cpu_and_build_disk_space() {
        let mut terminal = Terminal::new(TestBackend::new(300, 40)).unwrap();
        let mut app = App::new(10, 1_000);
        app.host_telemetry.cpu_utilization_percent = Some(42);
        app.host_telemetry.disk_available_bytes = Some(8 * 1024 * 1024 * 1024);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Host CPU: 42%"));
        assert!(output.contains("Disk 8.0 GiB"));
    }
    #[test]
    fn dashboard_renders_parse_progress() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.build.parse_current = Some(8);
        app.build.parse_total = Some(20);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Parse progress: 8/20"));
    }
    #[test]
    fn dashboard_renders_build_exit_code() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.build.exit_code = Some(1);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Exit code: 1"));
    }
    #[test]
    fn build_history_renders_completed_builds() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::BuildHistory;
        app.build_history.push_back(yoctui_model::BuildRecord {
            target: Some("core-image-minimal".into()),
            success: true,
            exit_code: Some(0),
            elapsed: Some(std::time::Duration::from_secs(65)),
            completed_tasks: 42,
            warnings: 1,
            errors: 0,
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Build history"));
        assert!(output.contains("core-image-minimal"));
        assert!(output.contains("Completed package tasks: 42"));
    }
    #[test]
    fn dependency_workspace_renders_typed_partial_graph_paths_and_responsive_states() {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Dependencies;
        let root = DependencyNodeId::recipe("image");
        let task = DependencyNodeId::task("busybox", "do_compile");
        let orphan = DependencyNodeId::recipe("orphan");
        let (graph, _) = DependencyGraph::normalize(
            root.clone(),
            vec![
                yoctui_model::DependencyNode {
                    id: task.clone(),
                    provider: Some("/layers/meta/busybox.bb".into()),
                    log: Some("/build/tmp/log.do_compile".into()),
                },
                yoctui_model::DependencyNode::identity(orphan.clone()),
            ],
            vec![
                yoctui_model::DependencyEdge {
                    from: root.clone(),
                    to: task.clone(),
                    kind: DependencyEdgeKind::Task,
                },
                yoctui_model::DependencyEdge {
                    from: task.clone(),
                    to: root.clone(),
                    kind: DependencyEdgeKind::Task,
                },
            ],
            100,
            100,
        );
        app.dependency_graph = DependencyGraphState::Partial {
            graph,
            limitations: vec!["runtime edges unavailable".into()],
        };
        app.dependency_graph_selection = Some(task);
        let output = rendered_text(&app, 160, 36);
        assert!(output.contains("Dependency graph"));
        assert!(output.contains("busybox:do_compile"));
        assert!(output.contains("runtime edges unavailable"));
        assert!(output.contains("--task-->"));
        assert!(output.contains("/layers/meta/busybox.bb"));
        assert!(output.contains("Reverse / incoming"));
        for width in [129, 100, 80] {
            let output = rendered_text(&app, width, 24);
            assert!(output.contains("Dependency graph"));
        }

        app.focus = FocusTarget::Inspector;
        app.dependency_graph_selection = Some(orphan);
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("unreachable from root"));

        app.focus = FocusTarget::Workspace;
        app.dependency_graph = DependencyGraphState::NotLoaded;
        assert!(rendered_text(&app, 80, 24).contains("not loaded"));
        app.dependency_graph = DependencyGraphState::Loading { root: root.clone() };
        assert!(rendered_text(&app, 80, 24).contains("Stale rows are hidden"));
        app.dependency_graph = DependencyGraphState::AvailableEmpty { root: root.clone() };
        assert!(rendered_text(&app, 80, 24).contains("No dependency edges reported"));
        app.dependency_graph = DependencyGraphState::Failed {
            root,
            message: "server unavailable".into(),
        };
        assert!(rendered_text(&app, 80, 24).contains("server unavailable"));
    }
    #[test]
    fn dependency_workspace_reports_path_bounds_without_panicking() {
        let root = DependencyNodeId::recipe("node-0");
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for index in 1..=66 {
            let previous = DependencyNodeId::recipe(format!("node-{}", index - 1));
            let current = DependencyNodeId::recipe(format!("node-{index}"));
            nodes.push(yoctui_model::DependencyNode::identity(current.clone()));
            edges.push(yoctui_model::DependencyEdge {
                from: previous,
                to: current,
                kind: DependencyEdgeKind::Build,
            });
        }
        let selected = DependencyNodeId::recipe("node-66");
        let (graph, _) = DependencyGraph::normalize(root, nodes, edges, 100, 100);
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Dependencies;
        app.focus = FocusTarget::Inspector;
        app.dependency_graph = DependencyGraphState::Available(graph);
        app.dependency_graph_selection = Some(selected);
        assert!(rendered_text(&app, 80, 24).contains("path limit reached"));
    }
    #[test]
    fn dashboard_renders_colored_task_progress_labels() {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.tasks.insert(
            yoctui_model::TaskId("busybox:do_compile".into()),
            yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(42),
                ..yoctui_model::TaskInfo::default()
            },
        );
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("busybox:do_compile 42%"));
    }
    #[test]
    fn dashboard_renders_completed_and_failed_package_tasks() {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(100),
                ..yoctui_model::TaskInfo::default()
            },
            success: true,
        });
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("bash:do_install".into()),
                recipe: "bash".into(),
                task: "do_install".into(),
                progress: Some(100),
                ..yoctui_model::TaskInfo::default()
            },
            success: false,
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("busybox:do_compile 100% complete"));
        assert!(output.contains("bash:do_install 100% failed"));
    }
    #[test]
    fn renders_build_target_editor() {
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::BuildTarget {
            content: "target = \"core-image-minimal\"\n".into(),
            task: None,
            editing: false,
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Build target.toml"));
        assert!(output.contains("core-image-minimal"));
    }
    #[test]
    fn renders_machine_aware_build_options() {
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::BuildOptions);
        app.build.target = Some("core-image-minimal".into());
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemuarm".into());
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Image build options"));
        assert!(output.contains("qemuarm"));
        assert!(output.contains("Clean image"));
    }
    #[test]
    fn logs_identify_evicted_warnings_and_errors() {
        let mut terminal = Terminal::new(TestBackend::new(300, 30)).unwrap();
        let mut app = App::new(1, 1_000);
        app.screen = Screen::Logs;
        app.logs.dropped = 3;
        app.logs.dropped_warnings = 1;
        app.logs.dropped_errors = 2;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("3 evicted [W 1 E 2]"));
    }
    #[test]
    fn renders_recipe_task_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs
            .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("cleansstate".into()),
                force: false,
            }));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm recipe task"));
        assert!(output.contains("cleansstate"));
    }
    #[test]
    fn devtool_target_reset_renders_exact_destructive_identity_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::DevtoolResetConfirmation(
            yoctui_model::DevtoolResetPlan {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                source_path: "/build/workspace/sources/busybox".into(),
            },
        ));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm Devtool reset"));
        assert!(output.contains("devtool reset busybox"));
        assert!(output.contains("busybox.bb"));
        assert!(output.contains("/build/workspace/sources/busybox"));
    }
    #[test]
    fn devtool_publish_update_renders_exact_identity_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::DevtoolUpdateConfirmation(
            yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
            },
        ));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm Devtool update-recipe"));
        assert!(output.contains("devtool update-recipe busybox"));
        assert!(output.contains("busybox.bb"));
    }
    #[test]
    fn devtool_publish_finish_renders_configured_picker_and_exact_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        let identity = yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
        };
        let layer = yoctui_model::Layer {
            name: "meta-demo".into(),
            path: "/layers/meta-demo".into(),
            priority: Some(7),
        };
        app.dialogs.push_back(Dialog::DevtoolFinishPicker(
            yoctui_model::DevtoolFinishPicker {
                identity: identity.clone(),
                layers: vec![layer.clone()],
                selection: 0,
            },
        ));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Configured layer"));
        assert!(output.contains("meta-demo"));
        assert!(output.contains("/layers/meta-demo"));

        app.dialogs.clear();
        app.dialogs.push_back(Dialog::DevtoolFinishConfirmation(
            yoctui_model::DevtoolFinishPlan { identity, layer },
        ));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm Devtool finish"));
        assert!(output.contains("devtool finish busybox /layers/meta-demo"));
        assert!(output.contains("busybox.bb"));
        assert!(output.contains("Configured layer: meta-demo"));
    }
    #[test]
    fn devtool_target_deploy_renders_identity_entry_and_exact_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        let identity = yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
        };
        app.dialogs
            .push_back(Dialog::DevtoolDeploy(yoctui_model::DevtoolDeployDraft {
                identity: identity.clone(),
                target: "qemuarm".into(),
            }));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Devtool deploy target"));
        assert!(output.contains("busybox.bb"));
        assert!(output.contains("qemuarm"));

        app.dialogs.clear();
        app.dialogs.push_back(Dialog::DevtoolDeployConfirmation(
            yoctui_model::DevtoolDeployPlan {
                identity,
                target: "qemuarm".into(),
            },
        ));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm Devtool deploy-target"));
        assert!(output.contains("devtool deploy-target busybox qemuarm"));
        assert!(output.contains("busybox.bb"));
    }
    #[test]
    fn devtool_modify_renders_confirmation_and_workspace_editor_build_shortcut() {
        let mut confirmation = App::new(10, 1_000);
        confirmation
            .dialogs
            .push_back(Dialog::DevtoolModifyConfirmation(
                yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
            ));
        let output = rendered_text(&confirmation, 120, 30);
        assert!(output.contains("Confirm Devtool modify"), "{output}");
        assert!(output.contains("devtool modify busybox"), "{output}");
        assert!(output.contains("busybox.bb"), "{output}");

        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::RecipeEditor(RecipeEditor {
            recipe: "busybox".into(),
            root: "/build/workspace/sources/busybox".into(),
            files: vec!["main.c".into()],
            selection: 0,
            content: "int main() {}".into(),
            editing: false,
            dirty: false,
        }));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Workspace file tree: busybox"));
        assert!(output.contains("int main() {}"));
        assert!(output.contains("Ctrl+B build recipe"));
    }
    #[test]
    fn layer_tree_renders_configured_layers_git_state_and_numbered_preview() {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Layers;
        app.workspace.layers.push(yoctui_model::Layer {
            name: "meta-demo".into(),
            path: "/layers/meta-demo".into(),
            priority: Some(7),
        });
        app.layer_relationships = Some(yoctui_model::LayerRelationships {
            layers: vec![yoctui_model::LayerRelationship {
                name: "meta-demo".into(),
                compatible: vec!["scarthgap".into()],
                ..yoctui_model::LayerRelationship::default()
            }],
        });
        let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
        browser.entries = vec![yoctui_model::LayerBrowserEntry {
            path: "/layers/meta-demo/conf/layer.conf".into(),
            is_dir: false,
            size: Some(31),
            git: yoctui_model::GitFileState::Modified,
            ..yoctui_model::LayerBrowserEntry::default()
        }];
        browser.preview = "BBFILE_COLLECTIONS += \\\"demo\\\"".into();
        browser.preview_kind = yoctui_model::PreviewKind::Text;
        app.layer_browser = Some(browser);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Configured layers"));
        assert!(output.contains("meta-demo"));
        assert!(output.contains("hidden off"));
        assert!(output.contains("layer.conf"));
        assert!(output.contains("BBFILE_COLLECTIONS"));
        assert!(output.contains("M"));
        assert!(output.contains("1"));
    }

    #[test]
    fn layer_tree_binary_preview_and_responsive_modes_never_render_bytes() {
        for (width, height) in [(160, 30), (110, 28), (90, 25), (70, 20)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Layers;
            let mut browser = LayerBrowser::new("meta-binary".into(), "/layers/meta-binary".into());
            browser.entries.push(yoctui_model::LayerBrowserEntry {
                path: "/layers/meta-binary/image.bin".into(),
                size: Some(100_000),
                git: yoctui_model::GitFileState::Unavailable,
                ..yoctui_model::LayerBrowserEntry::default()
            });
            browser.preview = "\0secret".into();
            browser.preview_kind = yoctui_model::PreviewKind::Binary;
            browser.preview_truncated = true;
            app.layer_browser = Some(browser);
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let output = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(!output.contains("secret"));
            if width >= 80 && height >= 24 {
                assert!(output.contains("Binary preview unavailable"));
            }
        }
    }
    #[test]
    fn bitbake_preview_highlights_assignments_and_comments() {
        let app = App::new(10, 1_000);
        let preview = source_preview("SUMMARY = \"demo\" # explanation", "demo.bb", &app);
        assert_eq!(
            preview.lines[0].spans[0].style.fg,
            Some(Color::Rgb(215, 175, 0))
        );
        assert_eq!(
            preview.lines[0].spans[1].style.fg,
            Some(Color::Rgb(175, 135, 215))
        );
        assert_eq!(
            preview.lines[0].spans[2].style.fg,
            Some(Color::Rgb(135, 215, 0))
        );
        assert_eq!(
            preview.lines[0].spans[3].style.fg,
            Some(Color::Rgb(154, 154, 154))
        );
    }
    #[test]
    fn renders_image_picker_for_active_machine() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.dialogs
            .push_back(Dialog::ImagePicker(yoctui_model::ImagePicker {
                images: vec!["core-image-minimal".into()],
                selection: 0,
            }));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Available image targets"));
        assert!(output.contains("qemux86-64"));
        assert!(output.contains("core-image-minimal"));
    }
    #[test]
    fn inspector_reflects_selected_recipe_and_layer_preview() {
        let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "busybox".into(),
            version: Some("1.36".into()),
            layer: Some("meta".into()),
            ..yoctui_model::Recipe::default()
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Recipe: busybox"));
        assert!(output.contains("Resolved version: 1.36"));

        app.screen = Screen::Layers;
        let mut browser = LayerBrowser::new("meta".into(), "/layers/meta".into());
        browser.directory = "/layers/meta/conf".into();
        browser.entries.push(yoctui_model::LayerBrowserEntry {
            path: "/layers/meta/conf/layer.conf".into(),
            ..yoctui_model::LayerBrowserEntry::default()
        });
        browser.preview = "BBFILE_COLLECTIONS += \"meta\"".into();
        browser.preview_kind = yoctui_model::PreviewKind::Text;
        app.layer_browser = Some(browser);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Path: /layers/meta/conf/layer.conf"));
        assert!(output.contains("BBFILE_COLLECTIONS"));
    }
    #[test]
    fn recipes_workspace_renders_authoritative_summary_and_inspector_sections() {
        let mut terminal = Terminal::new(TestBackend::new(180, 44)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.workspace.recipes = vec![
            yoctui_model::Recipe {
                name: "alpha".into(),
                ..yoctui_model::Recipe::default()
            },
            yoctui_model::Recipe {
                name: "busybox".into(),
                version: Some("1.36".into()),
                preferred_version: Some("1.36%".into()),
                layer: Some("core".into()),
                file: Some("/layers/meta/recipes-core/busybox/busybox_1.36.bb".into()),
                append_count: Some(2),
            },
        ];
        app.recipe_selection = 1;
        app.metadata_query = "busy".into();
        app.recipe_metadata.insert(
            "busybox".into(),
            yoctui_model::RecipeMetadata {
                recipe: "busybox".into(),
                workspace_status: Some(yoctui_model::RecipeWorkspaceStatus::Modified),
                build_status: None,
                tasks: Some(vec!["do_build".into(), "do_compile".into()]),
                sources: Some(vec![
                    "/layers/meta/recipes-core/busybox/busybox_1.36.bb".into(),
                ]),
                patches: Some(vec!["file://security.patch".into()]),
                packages: Some(vec!["busybox".into(), "busybox-src".into()]),
                history: None,
            },
        );
        let identity = yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox_1.36.bb".into(),
        };
        app.devtool_statuses.insert(
            identity.clone(),
            yoctui_model::DevtoolStatus {
                identity,
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: yoctui_model::DevtoolWorkspace::Present {
                    source_path: "/build/workspace/sources/busybox".into(),
                    recipe_file: Some("/layers/meta/recipes-core/busybox/busybox_1.36.bb".into()),
                },
                git: yoctui_model::DevtoolGitState::Available {
                    branch: Some("devtool".into()),
                    head: Some("abc123".into()),
                    modified: 1,
                    untracked: 0,
                    conflicted: 0,
                },
                error: None,
            },
        );
        app.dependencies = Some(yoctui_model::RecipeDependencies {
            recipe: "busybox".into(),
            build: vec!["virtual/libc".into()],
            runtime: vec!["busybox-udhcpc".into()],
        });
        app.tasks.insert(
            yoctui_model::TaskId("busybox:do_compile".into()),
            yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                state: TaskState::Active,
                ..yoctui_model::TaskInfo::default()
            },
        );
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Recipes (shown: 1 of 2)"));
        assert!(output.contains("Resolved"));
        assert!(output.contains("Preferred"));
        assert!(output.contains("Provider file"));
        assert!(output.contains("Workspace/Devtool: member at"));
        assert!(output.contains("Git branch devtool"));
        assert!(output.contains("Active tasks: do_compile"));
        assert!(output.contains("virtual/libc"));
        assert!(output.contains("security.patch"));
        assert!(output.contains("busybox-src"));
        assert!(output.contains("History: unavailable"));
    }

    #[test]
    fn recipes_workspace_partial_failure_and_all_responsive_modes_are_safe() {
        for (width, height) in [(160, 30), (110, 28), (90, 25), (70, 20)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Recipes;
            app.workspace.recipes.push(yoctui_model::Recipe {
                name: "demo".into(),
                ..yoctui_model::Recipe::default()
            });
            app.recipe_metadata_errors
                .insert("demo".into(), "metadata service unavailable".into());
            terminal.draw(|frame| render(frame, &app)).unwrap();
            if width >= 80 && height >= 24 {
                let output = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>();
                assert!(output.contains("demo"));
                assert!(output.contains("unavailable"));
            }
        }
    }
    #[test]
    fn recipe_bitbake_action_renders_task_picker_and_exact_forced_confirmation() {
        for (width, height) in [(120, 30), (90, 25)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut app = App::new(10, 1_000);
            app.dialogs
                .push_back(Dialog::RecipeTaskPicker(yoctui_model::RecipeTaskPicker {
                    recipe: "busybox".into(),
                    tasks: vec!["clean".into(), "compile".into(), "devshell".into()],
                    selection: 1,
                    force: true,
                }));
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let output = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(output.contains("Force task: busybox"));
            assert!(output.contains("Authoritative BitBake tasks"));
            assert!(output.contains("compile"));

            app.dialogs.clear();
            app.dialogs
                .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
                    targets: vec!["busybox".into()],
                    task: Some("compile".into()),
                    force: true,
                }));
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let output = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(output.contains("bitbake -f busybox -c compile"));
        }
    }
    #[test]
    fn recipe_navigation_renders_log_patch_pickers_and_disabled_reasons_responsively() {
        for (width, height) in [(160, 32), (110, 28), (90, 25)] {
            let mut app = App::new(10, 1_000);
            app.dialogs.push_back(Dialog::RecipeTaskLogPicker(
                yoctui_model::RecipeTaskLogPicker {
                    recipe: "busybox".into(),
                    logs: vec![
                        yoctui_model::RecipeTaskLogChoice {
                            task: "do_compile".into(),
                            state: TaskState::Failed,
                            path: "/tmp/log.do_compile".into(),
                        },
                        yoctui_model::RecipeTaskLogChoice {
                            task: "do_install".into(),
                            state: TaskState::Completed,
                            path: "/tmp/log.do_install".into(),
                        },
                    ],
                    selection: 1,
                },
            ));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("busybox retained task logs"), "{output}");
            assert!(output.contains("/tmp/log.do_install"), "{output}");

            app.dialogs.clear();
            app.dialogs
                .push_back(Dialog::RecipePatchPicker(yoctui_model::RecipePatchPicker {
                    recipe: "busybox".into(),
                    patches: vec!["/layers/meta/recipes-core/busybox/files/fix.patch".into()],
                    selection: 0,
                }));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("busybox patch review"), "{output}");
            assert!(output.contains("fix.patch"), "{output}");
        }

        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "demo".into(),
            ..yoctui_model::Recipe::default()
        });
        app.recipe_metadata.insert(
            "demo".into(),
            yoctui_model::RecipeMetadata {
                recipe: "demo".into(),
                patches: Some(vec!["file://unresolved.patch".into()]),
                ..yoctui_model::RecipeMetadata::default()
            },
        );
        let output = rendered_text(&app, 160, 36);
        assert!(
            output.contains("provider unavailable (provider path not reported)"),
            "{output}"
        );
        assert!(
            output.contains("patches unavailable (remote or")
                && output.contains("unresolved paths)"),
            "{output}"
        );
        assert!(
            output.contains("Devtool availability is authoritative"),
            "{output}"
        );
    }

    #[test]
    fn devtool_metadata_renders_typed_partial_and_disabled_states_responsively() {
        for (width, height) in [(160, 34), (110, 28), (90, 25)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Recipes;
            let file = std::path::PathBuf::from("/layers/core/demo.bb");
            app.workspace.recipes.push(yoctui_model::Recipe {
                name: "demo".into(),
                file: Some(file.clone()),
                ..yoctui_model::Recipe::default()
            });
            let identity = yoctui_model::RecipeIdentity {
                name: "demo".into(),
                file,
            };
            app.devtool_statuses.insert(
                identity.clone(),
                yoctui_model::DevtoolStatus {
                    identity,
                    capability: yoctui_model::DevtoolCapability::Available,
                    workspace: yoctui_model::DevtoolWorkspace::NotMember,
                    git: yoctui_model::DevtoolGitState::NotApplicable,
                    error: None,
                },
            );
            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("not in workspace"), "{output}");
                assert!(output.contains("u update: disabled"), "{output}");
            }
        }
    }
    #[test]
    fn devtool_job_lifecycle_renders_retained_stream_and_outcome() {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "demo".into(),
            ..yoctui_model::Recipe::default()
        });
        let id = yoctui_model::BackgroundJobId(1_u64 << 63);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
                id,
                kind: BackgroundJobKind::Devtool,
                title: "Devtool modify demo".into(),
                context: yoctui_model::BackgroundJobContext {
                    workspace: Some(Screen::Recipes),
                    recipe: Some("demo".into()),
                    ..yoctui_model::BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: UNIX_EPOCH,
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id,
                started_at: UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id });
        let _ = update(
            &mut app,
            Action::AppendBackgroundJobOutput {
                id,
                entry: yoctui_model::BackgroundJobOutputEntry {
                    severity: Severity::Info,
                    message: "workspace prepared".into(),
                    source: yoctui_model::BackgroundJobOutputSource::Stderr,
                    truncated: true,
                    timestamp: UNIX_EPOCH,
                },
            },
        );
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id,
                result: yoctui_model::BackgroundJobResult {
                    summary: "Devtool completed successfully".into(),
                    artifacts: vec![],
                },
                finished_at: UNIX_EPOCH,
            },
        );

        let output = rendered_text(&app, 200, 44);
        assert!(
            output.contains("Devtool modify demo [Succeeded]"),
            "{output}"
        );
        assert!(
            output.contains("Stderr: workspace prepared [truncated]"),
            "{output}"
        );
        assert!(
            output.contains("outcome: Devtool completed") && output.contains("successfully."),
            "{output}"
        );
    }
    #[test]
    fn recipe_qa_action_renders_capabilities_confirmation_and_honest_results() {
        for (width, height) in [(160, 32), (110, 28), (90, 25)] {
            let mut app = App::new(10, 1_000);
            app.dialogs
                .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
                    targets: vec!["busybox".into()],
                    task: Some("cve_check".into()),
                    force: false,
                }));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("bitbake busybox -c cve_check"), "{output}");
        }

        let mut app = App::new(20, 4_000);
        app.screen = Screen::Recipes;
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "busybox".into(),
            ..yoctui_model::Recipe::default()
        });
        app.recipe_metadata.insert(
            "busybox".into(),
            yoctui_model::RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(vec!["do_cve_check".into(), "do_create_spdx".into()]),
                ..yoctui_model::RecipeMetadata::default()
            },
        );
        let id = yoctui_model::BackgroundJobId(7);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
                id,
                kind: BackgroundJobKind::Spdx,
                title: "SPDX generation busybox".into(),
                context: yoctui_model::BackgroundJobContext {
                    workspace: Some(Screen::Recipes),
                    recipe: Some("busybox".into()),
                    task: Some("create_spdx".into()),
                    ..yoctui_model::BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: SystemTime::UNIX_EPOCH,
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id });
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id,
                result: yoctui_model::BackgroundJobResult {
                    summary: "SPDX generation completed; BitBake reported no result path".into(),
                    artifacts: vec![],
                },
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let output = rendered_text(&app, 200, 40);
        assert!(output.contains("QA actions: CVE check enabled"), "{output}");
        assert!(output.contains("SPDX generation enabled"), "{output}");
        assert!(
            output.contains("SPDX generation busybox [Succeeded]"),
            "{output}"
        );
        assert!(
            output.contains("artifacts: none") && output.contains("reported."),
            "{output}"
        );
        assert!(output.contains("V CVE"), "{output}");
        assert!(output.contains("X SPDX"), "{output}");

        app.recipe_metadata.get_mut("busybox").unwrap().tasks = Some(vec![]);
        let output = rendered_text(&app, 200, 40);
        assert!(
            output.contains("CVE check unavailable")
                && output.contains("do_cve_check not reported"),
            "{output}"
        );
        assert!(
            output.contains("SPDX generation unavailable")
                && output.contains("do_create_spdx not reported"),
            "{output}"
        );
    }
    #[test]
    fn build_completion_is_modal_but_running_builds_keep_the_shell_visible() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());
        app.build.status = yoctui_model::BuildStatus::Running;
        app.host_telemetry.cpu_utilization_percent = Some(50);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Dashboard"));
        assert!(output.contains("Host CPU: 50%"));

        app.build.status = yoctui_model::BuildStatus::Completed;
        app.dialogs.push_back(Dialog::BuildCompletion);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Build finished"));
        assert!(output.contains("Press any key"));
    }
    #[test]
    fn build_cancellation_completion_is_distinct_from_failure() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());
        app.build.status = yoctui_model::BuildStatus::Cancelled;
        app.dialogs.push_back(Dialog::BuildCompletion);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Build was cancelled"));
        assert!(!output.contains("Build failed"));
    }
    #[test]
    fn configuration_renders_bridge_provenance() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemuarm".into());
        app.workspace
            .variable_provenance
            .insert("MACHINE".into(), "conf/local.conf:12".into());
        app.workspace.variable_provenance_chain.insert(
            "MACHINE".into(),
            vec![
                "meta/conf/bitbake.conf:1".into(),
                "conf/local.conf:12".into(),
            ],
        );
        let identity = yoctui_model::VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            yoctui_model::VariableDetail {
                identity,
                effective_value: Some("qemuarm".into()),
                unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
                provenance: Some("conf/local.conf:12".into()),
                operations: vec![
                    yoctui_model::VariableOperation {
                        operation: "set".into(),
                        file: Some("meta/conf/bitbake.conf".into()),
                        line: Some(1),
                        value: Some("${DEFAULT_MACHINE}".into()),
                    },
                    yoctui_model::VariableOperation {
                        operation: "set".into(),
                        file: Some("conf/local.conf".into()),
                        line: Some(12),
                        value: Some("qemuarm".into()),
                    },
                ],
                active_overrides: vec!["qemuarm".into(), "poky".into()],
            },
        );
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("conf/local.conf:12"));
        assert!(output.contains("meta/conf/bitbake.conf:1"));
    }

    #[test]
    fn config_workspace_renders_lazy_partial_and_error_states_responsively() {
        for (width, height) in [(160, 30), (110, 26), (90, 24), (70, 20)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            let identity = yoctui_model::VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            };
            app.variable_detail_loading.insert(identity.clone());
            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("Loading authoritative detail"), "{output}");
            }

            app.variable_detail_loading.clear();
            app.variable_detail_errors
                .insert(identity, "Tinfoil unavailable".into());
            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("Detail unavailable"), "{output}");
                assert!(output.contains("Tinfoil unavailable"), "{output}");
            }

            app.variable_detail_errors.clear();
            let identity = yoctui_model::VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            };
            app.variable_details.insert(
                identity.clone(),
                yoctui_model::VariableDetail {
                    identity,
                    effective_value: Some("qemux86-64".into()),
                    unexpanded_value: None,
                    provenance: None,
                    operations: vec![],
                    active_overrides: vec![],
                },
            );
            let output = rendered_text(&app, width, height);
            if width >= 110 && height >= 24 {
                assert!(output.contains("Unexpanded value: unavailable"), "{output}");
                assert!(output.contains("Operations:"), "{output}");
                assert!(output.contains("none reported"), "{output}");
            }
        }
    }

    #[test]
    fn config_copy_renders_shortcuts_and_exact_availability_responsively() {
        for (width, height) in [(160, 32), (110, 28), (90, 24)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            let identity = yoctui_model::VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            };
            app.variable_details.insert(
                identity.clone(),
                yoctui_model::VariableDetail {
                    identity,
                    effective_value: Some("qemux86-64".into()),
                    unexpanded_value: None,
                    provenance: None,
                    operations: vec![],
                    active_overrides: vec![],
                },
            );
            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("C effective: enabled"), "{output}");
                assert!(output.contains("U unexpanded: disabled"), "{output}");
            }
        }
    }

    #[test]
    fn config_source_renders_typed_picker_and_disabled_reason_responsively() {
        for (width, height) in [(140, 30), (100, 26), (90, 24)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            let unloaded = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(unloaded.contains("o source: disabled"), "{unloaded}");
            }
            app.dialogs.push_back(Dialog::ConfigSourcePicker(
                yoctui_model::ConfigSourcePicker {
                    identity: yoctui_model::VariableIdentity {
                        name: "MACHINE".into(),
                        recipe: None,
                    },
                    sources: vec![
                        yoctui_model::ConfigSourceChoice {
                            operation: "set".into(),
                            path: "meta/conf/bitbake.conf".into(),
                            line: Some(10),
                        },
                        yoctui_model::ConfigSourceChoice {
                            operation: "override".into(),
                            path: "conf/local.conf".into(),
                            line: Some(12),
                        },
                    ],
                    selection: 1,
                },
            ));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("MACHINE defining sources"), "{output}");
            assert!(output.contains("local.conf"), "{output}");
            assert!(output.contains("12"), "{output}");
        }
    }

    #[test]
    fn config_scope_renders_picker_active_identity_and_global_fallback() {
        for (width, height) in [(140, 32), (100, 28), (90, 24)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace
                .variables
                .insert("MACHINE".into(), "global-summary".into());
            app.workspace.recipes.push(yoctui_model::Recipe {
                name: "base-files".into(),
                ..yoctui_model::Recipe::default()
            });
            app.config_scope = Some("base-files".into());
            let identity = yoctui_model::VariableIdentity {
                name: "MACHINE".into(),
                recipe: Some("base-files".into()),
            };
            app.variable_detail_errors
                .insert(identity, "scoped Tinfoil failure".into());
            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("scoped Tinfoil failure"), "{output}");
                assert!(output.contains("active base-files"), "{output}");
            }

            app.dialogs
                .push_back(Dialog::ConfigScopePicker(yoctui_model::ConfigScopePicker {
                    variable: "MACHINE".into(),
                    scopes: vec![None, Some("base-files".into())],
                    selection: 1,
                }));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("MACHINE scope"), "{output}");
            assert!(output.contains("(global)"), "{output}");
            assert!(output.contains("base-files"), "{output}");
        }

        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let output = rendered_text(&app, 120, 28);
        assert!(output.contains("global only"), "{output}");
        assert!(output.contains("no recipes reported"), "{output}");
    }

    #[test]
    fn config_compare_renders_typed_outcomes_and_disabled_reason_responsively() {
        for (width, height) in [(140, 32), (100, 28), (90, 24)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            app.workspace.recipes.push(yoctui_model::Recipe {
                name: "base-files".into(),
                ..yoctui_model::Recipe::default()
            });
            app.config_scope = Some("base-files".into());
            let unloaded = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(unloaded.contains("c compare: disabled"), "{unloaded}");
            }
            app.dialogs
                .push_back(Dialog::ConfigComparison(yoctui_model::ConfigComparison {
                    variable: "MACHINE".into(),
                    recipe: "base-files".into(),
                    effective: yoctui_model::ConfigComparisonField {
                        global: Some("qemux86-64".into()),
                        recipe: Some("qemux86-64".into()),
                        outcome: yoctui_model::ConfigComparisonOutcome::Equal,
                    },
                    unexpanded: yoctui_model::ConfigComparisonField {
                        global: Some("${DEFAULT_MACHINE}".into()),
                        recipe: None,
                        outcome: yoctui_model::ConfigComparisonOutcome::Unavailable,
                    },
                }));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Configuration comparison"), "{output}");
            assert!(output.contains("Effective: Equal"), "{output}");
            assert!(output.contains("Unexpanded: Unavailable"), "{output}");
            assert!(output.contains("base-files"), "{output}");
        }
    }

    #[test]
    fn config_edit_preview_renders_availability_editor_and_exact_confirmation_responsively() {
        for (width, height) in [(140, 32), (100, 28), (90, 24)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace.build_dir = Some("/build".into());
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            let identity = yoctui_model::VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            };
            app.variable_details.insert(
                identity.clone(),
                yoctui_model::VariableDetail {
                    identity: identity.clone(),
                    effective_value: Some("qemux86-64".into()),
                    unexpanded_value: None,
                    provenance: None,
                    operations: vec![],
                    active_overrides: vec![],
                },
            );

            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("E edit:"), "{output}");
                assert!(output.contains("enabled"), "{output}");
            }

            app.dialogs.push_back(Dialog::ConfigEdit {
                identity: identity.clone(),
                content: "# MACHINE\nvalue = \"qemux86-64\"\n".into(),
                editing: false,
            });
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Configuration.toml"), "{output}");
            assert!(output.contains("qemux86-64"), "{output}");

            app.dialogs.pop_back();
            app.dialogs.push_back(Dialog::ConfigEditConfirmation(
                yoctui_model::ConfigEditRequest {
                    identity,
                    value: "qemux86-64".into(),
                    destination: "/build/conf/local.conf".into(),
                    assignment: "MACHINE = \"qemux86-64\"".into(),
                },
            ));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Preview configuration edit"), "{output}");
            assert!(output.contains("/build/conf/local.conf"), "{output}");
            assert!(output.contains("MACHINE = \"qemux86-64\""), "{output}");
        }

        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.workspace
            .variables
            .insert("BB_NUMBER_THREADS".into(), "8".into());
        let output = rendered_text(&app, 120, 28);
        assert!(output.contains("E edit: disabled"), "{output}");
        assert!(output.contains("read-only"), "{output}");
    }

    #[test]
    fn bbmask_renders_effective_patterns_and_provenance() {
        let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Bbmask;
        app.workspace
            .variables
            .insert("BBMASK".into(), "meta-broken/.* meta-old/.*".into());
        app.workspace
            .variable_provenance
            .insert("BBMASK".into(), "conf/local.conf:42".into());
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Effective BBMASK"));
        assert!(output.contains("meta-broken/.*"));
        assert!(output.contains("conf/local.conf:42"));
    }
    #[test]
    fn bbmask_edit_preview_shows_the_exact_assignment() {
        let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs
            .push_back(Dialog::BbmaskConfirmation("meta-broken/.*".into()));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm BBMASK change"));
        assert!(output.contains("BBMASK = \"meta-broken/.*\""));
    }

    #[test]
    fn live_tasks_renders_summary_states_filters_and_selected_inspector() {
        let mut app = App::new(20, 2_000);
        app.screen = Screen::Tasks;
        app.build.completed = 2;
        app.build.total = Some(5);
        let mut active = yoctui_model::TaskInfo::active(
            yoctui_model::TaskId("busybox:do_compile".into()),
            "busybox".into(),
            "do_compile".into(),
        );
        active.progress = Some(42);
        active.worker = Some("worker-1".into());
        active.pid = Some(4242);
        active.log_path = Some("/build/tmp/work/busybox/temp/log.do_compile".into());
        app.tasks.insert(active.id.clone(), active);
        let mut failed = yoctui_model::TaskInfo::active(
            yoctui_model::TaskId("openssl:do_install".into()),
            "openssl".into(),
            "do_install".into(),
        );
        failed.state = TaskState::Failed;
        failed.progress = Some(100);
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: failed,
            success: false,
        });
        let output = rendered_text(&app, 180, 34);
        assert!(output.contains("40%  2/5"), "{output}");
        assert!(output.contains("active 1"), "{output}");
        assert!(output.contains("waiting 2"), "{output}");
        assert!(output.contains("FAILED"), "{output}");
        assert!(output.contains("State All"), "{output}");
        assert!(output.contains("PID: 4242"), "{output}");
        assert!(output.contains("log.do_compile"), "{output}");
    }

    #[test]
    fn live_tasks_unknown_progress_and_narrow_layout_are_honest_and_safe() {
        let mut app = App::new(20, 2_000);
        app.screen = Screen::Tasks;
        let task = yoctui_model::TaskInfo::active(
            yoctui_model::TaskId("linux-yocto:do_compile".into()),
            "linux-yocto".into(),
            "do_compile".into(),
        );
        app.tasks.insert(task.id.clone(), task);
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("progress unknown"), "{output}");
        assert!(!output.contains("0%"), "{output}");
        let _ = rendered_text(&app, 50, 16);
    }

    #[test]
    fn log_workspace_selection_drives_full_multiline_inspector_details() {
        let mut app = App::new(20, 4_000);
        app.screen = Screen::Logs;
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "compile failed\ncompiler context line".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: Some("/tmp/log.do_compile".into()),
            timestamp: SystemTime::UNIX_EPOCH,
            build: Some("core-image-minimal".into()),
            protected: true,
            diagnostic: None,
        });
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Info,
            message: "later output".into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: Some("core-image-minimal".into()),
            protected: false,
            diagnostic: None,
        });
        app.logs.follow = false;
        app.logs.selection = 0;
        let output = rendered_text(&app, 180, 34);
        assert!(output.contains("do_compile"), "{output}");
        assert!(output.contains("Source: /tmp/log.do_compile"), "{output}");
        assert!(output.contains("compiler context line"), "{output}");
        assert!(output.contains("Build: core-image-minimal"), "{output}");
    }

    #[test]
    fn log_workspace_exposes_search_filters_pressure_and_narrow_wrap_safely() {
        let mut app = App::new(20, 4_000);
        app.screen = Screen::Logs;
        app.logs.wrap = true;
        app.logs.searching = true;
        app.logs.query = "needle".into();
        app.logs.recipe_filter = Some("busybox".into());
        app.logs.task_filter = Some("do_compile".into());
        app.logs.build_filter = Some("core-image-minimal".into());
        app.logs.coalesced = 7;
        app.logs.dropped = 3;
        app.logs.dropped_warnings = 0;
        app.logs.dropped_errors = 0;
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Info,
            message: "needle in a long wrapped line".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: Some("core-image-minimal".into()),
            protected: false,
            diagnostic: None,
        });
        let output = rendered_text(&app, 220, 24);
        assert!(output.contains("7 coalesced"), "{output}");
        assert!(output.contains("search: needle_"), "{output}");
        assert!(output.contains("build: core-image-minimal"), "{output}");
        let _ = rendered_text(&app, 50, 16);
    }

    #[test]
    fn error_workspace_renders_structured_columns_inspector_and_related_entries() {
        let mut app = App::new(20, 4_000);
        app.screen = Screen::Errors;
        app.build.target = Some("core-image-minimal".into());
        let mut first = yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "compile failed\nfull compiler context".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: Some("/tmp/log.do_compile".into()),
            timestamp: SystemTime::UNIX_EPOCH,
            build: None,
            protected: true,
            diagnostic: None,
        };
        first.build = app.build.target.clone();
        app.logs.insert(first);
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Warning,
            message: "busybox follow-up warning".into(),
            recipe: Some("busybox".into()),
            task: Some("do_package".into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: Some("core-image-minimal".into()),
            protected: true,
            diagnostic: None,
        });
        let output = rendered_text(&app, 220, 36);
        assert!(output.contains("Time"), "{output}");
        assert!(output.contains("Severity"), "{output}");
        assert!(output.contains("Summary"), "{output}");
        assert!(output.contains("Category: BitBake error"), "{output}");
        assert!(output.contains("full compiler context"), "{output}");
        assert!(output.contains("Suggested actions"), "{output}");
        assert!(output.contains("busybox follow-up warning"), "{output}");
        assert!(output.contains("/tmp/log.do_compile"), "{output}");
    }

    #[test]
    fn error_workspace_and_actionable_failure_completion_are_narrow_safe() {
        let mut app = App::new(20, 4_000);
        app.screen = Screen::Errors;
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "backend connection lost".into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: None,
            protected: true,
            diagnostic: None,
        });
        let _ = rendered_text(&app, 50, 16);
        app.build.status = yoctui_model::BuildStatus::Failed;
        app.build.errors = 1;
        app.dialogs.push_back(Dialog::BuildCompletion);
        let output = rendered_text(&app, 100, 24);
        assert!(
            output.contains("Press Enter to investigate Errors"),
            "{output}"
        );
    }

    #[test]
    fn signature_workspace_renders_typed_records_differences_limitations_and_footer() {
        let target = yoctui_model::SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        };
        let left = yoctui_model::SignatureIdentity {
            target: target.clone(),
            hash: Some("aaa".into()),
            path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.aaa".into()),
        };
        let right = yoctui_model::SignatureIdentity {
            target: target.clone(),
            hash: Some("bbb".into()),
            path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.bbb".into()),
        };
        let mut app = App::new(20, 4_000);
        app.screen = Screen::Signatures;
        app.signature_selection = Some(left.clone());
        app.signature_dump = SignatureDumpState::Partial {
            target,
            records: vec![
                yoctui_model::SignatureRecord {
                    identity: left.clone(),
                    base_hash: Some("base-aaa".into()),
                    task_hash: Some("aaa".into()),
                    variables: vec![yoctui_model::SignatureValue {
                        name: "CC".into(),
                        value: Some("gcc".into()),
                    }],
                    dependencies: vec!["busybox:do_configure=dep-a".into()],
                },
                yoctui_model::SignatureRecord {
                    identity: right.clone(),
                    base_hash: Some("base-bbb".into()),
                    task_hash: Some("bbb".into()),
                    variables: Vec::new(),
                    dependencies: Vec::new(),
                },
            ],
            limitations: vec!["one malformed artifact was omitted".into()],
        };
        app.signature_comparison = SignatureComparisonState::Partial {
            request: yoctui_model::SignatureComparisonRequest { left, right },
            differences: vec![yoctui_model::SignatureDifference {
                category: SignatureDifferenceCategory::ChangedValue,
                key: "CC".into(),
                left: Some("gcc".into()),
                right: Some("clang".into()),
            }],
            limitations: vec!["recursive detail unavailable".into()],
        };

        let wide = rendered_text(&app, 160, 34);
        assert!(wide.contains("Signatures"), "{wide}");
        assert!(wide.contains("busybox:do_compile"), "{wide}");
        assert!(wide.contains("base-aaa"), "{wide}");
        assert!(wide.contains("CC = gcc"), "{wide}");
        assert!(wide.contains("[value] CC: gcc"), "{wide}");
        assert!(wide.contains("one malformed artifact"), "{wide}");
        assert!(wide.contains("recursive detail unavailable"), "{wide}");
        assert!(wide.contains("1/2 sides"), "{wide}");

        let narrow = rendered_text(&app, 90, 30);
        assert!(narrow.contains("Signatures"), "{narrow}");
        assert!(narrow.contains("Selected record"), "{narrow}");
        let tiny = rendered_text(&app, 50, 16);
        assert!(tiny.contains("needs at least 80x24"), "{tiny}");
    }

    #[test]
    fn signature_workspace_renders_explicit_loading_empty_failure_and_picker_states() {
        let target = yoctui_model::SignatureTarget {
            recipe: "busybox".into(),
            task: "do_fetch".into(),
        };
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Signatures;
        app.signature_dump = SignatureDumpState::Loading {
            target: target.clone(),
        };
        assert!(rendered_text(&app, 100, 24).contains("Loading authoritative signature artifacts"));
        app.signature_dump = SignatureDumpState::AvailableEmpty {
            target: target.clone(),
        };
        assert!(rendered_text(&app, 100, 24).contains("no signature artifacts"));
        app.signature_dump = SignatureDumpState::Failed {
            target,
            message: "tool missing".into(),
        };
        assert!(rendered_text(&app, 100, 24).contains("tool missing"));

        app.screen = Screen::Recipes;
        app.dialogs.push_back(Dialog::SignatureTaskPicker(
            yoctui_model::SignatureTaskPicker {
                recipe: RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/busybox.bb".into(),
                },
                tasks: vec!["do_fetch".into(), "do_compile".into()],
                selection: 1,
            },
        ));
        let picker = rendered_text(&app, 80, 24);
        assert!(picker.contains("Inspect signatures: busybox"), "{picker}");
        assert!(picker.contains("Authoritative signature tasks"), "{picker}");
    }

    #[test]
    fn pkgdata_workspace_renders_typed_partial_details_footer_and_responsive_modes() {
        let request = yoctui_model::PackageInventoryRequest { generation: 1 };
        let identity = PackageIdentity::new("busybox");
        let package = yoctui_model::PackageSummary {
            identity: identity.clone(),
            recipe: PackageField::Available("busybox".into()),
            provider: PackageField::Available("/layers/meta/recipes-core/busybox.bb".into()),
            version: PackageField::Available("1.37.0-r0".into()),
            installed_size_bytes: PackageField::Available(1_024),
            license: PackageField::Available("GPL-2.0-only".into()),
            image_membership: PackageField::Unavailable,
        };
        let detail_request = yoctui_model::PackageDetailRequest {
            identity: identity.clone(),
            generation: 2,
        };
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Packages;
        app.package_selection = Some(identity.clone());
        app.package_inventory = PackageInventoryState::Partial {
            request,
            packages: vec![package],
            limitations: vec!["image membership unavailable".into()],
        };
        app.package_details.insert(
            identity.clone(),
            PackageDetailState::Partial {
                request: detail_request,
                detail: yoctui_model::PackageDetail {
                    identity,
                    files: PackageField::Available(vec!["/bin/busybox".into()]),
                    runtime_dependencies: PackageField::Available(vec![PackageIdentity::new(
                        "libc6",
                    )]),
                    reverse_dependencies: PackageField::Available(Vec::new()),
                },
                limitations: vec!["reverse scan bounded".into()],
            },
        );

        let wide = rendered_text(&app, 160, 34);
        assert!(wide.contains("Packages"), "{wide}");
        assert!(wide.contains("busybox"), "{wide}");
        assert!(wide.contains("1.37.0-r0"), "{wide}");
        assert!(wide.contains("GPL-2.0-only"), "{wide}");
        assert!(wide.contains("/bin/busybox"), "{wide}");
        assert!(wide.contains("libc6"), "{wide}");
        assert!(wide.contains("Image membership: unavailable"), "{wide}");
        assert!(wide.contains("image membership unavailable"), "{wide}");
        assert!(wide.contains("Enter detail"), "{wide}");

        for (width, height) in [(120, 30), (90, 28)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Packages"), "{width}: {output}");
            assert!(output.contains("busybox"), "{width}: {output}");
        }
        for (theme, color) in [(Theme::WhiteClassic, true), (Theme::DarkPro, false)] {
            app.theme = theme;
            app.color_enabled = color;
            let output = rendered_text(&app, 140, 30);
            assert!(output.contains("busybox"), "{output}");
        }
        assert!(rendered_text(&app, 50, 16).contains("needs at least 80x24"));
    }

    #[test]
    fn pkgdata_workspace_renders_loading_empty_failed_and_unavailable_states() {
        let request = yoctui_model::PackageInventoryRequest { generation: 1 };
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Packages;
        app.package_inventory = PackageInventoryState::Loading { request };
        assert!(rendered_text(&app, 100, 25).contains("Loading authoritative package inventory"));
        app.package_inventory = PackageInventoryState::AvailableEmpty { request };
        assert!(rendered_text(&app, 100, 25).contains("No built runtime packages"));
        app.package_inventory = PackageInventoryState::Failed {
            request,
            message: "generated pkgdata is unavailable".into(),
        };
        let failed = rendered_text(&app, 100, 25);
        assert!(
            failed.contains("generated pkgdata is unavailable"),
            "{failed}"
        );
        assert!(failed.contains("do_package"), "{failed}");
    }

    fn qa_workflow_ui_app() -> App {
        let scope = yoctui_model::QaScope::new(RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox_1.36.bb".into(),
        })
        .unwrap();
        let check = yoctui_model::QaCheckId::new("recipe-package-busybox".into()).unwrap();
        let report_identity = yoctui_model::QaReportIdentity::new(
            "/build/tmp/log/qa/busybox.json".into(),
            512,
            SystemTime::UNIX_EPOCH,
            "reportfingerprint".into(),
            yoctui_model::QaReportFormat::Json,
            Some(check.clone()),
            Some(yoctui_model::QaFindingScope::Recipe(scope.clone())),
        )
        .unwrap();
        let finding_identity =
            yoctui_model::QaFindingIdentity::new(check.clone(), "findingfingerprint".into())
                .unwrap();
        let finding = yoctui_model::QaFinding {
            identity: finding_identity.clone(),
            status: yoctui_model::QaFindingStatus::Failed,
            severity: Some("error".into()),
            message: "installed-vs-shipped mismatch".into(),
            scope: yoctui_model::QaFindingScope::Recipe(scope.clone()),
            task: Some("do_package_qa".into()),
            test_name: None,
            source: Some(
                yoctui_model::QaSourceLocation::new(
                    "/layers/meta/classes-global/insane.bbclass".into(),
                    Some(42),
                    None,
                )
                .unwrap(),
            ),
            rule: Some("installed-vs-shipped".into()),
            suggestion: Some("add the installed file to FILES".into()),
            metadata: vec![
                yoctui_model::QaMetadata::new("package".into(), "busybox".into()).unwrap(),
            ],
        };
        let report = yoctui_model::QaReport {
            identity: report_identity.clone(),
            findings: vec![finding],
            metadata: Vec::new(),
            limitations: vec!["one unsupported record was retained".into()],
        };
        let request =
            yoctui_model::QaReportRequest::new(3, vec![report_identity.path.clone()]).unwrap();
        let available = yoctui_model::QaCheckCapability::new(
            check.clone(),
            yoctui_model::QaCheckFamily::RecipePackage,
            "Recipe and package QA".into(),
            scope.clone(),
            Some("do_package_qa".into()),
            vec!["/build/tmp/log/qa".into()],
            yoctui_model::QaCheckAvailability::Available,
            Vec::new(),
        )
        .unwrap();
        let disabled = yoctui_model::QaCheckCapability::new(
            yoctui_model::QaCheckId::new("kernel-configuration-busybox".into()).unwrap(),
            yoctui_model::QaCheckFamily::KernelConfiguration,
            "Kernel configuration".into(),
            scope.clone(),
            None,
            Vec::new(),
            yoctui_model::QaCheckAvailability::Disabled(
                "selected recipe is not a kernel provider".into(),
            ),
            Vec::new(),
        )
        .unwrap();
        let capability = yoctui_model::QaCapabilitySnapshot::new(
            Some("6.0".into()),
            "/build".into(),
            scope.clone(),
            vec![scope.clone()],
            vec![available, disabled],
            vec!["one optional report root was unavailable".into()],
        )
        .unwrap();
        let layer_identity =
            yoctui_model::QaLayerIdentity::new("meta-demo".into(), "/layers/meta-demo".into())
                .unwrap();
        let executable = yoctui_model::QaExecutableIdentity::new(
            "/workspace/scripts/yocto-check-layer".into(),
            128,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap();
        let arguments = vec![layer_identity.root.display().to_string()];
        let layer = yoctui_model::QaConfiguredLayerCapability::new(
            yoctui_model::QaCheckId::new("layer-meta-demo".into()).unwrap(),
            layer_identity.clone(),
            vec!["walnascar".into()],
            yoctui_model::QaLayerRunCapability::Available {
                executable,
                arguments,
                report_roots: vec!["/build/tmp/log/qa-layer".into()],
            },
            vec!["live compatibility not validated".into()],
        )
        .unwrap();
        let layer_capability = yoctui_model::QaLayerCapabilitySnapshot::new(
            Some("6.0".into()),
            "/build".into(),
            layer_identity.clone(),
            vec![layer],
            Vec::new(),
        )
        .unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Qa;
        app.focus = FocusTarget::Workspace;
        app.qa.scope = Some(scope);
        app.qa.check_selection = Some(check);
        app.qa.capability = yoctui_model::QaCapability::Partial {
            snapshot: Box::new(capability),
            limitations: vec!["one optional report root was unavailable".into()],
        };
        app.qa.inventory = yoctui_model::QaReportInventoryState::Partial {
            request,
            reports: vec![report],
            limitations: vec!["one exact record was malformed".into()],
        };
        app.qa.report_selection = Some(report_identity);
        app.qa.finding_selection = Some(finding_identity);
        app.qa.layer_capability =
            yoctui_model::QaLayerCapability::Available(Box::new(layer_capability));
        app.qa.layer_selection = Some(layer_identity);
        app
    }

    #[test]
    fn qa_workflow_renders_both_views_findings_inspector_themes_and_breakpoints() {
        let mut app = qa_workflow_ui_app();
        for (width, height) in [(160, 40), (120, 30), (90, 24), (80, 24)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Recipe & Kernel"), "{width}: {output}");
            assert!(output.contains("do_package_qa"), "{width}: {output}");
            assert!(output.contains("Partial"), "{width}: {output}");
        }
        app.qa.drilled = true;
        app.focus = FocusTarget::Inspector;
        let inspector = rendered_text(&app, 120, 30);
        assert!(
            inspector.contains("installed-vs-shipped mismatch"),
            "{inspector}"
        );
        assert!(inspector.contains("add the installed file"), "{inspector}");
        assert!(inspector.contains("insane.bbclass"), "{inspector}");

        app.focus = FocusTarget::Workspace;
        app.qa.view = QaView::LayerQa;
        let layer = rendered_text(&app, 160, 40);
        assert!(layer.contains("meta-demo"), "{layer}");
        assert!(layer.contains("/layers/meta-demo"), "{layer}");
        for (theme, color) in [
            (Theme::DarkPro, true),
            (Theme::WhiteClassic, true),
            (Theme::HighContrast, true),
            (Theme::Monochrome, false),
        ] {
            app.theme = theme;
            app.color_enabled = color;
            assert!(rendered_text(&app, 100, 24).contains("Layer QA"));
        }
    }

    #[test]
    fn qa_workflow_renders_every_report_and_capability_state_distinctly() {
        let mut app = qa_workflow_ui_app();
        let request = yoctui_model::QaReportRequest::new(8, vec!["/build/reports".into()]).unwrap();
        for (inventory, expected) in [
            (
                yoctui_model::QaReportInventoryState::NotLoaded,
                "Reports not loaded",
            ),
            (
                yoctui_model::QaReportInventoryState::Loading {
                    request: request.clone(),
                },
                "Loading report generation 8",
            ),
            (
                yoctui_model::QaReportInventoryState::AvailableEmpty {
                    request: request.clone(),
                },
                "available-empty",
            ),
            (
                yoctui_model::QaReportInventoryState::Failed {
                    request: request.clone(),
                    kind: yoctui_model::QaReportFailureKind::PermissionDenied,
                    message: "access denied".into(),
                },
                "permission denied",
            ),
            (
                yoctui_model::QaReportInventoryState::Cancelled {
                    request: request.clone(),
                },
                "acquisition cancelled",
            ),
            (
                yoctui_model::QaReportInventoryState::TimedOut {
                    request: request.clone(),
                },
                "acquisition timed out",
            ),
            (
                yoctui_model::QaReportInventoryState::Lost {
                    request,
                    message: "worker channel closed".into(),
                },
                "worker lost",
            ),
        ] {
            app.qa.inventory = inventory;
            let output = rendered_text(&app, 120, 28);
            assert!(output.contains(expected), "{expected}: {output}");
        }
        app.qa.capability = QaCapability::Inspecting;
        assert!(rendered_text(&app, 120, 28).contains("Inspecting recipe"));
        app.qa.capability = QaCapability::Failed("metadata denied".into());
        assert!(rendered_text(&app, 120, 28).contains("metadata denied"));
        app.qa.view = QaView::LayerQa;
        app.qa.layer_capability = QaLayerCapability::Failed("tool unsafe".into());
        assert!(rendered_text(&app, 120, 28).contains("tool unsafe"));
    }

    #[test]
    fn qa_workflow_dialogs_render_exact_previews_at_responsive_boundaries() {
        let mut app = qa_workflow_ui_app();
        let scope = app.qa.scope.clone().unwrap();
        let check = app.qa.check_selection.clone().unwrap();
        let operation = yoctui_model::QaOperationPreview {
            id: yoctui_model::QaOperationId(11),
            check,
            family: yoctui_model::QaCheckFamily::RecipePackage,
            scope,
            request: BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("do_package_qa".into()),
                force: false,
            },
            indexed_arguments: vec![
                "0: bitbake".into(),
                "1: busybox".into(),
                "2: -c".into(),
                "3: package_qa".into(),
            ],
            report_roots: vec!["/build/tmp/log/qa".into()],
            limitations: Vec::new(),
        };
        let layer = app.qa.selected_layer().unwrap();
        let QaLayerRunCapability::Available {
            executable,
            arguments,
            report_roots,
        } = &layer.run
        else {
            panic!("expected layer capability")
        };
        let layer_operation = yoctui_model::QaLayerOperationPreview {
            id: yoctui_model::QaLayerOperationId(12),
            check: layer.check.clone(),
            layer: layer.identity.clone(),
            executable: executable.clone(),
            arguments: arguments.clone(),
            indexed_arguments: vec![
                format!("0: {}", executable.path.display()),
                format!("1: {}", layer.identity.root.display()),
            ],
            report_roots: report_roots.clone(),
            limitations: Vec::new(),
        };
        for (dialog, expected) in [
            (QaDialog::Operation(operation), "Indexed BitBake request"),
            (
                QaDialog::LayerOperation(layer_operation),
                "Indexed native vector",
            ),
            (
                QaDialog::Import {
                    input: "/build/reports".into(),
                },
                "Exact absolute report",
            ),
            (
                QaDialog::Cancellation {
                    session: yoctui_model::QaSessionId(13),
                    background_job: yoctui_model::BackgroundJobId(4),
                },
                "attached to build job 4",
            ),
            (
                QaDialog::LayerCancellation(yoctui_model::QaLayerSessionId(14)),
                "exact layer-QA session 14",
            ),
        ] {
            app.dialogs.clear();
            app.dialogs.push_front(Dialog::Qa(dialog));
            app.focus = FocusTarget::Dialog;
            for (width, height) in [(160, 40), (100, 26), (80, 24)] {
                let output = rendered_text(&app, width, height);
                assert!(output.contains(expected), "{width}: {output}");
            }
        }
    }

    fn maintenance_identity(path: &str) -> yoctui_model::MaintenanceFileIdentity {
        yoctui_model::MaintenanceFileIdentity::new(path.into(), 42, UNIX_EPOCH).unwrap()
    }

    fn maintenance_preview(id: u64) -> yoctui_model::MaintenanceOperationPreview {
        yoctui_model::MaintenanceOperationPreview::new(
            id,
            7,
            yoctui_model::MaintenanceOperation::SstateReadiness(
                yoctui_model::SstateReadinessRequest::new(
                    vec!["core-image-minimal".into()],
                    yoctui_model::SstateReadinessMode::IsolatedTmpdir,
                    Some("/build/sstate-report.txt".into()),
                    None,
                    60,
                )
                .unwrap(),
            ),
            vec![
                "/tools/oe-check-sstate".into(),
                "--output".into(),
                "/build/sstate-report.txt".into(),
                "core-image-minimal".into(),
            ],
            vec!["fixture evidence is not live compatibility".into()],
        )
        .unwrap()
    }

    fn maintenance_workflow_ui_app() -> App {
        let mut app = App::new(100, 10_000);
        app.screen = Screen::Maintenance;
        app.focus = FocusTarget::Workspace;
        let available = |tool, path| MaintenanceToolCapability::Available {
            tool,
            executable: maintenance_identity(path),
            interface: if matches!(
                tool,
                MaintenanceTool::CreatePullRequest
                    | MaintenanceTool::SendPullRequest
                    | MaintenanceTool::SendErrorReport
                    | MaintenanceTool::Toaster
            ) {
                MaintenanceToolInterface::DetectionOnly
            } else {
                MaintenanceToolInterface::Native
            },
        };
        let snapshot = yoctui_model::MaintenanceCapabilitySnapshot::new(
            yoctui_model::MaintenanceMetadata::new(yoctui_model::MaintenanceMetadata {
                build_dir: Some("/build".into()),
                sstate_dir: Some("/cache/sstate".into()),
                tmp_dir: Some("/build/tmp".into()),
                stamps_dirs: vec!["/build/tmp/stamps".into()],
                buildhistory_dir: Some("/build/buildhistory".into()),
                prserv_host: Some("localhost:8585".into()),
                hashserve: Some("auto".into()),
                hashserve_upstream: None,
                signature_handler: Some("OEEquivHash".into()),
                native_lsb: Some("ubuntu-24.04".into()),
                machine: Some("qemux86-64".into()),
                distro: Some("poky".into()),
            })
            .unwrap(),
            vec![
                available(MaintenanceTool::OeCheckSstate, "/tools/oe-check-sstate"),
                available(
                    MaintenanceTool::SstateCacheManagement,
                    "/tools/sstate-cache-management.py",
                ),
                available(MaintenanceTool::PrServiceTool, "/tools/bitbake-prserv-tool"),
                available(
                    MaintenanceTool::LockedSignatureCache,
                    "/tools/gen-lockedsig-cache",
                ),
                available(
                    MaintenanceTool::BuildHistoryDiff,
                    "/tools/buildhistory-diff",
                ),
                MaintenanceToolCapability::Unavailable {
                    tool: MaintenanceTool::BuildCompare,
                    reason: "distinct optional interface is unsupported".into(),
                },
                available(MaintenanceTool::GitArchive, "/tools/oe-git-archive"),
                available(
                    MaintenanceTool::CreatePullRequest,
                    "/tools/create-pull-request",
                ),
                available(MaintenanceTool::SendPullRequest, "/tools/send-pull-request"),
                available(MaintenanceTool::SendErrorReport, "/tools/send-error-report"),
                available(MaintenanceTool::Toaster, "/tools/toaster"),
            ],
            vec!["bounded fixture snapshot".into()],
        )
        .unwrap();
        app.maintenance.capability = MaintenanceCapability::Partial {
            request: 7,
            limitations: snapshot.limitations.clone(),
            snapshot,
        };

        let endpoint = yoctui_model::ServiceEndpointDiagnostic::new(
            yoctui_model::ServiceEndpointRole::Primary,
            "localhost:8585".into(),
            yoctui_model::ServiceLocation::Local,
            yoctui_model::ServiceReachability::Reachable,
            None,
        )
        .unwrap();
        let service = yoctui_model::ServiceDiagnostic::new(
            yoctui_model::ServiceKind::Pr,
            yoctui_model::ServiceState::Reachable,
            vec![endpoint],
            vec![yoctui_model::ServiceProcessEvidence::new(42, "bitbake-prserv".into()).unwrap()],
            vec!["process evidence is observational".into()],
        )
        .unwrap();
        app.maintenance.services = MaintenanceServiceDiagnostics::Partial {
            request: 8,
            services: vec![service],
            limitations: vec!["remote endpoint was not probed".into()],
        };

        let directory = |path: &str| {
            yoctui_model::MaintenanceDirectoryIdentity::new(path.into(), UNIX_EPOCH).unwrap()
        };
        let integrations = yoctui_model::MaintenanceIntegrationsSnapshot::new(
            yoctui_model::MaintenanceIntegrationsSnapshot {
                pull_request: yoctui_model::OptionalPullRequestIntegration {
                    state: yoctui_model::OptionalIntegrationState::Available,
                    create_helper: Some(maintenance_identity("/tools/create-pull-request")),
                    send_helper: Some(maintenance_identity("/tools/send-pull-request")),
                    worktree: Some(yoctui_model::MaintenanceGitWorktreeIdentity {
                        root: directory("/sources/poky"),
                        head: maintenance_identity("/sources/poky/.git/HEAD"),
                    }),
                    limitations: Vec::new(),
                },
                error_report: yoctui_model::OptionalErrorReportIntegration {
                    state: yoctui_model::OptionalIntegrationState::Partial,
                    helper: Some(maintenance_identity("/tools/send-error-report")),
                    candidate_report: None,
                    limitations: vec!["candidate report unavailable".into()],
                },
                repo_manifest: yoctui_model::OptionalRepoManifestIntegration {
                    state: yoctui_model::OptionalIntegrationState::Available,
                    repo_executable: Some(maintenance_identity("/tools/repo")),
                    workspace: Some(directory("/workspace")),
                    manifest: Some(maintenance_identity(
                        "/workspace/.repo/manifests/default.xml",
                    )),
                    limitations: Vec::new(),
                },
                toaster: yoctui_model::OptionalToasterIntegration {
                    state: yoctui_model::OptionalIntegrationState::Available,
                    executable: Some(maintenance_identity("/tools/toaster")),
                    configurations: vec![maintenance_identity("/config/toaster.conf")],
                    observed_processes: vec![
                        yoctui_model::ServiceProcessEvidence::new(84, "toaster".into()).unwrap(),
                    ],
                    limitations: vec!["observational only".into()],
                },
                limitations: vec!["detection only".into()],
            },
        )
        .unwrap();
        app.maintenance.integrations = MaintenanceIntegrationDiagnostics::Partial {
            request: 9,
            limitations: integrations.limitations.clone(),
            snapshot: integrations,
        };
        app.maintenance.pending = Some(maintenance_preview(10));
        app.maintenance
            .sessions
            .push_back(yoctui_model::MaintenanceSession {
                id: yoctui_model::MaintenanceSessionId(11),
                preview: maintenance_preview(11),
                status: MaintenanceSessionStatus::Succeeded,
                started_at: Some(UNIX_EPOCH),
                finished_at: Some(UNIX_EPOCH),
                output: std::collections::VecDeque::from([
                    yoctui_model::MaintenanceOutputLine {
                        stream: yoctui_model::MaintenanceOutputStream::Stdout,
                        text: "created report".into(),
                    },
                    yoctui_model::MaintenanceOutputLine {
                        stream: yoctui_model::MaintenanceOutputStream::Stderr,
                        text: "bounded warning".into(),
                    },
                ]),
                dropped_lines: 3,
                exit_code: Some(0),
                message: None,
            });
        app.maintenance.evidence = vec![
            yoctui_model::MaintenanceEvidence::new(
                maintenance_identity("/build/sstate-report.txt"),
                "sstate report".into(),
            )
            .unwrap(),
        ];
        app
    }

    #[test]
    fn maintenance_workflow_renders_every_view_and_exact_typed_inspector() {
        let mut app = maintenance_workflow_ui_app();
        for (view, expected, action) in [
            (MaintenanceView::Sstate, "oe-check-sstate", "c check"),
            (
                MaintenanceView::Services,
                "bitbake-prserv-tool",
                "e PR export",
            ),
            (
                MaintenanceView::Release,
                "gen-lockedsig-cache",
                "l locked cache",
            ),
            (
                MaintenanceView::Integrations,
                "create-pull-request",
                "detection/inspection only",
            ),
        ] {
            app.maintenance.view = view;
            let output = rendered_text(&app, 180, 70);
            assert!(output.contains(expected), "{view:?}: {output}");
            assert!(output.contains(action), "{view:?}: {output}");
            assert!(output.contains("Partial capability"), "{output}");
        }

        app.focus = FocusTarget::Inspector;
        app.maintenance.view = MaintenanceView::Services;
        let services = rendered_text(&app, 180, 120);
        assert!(services.contains("localhost:8585"), "{services}");
        assert!(services.contains("PID 42 bitbake-prserv"), "{services}");
        assert!(services.contains("observational"), "{services}");
        assert!(services.contains("Indexed native vector"), "{services}");
        assert!(services.contains("Selected evidence"), "{services}");

        app.maintenance.view = MaintenanceView::Integrations;
        let integrations = rendered_text(&app, 180, 120);
        for expected in [
            "/sources/poky/.git/HEAD",
            "/workspace/.repo/manifests/default.xml",
            "/config/toaster.conf",
            "process evidence is observational only",
        ] {
            assert!(integrations.contains(expected), "{integrations}");
        }
    }

    #[test]
    fn maintenance_workflow_renders_terminal_states_responsively_in_every_theme() {
        let mut app = maintenance_workflow_ui_app();
        for status in [
            MaintenanceSessionStatus::Queued,
            MaintenanceSessionStatus::Running,
            MaintenanceSessionStatus::Cancelling,
            MaintenanceSessionStatus::Succeeded,
            MaintenanceSessionStatus::Failed,
            MaintenanceSessionStatus::Cancelled,
            MaintenanceSessionStatus::TimedOut,
            MaintenanceSessionStatus::Lost,
        ] {
            app.maintenance.sessions.back_mut().unwrap().status = status;
            let output = rendered_text(&app, 160, 40);
            assert!(output.contains(&format!("{status:?}")), "{output}");
        }
        for (width, expected) in [
            (130, "Inspector"),
            (100, "Maintenance"),
            (99, "Panes:"),
            (80, "Maintenance"),
        ] {
            let output = rendered_text(&app, width, 24);
            assert!(output.contains(expected), "{width}: {output}");
        }
        assert!(rendered_text(&app, 79, 24).contains("needs at least 80x24"));

        for theme in [
            Theme::DarkPro,
            Theme::WhiteClassic,
            Theme::MatrixGreen,
            Theme::HighContrast,
            Theme::Monochrome,
        ] {
            app.theme = theme;
            assert!(rendered_text(&app, 130, 30).contains("Maintenance"));
        }
        app.color_enabled = false;
        let mut terminal = Terminal::new(TestBackend::new(130, 30)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(
            terminal.backend().buffer().content.iter().any(|cell| {
                cell.symbol() == "▶" && cell.modifier.contains(Modifier::REVERSED)
            })
        );
    }

    #[test]
    fn maintenance_workflow_renders_loading_failed_disabled_and_unavailable_states() {
        let mut app = maintenance_workflow_ui_app();
        app.maintenance.view = MaintenanceView::Services;
        app.maintenance.capability = MaintenanceCapability::Loading(41);
        app.maintenance.services = MaintenanceServiceDiagnostics::Loading(42);
        let loading = rendered_text(&app, 160, 40);
        assert!(
            loading.contains("Inspecting capability (request 41)"),
            "{loading}"
        );
        assert!(loading.contains("loading request 42"), "{loading}");

        app.maintenance.capability = MaintenanceCapability::Failed {
            request: 43,
            message: "metadata unavailable".into(),
        };
        app.maintenance.services = MaintenanceServiceDiagnostics::Failed {
            request: 44,
            message: "process inspection unavailable".into(),
        };
        let failed = rendered_text(&app, 160, 40);
        assert!(failed.contains("metadata unavailable"), "{failed}");
        assert!(
            failed.contains("process inspection unavailable"),
            "{failed}"
        );

        let disabled = yoctui_model::ServiceDiagnostic::new(
            yoctui_model::ServiceKind::Hash,
            yoctui_model::ServiceState::Disabled,
            Vec::new(),
            Vec::new(),
            vec!["not configured".into()],
        )
        .unwrap();
        app.maintenance.services = MaintenanceServiceDiagnostics::Available {
            request: 45,
            services: vec![disabled],
        };
        let disabled = rendered_text(&app, 160, 40);
        assert!(disabled.contains("Hash: Disabled"), "{disabled}");

        app.maintenance.view = MaintenanceView::Integrations;
        app.maintenance.integrations = MaintenanceIntegrationDiagnostics::Loading(46);
        assert!(rendered_text(&app, 160, 40).contains("loading request 46"));
        app.maintenance.integrations = MaintenanceIntegrationDiagnostics::Failed {
            request: 47,
            message: "optional tools unavailable".into(),
        };
        assert!(rendered_text(&app, 160, 40).contains("optional tools unavailable"));
    }

    #[test]
    fn maintenance_workflow_dialogs_render_exact_safety_meaning_at_80x24() {
        let mut app = maintenance_workflow_ui_app();
        let ordinary = maintenance_preview(21);
        let cleanup_request = yoctui_model::SstateCleanupRequest::new(
            "/cache/sstate".into(),
            Vec::new(),
            vec![yoctui_model::SstateCleanupMode::Duplicates],
            1,
        )
        .unwrap();
        let cleanup = yoctui_model::MaintenanceOperationPreview::new(
            22,
            7,
            yoctui_model::MaintenanceOperation::SstateCleanup(
                yoctui_model::SstateCleanupPreview::new(
                    cleanup_request,
                    vec![maintenance_identity("/cache/sstate/a.tgz")],
                )
                .unwrap(),
            ),
            vec![
                "/tools/sstate-cache-management.py".into(),
                "--remove-duplicated".into(),
            ],
            vec!["files may be removed".into()],
        )
        .unwrap();
        let archive = yoctui_model::GitArchiveRequest::new(yoctui_model::GitArchiveRequest {
            data_dir: "/data".into(),
            git_dir: "/archive/release.git".into(),
            create: false,
            bare: false,
            create_tag: false,
            branch_name: "main".into(),
            tag_name: None,
            commit_subject: "archive".into(),
            commit_body: String::new(),
            tag_subject: "tag".into(),
            tag_body: String::new(),
            exclusions: Vec::new(),
            notes: Vec::new(),
            push_remote: Some("origin".into()),
        })
        .unwrap();
        let network = yoctui_model::MaintenanceOperationPreview::new(
            23,
            7,
            yoctui_model::MaintenanceOperation::GitArchive(archive),
            vec![
                "/tools/oe-git-archive".into(),
                "--push".into(),
                "origin".into(),
            ],
            Vec::new(),
        )
        .unwrap();
        for (dialog, expected) in [
            (
                MaintenanceDialog::Confirm(ordinary),
                "[0] /tools/oe-check-sstate",
            ),
            (
                MaintenanceDialog::Confirm(cleanup.clone()),
                "Confirm destructive Maintenance operation",
            ),
            (
                MaintenanceDialog::CleanupPhrase {
                    preview: cleanup,
                    input: "DELETE".into(),
                },
                "Typing alone cannot delete files",
            ),
            (
                MaintenanceDialog::ConfirmNetworkPush(network),
                "separately confirmed remote push",
            ),
            (
                MaintenanceDialog::ConfirmCancellation(yoctui_model::MaintenanceSessionId(11)),
                "partially cleaned cache",
            ),
        ] {
            app.dialogs.clear();
            app.dialogs
                .push_front(Dialog::Maintenance(Box::new(dialog)));
            app.focus = FocusTarget::Dialog;
            let output = rendered_text(&app, 80, 24);
            assert!(output.contains(expected), "{output}");
        }
    }

    #[test]
    fn maintenance_sstate_workspace_renders_forms_validation_and_responsive_fields() {
        let mut app = maintenance_workflow_ui_app();
        let readiness = yoctui_model::MaintenanceReadinessDraft {
            targets: "core-image-minimal busybox".into(),
            output: "/build/sstate.txt".into(),
            validation: Some("timeout must be a positive integer".into()),
            ..yoctui_model::MaintenanceReadinessDraft::default()
        };
        app.dialogs.push_front(Dialog::Maintenance(Box::new(
            MaintenanceDialog::ReadinessForm(Box::new(readiness)),
        )));
        app.focus = FocusTarget::Dialog;
        for (width, height) in [(160, 40), (100, 26), (80, 24)] {
            let output = rendered_text(&app, width, height);
            assert!(
                output.contains("Sstate readiness check"),
                "{width}: {output}"
            );
            assert!(
                output.contains("core-image-minimal busybox"),
                "{width}: {output}"
            );
            assert!(
                output.contains("timeout must be a positive"),
                "{width}: {output}"
            );
        }

        let metadata = yoctui_model::MaintenanceMetadata::new(yoctui_model::MaintenanceMetadata {
            sstate_dir: Some("/cache/sstate".into()),
            stamps_dirs: vec!["/build/tmp/stamps".into()],
            ..yoctui_model::MaintenanceMetadata::default()
        })
        .unwrap();
        let mut cleanup = yoctui_model::MaintenanceCleanupDraft::from_metadata(&metadata).unwrap();
        cleanup.orphans = true;
        app.dialogs.clear();
        app.dialogs.push_front(Dialog::Maintenance(Box::new(
            MaintenanceDialog::CleanupForm(Box::new(cleanup)),
        )));
        for theme in [Theme::DarkPro, Theme::WhiteClassic, Theme::Monochrome] {
            app.theme = theme;
            let output = rendered_text(&app, 80, 24);
            assert!(output.contains("Protected sstate cleanup"), "{output}");
            assert!(output.contains("/cache/sstate"), "{output}");
            assert!(output.contains("[x] orphans"), "{output}");
            assert!(output.contains("discover candidates"), "{output}");
        }
    }

    #[test]
    fn maintenance_service_workspace_renders_exact_context_and_side_effects() {
        let mut app = maintenance_workflow_ui_app();
        let metadata = app
            .maintenance
            .capability
            .snapshot()
            .unwrap()
            .metadata
            .clone();
        for operation in [
            yoctui_model::PrServiceOperation::Export,
            yoctui_model::PrServiceOperation::Import,
        ] {
            let mut draft =
                yoctui_model::MaintenancePrServiceDraft::from_metadata(&metadata, operation)
                    .unwrap();
            draft.file = "/build/pr-data.inc".into();
            if operation == yoctui_model::PrServiceOperation::Export {
                draft.validation = Some("destination parent is unavailable".into());
            }
            app.dialogs.clear();
            app.dialogs.push_front(Dialog::Maintenance(Box::new(
                MaintenanceDialog::PrServiceForm(Box::new(draft)),
            )));
            app.focus = FocusTarget::Dialog;
            for (width, height) in [(160, 40), (100, 26), (80, 24)] {
                let output = rendered_text(&app, width, height);
                assert!(output.contains("/build/pr-data.inc"), "{width}: {output}");
                assert!(output.contains("localhost:8585"), "{width}: {output}");
                assert!(output.contains("memory-resident"), "{width}: {output}");
                if operation == yoctui_model::PrServiceOperation::Import {
                    assert!(output.contains("changes PR service data"), "{output}");
                } else {
                    assert!(output.contains("destination parent"), "{output}");
                }
            }
        }
    }

    #[test]
    fn maintenance_release_locked_workspace_renders_context_warning_and_validation_responsively() {
        let mut app = maintenance_workflow_ui_app();
        let metadata = yoctui_model::MaintenanceMetadata {
            native_lsb: Some("ubuntu-24.04".into()),
            ..yoctui_model::MaintenanceMetadata::default()
        };
        let mut draft =
            yoctui_model::MaintenanceLockedCacheDraft::from_metadata(&metadata).unwrap();
        draft.locked_signatures = "/build/conf/locked-sigs.inc".into();
        draft.input_cache = "/cache/input".into();
        draft.output_cache = "/cache/release".into();
        draft.filter = "/build/conf/filter.inc".into();
        draft.validation = Some("input and output cache must differ".into());
        app.dialogs.push_front(Dialog::Maintenance(Box::new(
            MaintenanceDialog::LockedCacheForm(Box::new(draft)),
        )));
        app.focus = FocusTarget::Dialog;
        for (width, height) in [(160, 40), (100, 26), (80, 24)] {
            let output = rendered_text(&app, width, height);
            assert!(
                output.contains("Locked-signature cache"),
                "{width}: {output}"
            );
            assert!(output.contains("/cache/release"), "{width}: {output}");
            assert!(output.contains("ubuntu-24.04"), "{width}: {output}");
            assert!(output.contains("may be replaced"), "{width}: {output}");
            assert!(
                output.contains("input and output cache must differ"),
                "{width}: {output}"
            );
        }
    }

    #[test]
    fn maintenance_release_history_workspace_renders_exact_choices_responsively() {
        let mut app = maintenance_workflow_ui_app();
        let metadata = yoctui_model::MaintenanceMetadata {
            buildhistory_dir: Some("/build/buildhistory".into()),
            ..yoctui_model::MaintenanceMetadata::default()
        };
        let mut draft =
            yoctui_model::MaintenanceBuildHistoryDraft::from_metadata(&metadata).unwrap();
        draft.from_revision = "HEAD~2".into();
        draft.to_revision = "HEAD".into();
        draft.signatures = true;
        draft.signature_diff = true;
        draft.exclude_paths = "images/*,packages/*".into();
        draft.no_colour = true;
        draft.validation = Some("repository identity changed".into());
        app.dialogs.push_front(Dialog::Maintenance(Box::new(
            MaintenanceDialog::BuildHistoryForm(Box::new(draft)),
        )));
        app.focus = FocusTarget::Dialog;
        for (width, height) in [(160, 40), (100, 26), (80, 24)] {
            let output = rendered_text(&app, width, height);
            assert!(
                output.contains("Build-history comparison"),
                "{width}: {output}"
            );
            assert!(output.contains("/build/buildhistory"), "{width}: {output}");
            assert!(output.contains("HEAD~2"), "{width}: {output}");
            assert!(output.contains("[x] signature diff"), "{width}: {output}");
            assert!(output.contains("[x] no colour"), "{width}: {output}");
            assert!(
                output.contains("repository identity changed"),
                "{width}: {output}"
            );
        }
    }

    #[test]
    fn maintenance_release_archive_workspace_renders_local_and_network_intent_responsively() {
        let mut app = maintenance_workflow_ui_app();
        let mut draft = yoctui_model::MaintenanceGitArchiveDraft {
            data_dir: "/release/data".into(),
            git_dir: "/release/archive.git".into(),
            bare: true,
            exclusions: "tmp/*,downloads/*".into(),
            notes: "release=/release/note.txt".into(),
            push_remote: "origin".into(),
            validation: Some("note identity changed".into()),
            ..yoctui_model::MaintenanceGitArchiveDraft::default()
        };
        draft.field = yoctui_model::MaintenanceGitArchiveField::PushRemote;
        app.dialogs.push_front(Dialog::Maintenance(Box::new(
            MaintenanceDialog::GitArchiveForm(Box::new(draft)),
        )));
        app.focus = FocusTarget::Dialog;
        for (width, height) in [(160, 40), (100, 26), (80, 24)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Git release archive"), "{width}: {output}");
            assert!(output.contains("/release/archive.git"), "{width}: {output}");
            assert!(output.contains("[x] bare"), "{width}: {output}");
            assert!(output.contains("origin"), "{width}: {output}");
            assert!(output.contains("second network"), "{width}: {output}");
            assert!(
                output.contains("note identity changed"),
                "{width}: {output}"
            );
        }
    }
}
