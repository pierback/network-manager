use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, IntoElement, SharedString,
    StatefulInteractiveElement,
};

use crate::app::NetworkManagerApp;
use crate::routes::Route;
use crate::theme::LiquidGlassTokens;

pub fn window_shell(content: impl IntoElement, tokens: LiquidGlassTokens) -> Div {
    div()
        .size_full()
        .overflow_hidden()
        .rounded(px(12.0))
        .bg(tokens.colors.background)
        .text_color(tokens.colors.text)
        .child(content)
}

pub fn app_body(sidebar: impl IntoElement, content: impl IntoElement) -> Div {
    div()
        .size_full()
        .flex()
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
        .w(px(220.0))
        .h_full()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .pt(px(60.0))
        .pr(px(12.0))
        .pb(px(16.0))
        .pl(px(12.0))
        .bg(tokens.colors.sidebar)
        .child(
            div()
                .h(px(36.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(18.0))
                        .h(px(18.0))
                        .rounded(px(5.0))
                        .bg(tokens.colors.accent),
                )
                .child(
                    div()
                        .font_family("Inter")
                        .text_size(px(14.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens.colors.text)
                        .child("Network Manager"),
                ),
        )
        .child(div().h(px(20.0)))
        .child(section_label("MAIN", tokens))
        .child(nav_item(Route::Dashboard, active, None, tokens, cx))
        .child(nav_item(Route::Discovery, active, Some("14"), tokens, cx))
        .child(div().h(px(12.0)))
        .child(section_label("System", tokens))
        .child(nav_item(Route::Settings, active, None, tokens, cx))
}

fn section_label(label: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .px(px(8.0))
        .py(px(8.0))
        .font_family("Inter")
        .text_size(px(10.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens.colors.text_muted)
        .child(label.to_ascii_uppercase())
}

fn nav_item(
    route: Route,
    active: Route,
    count: Option<&'static str>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    let selected = route == active;
    div()
        .id(SharedString::from(format!("route-{}", route.label())))
        .w(px(200.0))
        .h(px(36.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(12.0))
        .rounded(px(6.0))
        .bg(if selected {
            tokens.colors.selected
        } else {
            tokens.colors.sidebar
        })
        .hover(|style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .child(
            div()
                .w(px(16.0))
                .text_size(px(13.0))
                .text_color(if selected {
                    tokens.colors.accent
                } else {
                    tokens.colors.text_secondary
                })
                .child(route.symbol().to_string()),
        )
        .child(
            div()
                .flex_1()
                .font_family("Inter")
                .text_size(px(13.0))
                .font_weight(FontWeight::MEDIUM)
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
                    .text_size(px(11.0))
                    .text_color(tokens.colors.text_muted)
                    .child(count),
            )
        })
        .on_click(cx.listener(move |app, _, _, cx| app.set_route(route, cx)))
}

pub fn content_frame(content: impl IntoElement) -> impl IntoElement {
    div().flex_1().h_full().overflow_hidden().child(content)
}
