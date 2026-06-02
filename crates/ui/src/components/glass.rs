use gpui::{div, prelude::*, px, rgba, Div, FontWeight};

use crate::theme::{spacing, LiquidGlassTokens};

pub fn panel(tokens: LiquidGlassTokens) -> Div {
    div()
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .rounded_xl()
        .shadow_lg()
}

pub fn panel_strong(tokens: LiquidGlassTokens) -> Div {
    panel(tokens).bg(tokens.colors.panel_strong)
}

pub fn slab(tokens: LiquidGlassTokens) -> Div {
    div()
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .rounded_lg()
}

pub fn header(title: &str, subtitle: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(title.to_string()),
        )
        .child(
            div()
                .text_sm()
                .text_color(tokens.colors.text_muted)
                .child(subtitle.to_string()),
        )
}

pub fn system_note(title: &str, body: &str, tokens: LiquidGlassTokens) -> Div {
    panel(tokens)
        .p_3()
        .gap_1()
        .flex()
        .flex_col()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(title.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(tokens.colors.text_secondary)
                .child(body.to_string()),
        )
}

pub fn specular_edge() -> Div {
    div().h(px(1.0)).w_full().bg(rgba(0xffffff35))
}

pub fn spacer() -> Div {
    div().h(spacing(1.0))
}
