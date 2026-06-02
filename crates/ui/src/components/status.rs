use gpui::{div, prelude::*, Div, FontWeight};
use network_manager_core::AvailabilityState;

use crate::theme::LiquidGlassTokens;

pub fn status_dot(state: AvailabilityState, tokens: LiquidGlassTokens) -> Div {
    div()
        .w_2()
        .h_2()
        .rounded_full()
        .bg(tokens.status_color(state))
}

pub fn status_label(state: AvailabilityState, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(status_dot(state, tokens))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(tokens.colors.text_secondary)
                .child(state.as_str().to_string()),
        )
}

pub fn status_pill(label: &str, state: AvailabilityState, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .child(status_dot(state, tokens))
        .child(
            div()
                .text_xs()
                .text_color(tokens.colors.text_secondary)
                .child(label.to_string()),
        )
}
