use gpui::{
    div, prelude::*, px, AnyElement, Context, Div, FontWeight, InteractiveElement, IntoElement,
    SharedString, StatefulInteractiveElement,
};

use crate::app::NetworkManagerApp;
use crate::components::icons::{self, Icon};
use crate::routes::Route;
use crate::theme::LiquidGlassTokens;

pub fn window_shell(content: impl IntoElement, tokens: LiquidGlassTokens) -> Div {
    div()
        .size_full()
        .relative()
        .rounded(px(22.0))
        .bg(tokens.colors.background)
        .text_color(tokens.colors.text)
        .child(
            div()
                .relative()
                .w(px(1280.0))
                .h(px(800.0))
                .child(v4_refraction_layers())
                .child(content),
        )
}

fn v4_refraction_layers() -> Div {
    // Keep the V4 liquid-glass material, but remove the oversized translucent
    // refraction blobs. They read like giant tooltips/hover overlays in the
    // fixed artboards and make navigation/content look obscured.
    div().absolute().inset_0()
}

#[allow(dead_code)]
pub fn app_body(_sidebar: impl IntoElement, content: impl IntoElement) -> Div {
    div().size_full().relative().child(content)
}

#[allow(clippy::too_many_arguments)]
pub fn v4_route_shell(
    active: Route,
    title_icon: Icon,
    title: &str,
    action_icons: &[Icon],
    include_sidebar: bool,
    content: impl IntoElement,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .size_full()
        .relative()
        .child(liquid_titlebar(title_icon, title, action_icons, tokens, cx))
        .when(include_sidebar, |this| {
            this.child(glass_sidebar(active, tokens, cx))
        })
        .child(content)
}

pub fn liquid_titlebar(
    icon: Icon,
    title: &str,
    action_icons: &[Icon],
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
                .children(action_icons.iter().map(|icon| {
                    let icon = *icon;
                    titlebar_action(icon, tokens, cx)
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
    icon: Icon,
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
        .child(icons::icon(icon, 15.0, tokens.colors.text_secondary));

    match icon {
        Icon::Dashboard => base
            .hover(|style| style.bg(tokens.colors.selected))
            .cursor_pointer()
            .id(SharedString::from("titlebar-dashboard"))
            .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Dashboard, cx)))
            .into_any_element(),
        Icon::Search => base
            .hover(|style| style.bg(tokens.colors.selected))
            .cursor_pointer()
            .id(SharedString::from("titlebar-search"))
            .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Discovery, cx)))
            .into_any_element(),
        Icon::Settings => base
            .hover(|style| style.bg(tokens.colors.selected))
            .cursor_pointer()
            .id(SharedString::from("titlebar-settings"))
            .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Settings, cx)))
            .into_any_element(),
        Icon::SlidersHorizontal => base
            .hover(|style| style.bg(tokens.colors.selected))
            .cursor_pointer()
            .id(SharedString::from("titlebar-sliders"))
            .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Settings, cx)))
            .into_any_element(),
        Icon::Refresh | Icon::RotateCcw => base
            .hover(|style| style.bg(tokens.colors.selected))
            .cursor_pointer()
            .id(SharedString::from("titlebar-refresh"))
            .on_click(cx.listener(|app, _, _, cx| app.refresh_quick(cx)))
            .into_any_element(),
        Icon::Bell => base
            .hover(|style| style.bg(tokens.colors.selected))
            .cursor_pointer()
            .id(SharedString::from("titlebar-shortcuts"))
            .on_click(cx.listener(|app, _, _, cx| app.show_keyboard_shortcuts(cx)))
            .into_any_element(),
        Icon::Terminal => base
            .hover(|style| style.bg(tokens.colors.selected))
            .cursor_pointer()
            .id(SharedString::from("titlebar-terminal"))
            .on_click(cx.listener(|app, _, _, cx| app.copy_selected_ssh_command(cx)))
            .into_any_element(),
        Icon::Copy => base
            .hover(|style| style.bg(tokens.colors.selected))
            .cursor_pointer()
            .id(SharedString::from("titlebar-copy"))
            .on_click(cx.listener(|app, _, _, cx| app.copy_selected_target(cx)))
            .into_any_element(),
        _ => base.into_any_element(),
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
            None,
            tokens,
            cx,
        ))
        .child(nav_item(
            Route::Discovery,
            active,
            Icon::Radar,
            None,
            tokens,
            cx,
        ))
        .child(nav_item(
            Route::DeviceDetail,
            active,
            Icon::PanelRight,
            None,
            tokens,
            cx,
        ))
        .child(nav_item(
            Route::Settings,
            active,
            Icon::Settings,
            None,
            tokens,
            cx,
        ))
        .child(div().flex_1())
        .child(tailscale_footer(tokens))
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
    count: Option<&'static str>,
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
        .when_some(count, |this, count| {
            this.child(
                div()
                    .font_family("Geist Mono")
                    .text_size(px(10.0))
                    .text_color(tokens.colors.text_muted)
                    .child(count),
            )
        })
        .on_click(cx.listener(move |app, _, _, cx| app.set_route(route, cx)))
}

fn tailscale_footer(tokens: LiquidGlassTokens) -> Div {
    div()
        .rounded(px(18.0))
        .bg(gpui::rgba(0xffffff0d))
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .p(px(14.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_family("Geist Mono")
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .text_color(tokens.colors.text_muted)
                .child("LOCAL TAILSCALE"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(7.0))
                .child(
                    div()
                        .w(px(8.0))
                        .h(px(8.0))
                        .rounded_full()
                        .bg(tokens.colors.online),
                )
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text_secondary)
                        .child("Local only"),
                ),
        )
        .child(
            div()
                .font_family("Geist")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_muted)
                .child("Used for presence and fallback SSH routes."),
        )
}

#[allow(dead_code)]
pub fn sidebar(
    active: Route,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    glass_sidebar(active, tokens, cx)
}

#[allow(dead_code)]
pub fn content_frame(content: impl IntoElement) -> impl IntoElement {
    div().size_full().child(content)
}
