use gpui::{prelude::*, px, svg, Hsla, Svg};

macro_rules! define_icons {
    ($($variant:ident => $file:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Icon {
            $($variant),+
        }

        impl Icon {
            pub fn path(self) -> &'static str {
                match self {
                    $(Self::$variant => concat!("icons/", $file)),+
                }
            }
        }

        pub(crate) const ASSETS: &[(&str, &[u8])] = &[
            $((
                concat!("icons/", $file),
                include_bytes!(concat!("../../assets/icons/", $file)),
            )),+
        ];
    };
}

define_icons! {
    Activity => "activity.svg",
    ArrowUpRight => "arrow-up-right.svg",
    Check => "check.svg",
    Copy => "copy.svg",
    Dashboard => "layout-dashboard.svg",
    Folder => "folder.svg",
    Info => "info.svg",
    Network => "network.svg",
    PanelRight => "panel-right.svg",
    Plus => "plus.svg",
    Radar => "radar.svg",
    Refresh => "refresh-cw.svg",
    RotateCcw => "rotate-ccw.svg",
    Server => "server.svg",
    Settings => "settings.svg",
    ShieldCheck => "shield-check.svg",
    Terminal => "terminal.svg",
    Wifi => "wifi.svg",
}

pub fn icon(kind: Icon, size: f32, color: Hsla) -> Svg {
    svg()
        .path(kind.path())
        .w(px(size))
        .h(px(size))
        .text_color(color)
}
