use gpui::{prelude::*, px, svg, Hsla, Svg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Activity,
    ArrowRight,
    ArrowUpRight,
    Bell,
    Check,
    ChevronDown,
    Copy,
    Dashboard,
    Edit,
    ExternalLink,
    Folder,
    GitBranch,
    GitMerge,
    Globe,
    Info,
    Network,
    PanelRight,
    Pencil,
    Plus,
    Radar,
    Refresh,
    RotateCcw,
    Router,
    Route,
    Search,
    Server,
    Settings,
    ShieldCheck,
    SlidersHorizontal,
    Terminal,
    Wifi,
}

impl Icon {
    pub fn path(self) -> &'static str {
        match self {
            Self::Activity => "icons/activity.svg",
            Self::ArrowRight => "icons/arrow-right.svg",
            Self::ArrowUpRight => "icons/arrow-up-right.svg",
            Self::Bell => "icons/bell.svg",
            Self::Check => "icons/check.svg",
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::Copy => "icons/copy.svg",
            Self::Dashboard => "icons/layout-dashboard.svg",
            Self::Edit => "icons/edit-3.svg",
            Self::ExternalLink => "icons/external-link.svg",
            Self::Folder => "icons/folder.svg",
            Self::GitBranch => "icons/git-branch.svg",
            Self::GitMerge => "icons/git-merge.svg",
            Self::Globe => "icons/globe.svg",
            Self::Info => "icons/info.svg",
            Self::Network => "icons/network.svg",
            Self::PanelRight => "icons/panel-right.svg",
            Self::Pencil => "icons/pencil.svg",
            Self::Plus => "icons/plus.svg",
            Self::Radar => "icons/radar.svg",
            Self::Refresh => "icons/refresh-cw.svg",
            Self::RotateCcw => "icons/rotate-ccw.svg",
            Self::Router => "icons/router.svg",
            Self::Route => "icons/route.svg",
            Self::Search => "icons/search.svg",
            Self::Server => "icons/server.svg",
            Self::Settings => "icons/settings.svg",
            Self::ShieldCheck => "icons/shield-check.svg",
            Self::SlidersHorizontal => "icons/sliders-horizontal.svg",
            Self::Terminal => "icons/terminal.svg",
            Self::Wifi => "icons/wifi.svg",
        }
    }
}

pub fn icon(kind: Icon, size: f32, color: Hsla) -> Svg {
    svg()
        .path(kind.path())
        .w(px(size))
        .h(px(size))
        .text_color(color)
}
