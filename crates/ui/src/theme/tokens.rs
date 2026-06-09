use gpui::{px, rgba, Hsla, Pixels};
use network_manager_core::AvailabilityState;

#[derive(Debug, Clone, Copy)]
pub struct AppColors {
    pub background: Hsla,
    pub panel: Hsla,
    pub panel_strong: Hsla,
    pub popover: Hsla,
    pub sidebar: Hsla,
    pub edge: Hsla,
    pub edge_soft: Hsla,
    pub text: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    pub text_inverse: Hsla,
    pub selected: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub icy: Hsla,
    pub window: Hsla,
    pub ssh_capable: Hsla,
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
                background: rgba(0x08090aff).into(),
                panel: rgba(0xffffff14).into(),
                panel_strong: rgba(0xffffff20).into(),
                popover: rgba(0x121315d9).into(),
                sidebar: rgba(0xffffff14).into(),
                edge: rgba(0xffffff24).into(),
                edge_soft: rgba(0xffffff12).into(),
                text: rgba(0xf7f7f8ff).into(),
                text_secondary: rgba(0xb7b8baff).into(),
                text_muted: rgba(0x74767aff).into(),
                text_inverse: rgba(0x08090aff).into(),
                selected: rgba(0xffffff18).into(),
                accent: rgba(0xa9d8ffff).into(),
                accent_hover: rgba(0xffffff20).into(),
                icy: rgba(0xa9d8ffff).into(),
                window: rgba(0x101113ff).into(),
                ssh_capable: rgba(0xbf5af2ff).into(),
                online: rgba(0x36d67aff).into(),
                offline: rgba(0xff5a52ff).into(),
                unknown: rgba(0x72757aff).into(),
                stale: rgba(0xe7c65aff).into(),
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
