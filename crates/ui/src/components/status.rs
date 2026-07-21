use gpui::{div, prelude::*, px, Div, FontWeight};
use network_manager_core::AvailabilityState;

use crate::components::icons::{self, Icon};
use crate::data::ActionStatus;
use crate::theme::LiquidGlassTokens;

pub fn action_banner(status: &ActionStatus, tokens: LiquidGlassTokens) -> Div {
    let (icon, color) = if status.is_error() {
        (Icon::Info, tokens.colors.offline)
    } else if status.is_pending() {
        (Icon::Refresh, tokens.colors.unknown)
    } else {
        (Icon::Check, tokens.colors.online)
    };

    div()
        .min_h(px(42.0))
        .rounded(px(14.0))
        .bg(gpui::rgba(0xffffff0c))
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .px(px(14.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(icons::icon(icon, 15.0, color))
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(tokens.colors.text_secondary)
                        .truncate()
                        .child(status.message.clone()),
                )
                .when(status.detail.is_some(), |this| {
                    this.child(
                        div()
                            .font_family("Geist")
                            .text_size(px(11.0))
                            .text_color(tokens.colors.text_muted)
                            .truncate()
                            .child("Open Logs for daemon diagnostics."),
                    )
                }),
        )
}

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

pub fn status_text(state: AvailabilityState) -> &'static str {
    match state {
        AvailabilityState::Online => "Online",
        AvailabilityState::Offline => "Offline",
        AvailabilityState::Unknown => "Unknown",
    }
}
