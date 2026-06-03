use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, InteractiveElement, SharedString,
    StatefulInteractiveElement,
};
use network_manager_core::AvailabilityState;

use crate::app::{ActionStatus, NetworkManagerApp};
use crate::components::{buttons, glass, status, table as table_components};
use crate::data::{DeviceDetailVm, DeviceIdentityVm, EndpointGroup, EndpointVm};
use crate::theme::LiquidGlassTokens;

pub fn screen(
    vm: &DeviceDetailVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .size_full()
        .flex()
        .child(device_list(vm, tokens, cx))
        .child(inspector(vm, action_status, tokens))
}

fn device_list(
    vm: &DeviceDetailVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .w(px(560.0))
        .h_full()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .pt(px(60.0))
        .pr(px(24.0))
        .pb(px(24.0))
        .pl(px(24.0))
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
                        .child(format!("{} tracked devices", vm.device_list.len().max(1))),
                ),
        )
        .child(div().h(px(1.0)).bg(tokens.colors.edge_soft))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .children(vm.device_list.iter().map(|device| {
                    selectable_device(device, device.id == vm.identity.id, tokens, cx)
                }))
                .when(vm.device_list.is_empty(), |this| {
                    this.child(selected_device(&vm.identity, true, tokens))
                }),
        )
}

fn selectable_device(
    device: &DeviceIdentityVm,
    selected: bool,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    let identity_id = device.id.clone();
    selected_device(device, selected, tokens)
        .id(SharedString::from(format!("detail-device-{}", device.id)))
        .hover(move |style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .on_click(
            cx.listener(move |app, _, _, cx| app.select_device_detail(identity_id.clone(), cx)),
        )
}

fn selected_device(device: &DeviceIdentityVm, selected: bool, tokens: LiquidGlassTokens) -> Div {
    div()
        .h(px(44.0))
        .rounded(px(6.0))
        .bg(if selected {
            tokens.colors.panel_strong
        } else {
            tokens.colors.panel
        })
        .when(selected, |this| {
            this.border_1().border_color(tokens.colors.accent)
        })
        .px(px(12.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(status::status_dot(AvailabilityState::Online, tokens))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .child(
                    div()
                        .font_family("Inter")
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
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
        .child(
            div()
                .font_family("Inter")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_muted)
                .child(device.category.clone()),
        )
}

fn inspector(
    vm: &DeviceDetailVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
) -> Div {
    div()
        .flex_1()
        .h_full()
        .bg(tokens.colors.panel)
        .border_l_1()
        .border_color(tokens.colors.edge_soft)
        .pt(px(60.0))
        .pr(px(28.0))
        .pb(px(28.0))
        .pl(px(28.0))
        .flex()
        .flex_col()
        .gap(px(20.0))
        .child(inspector_header(vm, tokens))
        .when_some(action_status, |this, action_status| {
            this.child(action_note(action_status, tokens))
        })
        .child(identity_section(vm, tokens))
        .child(status_breakdown(vm, tokens))
        .child(endpoints_section(vm, tokens))
        .child(ssh_target_section(vm, tokens))
        .child(identity_corrections(tokens))
}

fn inspector_header(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .w(px(12.0))
                        .h(px(12.0))
                        .rounded_full()
                        .bg(status_color(overall(vm), tokens)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .font_family("Inter")
                                .text_size(px(18.0))
                                .font_weight(FontWeight::BOLD)
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
                ),
        )
        .child(buttons::disabled_button("Edit", tokens))
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

fn identity_section(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    section("IDENTITY", tokens)
        .child(field_row("Device Label", &vm.identity.label, false, tokens))
        .child(field_row("Device Alias", &vm.identity.alias, true, tokens))
        .child(field_row("Category", &vm.identity.category, false, tokens))
}

fn field_row(label: &str, value: &str, mono: bool, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(16.0))
        .child(
            div()
                .w(px(116.0))
                .font_family("Inter")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_muted)
                .child(label.to_string()),
        )
        .child(
            div()
                .flex_1()
                .rounded(px(5.0))
                .border_1()
                .border_color(tokens.colors.edge_soft)
                .bg(tokens.colors.background)
                .px(px(10.0))
                .py(px(6.0))
                .font_family(if mono { "Geist Mono" } else { "Inter" })
                .text_size(px(13.0))
                .text_color(tokens.colors.text)
                .child(value.to_string()),
        )
}

fn status_breakdown(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    let tailscale = aggregate(vm, |endpoint| endpoint.group == EndpointGroup::Tailscale);
    let lan = aggregate(vm, |endpoint| endpoint.group == EndpointGroup::Lan);
    let ssh = aggregate_ssh(vm);
    section("STATUS BREAKDOWN", tokens).child(
        div()
            .grid()
            .grid_cols(4)
            .gap(px(12.0))
            .child(status_card(
                "Running",
                "Tailscale Service",
                tailscale != AvailabilityState::Offline,
                tokens.colors.online,
                tokens,
            ))
            .child(status_card(
                status::status_text(tailscale),
                "Tailscale Presence",
                tailscale == AvailabilityState::Online,
                status_color(tailscale, tokens),
                tokens,
            ))
            .child(status_card(
                status::status_text(lan),
                "LAN Reachability",
                lan == AvailabilityState::Online,
                status_color(lan, tokens),
                tokens,
            ))
            .child(status_card(
                ssh_text(ssh),
                "SSH Capability",
                ssh == AvailabilityState::Online,
                if ssh == AvailabilityState::Online {
                    tokens.colors.ssh_capable
                } else {
                    status_color(ssh, tokens)
                },
                tokens,
            )),
    )
}

fn status_card(
    value: &str,
    label: &str,
    positive: bool,
    color: gpui::Hsla,
    tokens: LiquidGlassTokens,
) -> Div {
    div()
        .rounded(px(6.0))
        .bg(tokens.colors.background)
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(div().w(px(6.0)).h(px(6.0)).rounded_full().bg(color))
                .child(
                    div()
                        .font_family("Inter")
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if positive {
                            color
                        } else {
                            tokens.colors.text_secondary
                        })
                        .child(value.to_string()),
                ),
        )
        .child(
            div()
                .font_family("Inter")
                .text_size(px(11.0))
                .text_color(tokens.colors.text_muted)
                .child(label.to_string()),
        )
}

