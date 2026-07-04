use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, InteractiveElement, SharedString,
    StatefulInteractiveElement,
};
use network_manager_core::AvailabilityState;
use std::net::IpAddr;

use crate::app::NetworkManagerApp;
use crate::components::{
    buttons,
    icons::{self, Icon},
    status,
};
use crate::data::{ActionStatus, DeviceDetailVm, DeviceIdentityVm, EndpointGroup, EndpointVm};
use crate::layout::app_shell::liquid_titlebar;
use crate::theme::LiquidGlassTokens;

pub fn screen(
    vm: &DeviceDetailVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .size_full()
        .relative()
        .child(liquid_titlebar(
            Icon::Server,
            "Device Detail",
            &[Icon::Dashboard, Icon::Terminal, Icon::Copy, Icon::Settings],
            tokens,
            cx,
        ))
        .child(detail_list(vm, tokens, cx))
        .child(inspector(vm, action_status, tokens, cx))
}

fn detail_list(
    vm: &DeviceDetailVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    div()
        .absolute()
        .left(px(24.0))
        .top(px(80.0))
        .w(px(386.0))
        .h(px(688.0))
        .id(SharedString::from("detail-list-scroll"))
        .overflow_y_scroll()
        .rounded(px(22.0))
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .p(px(18.0))
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(
            div()
                .font_family("Geist")
                .text_size(px(20.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child("Tracked Devices"),
        )
        .child(
            div()
                .font_family("Geist")
                .text_size(px(12.0))
                .text_color(tokens.colors.text_muted)
                .child("Device Identities with current availability"),
        )
        .children(
            vm.device_list
                .iter()
                .map(|device| selector_row(device, device.id == vm.identity.id, tokens, cx)),
        )
        .when(vm.device_list.is_empty(), |this| {
            this.child(selector_row(&vm.identity, true, tokens, cx))
        })
}

fn selector_row(
    device: &DeviceIdentityVm,
    selected: bool,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    let identity_id = device.id.clone();
    div()
        .id(SharedString::from(format!("detail-device-{}", device.id)))
        .rounded(px(14.0))
        .bg(if selected {
            gpui::rgba(0xffffff16)
        } else {
            gpui::rgba(0xffffff06)
        })
        .px(px(12.0))
        .py(px(10.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .hover(|style| style.bg(gpui::rgba(0xffffff16)))
        .cursor_pointer()
        .child(status::status_dot(device.availability, tokens))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child(device.label.clone()),
                )
                .child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(10.0))
                        .text_color(tokens.colors.text_muted)
                        .child(device.alias.clone()),
                ),
        )
        .on_click(
            cx.listener(move |app, _, _, cx| app.select_device_detail(identity_id.clone(), cx)),
        )
}

fn inspector(
    vm: &DeviceDetailVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    div()
        .absolute()
        .left(px(434.0))
        .top(px(80.0))
        .w(px(822.0))
        .h(px(688.0))
        .id(SharedString::from("detail-inspector-scroll"))
        .overflow_y_scroll()
        .pr(px(6.0))
        .flex()
        .flex_col()
        .gap(px(18.0))
        .child(hero(vm, tokens, cx))
        .when_some(action_status, |this, status| {
            this.child(detail_status_banner(status, tokens))
        })
        .child(
            div()
                .h(px(554.0))
                .flex()
                .gap(px(18.0))
                .child(endpoint_groups(vm, tokens))
                .child(reasoning_rail(vm, tokens)),
        )
}

fn hero(
    vm: &DeviceDetailVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let target = vm
        .preferred_target
        .as_ref()
        .map(|target| target.destination.clone());
    let ssh_command = target.as_ref().map(|target| format!("ssh {target}"));
    div()
        .h(px(116.0))
        .rounded(px(22.0))
        .bg(gpui::rgba(0xffffff12))
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .p(px(20.0))
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(30.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child(vm.identity.label.clone()),
                )
                .child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(12.0))
                        .text_color(tokens.colors.text_muted)
                        .child(vm.identity.alias.clone()),
                ),
        )
        .child(
            div()
                .flex()
                .gap(px(10.0))
                .child(match ssh_command {
                    Some(_) => buttons::toolbar_icon_button("SSH", Icon::Terminal, tokens)
                        .id(SharedString::from("detail-copy-ssh-command"))
                        .active(|style| style.bg(gpui::rgba(0xa9d8ff33)))
                        .on_click(cx.listener(|app, _, _, cx| app.copy_selected_ssh_command(cx)))
                        .into_any_element(),
                    None => buttons::disabled_icon_button("SSH", Icon::Terminal, tokens)
                        .into_any_element(),
                })
                .child(match target {
                    Some(_) => buttons::toolbar_icon_button("Copy target", Icon::Copy, tokens)
                        .id(SharedString::from("detail-copy-target"))
                        .active(|style| style.bg(gpui::rgba(0xa9d8ff33)))
                        .on_click(cx.listener(|app, _, _, cx| app.copy_selected_target(cx)))
                        .into_any_element(),
                    None => buttons::disabled_icon_button("Copy target", Icon::Copy, tokens)
                        .into_any_element(),
                }),
        )
}

