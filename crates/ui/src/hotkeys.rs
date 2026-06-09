use gpui::{actions, Action, App, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType};

// Hotkey-first action surface for the app. Route/action shortcuts are scoped to the
// focused NetworkManager root so future text inputs can own editing bindings inside
// their own key contexts.
actions!(
    network_manager,
    [
        NewWindow,
        Open,
        Save,
        SaveAs,
        Print,
        Find,
        FindNext,
        FindPrevious,
        ShowDashboard,
        ShowDiscovery,
        ShowDeviceDetail,
        ShowQuickAccess,
        ShowSettings,
        NextRoute,
        PreviousRoute,
        ToggleSidebar,
        RefreshQuick,
        RefreshFull,
        ShowKeyboardShortcuts,
        ZoomIn,
        ZoomOut,
        ActualSize,
        CloseWindow,
        MinimizeWindow,
        ZoomWindow,
        ToggleFullscreen,
        BringAllToFront,
        Quit,
        Hide,
        HideOthers,
        ShowAll,
        Undo,
        Redo,
        CutSelection,
        CopySelection,
        PasteSelection,
        SelectAll,
    ]
);

pub const KEY_CONTEXT: &str = "NetworkManager";

pub fn install_app_hotkeys(cx: &mut App) {
    cx.bind_keys(default_key_bindings());
    cx.set_menus(app_menus());
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
}

pub fn default_key_bindings() -> Vec<KeyBinding> {
    vec![
        // Standard macOS app/window bindings remain global.
        global_binding("cmd-n", NewWindow),
        global_binding("cmd-q", Quit),
        global_binding("cmd-h", Hide),
        global_binding("cmd-alt-h", HideOthers),
        global_binding("cmd-w", CloseWindow),
        global_binding("cmd-m", MinimizeWindow),
        global_binding("cmd-ctrl-f", ToggleFullscreen),
        // Network Manager file/find actions.
        app_binding("cmd-o", Open),
        app_binding("cmd-s", Save),
        app_binding("cmd-shift-s", SaveAs),
        app_binding("cmd-p", Print),
        app_binding("cmd-f", Find),
        app_binding("cmd-g", FindNext),
        app_binding("cmd-shift-g", FindPrevious),
        // Primary route navigation.
        app_binding("cmd-1", ShowDashboard),
        app_binding("cmd-2", ShowDiscovery),
        app_binding("cmd-3", ShowDeviceDetail),
        app_binding("cmd-4", ShowQuickAccess),
        app_binding("cmd-5", ShowSettings),
        app_binding("cmd-,", ShowSettings),
        app_binding("escape", ShowDashboard),
        // Fast app actions.
        app_binding("cmd-r", RefreshQuick),
        app_binding("cmd-shift-r", RefreshFull),
        app_binding("cmd-k", ShowQuickAccess),
        app_binding("cmd-alt-s", ToggleSidebar),
        app_binding("cmd-/", ShowKeyboardShortcuts),
        app_binding("cmd-shift-/", ShowKeyboardShortcuts),
        // View sizing and history/cycling style navigation.
        app_binding("cmd-+", ZoomIn),
        app_binding("cmd-=", ZoomIn),
        app_binding("cmd--", ZoomOut),
        app_binding("cmd-0", ActualSize),
        app_binding("cmd-[", PreviousRoute),
        app_binding("cmd-]", NextRoute),
        app_binding("cmd-left", PreviousRoute),
        app_binding("cmd-right", NextRoute),
        app_binding("cmd-alt-left", PreviousRoute),
        app_binding("cmd-alt-right", PreviousRoute),
        app_binding("ctrl-tab", NextRoute),
        app_binding("ctrl-shift-tab", PreviousRoute),
    ]
}

fn app_binding<A: Action>(keystrokes: &str, action: A) -> KeyBinding {
    KeyBinding::new(keystrokes, action, Some(KEY_CONTEXT))
}

fn global_binding<A: Action>(keystrokes: &str, action: A) -> KeyBinding {
    KeyBinding::new(keystrokes, action, None)
}

