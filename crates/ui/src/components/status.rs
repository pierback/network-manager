use gpui::{div, prelude::*, px, Div, FontWeight};
use network_manager_core::AvailabilityState;

use crate::theme::LiquidGlassTokens;

pub fn status_dot(state: AvailabilityState, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(8.0))
        .h(px(8.0))
        .rounded_full()
        .bg(tokens.status_color(state))
}

pub fn status_mini_dot(state: AvailabilityState, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(6.0))
        .h(px(6.0))
        .rounded_full()
        .bg(tokens.status_color(state))
}

pub fn status_label(state: AvailabilityState, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(status_mini_dot(state, tokens))
        .child(
            div()
                .font_family("Inter")
                .text_size(px(12.0))
                .font_weight(FontWeight::NORMAL)
                .text_color(tokens.colors.text_secondary)
                .child(status_text(state)),
        )
}

pub fn status_text(state: AvailabilityState) -> &'static str {
    match state {
        AvailabilityState::Online => "Online",
        AvailabilityState::Offline => "Offline",
        AvailabilityState::Unknown => "Unknown",
    }
}

pub fn status_pill(label: &str, state: AvailabilityState, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(status_mini_dot(state, tokens))
        .child(
            div()
                .font_family("Inter")
                .text_size(px(12.0))
                .text_color(tokens.colors.text_secondary)
                .child(label.to_string()),
        )
}
