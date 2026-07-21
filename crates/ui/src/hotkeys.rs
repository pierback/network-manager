use gpui::{actions, Action, App, KeyBinding, Menu, MenuItem, SystemMenuType};

// Hotkey-first action surface for the app. Route/action shortcuts are scoped to the
// focused NetworkManager root so future text inputs can own editing bindings inside
// their own key contexts.
actions!(
    network_manager,
    [
        NewWindow,
        ShowDashboard,
        ShowDiscovery,
        ShowDeviceDetail,
        ShowQuickAccess,
        ShowSettings,
        NextRoute,
        PreviousRoute,
        RefreshQuick,
        RefreshFull,
        ShowKeyboardShortcuts,
        CloseWindow,
        MinimizeWindow,
        ZoomWindow,
        ToggleFullscreen,
        BringAllToFront,
        Quit,
        Hide,
        HideOthers,
        ShowAll,
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
        app_binding("cmd-/", ShowKeyboardShortcuts),
        app_binding("cmd-shift-/", ShowKeyboardShortcuts),
        // History/cycling style navigation.
        app_binding("cmd-[", PreviousRoute),
        app_binding("cmd-]", NextRoute),
        app_binding("cmd-left", PreviousRoute),
        app_binding("cmd-right", NextRoute),
        app_binding("cmd-alt-left", PreviousRoute),
        app_binding("cmd-alt-right", NextRoute),
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
                MenuItem::action("Settings…", ShowSettings),
                MenuItem::action("Keyboard Shortcuts", ShowKeyboardShortcuts),
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
                MenuItem::separator(),
                MenuItem::action("Close Window", CloseWindow),
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
    fn keymap_contains_only_real_window_navigation_and_refresh_shortcuts() {
        let bindings = default_key_bindings();
        let binding_for = |needle: &str| {
            let parsed = gpui::Keystroke::parse(needle).expect("valid keystroke");
            bindings.iter().find(|binding| {
                binding.match_keystrokes(std::slice::from_ref(&parsed)) == Some(false)
            })
        };

        for key in [
            "cmd-n",
            "cmd-q",
            "cmd-w",
            "cmd-m",
            "cmd-h",
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
            "cmd-[",
            "cmd-]",
            "cmd-ctrl-f",
        ] {
            assert!(binding_for(key).is_some(), "missing binding {key}");
        }

        for key in [
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
            "cmd-[",
            "cmd-]",
        ] {
            let binding = binding_for(key).expect("binding exists");
            assert!(binding.predicate().is_some(), "binding {key} is global");
        }

        for key in [
            "cmd-o",
            "cmd-s",
            "cmd-shift-s",
            "cmd-p",
            "cmd-f",
            "cmd-g",
            "cmd-shift-g",
            "cmd-alt-s",
            "cmd-+",
            "cmd--",
            "cmd-0",
            "cmd-z",
            "cmd-shift-z",
            "cmd-x",
            "cmd-c",
            "cmd-v",
            "cmd-a",
        ] {
            assert!(
                binding_for(key).is_none(),
                "irrelevant binding {key} should not be advertised"
            );
        }
    }
}
