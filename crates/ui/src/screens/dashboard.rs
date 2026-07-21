use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, InteractiveElement, SharedString,
    StatefulInteractiveElement,
};
use network_manager_core::AvailabilityState;

use crate::app::NetworkManagerApp;
use crate::components::{buttons, icons::Icon, status};
use crate::data::{ActionStatus, DashboardVm, TrackedDeviceRowVm};
use crate::layout::app_shell::{route_shell, TitlebarAction};
use crate::routes::Route;
use crate::theme::LiquidGlassTokens;

const DASH_COLS: [f32; 6] = [250.0, 80.0, 92.0, 200.0, 92.0, 70.0];

pub fn screen(
    vm: &DashboardVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let main = dashboard_main(vm, action_status, tokens, cx);
    route_shell(
        Route::Dashboard,
        Icon::Network,
        "Network Manager",
        &[TitlebarAction::ShowDiscovery, TitlebarAction::ShowSettings],
        main,
        tokens,
        cx,
    )
}

fn dashboard_main(
    vm: &DashboardVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    div()
        .absolute()
        .left(px(272.0))
        .top(px(80.0))
        .w(px(976.0))
        .h(px(688.0))
        .id(SharedString::from("dashboard-main-scroll"))
        .overflow_y_scroll()
        .pr(px(6.0))
        .flex()
        .flex_col()
        .gap(px(22.0))
        .child(header(vm, action_status, tokens, cx))
        .children(action_status.map(|status| status::action_banner(status, tokens)))
        .child(metrics(vm, tokens))
        .child(device_table(&vm.tracked, tokens, cx))
}

fn header(
    vm: &DashboardVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let label = if action_status.is_some_and(ActionStatus::is_pending) {
        "Scanning"
    } else {
        "Quick scan"
    };
    div()
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
                        .font_family("Geist")
                        .text_size(px(30.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child("Tracked Devices"),
                )
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(13.0))
                        .text_color(tokens.colors.text_muted)
                        .child(
                            "Current availability for device identities you intentionally track.",
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(local_ip_badge(&vm.daemon.local_ip_address, tokens))
                .child(
                    buttons::toolbar_icon_button(label, Icon::Refresh, tokens)
                        .id(SharedString::from("dashboard-refresh"))
                        .on_click(cx.listener(|app, _, _, cx| app.refresh_quick(cx))),
                ),
        )
}

fn local_ip_badge(local_ip_address: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .h(px(36.0))
        .rounded(px(18.0))
        .bg(gpui::rgba(0xffffff08))
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .px(px(12.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(crate::components::icons::icon(
            Icon::Wifi,
            15.0,
            tokens.colors.text_secondary,
        ))
        .child(
            div()
                .font_family("Geist Mono")
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .text_color(tokens.colors.text_muted)
                .child("THIS MAC"),
        )
        .child(
            div()
                .font_family("Geist Mono")
                .text_size(px(12.0))
                .text_color(tokens.colors.text)
                .child(local_ip_address.to_string()),
        )
}

fn metrics(vm: &DashboardVm, tokens: LiquidGlassTokens) -> Div {
    let ssh_ready = vm
        .tracked
        .iter()
        .filter(|row| row.ssh == AvailabilityState::Online)
        .count();
    let lan_reachable = vm
        .tracked
        .iter()
        .filter(|row| row.lan == AvailabilityState::Online)
        .count();
    let unknown = vm
        .tracked
        .iter()
        .filter(|row| row.overall == AvailabilityState::Unknown)
        .count();
    div()
        .h(px(116.0))
        .grid()
        .grid_cols(4)
        .gap(px(12.0))
        .child(metric_tile(
            "Tracked",
            vm.tracked.len(),
            "intentional identities",
            Icon::PanelRight,
            tokens,
        ))
        .child(metric_tile(
            "LAN Reachable",
            lan_reachable,
            "local endpoint online",
            Icon::Wifi,
            tokens,
        ))
        .child(metric_tile(
            "SSH Ready",
            ssh_ready,
            "capable endpoint",
            Icon::Terminal,
            tokens,
        ))
        .child(metric_tile(
            "Unknown",
            unknown,
            "needs refresh",
            Icon::Activity,
            tokens,
        ))
}

fn metric_tile(
    label: &str,
    value: usize,
    caption: &str,
    icon: Icon,
    tokens: LiquidGlassTokens,
) -> Div {
    div()
        .h_full()
        .rounded(px(20.0))
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .p(px(16.0))
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
                        .font_family("Geist Mono")
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens.colors.text_muted)
                        .child(label.to_ascii_uppercase()),
                )
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(28.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child(value.to_string()),
                )
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(11.0))
                        .text_color(tokens.colors.text_muted)
                        .child(caption.to_string()),
                ),
        )
        .child(crate::components::icons::icon(
            icon,
            18.0,
            tokens.colors.text_secondary,
        ))
}

