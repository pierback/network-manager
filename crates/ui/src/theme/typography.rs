use gpui::{prelude::*, Div, FontWeight};

use crate::theme::LiquidGlassTokens;

pub fn title(div: Div, tokens: LiquidGlassTokens) -> Div {
    div.text_2xl()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens.colors.text)
}

pub fn subtitle(div: Div, tokens: LiquidGlassTokens) -> Div {
    div.text_sm().text_color(tokens.colors.text_muted)
}

pub fn label(div: Div, tokens: LiquidGlassTokens) -> Div {
    div.text_sm()
        .font_weight(FontWeight::MEDIUM)
        .text_color(tokens.colors.text)
}

pub fn mono(div: Div, tokens: LiquidGlassTokens) -> Div {
    div.text_xs().text_color(tokens.colors.text_secondary)
}