fn app_menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "Network Manager".into(),
            items: vec![
                MenuItem::action("About Network Manager", ShowKeyboardShortcuts),
                MenuItem::action("Settings…", ShowSettings),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Hide Network Manager", Hide),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quit Network Manager", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Window", NewWindow),
                MenuItem::action("Open Discovery", Open),
                MenuItem::separator(),
                MenuItem::action("Save", Save),
                MenuItem::action("Save As…", SaveAs),
                MenuItem::separator(),
                MenuItem::action("Print…", Print),
                MenuItem::separator(),
                MenuItem::action("Close Window", CloseWindow),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", CutSelection, OsAction::Cut),
                MenuItem::os_action("Copy", CopySelection, OsAction::Copy),
                MenuItem::os_action("Paste", PasteSelection, OsAction::Paste),
                MenuItem::separator(),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
                MenuItem::separator(),
                MenuItem::submenu(Menu {
                    name: "Find".into(),
                    items: vec![
                        MenuItem::action("Find…", Find),
                        MenuItem::action("Find Next", FindNext),
                        MenuItem::action("Find Previous", FindPrevious),
                    ],
                }),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Dashboard", ShowDashboard),
                MenuItem::action("Discovery", ShowDiscovery),
                MenuItem::action("Device Detail", ShowDeviceDetail),
                MenuItem::action("Quick Access", ShowQuickAccess),
                MenuItem::action("Settings", ShowSettings),
                MenuItem::separator(),
                MenuItem::action("Previous Screen", PreviousRoute),
                MenuItem::action("Next Screen", NextRoute),
                MenuItem::separator(),
                MenuItem::action("Refresh", RefreshQuick),
                MenuItem::action("Full Refresh", RefreshFull),
                MenuItem::separator(),
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::action("Actual Size", ActualSize),
                MenuItem::separator(),
                MenuItem::action("Keyboard Shortcuts", ShowKeyboardShortcuts),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Minimize", MinimizeWindow),
                MenuItem::action("Zoom", ZoomWindow),
                MenuItem::action("Enter Full Screen", ToggleFullscreen),
                MenuItem::separator(),
                MenuItem::action("Bring All to Front", BringAllToFront),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keymap_covers_standard_and_app_navigation_shortcuts() {
        let bindings = default_key_bindings();
        let binding_for = |needle: &str| {
            let parsed = gpui::Keystroke::parse(needle).expect("valid keystroke");
            bindings.iter().find(|binding| {
                binding.match_keystrokes(std::slice::from_ref(&parsed)) == Some(false)
            })
        };

        for key in [
            "cmd-n",
            "cmd-o",
            "cmd-s",
            "cmd-shift-s",
            "cmd-p",
            "cmd-q",
            "cmd-w",
            "cmd-m",
            "cmd-h",
            "cmd-f",
            "cmd-g",
            "cmd-shift-g",
            "cmd-1",
            "cmd-2",
            "cmd-3",
            "cmd-4",
            "cmd-5",
            "cmd-r",
            "cmd-shift-r",
            "cmd-k",
            "cmd-/",
            "cmd-shift-/",
            "cmd-+",
            "cmd--",
            "cmd-0",
            "cmd-[",
            "cmd-]",
            "cmd-ctrl-f",
        ] {
            assert!(binding_for(key).is_some(), "missing binding {key}");
        }

        for key in [
            "cmd-o",
            "cmd-s",
            "cmd-shift-s",
            "cmd-p",
            "cmd-f",
            "cmd-g",
            "cmd-shift-g",
            "cmd-1",
            "cmd-2",
            "cmd-3",
            "cmd-4",
            "cmd-5",
            "cmd-r",
            "cmd-shift-r",
            "cmd-k",
            "cmd-/",
            "cmd-shift-/",
            "cmd-+",
            "cmd--",
            "cmd-0",
            "cmd-[",
            "cmd-]",
        ] {
            let binding = binding_for(key).expect("binding exists");
            assert!(binding.predicate().is_some(), "binding {key} is global");
        }

        for key in ["cmd-z", "cmd-shift-z", "cmd-x", "cmd-c", "cmd-v", "cmd-a"] {
            assert!(
                binding_for(key).is_none(),
                "edit binding {key} should stay owned by editable controls"
            );
        }
    }
}
