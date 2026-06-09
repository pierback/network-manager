use gpui::{div, prelude::*, Div, FontWeight};

use crate::components::icons::{self, Icon};
use crate::theme::LiquidGlassTokens;

pub fn search_field(placeholder: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .h_8()
        .px_3()
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .child(icons::icon(Icon::Search, 14.0, tokens.colors.text_muted))
        .child(
            div()
                .text_xs()
                .text_color(tokens.colors.text_muted)
                .child(placeholder.to_string()),
        )
}

pub fn filter_chip(label: &str, selected: bool, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .px_3()
        .py_1()
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(if selected {
            tokens.colors.selected
        } else {
            tokens.colors.panel
        })
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(if selected {
            tokens.colors.text
        } else {
            tokens.colors.text_secondary
        })
        .child(label.to_string())
}

pub fn setting_row(label: &str, description: &str, value: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .py_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(tokens.colors.text)
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(tokens.colors.text_muted)
                        .child(description.to_string()),
                ),
        )
        .child(
            div()
                .flex_none()
                .px_3()
                .py_1()
                .rounded_lg()
                .border_1()
                .border_color(tokens.colors.edge_soft)
                .bg(tokens.colors.panel)
                .text_xs()
                .text_color(tokens.colors.text_secondary)
                .child(value.to_string()),
        )
}

pub fn toggle(on: bool, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w_8()
        .h_4()
        .rounded_full()
        .bg(if on {
            tokens.colors.selected
        } else {
            tokens.colors.panel
        })
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .child(div().w_3().h_3().rounded_full().bg(if on {
            tokens.colors.text
        } else {
            tokens.colors.text_muted
        }))
}
