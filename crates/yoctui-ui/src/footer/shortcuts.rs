pub(crate) fn footer_shortcuts(app: &App) -> String {
    if app.screen == Screen::Signatures {
        return with_compatibility_footer(
            app,
            WorkspaceDestination::Signatures,
            with_focus_shortcuts(
                app,
                "↑/↓ select | 1/2 sides | c compare | r refresh | e provider | Esc back/cancel",
            ),
        );
    }
    if app.focus == FocusTarget::Navigator {
        return with_compatibility_footer(
            app,
            app.navigator_compatibility_destination(),
            with_focus_shortcuts(
                app,
                "↑/↓ select | h/l groups | Enter open | Ctrl+B prefix | q quit",
            ),
        );
    }
    if app.focus == FocusTarget::Inspector {
        return with_compatibility_footer(
            app,
            yoctui_model::workspace_screen_destination(app.screen),
            with_focus_shortcuts(app, "↑/↓ scroll inspector | q quit"),
        );
    }
    if app.layer_browser.is_some()
        && (app.screen == Screen::Layers
            || (app.screen == Screen::Images
                && app.images_view == ImagesView::RootfsFilesystem
                && app.rootfs_browser().is_some()))
    {
        if app.screen == Screen::Images {
            return with_focus_shortcuts(
                app,
                "↑/↓ select | → expand/preview | ← collapse/tree | Enter toggle/preview | e edit | . hidden | / search | r refresh | Tab views",
            );
        }
        return with_compatibility_footer(
            app,
            yoctui_model::workspace_screen_destination(app.screen),
            with_focus_shortcuts(
                app,
                "↑/↓ select | PgUp/PgDn page | →/l expand or focus preview | preview arrows scroll | ← tree | e editor | i info | r refresh | . hidden | / search",
            ),
        );
    }
    if app.platform_menuconfig_visible() {
        return with_compatibility_footer(
            app,
            yoctui_model::workspace_screen_destination(app.screen),
            with_focus_shortcuts(app, "Ctrl+G Yoctui | all other keys go to menuconfig"),
        );
    }
    let shortcuts = match app.screen {
        Screen::Dashboard => {
            "Alt+b build | f favorites | t terminals | F2 Tasks | e errors | Ctrl+B prefix | F8 artifacts | l logs | F3 work | Alt+e environment | Alt+m sstate | Ctrl+P commands | Tab focus | c cancel | ? help | q quit"
        }
        Screen::Insights => "1-8 view | [/] previous/next | Tab focus | Esc dashboard",
        Screen::Tasks => {
            "↑/↓ select | f state | Alt+f field | / edit filter | d duration | c cancel | Tab focus"
        }
        Screen::BuildHistory => {
            "↑/↓ select | Enter details | o Load environment | ←/→ view | PgUp/PgDn scroll | r refresh | l live/saved | Esc back"
        }
        Screen::Dependencies => {
            "↑/↓ or j/k select | Enter recipe | o provider | Alt+l task log | r refresh | Tab focus | Esc dashboard"
        }
        Screen::Signatures => {
            "↑/↓ select | 1/2 sides | c compare | r refresh | e provider | Esc back/cancel"
        }
        Screen::LayerRelationships => "Esc dashboard | y layers | ? help | q quit",
        Screen::Recipes => {
            "↑/↓ select | e provider | o logs | p patches | b/f tasks | v devshell | s shell | Alt+w workspace | Alt+g GitUI | Alt+e edit-recipe | Alt+v CVE | Alt+x SPDX | d modify | u update | Alt+f finish | Alt+p deploy | Alt+d reset | / search"
        }
        Screen::Devtool => {
            "↑/↓ recipe | Enter refresh | d/e/Alt+w source | b build | Alt+p deploy SSH/SCP | u create patches | Alt+f finish into layer | s shell | Alt+g GitUI | Alt+d reset | / search"
        }
        Screen::Packages => {
            "↑/↓ select | Enter detail | / search | Alt+r refresh | Alt+d dep kind | [/] dep | d follow | u back | o recipe | e provider | c cancel"
        }
        Screen::Images => match app.images_view {
            ImagesView::Artifacts => {
                "↑/↓ select | Enter/o/e view/decompile | v rootfs files | p rootfs | Tab view | Alt+q QEMU | Alt+w create Wic | Alt+d write device | x cancel | [/] output | Alt+o open output | / search | Alt+r refresh | b build | m manifest | l license | s SPDX | w Wic"
            }
            ImagesView::RootfsPackages => {
                "h/l group | j/k package | PgUp/PgDn page | r refresh | Tab filesystem | Shift+Tab artifacts"
            }
            ImagesView::RootfsFilesystem => {
                "j/k select path | Enter/→ explore actual IMAGE_ROOTFS | r refresh | Tab view"
            }
            ImagesView::SystemdServices => {
                "j/k service | Enter/e view unit | → explore rootfs | r refresh | Tab view"
            }
            ImagesView::SystemDbus => {
                "j/k bus name | Enter/e view activation file | → explore rootfs | r refresh | Tab view"
            }
            ImagesView::UdevRules => {
                "↑/↓ rule | PgUp/PgDn | [/] preview | Enter/e view rule | → rootfs | r refresh | Tab view"
            }
        },
        Screen::Hardware => {
            if app.hardware.viewer.is_some() {
                "Esc library | PgUp/PgDn page | +/- zoom | 0 fit | w width | arrows pan | o desktop PDF | / search | v text"
            } else if app.hardware.projects.form.is_some() {
                "Enter save/create | Esc cancel | progress: ↑/↓ stage · ←/→ ±5 · digits 0–100"
            } else if app.hardware.projects.import_browser.is_some() {
                "↑/↓ select | Enter directory/copy | Backspace parent | Esc cancel"
            } else if app.hardware.projects.visible {
                "↑/↓ select | Enter open | n new project/folder | a import | s bring-up | Backspace parent | p library"
            } else if app.hardware.browser.is_some() {
                "↑/↓ select | Enter directory | Backspace parent | ←/→ category | a add | Esc cancel"
            } else {
                "←/→ category | ↑/↓ select | Enter view | a add | d remove | r reload | p Projects | F12 menu"
            }
        }
        Screen::Kernel => {
            if app.kernel_debug.visible {
                "1/2/3 or Tab view | ↑/↓ technique | Enter tool/guide | r tools | b Debugging | F12 menu"
            } else if app.platform_menuconfig_hidden() {
                "Ctrl+G Resume menuconfig | Tab view | ↑/↓ select | Enter view | e edit | o explore | r refresh"
            } else {
                "Tab view | 3/b Debugging | ↑/↓ select | m menuconfig | Enter view | e edit | o explore | c compile DTS | d decompile DTB | r refresh"
            }
        }
        Screen::Firmware => {
            if app.platform_menuconfig_hidden() {
                "Ctrl+G Resume menuconfig | Tab view | ↑/↓ select | Enter view | e edit | o explore | r refresh"
            } else {
                "Tab view | ↑/↓ select | m menuconfig | Enter view | e edit | o explore | c compile DTS | d decompile DTB | r refresh"
            }
        }
        Screen::Sdk => {
            "↑/↓ select | c cancel | i image | s standard | Alt+e extensible | t testsdk | Alt+t testsdkext | Alt+r refresh | Alt+p publish | n native | o open"
        }
        Screen::Testing => {
            "Tab view | ↑/↓ select | Enter open | r run | i image | / search | Alt+i import | Alt+r refresh | c compare | Alt+j JUnit | o result | l log | x cancel"
        }
        Screen::Security => {
            "Tab view | ↑/↓ select | s scope | i image | / search | f status | Alt+v CVE check | Alt+m map | Alt+x SBOM | Alt+i import | Alt+r refresh | Enter details | o report | e recipe | v advisory | c cancel"
        }
        Screen::Qa => {
            "Tab view | ↑/↓ select | s scope | / search | f status | r run | Alt+i import | Alt+r refresh | Enter details | o report | e provider | l source | c cancel"
        }
        Screen::RawMode => {
            if app.raw_mode.view == yoctui_model::RawModeView::Execution {
                "↑/↓ Scroll | ←/→ Horizontal | 1/2 Stream | f Follow | / Search | c Cancel | d Detach | r Reattach | Esc Back"
            } else {
                "←/→ Pane | ↑/↓ Select | Enter Open | / Search | f Favorite | Alt+h History | Tab Focus | F1 Help | F12 Menu | q Quit"
            }
        }
        Screen::TerminalSessions => {
            if app.suspended_recipe_editor.is_some() {
                "Ctrl+B prefix: editor (e) | control (o) | [ copy | / search | r rename | Alt+o release | z zoom | paste review"
            } else {
                "Ctrl+B prefix | o take control | [ copy | / search | r rename | Alt+o release | Alt+k confirmed kill | z zoom | paste review"
            }
        }
        Screen::Layers => {
            "↑/↓ select | Enter browse | i image | Alt+r relationships | e in-TUI edit | o external editor | / search | Esc dashboard | ? help | q quit"
        }
        Screen::Configuration => {
            "↑/↓ select | Enter inspect | s scope | c compare | Alt+c copy effective | Alt+u copy unexpanded | o source | Alt+e edit | / search | x BBMASK | Esc dashboard | ? help | q quit"
        }
        Screen::Bbmask => {
            "e edit BBMASK | Enter preview/confirm | Esc cancel/dashboard | v configuration | ? help | q quit"
        }
        Screen::Maintenance => match app.maintenance.view {
            MaintenanceView::Sstate => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | Alt+s signatures | c check | d cleanup"
            }
            MaintenanceView::Services => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | Alt+s signatures | e PR export | m PR import"
            }
            MaintenanceView::Release => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | Alt+s signatures | l locked cache | h compare | a archive"
            }
            MaintenanceView::Integrations => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | Alt+s signatures | detection/inspection only"
            }
        },
        Screen::Compatibility => {
            "↑/↓ or j/k select | 1 All | 2 Available | 3 Limited | 4 Unavailable | 5 Attention | / search | Tab focus"
        }
        Screen::Daemons => "1 health | 2 logs | r refresh | s start | x stop | t restart | c configure | ↑/↓ PgUp/PgDn Home/End scroll | Esc back",
        Screen::Logs => match app.log_workspace_view {
            LogWorkspaceView::BitBake if app.logs.follow => {
                "v diagnostics | ↑/↓ select | ←/→ horizontal | f pause | w wrap | s/Alt+r/Alt+t/Alt+b/Alt+s/Alt+i filters | / search | m bookmark | Alt+c copy | Alt+e export"
            }
            LogWorkspaceView::BitBake => {
                "v diagnostics | ↑/↓ select | ←/→ horizontal | f follow | w wrap | s/Alt+r/Alt+t/Alt+b/Alt+s/Alt+i filters | / search | m bookmark | Alt+c copy | Alt+e export"
            }
            LogWorkspaceView::Yoctui if app.internal_logs.follow => {
                "v BitBake | ↑/↓ select | f pause | s level | Alt+t target | / search | Alt+e export | c clear"
            }
            LogWorkspaceView::Yoctui => {
                "v BitBake | ↑/↓ select | f follow | s level | Alt+t target | / search | Alt+e export | c clear"
            }
        },
        Screen::Errors => {
            if app.error_workspace.viewer.is_some() {
                "↑/↓ PgUp/PgDn scroll | Home/End | Esc error list"
            } else if app.error_workspace.view == yoctui_model::ErrorWorkspaceView::History {
                "1 current | 2 past | Tab switch | ↑/↓ select | Enter view log | o external | d/Delete remove resolved"
            } else {
                "1 current | 2 past | Tab switch | ↑/↓ select | Enter view log | l matching live log | o external"
            }
        }
        Screen::Help => "Esc dashboard | q quit",
        Screen::Settings => {
            "↑/↓ select | ←/→ change | r retry save | Ctrl+P commands | Tab focus | q quit"
        }
        Screen::BuildEnvironment => {
            "e configure | b browse paths | Alt+a advanced | Alt+v verify | Tab focus | q quit"
        }
    };
    with_compatibility_footer(
        app,
        yoctui_model::workspace_screen_destination(app.screen),
        with_focus_shortcuts(app, shortcuts),
    )
}

