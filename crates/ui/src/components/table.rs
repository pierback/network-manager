use gpui::{div, prelude::*, px, Div, FontWeight};

use crate::theme::LiquidGlassTokens;

pub fn metric_tile(label: &str, value: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .p(px(10.0))
        .rounded(px(6.0))
        .bg(tokens.colors.panel)
        .child(
            div()
                .font_family("Inter")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_muted)
                .child(label.to_string()),
        )
        .child(
            div()
                .font_family("Inter")
                .text_size(px(18.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(value.to_string()),
        )
}

pub fn header_cell(label: &str, width: f32, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(width))
        .font_family("Inter")
        .text_size(px(11.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens.colors.text_muted)
        .child(label.to_string())
}

pub fn source_pill(label: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .bg(tokens.colors.panel_strong)
        .font_family("Inter")
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(tokens.colors.text_secondary)
        .child(label.to_string())
}
