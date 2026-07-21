use gpui::{rgba, Hsla};
use network_manager_core::AvailabilityState;

#[derive(Debug, Clone, Copy)]
pub struct AppColors {
    pub background: Hsla,
    pub panel: Hsla,
    pub popover: Hsla,
    pub sidebar: Hsla,
    pub edge: Hsla,
    pub edge_soft: Hsla,
    pub text: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    pub selected: Hsla,
    pub icy: Hsla,
    pub online: Hsla,
    pub offline: Hsla,
    pub unknown: Hsla,
}

#[derive(Debug, Clone, Copy)]
pub struct LiquidGlassTokens {
    pub colors: AppColors,
}

impl Default for LiquidGlassTokens {
    fn default() -> Self {
        Self {
            colors: AppColors {
                background: rgba(0x08090aff).into(),
                panel: rgba(0xffffff14).into(),
                popover: rgba(0x121315d9).into(),
                sidebar: rgba(0xffffff14).into(),
                edge: rgba(0xffffff24).into(),
                edge_soft: rgba(0xffffff12).into(),
                text: rgba(0xf7f7f8ff).into(),
                text_secondary: rgba(0xb7b8baff).into(),
                text_muted: rgba(0x74767aff).into(),
                selected: rgba(0xffffff18).into(),
                icy: rgba(0xa9d8ffff).into(),
                online: rgba(0x36d67aff).into(),
                offline: rgba(0xff5a52ff).into(),
                unknown: rgba(0x72757aff).into(),
            },
        }
    }
}

impl LiquidGlassTokens {
    pub fn status_color(self, state: AvailabilityState) -> Hsla {
        match state {
            AvailabilityState::Online => self.colors.online,
            AvailabilityState::Offline => self.colors.offline,
            AvailabilityState::Unknown => self.colors.unknown,
        }
    }
}