fn endpoints_section(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    section("NETWORK ENDPOINTS", tokens)
        .when(vm.endpoints.is_empty(), |this| {
            this.child(
                div()
                    .rounded(px(5.0))
                    .bg(tokens.colors.background)
                    .px(px(12.0))
                    .h(px(36.0))
                    .flex()
                    .items_center()
                    .font_family("Inter")
                    .text_size(px(12.0))
                    .text_color(tokens.colors.text_muted)
                    .child("No endpoints recorded yet."),
            )
        })
        .children(
            vm.endpoints
                .iter()
                .map(|endpoint| endpoint_row(endpoint, tokens)),
        )
}

fn endpoint_row(endpoint: &EndpointVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .h(px(36.0))
        .rounded(px(5.0))
        .bg(tokens.colors.background)
        .px(px(12.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .w(px(6.0))
                .h(px(6.0))
                .rounded_full()
                .bg(status_color(endpoint.reachability, tokens)),
        )
        .child(table_components::source_pill(
            endpoint.group.label(),
            tokens,
        ))
        .child(
            div()
                .flex_1()
                .font_family("Geist Mono")
                .text_size(px(12.0))
                .text_color(if endpoint.preferred {
                    tokens.colors.accent
                } else {
                    tokens.colors.text_secondary
                })
                .child(endpoint.address.clone()),
        )
        .child(
            div()
                .font_family("Inter")
                .text_size(px(11.0))
                .text_color(status_color(endpoint.reachability, tokens))
                .child(status::status_text(endpoint.reachability)),
        )
        .when(endpoint.preferred, |this| {
            this.child(
                div()
                    .rounded(px(4.0))
                    .bg(tokens.colors.accent)
                    .px(px(6.0))
                    .py(px(2.0))
                    .font_family("Inter")
                    .text_size(px(10.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(tokens.colors.text_inverse)
                    .child("PREFERRED"),
            )
        })
}

fn ssh_target_section(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    section("SSH TARGET", tokens).child(
        vm.preferred_target
            .as_ref()
            .map(|target| {
                div()
                    .rounded(px(6.0))
                    .bg(tokens.colors.background)
                    .p(px(14.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(tokens.colors.accent)
                                    .child("⌘"),
                            )
                            .child(
                                div()
                                    .font_family("Geist Mono")
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(tokens.colors.accent)
                                    .child(target.destination.clone()),
                            ),
                    )
                    .child(
                        div()
                            .font_family("Inter")
                            .text_size(px(12.0))
                            .text_color(tokens.colors.text_muted)
                            .child(target.reason.clone()),
                    )
            })
            .unwrap_or_else(|| {
                div()
                    .rounded(px(6.0))
                    .bg(tokens.colors.background)
                    .p(px(14.0))
                    .font_family("Inter")
                    .text_size(px(12.0))
                    .text_color(tokens.colors.text_muted)
                    .child("No SSH target")
            }),
    )
}

fn identity_corrections(tokens: LiquidGlassTokens) -> Div {
    section("IDENTITY CORRECTIONS", tokens).child(
        div()
            .flex()
            .gap(px(8.0))
            .child(buttons::disabled_button("Merge with...", tokens))
            .child(buttons::disabled_button("Split identity", tokens)),
    )
}

fn section(title: &str, tokens: LiquidGlassTokens) -> Div {
    div().flex().flex_col().gap(px(12.0)).child(
        div()
            .font_family("Inter")
            .text_size(px(11.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(tokens.colors.text_muted)
            .child(title.to_string()),
    )
}

fn aggregate(vm: &DeviceDetailVm, predicate: impl Fn(&EndpointVm) -> bool) -> AvailabilityState {
    let mut saw_offline = false;
    for endpoint in vm.endpoints.iter().filter(|endpoint| predicate(endpoint)) {
        match endpoint.reachability {
            AvailabilityState::Online => return AvailabilityState::Online,
            AvailabilityState::Offline => saw_offline = true,
            AvailabilityState::Unknown => {}
        }
    }
    if saw_offline {
        AvailabilityState::Offline
    } else {
        AvailabilityState::Unknown
    }
}

fn aggregate_ssh(vm: &DeviceDetailVm) -> AvailabilityState {
    let mut saw_offline = false;
    for endpoint in &vm.endpoints {
        match endpoint.ssh_capability {
            AvailabilityState::Online => return AvailabilityState::Online,
            AvailabilityState::Offline => saw_offline = true,
            AvailabilityState::Unknown => {}
        }
    }
    if saw_offline {
        AvailabilityState::Offline
    } else {
        AvailabilityState::Unknown
    }
}

fn overall(vm: &DeviceDetailVm) -> AvailabilityState {
    aggregate(vm, |_| true)
}

fn status_color(state: AvailabilityState, tokens: LiquidGlassTokens) -> gpui::Hsla {
    tokens.status_color(state)
}

fn ssh_text(state: AvailabilityState) -> &'static str {
    match state {
        AvailabilityState::Online => "Ready",
        AvailabilityState::Offline => "N/A",
        AvailabilityState::Unknown => "Unknown",
    }
}
