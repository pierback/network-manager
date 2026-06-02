use gpui::{px, rgba, Hsla, Pixels};
use network_manager_core::AvailabilityState;

#[derive(Debug, Clone, Copy)]
pub struct AppColors {
    pub background: Hsla,
    pub panel: Hsla,
    pub panel_strong: Hsla,
    pub sidebar: Hsla,
    pub edge: Hsla,
    pub edge_soft: Hsla,
    pub text: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    pub selected: Hsla,
    pub online: Hsla,
    pub offline: Hsla,
    pub unknown: Hsla,
    pub stale: Hsla,
}

#[derive(Debug, Clone, Copy)]
pub struct LiquidGlassTokens {
    pub colors: AppColors,
}

impl LiquidGlassTokens {
    pub fn v4() -> Self {
        Self {
            colors: AppColors {
                background: rgba(0x070809ff).into(),
                panel: rgba(0xffffff12).into(),
                panel_strong: rgba(0xffffff1f).into(),
                sidebar: rgba(0xffffff0f).into(),
                edge: rgba(0xffffff30).into(),
                edge_soft: rgba(0xffffff18).into(),
                text: rgba(0xf7f7f7ff).into(),
                text_secondary: rgba(0xc9cbd1ff).into(),
                text_muted: rgba(0x858991ff).into(),
                selected: rgba(0xffffff1a).into(),
                online: rgba(0x52d273ff).into(),
                offline: rgba(0xff5f58ff).into(),
                unknown: rgba(0x8b9098ff).into(),
                stale: rgba(0xf6c85fff).into(),
            },
        }
    }

    pub fn status_color(self, state: AvailabilityState) -> Hsla {
        match state {
            AvailabilityState::Online => self.colors.online,
            AvailabilityState::Offline => self.colors.offline,
            AvailabilityState::Unknown => self.colors.unknown,
        }
    }
}

pub fn spacing(units: f32) -> Pixels {
    px(8.0 * units)
}