fn device_table(
    rows: &[TrackedDeviceRowVm],
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .rounded(px(22.0))
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .p(px(14.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(table_header(tokens))
        .when(rows.is_empty(), |this| this.child(empty_row(tokens, cx)))
        .children(rows.iter().map(|row| device_row(row, tokens, cx)))
}

fn table_header(tokens: LiquidGlassTokens) -> Div {
    div()
        .h(px(34.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(12.0))
        .font_family("Geist Mono")
        .text_size(px(10.0))
        .font_weight(FontWeight::BOLD)
        .text_color(tokens.colors.text_muted)
        .child(header_cell("DEVICE IDENTITY", DASH_COLS[0]))
        .child(header_cell("LAN", DASH_COLS[1]))
        .child(header_cell("TAILSCALE", DASH_COLS[2]))
        .child(header_cell("SSH TARGET", DASH_COLS[3]))
        .child(header_cell("LAST SEEN", DASH_COLS[4]))
        .child(header_cell("ACTION", DASH_COLS[5]))
}

fn header_cell(label: &str, width: f32) -> Div {
    div().w(px(width)).child(label.to_string())
}

fn empty_row(tokens: LiquidGlassTokens, cx: &mut Context<NetworkManagerApp>) -> Div {
    div()
        .h(px(54.0))
        .rounded(px(16.0))
        .bg(gpui::rgba(0xffffff08))
        .px(px(14.0))
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .font_family("Geist")
                .text_size(px(13.0))
                .text_color(tokens.colors.text_secondary)
                .child("No tracked devices yet"),
        )
        .child(
            buttons::toolbar_icon_button("Open Discovery", Icon::Radar, tokens)
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
        .h(px(54.0))
        .rounded(px(16.0))
        .bg(gpui::rgba(0xffffff0a))
        .px(px(12.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(identity_cell(row, tokens))
        .child(status_cell(row.lan, DASH_COLS[1], tokens))
        .child(status_cell(row.tailscale, DASH_COLS[2], tokens))
        .child(target_cell(row, tokens))
        .child(
            div()
                .w(px(DASH_COLS[4]))
                .font_family("Geist")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_muted)
                .child(row.last_seen.clone()),
        )
        .child(action_cell(row, tokens, cx))
}

fn identity_cell(row: &TrackedDeviceRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(DASH_COLS[0]))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .font_family("Geist")
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(row.label.clone()),
        )
        .child(
            div()
                .font_family("Geist Mono")
                .text_size(px(10.0))
                .text_color(tokens.colors.text_muted)
                .child(row.alias.clone()),
        )
}

fn status_cell(state: AvailabilityState, width: f32, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(width))
        .flex()
        .items_center()
        .gap(px(7.0))
        .child(status::status_dot(state, tokens))
        .child(
            div()
                .font_family("Geist")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_secondary)
                .child(status::status_text(state).to_ascii_lowercase()),
        )
}

fn target_cell(row: &TrackedDeviceRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(DASH_COLS[3]))
        .font_family("Geist Mono")
        .text_size(px(11.0))
        .text_color(if row.ssh == AvailabilityState::Online {
            tokens.colors.icy
        } else {
            tokens.colors.text_secondary
        })
        .child(row.preferred_target.clone())
}

fn action_cell(
    row: &TrackedDeviceRowVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let identity_id = row.id.clone();
    div().w(px(DASH_COLS[5])).flex().justify_end().child(
        buttons::action_button("Details", tokens)
            .id(SharedString::from(format!("dashboard-action-{}", row.id)))
            .on_click(
                cx.listener(move |app, _, _, cx| app.select_device_detail(identity_id.clone(), cx)),
            ),
    )
}
