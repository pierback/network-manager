use gpui::{
    div, prelude::*, px, AnyElement, Context, Div, FontWeight, InteractiveElement, IntoElement,
    SharedString, StatefulInteractiveElement,
};

use crate::app::NetworkManagerApp;
use crate::components::icons::{self, Icon};
use crate::routes::Route;
use crate::theme::LiquidGlassTokens;

#[derive(Clone, Copy)]
pub enum TitlebarAction {
    ShowDashboard,
    ShowDiscovery,
    ShowSettings,
    Refresh,
    CopySshCommand,
    CopyTarget,
}

impl TitlebarAction {
    fn icon(self) -> Icon {
        match self {
            Self::ShowDashboard => Icon::Dashboard,
            Self::ShowDiscovery => Icon::Radar,
            Self::ShowSettings => Icon::Settings,
            Self::Refresh => Icon::Refresh,
            Self::CopySshCommand => Icon::Terminal,
            Self::CopyTarget => Icon::Copy,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::ShowDashboard => "titlebar-dashboard",
            Self::ShowDiscovery => "titlebar-discovery",
            Self::ShowSettings => "titlebar-settings",
            Self::Refresh => "titlebar-refresh",
            Self::CopySshCommand => "titlebar-terminal",
            Self::CopyTarget => "titlebar-copy",
        }
    }
}

pub fn window_shell(content: impl IntoElement, tokens: LiquidGlassTokens) -> Div {
    div()
        .size_full()
        .relative()
        .rounded(px(22.0))
        .bg(tokens.colors.background)
        .text_color(tokens.colors.text)
        .child(div().relative().w(px(1280.0)).h(px(800.0)).child(content))
}

pub fn route_shell(
    active: Route,
    title_icon: Icon,
    title: &str,
    actions: &[TitlebarAction],
    content: impl IntoElement,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .size_full()
        .relative()
        .child(liquid_titlebar(title_icon, title, actions, tokens, cx))
        .child(glass_sidebar(active, tokens, cx))
        .child(content)
}

pub fn liquid_titlebar(
    icon: Icon,
    title: &str,
    actions: &[TitlebarAction],
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .w(px(1280.0))
        .h(px(56.0))
        .px(px(20.0))
        .flex()
        .items_center()
        .justify_between()
        .bg(gpui::rgba(0xffffff0d))
        .border_b_1()
        .border_color(tokens.colors.edge_soft)
        .child(window_controls())
        .child(
            div()
                .absolute()
                .left(px(0.0))
                .right(px(0.0))
                .top(px(0.0))
                .h(px(56.0))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .child(icons::icon(icon, 16.0, tokens.colors.text_secondary))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child(title.to_string()),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .children(actions.iter().map(|action| {
                    let action = *action;
                    titlebar_action(action, tokens, cx)
                })),
        )
}

fn window_controls() -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(window_control(
            0xff5f57ff,
            "window-close",
            WindowControl::Close,
        ))
        .child(window_control(
            0xfebc2eff,
            "window-minimize",
            WindowControl::Minimize,
        ))
        .child(window_control(
            0x28c840ff,
            "window-zoom",
            WindowControl::Zoom,
        ))
}

#[derive(Clone, Copy)]
enum WindowControl {
    Close,
    Minimize,
    Zoom,
}

fn window_control(color: u32, id: &'static str, action: WindowControl) -> AnyElement {
    let base = div()
        .id(SharedString::from(id))
        .w(px(12.0))
        .h(px(12.0))
        .rounded_full()
        .bg(gpui::rgba(color))
        .cursor_pointer();
    match action {
        WindowControl::Close => base
            .on_click(|_, window, _| window.remove_window())
            .into_any_element(),
        WindowControl::Minimize => base
            .on_click(|_, window, _| window.minimize_window())
            .into_any_element(),
        WindowControl::Zoom => base
            .on_click(|_, window, _| window.zoom_window())
            .into_any_element(),
    }
}

fn titlebar_action(
    action: TitlebarAction,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> AnyElement {
    let base = div()
        .w(px(30.0))
        .h(px(30.0))
        .rounded(px(10.0))
        .bg(gpui::rgba(0xffffff0a))
        .flex()
        .items_center()
        .justify_center()
        .hover(|style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .id(SharedString::from(action.id()))
        .child(icons::icon(
            action.icon(),
            15.0,
            tokens.colors.text_secondary,
        ));

    match action {
        TitlebarAction::ShowDashboard => base
            .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Dashboard, cx)))
            .into_any_element(),
        TitlebarAction::ShowDiscovery => base
            .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Discovery, cx)))
            .into_any_element(),
        TitlebarAction::ShowSettings => base
            .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Settings, cx)))
            .into_any_element(),
        TitlebarAction::Refresh => base
            .on_click(cx.listener(|app, _, _, cx| app.refresh_quick(cx)))
            .into_any_element(),
        TitlebarAction::CopySshCommand => base
            .on_click(cx.listener(|app, _, _, cx| app.copy_selected_ssh_command(cx)))
            .into_any_element(),
        TitlebarAction::CopyTarget => base
            .on_click(cx.listener(|app, _, _, cx| app.copy_selected_target(cx)))
            .into_any_element(),
    }
}

pub fn glass_sidebar(
    active: Route,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .absolute()
        .left(px(16.0))
        .top(px(72.0))
        .w(px(232.0))
        .h(px(704.0))
        .rounded(px(20.0))
        .bg(tokens.colors.sidebar)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(workspace_header(tokens))
        .child(div().h(px(18.0)))
        .child(nav_item(
            Route::Dashboard,
            active,
            Icon::Dashboard,
            tokens,
            cx,
        ))
        .child(nav_item(Route::Discovery, active, Icon::Radar, tokens, cx))
        .child(nav_item(
            Route::DeviceDetail,
            active,
            Icon::PanelRight,
            tokens,
            cx,
        ))
        .child(nav_item(
            Route::Settings,
            active,
            Icon::Settings,
            tokens,
            cx,
        ))
        .child(div().flex_1())
}

fn workspace_header(tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(
            div()
                .w(px(30.0))
                .h(px(30.0))
                .rounded(px(10.0))
                .bg(gpui::rgba(0xffffff18))
                .flex()
                .items_center()
                .justify_center()
                .font_family("Geist")
                .text_size(px(14.0))
                .font_weight(FontWeight::BOLD)
                .text_color(tokens.colors.text)
                .child("N"),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child("Home Network"),
                )
                .child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(10.0))
                        .text_color(tokens.colors.text_muted)
                        .child("local observations"),
                ),
        )
}

fn nav_item(
    route: Route,
    active: Route,
    icon: Icon,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    let selected = route == active;
    div()
        .id(SharedString::from(format!("route-{}", route.label())))
        .h(px(36.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(10.0))
        .py(px(8.0))
        .bg(gpui::rgba(0xffffff00))
        .cursor_pointer()
        .child(div().w(px(3.0)).h(px(20.0)).rounded_full().bg(if selected {
            tokens.colors.icy
        } else {
            gpui::rgba(0xffffff00).into()
        }))
        .child(icons::icon(
            icon,
            16.0,
            if selected {
                tokens.colors.text
            } else {
                tokens.colors.text_muted
            },
        ))
        .child(
            div()
                .flex_1()
                .font_family("Geist")
                .text_size(px(13.0))
                .font_weight(if selected {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(if selected {
                    tokens.colors.text
                } else {
                    tokens.colors.text_secondary
                })
                .child(route.label().to_string()),
        )
        .on_click(cx.listener(move |app, _, _, cx| app.set_route(route, cx)))
}
