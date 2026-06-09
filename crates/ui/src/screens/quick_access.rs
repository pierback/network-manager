use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, InteractiveElement, SharedString,
    StatefulInteractiveElement,
};
use network_manager_core::AvailabilityState;

use crate::app::NetworkManagerApp;
use crate::components::{
    buttons,
    icons::{self, Icon},
    status,
};
use crate::data::{ActionStatus, QuickAccessVm, TrackedDeviceRowVm};
use crate::routes::Route;
use crate::theme::LiquidGlassTokens;

pub fn screen(
    vm: &QuickAccessVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .size_full()
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .child(popover_artboard(vm, action_status, tokens, cx))
}

fn popover_artboard(
    vm: &QuickAccessVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .relative()
        .w(px(440.0))
        .h(px(620.0))
        .rounded(px(24.0))
        .bg(tokens.colors.background)
        .overflow_hidden()
        .child(menu_bar(tokens))
        .child(popover_pointer(tokens))
        .child(popover_shell(vm, action_status, tokens, cx))
}

fn menu_bar(tokens: LiquidGlassTokens) -> Div {
    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .w(px(440.0))
        .h(px(34.0))
        .bg(gpui::rgba(0x0b0c0dff))
        .px(px(18.0))
        .flex()
        .items_center()
        .child(
            div()
                .rounded(px(9.0))
                .bg(gpui::rgba(0xffffff14))
                .px(px(9.0))
                .py(px(5.0))
                .flex()
                .items_center()
                .gap(px(7.0))
                .child(icons::icon(Icon::Network, 13.0, tokens.colors.text))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child("Network"),
                ),
        )
}

fn popover_pointer(tokens: LiquidGlassTokens) -> Div {
    div()
        .absolute()
        .left(px(194.0))
        .top(px(38.0))
        .w(px(52.0))
        .h(px(18.0))
        .rounded(px(12.0))
        .bg(tokens.colors.popover)
        .border_1()
        .border_color(tokens.colors.edge)
        .child(
            div()
                .absolute()
                .left(px(10.0))
                .top(px(3.0))
                .w(px(32.0))
                .h(px(1.0))
                .bg(gpui::rgba(0xffffff35)),
        )
}

fn popover_shell(
    vm: &QuickAccessVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    div()
        .absolute()
        .left(px(30.0))
        .top(px(48.0))
        .w(px(380.0))
        .h(px(540.0))
        .rounded(px(24.0))
        .bg(tokens.colors.popover)
        .border_1()
        .border_color(tokens.colors.edge)
        .id(SharedString::from("quick-access-popover-scroll"))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .child(popover_header(action_status, tokens, cx))
        .child(summary(vm, tokens))
        .child(device_list(vm, tokens, cx))
        .child(footer(vm, tokens, cx))
}

fn popover_header(
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let refresh_accent = action_status.is_some_and(|status| status.is_pending);
    div()
        .h(px(60.0))
        .px(px(16.0))
        .py(px(12.0))
        .bg(gpui::rgba(0xffffff08))
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(15.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child("Quick Access"),
                )
                .child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(10.0))
                        .text_color(tokens.colors.text_muted)
                        .child("Tracked Devices"),
                ),
        )
        .child(
            div()
                .flex()
                .gap(px(8.0))
                .child(
                    buttons::small_icon_button(Icon::Refresh, refresh_accent, tokens)
                        .id(SharedString::from("quick-access-refresh"))
                        .on_click(cx.listener(|app, _, _, cx| app.refresh_quick(cx))),
                )
                .child(
                    buttons::small_icon_button(Icon::Settings, false, tokens)
                        .id(SharedString::from("quick-access-settings"))
                        .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Settings, cx))),
                ),
        )
}

