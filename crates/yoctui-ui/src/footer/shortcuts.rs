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
    if app.layer_browser.is_some() {
        return with_compatibility_footer(
            app,
            WorkspaceDestination::Layers,
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
            with_focus_shortcuts(
                app,
                "Ctrl+B prefix | [ copy | / search | O release | K confirmed kill | o take (viewer)",
            ),
        );
    }
    let shortcuts = match app.screen {
        Screen::Dashboard => {
            "B build | f favorites | t terminals | F2 Tasks | e errors | Ctrl+B prefix | F8 artifacts | l logs | F3 work | E environment | M sstate | Ctrl+P commands | Tab focus | c cancel | ? help | q quit"
        }
        Screen::Insights => "1-8 view | [/] previous/next | Tab focus | Esc dashboard",
        Screen::Tasks => {
            "↑/↓ select | f state | F field | / edit filter | d duration | c cancel | Tab focus"
        }
        Screen::BuildHistory => {
            "↑/↓ select | Enter details | ←/→ view | PgUp/PgDn scroll | r refresh | l live/saved | Esc back"
        }
        Screen::Dependencies => {
            "↑/↓ or j/k select | Enter recipe | o provider | L task log | r refresh | Tab focus | Esc dashboard"
        }
        Screen::Signatures => {
            "↑/↓ select | 1/2 sides | c compare | r refresh | e provider | Esc back/cancel"
        }
        Screen::LayerRelationships => "Esc dashboard | y layers | ? help | q quit",
        Screen::Recipes => {
            "↑/↓ select | [/] preview scroll | e provider | o logs | p patches | b/f tasks | v devshell | s workspace shell | E edit-recipe | V CVE | X SPDX | d modify | u update | F finish | P deploy | D reset | / search"
        }
        Screen::Devtool => {
            "↑/↓ recipe | Enter refresh | d start/edit | e source | b build | P deploy SSH/SCP | u create patches | F finish into layer | s shell | G GitUI | D reset | / search"
        }
        Screen::Packages => {
            "↑/↓ select | Enter detail | / search | R refresh | D dep kind | [/] dep | d follow | u back | o recipe | e provider | c cancel"
        }
        Screen::Images => match app.images_view {
            ImagesView::Artifacts => {
                "↑/↓ select | Enter/p rootfs | Tab view | Q QEMU | W create Wic | D write device | x cancel | [/] output | O open output | / search | R refresh | b build | o artifact | m manifest | l license | s SPDX | w Wic"
            }
            ImagesView::RootfsPackages => {
                "h/l group | j/k package | PgUp/PgDn page | r refresh | Tab filesystem | Shift+Tab artifacts"
            }
            ImagesView::RootfsFilesystem => {
                "j/k select path | Enter/→ explore actual IMAGE_ROOTFS | r refresh | Tab view"
            }
            ImagesView::SystemdServices => {
                "j/k service | e edit unit | Enter/→ explore rootfs | r refresh | Tab view"
            }
            ImagesView::SystemDbus => {
                "j/k bus name | e edit activation file | Enter/→ explore rootfs | r refresh | Tab view"
            }
            ImagesView::UdevRules => {
                "↑/↓ rule | PgUp/PgDn | [/] preview | Enter explore rootfs | r refresh | Tab view"
            }
        },
        Screen::Kernel => {
            "Tab view | ↑/↓ select | m menuconfig | Enter view | e edit | o explore | c compile DTS | d decompile DTB | r refresh"
        }
        Screen::Firmware => {
            "Tab view | ↑/↓ select | m menuconfig | Enter view | e edit | o explore | c compile DTS | d decompile DTB | r refresh"
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
        Screen::RawMode => {
            if app.raw_mode.view == yoctui_model::RawModeView::Execution {
                "↑/↓ Scroll | ←/→ Horizontal | 1/2 Stream | f Follow | / Search | c Cancel | d Detach | r Reattach | Esc Back"
            } else {
                "←/→ Pane | ↑/↓ Select | Enter Open | / Search | f Favorite | H History | Tab Focus | F1 Help | F12 Menu | q Quit"
            }
        }
        Screen::TerminalSessions => {
            "Ctrl+B prefix | [ copy | / search | r rename | O release | K confirmed kill | z zoom | paste review | o take (viewer)"
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
        Screen::Compatibility => {
            "↑/↓ or j/k select | 1 All | 2 Available | 3 Limited | 4 Unavailable | 5 Attention | / search | Tab focus"
        }
        Screen::Logs => match app.log_workspace_view {
            LogWorkspaceView::BitBake if app.logs.follow => {
                "v diagnostics | ↑/↓ select | ←/→ horizontal | f pause | w wrap | s/R/T/B/S/I filters | / search | m bookmark | C copy | E export"
            }
            LogWorkspaceView::BitBake => {
                "v diagnostics | ↑/↓ select | ←/→ horizontal | f follow | w wrap | s/R/T/B/S/I filters | / search | m bookmark | C copy | E export"
            }
            LogWorkspaceView::Yoctui if app.internal_logs.follow => {
                "v BitBake | ↑/↓ select | f pause | s level | T target | / search | E export | c clear"
            }
            LogWorkspaceView::Yoctui => {
                "v BitBake | ↑/↓ select | f follow | s level | T target | / search | E export | c clear"
            }
        },
        Screen::Errors => {
            "↑/↓ select | Enter matching log | o source | s severity filter | f pause/follow | B rebuild options"
        }
        Screen::Help => "Esc dashboard | q quit",
        Screen::Settings => {
            "↑/↓ select | ←/→ change | r retry save | Ctrl+P commands | Tab focus | q quit"
        }
        Screen::BuildEnvironment => {
            "e configure | b browse paths | A advanced | V verify | Tab focus | q quit"
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
                "↑↓ select | R refresh | Enter/p rootfs | Tab view | Q QEMU | W Wic | D write"
                    .into()
            }
            ImagesView::RootfsPackages => {
                "h/l group | j/k package | PgUp/PgDn | r refresh | Tab view".into()
            }
            ImagesView::RootfsFilesystem => "j/k path | PgUp/PgDn | r refresh | Tab view".into(),
            ImagesView::SystemdServices => "j/k service | e edit | Enter explore | Tab view".into(),
            ImagesView::SystemDbus => "j/k bus | e edit | Enter explore | Tab view".into(),
            ImagesView::UdevRules => {
                "↑↓ rule | [/] preview | Enter explore | r refresh | Tab view".into()
            }
        }
    } else if app.screen == Screen::Sdk && width <= 90 {
        "↑↓ i:image s/E:SDK t/T:test R:scan P:publish n:native o:open c:cancel".into()
    } else if app.screen == Screen::Testing && width <= 90 {
        "Tab:view ↑↓ Enter r:run i:image /:find I/R:results c:compare J:JUnit o/l:open x:cancel"
            .into()
    } else if app.screen == Screen::Security && width <= 90 {
        "Tab:view ↑↓ s:scope i:image /:find f:status V:check M:map X:SBOM I/R:data Enter o/e/v:open c:cancel"
            .into()
    } else if app.screen == Screen::Qa && width <= 90 {
        "Tab:view ↑↓ s:scope /:find f:status r:run I/R:data Enter o/e/l:open c:cancel".into()
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
