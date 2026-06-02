use gpui::{div, prelude::*, Div, FontWeight};

use crate::components::{buttons, status};
use crate::data::{QuickAccessVm, TrackedDeviceRowVm};
use crate::layout::popover::{POPOVER_RADIUS, POPOVER_ROW_HEIGHT, POPOVER_WIDTH};
use crate::theme::LiquidGlassTokens;

pub fn screen(vm: &QuickAccessVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(popover(vm, tokens))
}

fn popover(vm: &QuickAccessVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .relative()
        .w(gpui::px(POPOVER_WIDTH))
        .rounded(gpui::px(POPOVER_RADIUS))
        .border_1()
        .border_color(tokens.colors.edge)
        .bg(tokens.colors.panel_strong)
        .shadow_lg()
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .absolute()
                .top(gpui::px(-8.0))
                .right(gpui::px(44.0))
                .w_4()
                .h_4()
                .rounded_sm()
                .bg(tokens.colors.panel_strong)
                .border_1()
                .border_color(tokens.colors.edge_soft),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child("Network Manager"),
                )
                .child(buttons::icon_button("↻", tokens)),
        )
        .children(vm.rows.iter().map(|row| popover_row(row, tokens)))
        .child(
            div()
                .pt_2()
                .flex()
                .items_center()
                .justify_between()
                .text_xs()
                .text_color(tokens.colors.text_muted)
                .child(format!("Last scan {}", vm.last_scan))
                .child("Open Dashboard"),
        )
}

fn popover_row(row: &TrackedDeviceRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .h(gpui::px(POPOVER_ROW_HEIGHT))
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(status::status_dot(row.overall, tokens))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(tokens.colors.text)
                                .child(row.label.clone()),
                        )
                        .child(
                            div()
                                .font_family("SF Mono")
                                .text_xs()
                                .text_color(tokens.colors.text_muted)
                                .child(row.preferred_target.clone()),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .gap_1()
                .child(buttons::icon_button("⌘", tokens))
                .child(buttons::icon_button("⧉", tokens)),
        )
}
