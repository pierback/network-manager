use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

use crate::components::icons::ASSETS;

pub struct NetworkManagerAssets;

impl AssetSource for NetworkManagerAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(icon_bytes(path).map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        if path == "icons" {
            Ok(ASSETS
                .iter()
                .filter_map(|(path, _)| path.strip_prefix("icons/"))
                .map(SharedString::from)
                .collect())
        } else {
            Ok(Vec::new())
        }
    }
}

fn icon_bytes(path: &str) -> Option<&'static [u8]> {
    ASSETS
        .iter()
        .find_map(|(asset_path, bytes)| (*asset_path == path).then_some(*bytes))
}
