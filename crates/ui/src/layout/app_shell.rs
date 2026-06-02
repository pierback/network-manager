use gpui::{div, prelude::*, Context, Div, FontWeight, IntoElement, SharedString};

use crate::app::NetworkManagerApp;
use crate::routes::Route;
use crate::theme::LiquidGlassTokens;

pub fn window_shell(content: impl IntoElement, tokens: LiquidGlassTokens) -> Div {
    div()
        .size_full()
        .bg(tokens.colors.background)
        .text_color(tokens.colors.text)
        .font_family("SF Pro")
        .child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(titlebar(tokens))
                .child(content),
        )
}

pub fn titlebar(tokens: LiquidGlassTokens) -> Div {
    div()
        .h(gpui::px(52.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_5()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .child(
            div()
                .flex()
                .gap_2()
                .items_center()
                .child(window_control(0xff5f57ff))
                .child(window_control(0xffbd2eff))
                .child(window_control(0x28c840ff)),
        )
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text_secondary)
                .child("Network Manager"),
        )
        .child(div().w(gpui::px(64.0)))
}

fn window_control(color: u32) -> Div {
    div().w_3().h_3().rounded_full().bg(gpui::rgba(color))
}

pub fn app_body(sidebar: impl IntoElement, content: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_1()
        .overflow_hidden()
        .child(sidebar)
        .child(content)
}

pub fn sidebar(
    active: Route,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .w(gpui::px(232.0))
        .h_full()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .bg(tokens.colors.sidebar)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text_muted)
                .child("Views"),
        )
        .child(nav_item(Route::Dashboard, active, tokens, cx))
        .child(nav_item(Route::Discovery, active, tokens, cx))
        .child(nav_item(Route::DeviceDetail, active, tokens, cx))
        .child(nav_item(Route::QuickAccess, active, tokens, cx))
        .child(nav_item(Route::Settings, active, tokens, cx))
}

fn nav_item(
    route: Route,
    active: Route,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    let selected = route == active;
    div()
        .id(SharedString::from(format!("route-{}", route.label())))
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .py_2()
        .rounded_lg()
        .bg(if selected {
            tokens.colors.selected
        } else {
            tokens.colors.panel
        })
        .border_1()
        .border_color(if selected {
            tokens.colors.edge
        } else {
            tokens.colors.edge_soft
        })
        .hover(|style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .child(
            div()
                .w_4()
                .text_color(tokens.colors.text_secondary)
                .child(route.symbol().to_string()),
        )
        .child(
            div()
                .text_sm()
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

pub fn content_frame(content: impl IntoElement) -> Div {
    div().flex_1().p_5().overflow_hidden().child(content)
}
