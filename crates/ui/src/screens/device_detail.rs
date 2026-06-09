use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, InteractiveElement, SharedString,
    StatefulInteractiveElement,
};
use network_manager_core::AvailabilityState;

use crate::app::{ActionStatus, NetworkManagerApp};
use crate::components::{buttons, icons::Icon, status};
use crate::data::{DeviceDetailVm, DeviceIdentityVm, EndpointGroup, EndpointVm};
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
    _action_status: Option<&ActionStatus>,
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
                        .on_click(cx.listener(|app, _, _, cx| app.copy_selected_ssh_command(cx)))
                        .into_any_element(),
                    None => buttons::disabled_icon_button("SSH", Icon::Terminal, tokens)
                        .into_any_element(),
                })
                .child(match target {
                    Some(_) => buttons::toolbar_icon_button("Copy target", Icon::Copy, tokens)
                        .id(SharedString::from("detail-copy-target"))
                        .on_click(cx.listener(|app, _, _, cx| app.copy_selected_target(cx)))
                        .into_any_element(),
                    None => buttons::disabled_icon_button("Copy target", Icon::Copy, tokens)
                        .into_any_element(),
                }),
        )
}

fn endpoint_groups(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    let lan = first_endpoint(vm, EndpointGroup::Lan);
    let tailscale = first_endpoint(vm, EndpointGroup::Tailscale);
    let observed = vm.endpoints.first();
    div()
        .flex_1()
        .grid()
        .grid_cols(1)
        .gap(px(14.0))
        .child(endpoint_card(
            "LAN Endpoints",
            lan,
            "Network Proximity · SSH capable",
            tokens,
        ))
        .child(endpoint_card(
            "Tailscale Endpoints",
            tailscale,
            "Tailscale Presence · SSH capable",
            tokens,
        ))
        .child(endpoint_card(
            "Observed Names",
            observed,
            "Identity Evidence · discovery",
            tokens,
        ))
}

fn endpoint_card(
    title: &str,
    endpoint: Option<&EndpointVm>,
    description: &str,
    tokens: LiquidGlassTokens,
) -> Div {
    let endpoint_text = endpoint
        .map(|endpoint| endpoint.address.clone())
        .unwrap_or_else(|| "—".into());
    let reachability = endpoint
        .map(|endpoint| endpoint.reachability)
        .unwrap_or(AvailabilityState::Unknown);
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
        .child(
            div()
                .font_family("Geist")
                .text_size(px(12.0))
                .text_color(tokens.colors.text_muted)
                .child(description.to_string()),
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

fn first_endpoint(vm: &DeviceDetailVm, group: EndpointGroup) -> Option<&EndpointVm> {
    vm.endpoints.iter().find(|endpoint| endpoint.group == group)
}
