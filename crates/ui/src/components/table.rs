use gpui::{div, prelude::*, Div, FontWeight};

use crate::theme::LiquidGlassTokens;

pub fn metric_tile(label: &str, value: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .child(
            div()
                .text_xs()
                .text_color(tokens.colors.text_muted)
                .child(label.to_string()),
        )
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(value.to_string()),
        )
}

pub fn header_cell(label: &str, width: f32, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(gpui::px(width))
        .text_xs()
        .font_family("SF Mono")
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens.colors.text_muted)
        .child(label.to_string())
}

pub fn source_pill(label: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .px_2()
        .py_1()
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .text_xs()
        .text_color(tokens.colors.text_muted)
        .child(label.to_string())
}
