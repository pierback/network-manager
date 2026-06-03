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
                background: rgba(0x1c1c1eff).into(),
                panel: rgba(0x2c2c2eff).into(),
                panel_strong: rgba(0x3a3a3cff).into(),
                popover: rgba(0x2c2c2eff).into(),
                sidebar: rgba(0x252527ff).into(),
                edge: rgba(0x48484aff).into(),
                edge_soft: rgba(0x38383aff).into(),
                text: rgba(0xffffffff).into(),
                text_secondary: rgba(0x98989dff).into(),
                text_muted: rgba(0x636366ff).into(),
                text_inverse: rgba(0x1c1c1eff).into(),
                selected: rgba(0x3a3a3cff).into(),
                accent: rgba(0x0a84ffff).into(),
                accent_hover: rgba(0x409cffff).into(),
                ssh_capable: rgba(0xbf5af2ff).into(),
                online: rgba(0x30d158ff).into(),
                offline: rgba(0xff453aff).into(),
                unknown: rgba(0x636366ff).into(),
                stale: rgba(0xffd60aff).into(),
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