pub(crate) fn responsive_footer_shortcuts(app: &App, width: u16) -> String {
    if app.screen == Screen::Images && width <= 129 {
        match app.images_view {
            ImagesView::Artifacts => {
                "↑↓ select | Enter view | p rootfs | Tab view | Alt+q QEMU | Alt+w Wic | Alt+d write"
                    .into()
            }
            ImagesView::RootfsPackages => {
                "h/l group | j/k package | PgUp/PgDn | r refresh | Tab view".into()
            }
            ImagesView::RootfsFilesystem => "j/k path | PgUp/PgDn | r refresh | Tab view".into(),
            ImagesView::SystemdServices => "j/k service | Enter/e view | → rootfs | Tab view".into(),
            ImagesView::SystemDbus => "j/k bus | Enter/e view | → rootfs | Tab view".into(),
            ImagesView::UdevRules => {
                "↑↓ rule | [/] preview | Enter/e view | → rootfs | Tab view".into()
            }
        }
    } else if app.screen == Screen::Sdk && width < 100 {
        compact_sdk_shortcuts(width)
    } else if app.screen == Screen::Testing && width <= 90 {
        "Tab:view ↑↓ Enter r:run i:image /:find Alt+i/Alt+r:results c:compare Alt+j:JUnit o/l:open x:cancel"
            .into()
    } else if app.screen == Screen::Security && width <= 90 {
        "Tab:view ↑↓ s:scope i:image /:find f:status Alt+v:check Alt+m:map Alt+x:SBOM Alt+i/Alt+r:data Enter o/e/v:open c:cancel"
            .into()
    } else if app.screen == Screen::Qa && width <= 90 {
        "Tab:view ↑↓ s:scope /:find f:status r:run Alt+i/Alt+r:data Enter o/e/l:open c:cancel".into()
    } else {
        let shortcuts = footer_shortcuts(app);
        if width < 100 && pane_focus_shortcuts(app).is_some() {
            shortcuts
                .split(" | ")
                .skip(3)
                .collect::<Vec<_>>()
                .join(" | ")
        } else {
            shortcuts
        }
    }
}

fn compact_sdk_shortcuts(width: u16) -> String {
    let mut output = String::new();
    for token in [
        "↑↓", "i:img", "s/Alt+e:SDK", "t/Alt+t:tst", "c:cancel", "Alt+r:scan",
        "Alt+p:pub", "n:native", "o:open",
    ] {
        let separator = usize::from(!output.is_empty());
        if Line::from(output.as_str()).width() + separator + Line::from(token).width()
            > usize::from(width)
        {
            break;
        }
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(token);
    }
    output
}
