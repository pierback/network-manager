use gpui::{div, prelude::*, px, Div};

use crate::theme::LiquidGlassTokens;

pub fn material(tokens: LiquidGlassTokens) -> Div {
    div()
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .rounded_xl()
        .shadow_lg()
}

pub fn stronger_material(tokens: LiquidGlassTokens) -> Div {
    material(tokens).bg(tokens.colors.panel_strong)
}

pub fn hairline(tokens: LiquidGlassTokens) -> Div {
    div().h(px(1.0)).w_full().bg(tokens.colors.edge_soft)
}
