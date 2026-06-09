use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

pub struct NetworkManagerAssets;

impl AssetSource for NetworkManagerAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(icon_bytes(path).map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        if path == "icons" {
            Ok(ICON_PATHS
                .iter()
                .map(|path| SharedString::from(*path))
                .collect())
        } else {
            Ok(Vec::new())
        }
    }
}

const ICON_PATHS: &[&str] = &[
    "activity.svg",
    "arrow-right.svg",
    "arrow-up-right.svg",
    "bell.svg",
    "check.svg",
    "chevron-down.svg",
    "copy.svg",
    "edit-3.svg",
    "external-link.svg",
    "folder.svg",
    "git-branch.svg",
    "git-merge.svg",
    "globe.svg",
    "info.svg",
    "layout-dashboard.svg",
    "network.svg",
    "panel-right.svg",
    "pencil.svg",
    "plus.svg",
    "radar.svg",
    "refresh-cw.svg",
    "rotate-ccw.svg",
    "router.svg",
    "route.svg",
    "search.svg",
    "server.svg",
    "settings.svg",
    "shield-check.svg",
    "sliders-horizontal.svg",
    "terminal.svg",
    "wifi.svg",
];

fn icon_bytes(path: &str) -> Option<&'static [u8]> {
    Some(match path {
        "icons/activity.svg" => include_bytes!("../assets/icons/activity.svg"),
        "icons/arrow-right.svg" => include_bytes!("../assets/icons/arrow-right.svg"),
        "icons/arrow-up-right.svg" => include_bytes!("../assets/icons/arrow-up-right.svg"),
        "icons/bell.svg" => include_bytes!("../assets/icons/bell.svg"),
        "icons/check.svg" => include_bytes!("../assets/icons/check.svg"),
        "icons/chevron-down.svg" => include_bytes!("../assets/icons/chevron-down.svg"),
        "icons/copy.svg" => include_bytes!("../assets/icons/copy.svg"),
        "icons/edit-3.svg" => include_bytes!("../assets/icons/edit-3.svg"),
        "icons/external-link.svg" => include_bytes!("../assets/icons/external-link.svg"),
        "icons/folder.svg" => include_bytes!("../assets/icons/folder.svg"),
        "icons/git-branch.svg" => include_bytes!("../assets/icons/git-branch.svg"),
        "icons/git-merge.svg" => include_bytes!("../assets/icons/git-merge.svg"),
        "icons/globe.svg" => include_bytes!("../assets/icons/globe.svg"),
        "icons/info.svg" => include_bytes!("../assets/icons/info.svg"),
        "icons/layout-dashboard.svg" => include_bytes!("../assets/icons/layout-dashboard.svg"),
        "icons/network.svg" => include_bytes!("../assets/icons/network.svg"),
        "icons/panel-right.svg" => include_bytes!("../assets/icons/panel-right.svg"),
        "icons/pencil.svg" => include_bytes!("../assets/icons/pencil.svg"),
        "icons/plus.svg" => include_bytes!("../assets/icons/plus.svg"),
        "icons/radar.svg" => include_bytes!("../assets/icons/radar.svg"),
        "icons/refresh-cw.svg" => include_bytes!("../assets/icons/refresh-cw.svg"),
        "icons/rotate-ccw.svg" => include_bytes!("../assets/icons/rotate-ccw.svg"),
        "icons/router.svg" => include_bytes!("../assets/icons/router.svg"),
        "icons/route.svg" => include_bytes!("../assets/icons/route.svg"),
        "icons/search.svg" => include_bytes!("../assets/icons/search.svg"),
        "icons/server.svg" => include_bytes!("../assets/icons/server.svg"),
        "icons/settings.svg" => include_bytes!("../assets/icons/settings.svg"),
        "icons/shield-check.svg" => include_bytes!("../assets/icons/shield-check.svg"),
        "icons/sliders-horizontal.svg" => include_bytes!("../assets/icons/sliders-horizontal.svg"),
        "icons/terminal.svg" => include_bytes!("../assets/icons/terminal.svg"),
        "icons/wifi.svg" => include_bytes!("../assets/icons/wifi.svg"),
        _ => return None,
    })
}