fn detail_status_banner(status: &ActionStatus, tokens: LiquidGlassTokens) -> Div {
    let color = if status.is_error {
        tokens.colors.offline
    } else if status.is_pending {
        tokens.colors.unknown
    } else {
        tokens.colors.online
    };
    let icon = if status.is_error {
        Icon::Info
    } else if status.is_pending {
        Icon::Refresh
    } else {
        Icon::Check
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

fn endpoint_groups(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    let lan = endpoints_for_group(vm, EndpointGroup::Lan);
    let tailscale = endpoints_for_group(vm, EndpointGroup::Tailscale);
    let observed = observed_name_endpoints(vm);
    div()
        .flex_1()
        .grid()
        .grid_cols(1)
        .gap(px(14.0))
        .child(endpoint_card(
            "LAN Endpoints",
            &lan,
            "Network Proximity · SSH capable",
            tokens,
        ))
        .child(endpoint_card(
            "Tailscale Endpoints",
            &tailscale,
            "Tailscale Presence · SSH capable",
            tokens,
        ))
        .child(endpoint_card(
            "Observed Names",
            &observed,
            "Identity Evidence · discovery",
            tokens,
        ))
}

fn endpoint_card(
    title: &str,
    endpoints: &[&EndpointVm],
    description: &str,
    tokens: LiquidGlassTokens,
) -> Div {
    let endpoint_text = endpoint_primary_text(endpoints);
    let host_text = endpoint_host_text(endpoints);
    let ip_text = endpoint_ip_text(endpoints);
    let port_text = endpoint_port_text(endpoints);
    let last_checked = endpoint_last_checked_text(endpoints);
    let reachability = endpoint_group_reachability(endpoints);
    div()
        .rounded(px(20.0))
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(15.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .rounded(px(10.0))
                        .bg(gpui::rgba(0xffffff0c))
                        .px(px(8.0))
                        .py(px(4.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(status::status_mini_dot(reachability, tokens))
                        .child(
                            div()
                                .font_family("Geist")
                                .text_size(px(11.0))
                                .text_color(tokens.colors.text_secondary)
                                .child(status::status_text(reachability).to_ascii_lowercase()),
                        ),
                ),
        )
        .child(
            div()
                .font_family("Geist Mono")
                .text_size(px(18.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(endpoint_text),
        )
        .child(endpoint_detail_row("Host", host_text, tokens))
        .child(endpoint_detail_row("IP", ip_text, tokens))
        .child(endpoint_detail_row("Port", port_text, tokens))
        .child(endpoint_detail_row("Last checked", last_checked, tokens))
        .child(
            div()
                .font_family("Geist")
                .text_size(px(12.0))
                .text_color(tokens.colors.text_muted)
                .child(description.to_string()),
        )
}

fn endpoints_for_group(vm: &DeviceDetailVm, group: EndpointGroup) -> Vec<&EndpointVm> {
    vm.endpoints
        .iter()
        .filter(|endpoint| endpoint.group == group)
        .collect()
}

fn observed_name_endpoints(vm: &DeviceDetailVm) -> Vec<&EndpointVm> {
    vm.endpoints
        .iter()
        .filter(|endpoint| !is_ip_address(&endpoint.address))
        .collect()
}

fn endpoint_primary_text(endpoints: &[&EndpointVm]) -> String {
    endpoint_host(endpoints)
        .or_else(|| endpoint_ips(endpoints).into_iter().next())
        .unwrap_or_else(|| "—".into())
}

fn endpoint_host_text(endpoints: &[&EndpointVm]) -> String {
    endpoint_host(endpoints).unwrap_or_else(|| "—".into())
}

fn endpoint_ip_text(endpoints: &[&EndpointVm]) -> String {
    let ips = endpoint_ips(endpoints);
    if ips.is_empty() {
        "—".into()
    } else {
        ips.join(", ")
    }
}

fn endpoint_port_text(endpoints: &[&EndpointVm]) -> String {
    endpoints
        .iter()
        .find_map(|endpoint| endpoint.port)
        .map(|port| port.to_string())
        .unwrap_or_else(|| "—".into())
}

fn endpoint_last_checked_text(endpoints: &[&EndpointVm]) -> String {
    endpoints
        .iter()
        .map(|endpoint| endpoint.last_checked.as_str())
        .find(|value| *value != "never")
        .or_else(|| {
            endpoints
                .first()
                .map(|endpoint| endpoint.last_checked.as_str())
        })
        .unwrap_or("—")
        .to_string()
}

fn endpoint_host(endpoints: &[&EndpointVm]) -> Option<String> {
    endpoints
        .iter()
        .find_map(|endpoint| endpoint.hostname.as_deref())
        .or_else(|| {
            endpoints
                .iter()
                .map(|endpoint| endpoint.address.as_str())
                .find(|address| !is_ip_address(address))
        })
        .map(ToString::to_string)
}

fn endpoint_ips(endpoints: &[&EndpointVm]) -> Vec<String> {
    let mut ips = Vec::new();
    for endpoint in endpoints {
        if is_ip_address(&endpoint.address) && !ips.contains(&endpoint.address) {
            ips.push(endpoint.address.clone());
        }
    }
    ips
}

fn endpoint_group_reachability(endpoints: &[&EndpointVm]) -> AvailabilityState {
    if endpoints
        .iter()
        .any(|endpoint| endpoint.reachability == AvailabilityState::Online)
    {
        return AvailabilityState::Online;
    }
    if endpoints
        .iter()
        .any(|endpoint| endpoint.reachability == AvailabilityState::Offline)
    {
        return AvailabilityState::Offline;
    }
    AvailabilityState::Unknown
}

fn is_ip_address(value: &str) -> bool {
    value.parse::<IpAddr>().is_ok()
}

fn endpoint_detail_row(label: &str, value: String, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
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
                .font_family("Geist Mono")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_secondary)
                .child(value),
        )
}

fn reasoning_rail(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(280.0))
        .h_full()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(ssh_reasoning(vm, tokens))
        .child(metadata(vm, tokens))
}

fn ssh_reasoning(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    let title = vm
        .preferred_target
        .as_ref()
        .map(|_| "Preferred SSH target")
        .unwrap_or("No SSH target");
    let reason = vm
        .preferred_target
        .as_ref()
        .map(|target| target.reason.clone())
        .unwrap_or_else(|| {
            "Refresh this device to prove endpoint reachability before suggesting SSH.".into()
        });
    let target = vm
        .preferred_target
        .as_ref()
        .map(|target| target.destination.clone())
        .unwrap_or_else(|| "Refresh required".into());
    div()
        .rounded(px(22.0))
        .bg(gpui::rgba(0xffffff12))
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .p(px(18.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            div()
                .font_family("Geist Mono")
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .text_color(tokens.colors.text_muted)
                .child("SSH TARGET"),
        )
        .child(
            div()
                .font_family("Geist")
                .text_size(px(19.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(title),
        )
        .child(
            div()
                .font_family("Geist")
                .text_size(px(12.0))
                .text_color(tokens.colors.text_secondary)
                .child(reason),
        )
        .child(
            div()
                .rounded(px(14.0))
                .bg(gpui::rgba(0xffffff0a))
                .p(px(12.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(rule("1. Use LAN when locally reachable", tokens))
                .child(rule("2. Fall back to Tailscale", tokens))
                .child(rule("3. Do not infer SSH from presence", tokens)),
        )
        .child(
            div()
                .font_family("Geist Mono")
                .text_size(px(11.0))
                .text_color(tokens.colors.icy)
                .child(target),
        )
}

fn rule(text: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .font_family("Geist Mono")
        .text_size(px(10.0))
        .text_color(tokens.colors.text_muted)
        .child(text.to_string())
}

fn metadata(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    let last_seen = vm
        .endpoints
        .iter()
        .find(|endpoint| endpoint.last_checked != "never")
        .map(|endpoint| endpoint.last_checked.as_str())
        .unwrap_or("never");
    div()
        .flex_1()
        .rounded(px(20.0))
        .bg(gpui::rgba(0xffffff0d))
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(meta_row("Device Category", &vm.identity.category, tokens))
        .child(meta_row("Device Tags", "—", tokens))
        .child(meta_row("Last Seen", last_seen, tokens))
        .child(meta_row(
            "Endpoint Preference",
            &format!("{:?}", vm.identity.endpoint_preference),
            tokens,
        ))
}

fn meta_row(label: &str, value: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .font_family("Geist")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_muted)
                .child(label.to_string()),
        )
        .child(
            div()
                .font_family("Geist")
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text_secondary)
                .child(value.to_string()),
        )
}
