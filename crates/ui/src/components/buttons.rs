use gpui::{div, prelude::*, Div, FontWeight};

use crate::theme::LiquidGlassTokens;

pub fn toolbar_button(label: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .px_3()
        .py_1()
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .hover(|style| style.bg(tokens.colors.selected))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens.colors.text)
        .child(label.to_string())
}

pub fn icon_button(symbol: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w_8()
        .h_8()
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .hover(|style| style.bg(tokens.colors.selected))
        .text_color(tokens.colors.text_secondary)
        .child(symbol.to_string())
}

pub fn action_button(label: &str, tokens: LiquidGlassTokens) -> Div {
    toolbar_button(label, tokens).bg(tokens.colors.panel_strong)
}
