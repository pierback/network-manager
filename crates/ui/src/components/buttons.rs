use gpui::{div, prelude::*, px, Div, FontWeight};

use crate::theme::LiquidGlassTokens;

pub fn toolbar_button(label: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .bg(tokens.colors.panel_strong)
        .hover(|style| style.bg(tokens.colors.accent_hover))
        .cursor_pointer()
        .font_family("Inter")
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(tokens.colors.text_secondary)
        .child(label.to_string())
}

pub fn icon_button(symbol: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(32.0))
        .h(px(32.0))
        .rounded(px(6.0))
        .bg(tokens.colors.panel_strong)
        .hover(|style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .font_family("Inter")
        .text_size(px(16.0))
        .text_color(tokens.colors.text_secondary)
        .child(symbol.to_string())
}

pub fn small_icon_button(symbol: &str, accent: bool, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(26.0))
        .h(px(26.0))
        .rounded(px(5.0))
        .bg(tokens.colors.panel_strong)
        .hover(|style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .font_family("Inter")
        .text_size(px(13.0))
        .text_color(if accent {
            tokens.colors.accent
        } else {
            tokens.colors.text_secondary
        })
        .child(symbol.to_string())
}

pub fn action_button(label: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(4.0))
        .bg(tokens.colors.panel_strong)
        .hover(|style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .font_family("Inter")
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(tokens.colors.text_muted)
        .child(label.to_string())
}

pub fn accent_button(label: &str, tokens: LiquidGlassTokens) -> Div {
    action_button(label, tokens)
        .bg(tokens.colors.accent)
        .text_color(tokens.colors.text_inverse)
        .hover(|style| style.bg(tokens.colors.accent_hover))
}

pub fn disabled_button(label: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(4.0))
        .bg(tokens.colors.panel)
        .font_family("Inter")
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(tokens.colors.text_muted)
        .child(label.to_string())
}
