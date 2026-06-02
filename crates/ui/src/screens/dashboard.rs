use gpui::{div, prelude::*, Div, FontWeight};

use crate::components::{buttons, glass, status, table as table_components};
use crate::data::{DashboardVm, TrackedDeviceRowVm};
use crate::layout::table::DASHBOARD_COLUMNS;
use crate::theme::LiquidGlassTokens;

pub fn screen(vm: &DashboardVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap_4()
        .child(toolbar(vm, tokens))
        .child(summary(vm, tokens))
        .child(devices_table(&vm.tracked, tokens))
}

fn toolbar(vm: &DashboardVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_start()
        .justify_between()
        .child(glass::header(
            "Tracked Devices",
            "Local-first SSH targets with Tailscale fallback.",
            tokens,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(status::status_pill("Daemon", vm.daemon.state, tokens))
                .child(status::status_pill(
                    "Tailscale",
                    vm.daemon.tailscale_service,
                    tokens,
                ))
                .child(buttons::toolbar_button("Refresh", tokens)),
        )
}

fn summary(vm: &DashboardVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .grid()
        .grid_cols(4)
        .gap_3()
        .child(table_components::metric_tile(
            "Tracked",
            &vm.tracked.len().to_string(),
            tokens,
        ))
        .child(table_components::metric_tile(
            "Online",
            &vm.online_count.to_string(),
            tokens,
        ))
        .child(table_components::metric_tile(
            "Tailscale visible",
            &vm.tailscale_count.to_string(),
            tokens,
        ))
        .child(table_components::metric_tile(
            "Last scan",
            &vm.daemon.last_scan,
            tokens,
        ))
}

fn devices_table(rows: &[TrackedDeviceRowVm], tokens: LiquidGlassTokens) -> Div {
    glass::panel(tokens)
        .p_3()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div().flex().items_center().gap_3().px_2().h_8().children(
                DASHBOARD_COLUMNS
                    .into_iter()
                    .map(|(label, width)| table_components::header_cell(label, width, tokens)),
            ),
        )
        .children(rows.iter().map(|row| device_row(row, tokens)))
}

fn device_row(row: &TrackedDeviceRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .px_2()
        .h(gpui::px(62.0))
        .rounded_lg()
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .child(identity_cell(row, tokens))
        .child(status_cell(row.lan, DASHBOARD_COLUMNS[1].1, tokens))
        .child(status_cell(row.tailscale, DASHBOARD_COLUMNS[2].1, tokens))
        .child(target_cell(row, tokens))
        .child(
            div()
                .w(gpui::px(DASHBOARD_COLUMNS[4].1))
                .text_xs()
                .text_color(tokens.colors.text_muted)
                .child(row.last_seen.clone()),
        )
        .child(
            div()
                .w(gpui::px(DASHBOARD_COLUMNS[5].1))
                .child(buttons::action_button(
                    if row.ssh == network_manager_core::AvailabilityState::Online {
                        "SSH"
                    } else {
                        "Details"
                    },
                    tokens,
                )),
        )
}

fn identity_cell(row: &TrackedDeviceRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(gpui::px(DASHBOARD_COLUMNS[0].1))
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
                .flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .font_family("SF Mono")
                        .text_xs()
                        .text_color(tokens.colors.text_muted)
                        .child(row.alias.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(tokens.colors.text_muted)
                        .child(format!("· {}", row.category)),
                ),
        )
}

fn status_cell(
    state: network_manager_core::AvailabilityState,
    width: f32,
    tokens: LiquidGlassTokens,
) -> Div {
    div()
        .w(gpui::px(width))
        .child(status::status_label(state, tokens))
}

fn target_cell(row: &TrackedDeviceRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(gpui::px(DASHBOARD_COLUMNS[3].1))
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .font_family("SF Mono")
                .text_xs()
                .text_color(tokens.colors.text_secondary)
                .child(row.preferred_target.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(tokens.colors.text_muted)
                .child(row.target_reason.clone()),
        )
}
