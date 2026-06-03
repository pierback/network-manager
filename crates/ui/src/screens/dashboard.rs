use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, InteractiveElement, SharedString,
    StatefulInteractiveElement,
};
use network_manager_core::AvailabilityState;

use crate::app::{ActionStatus, NetworkManagerApp};
use crate::components::{buttons, glass, status, table as table_components};
use crate::data::{DashboardVm, TrackedDeviceRowVm};
use crate::layout::table::DASHBOARD_COLUMNS;
use crate::routes::Route;
use crate::theme::LiquidGlassTokens;

pub fn screen(
    vm: &DashboardVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(px(24.0))
        .pt(px(60.0))
        .pr(px(32.0))
        .pb(px(32.0))
        .pl(px(32.0))
        .child(toolbar(vm, action_status, tokens, cx))
        .when(vm.daemon.stale, |this| {
            this.child(glass::system_note(
                "Daemon state is stale",
                &format!("Showing {} data from the local store.", vm.daemon.source),
                tokens,
            ))
        })
        .when_some(action_status, |this, status| {
            this.child(action_note(status, tokens))
        })
        .child(devices_table(&vm.tracked, tokens, cx))
}

fn toolbar(
    vm: &DashboardVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let refresh_label = if action_status.is_some_and(|status| status.is_pending) {
        "Refreshing"
    } else {
        "Refresh"
    };
    div()
        .w_full()
        .flex()
        .items_start()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .font_family("Inter")
                        .text_size(px(22.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens.colors.text)
                        .child("Dashboard"),
                )
                .child(
                    div()
                        .font_family("Inter")
                        .text_size(px(13.0))
                        .text_color(tokens.colors.text_secondary)
                        .child(format!("{} tracked devices", vm.tracked.len())),
                ),
        )
        .child(
            buttons::toolbar_button(refresh_label, tokens)
                .id(SharedString::from("dashboard-refresh"))
                .on_click(cx.listener(|app, _, _, cx| app.refresh_quick(cx))),
        )
}

fn action_note(status: &ActionStatus, tokens: LiquidGlassTokens) -> Div {
    let title = if status.is_pending {
        "Action running"
    } else if status.is_error {
        "Action failed"
    } else {
        "Action complete"
    };
    glass::system_note(title, &status.message, tokens)
}

fn devices_table(
    rows: &[TrackedDeviceRowVm],
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(table_header(tokens))
        .when(rows.is_empty(), |this| this.child(empty_state(tokens, cx)))
        .children(rows.iter().map(|row| device_row(row, tokens, cx)))
}

fn table_header(tokens: LiquidGlassTokens) -> Div {
    div()
        .h(px(32.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(16.0))
        .border_b_1()
        .border_color(tokens.colors.edge_soft)
        .children(
            DASHBOARD_COLUMNS
                .into_iter()
                .map(|(label, width)| table_components::header_cell(label, width, tokens)),
        )
}

fn empty_state(tokens: LiquidGlassTokens, cx: &mut Context<NetworkManagerApp>) -> Div {
    div()
        .h(px(56.0))
        .flex()
        .items_center()
        .justify_between()
        .rounded(px(8.0))
        .bg(tokens.colors.panel)
        .px(px(16.0))
        .child(
            div()
                .font_family("Inter")
                .text_size(px(13.0))
                .text_color(tokens.colors.text_secondary)
                .child("No tracked devices yet"),
        )
        .child(
            buttons::action_button("Open Discovery", tokens)
                .id(SharedString::from("dashboard-empty-discovery"))
                .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Discovery, cx))),
        )
}

fn device_row(
    row: &TrackedDeviceRowVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .h(px(56.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(16.0))
        .rounded(px(8.0))
        .bg(tokens.colors.panel)
        .child(
            div()
                .w(px(DASHBOARD_COLUMNS[0].1))
                .flex()
                .justify_center()
                .child(status::status_dot(row.overall, tokens)),
        )
        .child(identity_cell(row, tokens))
        .child(text_cell(&row.category, DASHBOARD_COLUMNS[2].1, tokens))
        .child(status_cell(row.lan, DASHBOARD_COLUMNS[3].1, tokens))
        .child(status_cell(row.tailscale, DASHBOARD_COLUMNS[4].1, tokens))
        .child(ssh_cell(row.ssh, DASHBOARD_COLUMNS[5].1, tokens))
        .child(target_cell(row, tokens))
        .child(
            div()
                .w(px(DASHBOARD_COLUMNS[7].1))
                .font_family("Inter")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_muted)
                .child(row.last_seen.clone()),
        )
        .child(actions_cell(row, tokens, cx))
}

fn identity_cell(row: &TrackedDeviceRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(DASHBOARD_COLUMNS[1].1))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .font_family("Inter")
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(row.label.clone()),
        )
        .child(
            div()
                .font_family("Geist Mono")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_muted)
                .child(row.alias.clone()),
        )
}

fn status_cell(state: AvailabilityState, width: f32, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(width))
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(status::status_mini_dot(state, tokens))
        .child(
            div()
                .font_family("Inter")
                .text_size(px(12.0))
                .text_color(tokens.colors.text_secondary)
                .child(status::status_text(state)),
        )
}

fn ssh_cell(state: AvailabilityState, width: f32, tokens: LiquidGlassTokens) -> Div {
    let (label, color) = match state {
        AvailabilityState::Online => ("Ready", tokens.colors.ssh_capable),
        AvailabilityState::Offline => ("N/A", tokens.colors.text_muted),
        AvailabilityState::Unknown => ("Unknown", tokens.colors.text_muted),
    };
    div()
        .w(px(width))
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(status::status_mini_dot(state, tokens))
        .child(
            div()
                .font_family("Inter")
                .text_size(px(12.0))
                .text_color(color)
                .child(label),
        )
}

fn target_cell(row: &TrackedDeviceRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(DASHBOARD_COLUMNS[6].1))
        .font_family("Geist Mono")
        .text_size(px(11.0))
        .text_color(if row.ssh == AvailabilityState::Online {
            tokens.colors.accent
        } else {
            tokens.colors.text_muted
        })
        .child(row.preferred_target.clone())
}

fn text_cell(text: &str, width: f32, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(width))
        .font_family("Inter")
        .text_size(px(12.0))
        .text_color(tokens.colors.text_secondary)
        .child(text.to_string())
}

fn actions_cell(
    row: &TrackedDeviceRowVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let identity_id = row.id.clone();
    div()
        .w(px(DASHBOARD_COLUMNS[8].1))
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(buttons::icon_button("⌘", tokens))
        .child(buttons::icon_button("⧉", tokens))
        .child(
            buttons::icon_button("i", tokens)
                .id(SharedString::from(format!("dashboard-details-{}", row.id)))
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.select_device_detail(identity_id.clone(), cx)
                })),
        )
}