fn summary(vm: &QuickAccessVm, tokens: LiquidGlassTokens) -> Div {
    let lan = vm
        .rows
        .iter()
        .filter(|row| row.lan == AvailabilityState::Online)
        .count();
    let tailnet = vm
        .rows
        .iter()
        .filter(|row| row.tailscale == AvailabilityState::Online)
        .count();
    let unknown = vm
        .rows
        .iter()
        .filter(|row| row.overall == AvailabilityState::Unknown)
        .count();
    div()
        .h(px(58.0))
        .px(px(12.0))
        .py(px(10.0))
        .grid()
        .grid_cols(3)
        .gap(px(8.0))
        .child(summary_card("LAN", lan, AvailabilityState::Online, tokens))
        .child(summary_card(
            "Tailnet",
            tailnet,
            AvailabilityState::Online,
            tokens,
        ))
        .child(summary_card(
            "Unknown",
            unknown,
            AvailabilityState::Unknown,
            tokens,
        ))
}

fn summary_card(
    label: &str,
    value: usize,
    state: AvailabilityState,
    tokens: LiquidGlassTokens,
) -> Div {
    div()
        .rounded(px(14.0))
        .bg(gpui::rgba(0xffffff0b))
        .px(px(10.0))
        .py(px(7.0))
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(status::status_mini_dot(state, tokens))
                .child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(9.0))
                        .text_color(tokens.colors.text_muted)
                        .child(label.to_string()),
                ),
        )
        .child(
            div()
                .font_family("Geist")
                .text_size(px(15.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(value.to_string()),
        )
}

fn device_list(
    vm: &QuickAccessVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .h(px(338.0))
        .px(px(8.0))
        .py(px(6.0))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .when(vm.rows.is_empty(), |this| this.child(empty_row(tokens)))
        .children(
            vm.rows
                .iter()
                .take(6)
                .enumerate()
                .map(|(index, row)| quick_row(index, row, tokens, cx)),
        )
}

fn empty_row(tokens: LiquidGlassTokens) -> Div {
    div()
        .h(px(52.0))
        .px(px(10.0))
        .flex()
        .items_center()
        .font_family("Geist")
        .text_size(px(12.0))
        .text_color(tokens.colors.text_muted)
        .child("No tracked devices")
}

fn quick_row(
    index: usize,
    row: &TrackedDeviceRowVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    let identity_id = row.id.clone();
    div()
        .id(SharedString::from(format!("quick-row-{}", row.id)))
        .h(px(52.0))
        .rounded(px(14.0))
        .bg(if index == 0 {
            gpui::rgba(0xffffff16)
        } else {
            gpui::rgba(0xffffff00)
        })
        .px(px(10.0))
        .py(px(7.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .hover(|style| style.bg(gpui::rgba(0xffffff16)))
        .cursor_pointer()
        .child(status::status_dot(row.overall, tokens))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child(row.label.clone()),
                )
                .child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(9.0))
                        .text_color(tokens.colors.text_muted)
                        .child(row.preferred_target.clone()),
                ),
        )
        .child(action_pill(row, tokens))
        .on_click(
            cx.listener(move |app, _, _, cx| app.select_device_detail(identity_id.clone(), cx)),
        )
}

fn action_pill(row: &TrackedDeviceRowVm, tokens: LiquidGlassTokens) -> Div {
    let label = if row.overall == AvailabilityState::Unknown {
        "Details"
    } else {
        "Open"
    };
    div()
        .w(px(58.0))
        .h(px(28.0))
        .rounded(px(10.0))
        .bg(gpui::rgba(0xffffff0b))
        .flex()
        .items_center()
        .justify_center()
        .font_family("Geist")
        .text_size(px(10.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens.colors.text_secondary)
        .child(label)
}

fn footer(
    vm: &QuickAccessVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .h(px(82.0))
        .bg(gpui::rgba(0xffffff06))
        .px(px(16.0))
        .py(px(12.0))
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text_secondary)
                        .child(format!("Last scan: {}", vm.last_scan)),
                )
                .child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(9.0))
                        .text_color(tokens.colors.text_muted)
                        .child("Network Proximity checked"),
                ),
        )
        .child(
            div()
                .id(SharedString::from("quick-access-open-dashboard"))
                .rounded(px(12.0))
                .bg(gpui::rgba(0xffffff16))
                .px(px(10.0))
                .py(px(8.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .cursor_pointer()
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child("Open"),
                )
                .child(icons::icon(
                    Icon::ArrowUpRight,
                    12.0,
                    tokens.colors.text_secondary,
                ))
                .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Dashboard, cx))),
        )
}
