use gpui::{div, prelude::*, Div, FontWeight};

use crate::components::{buttons, glass, status, table as table_components};
use crate::data::{DeviceDetailVm, EndpointGroup, EndpointVm};
use crate::layout::inspector::{INSPECTOR_WIDTH, LIST_WIDTH};
use crate::theme::LiquidGlassTokens;

pub fn screen(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .size_full()
        .flex()
        .gap_4()
        .child(device_list(tokens))
        .child(inspector(vm, tokens))
}

fn device_list(tokens: LiquidGlassTokens) -> Div {
    glass::panel(tokens)
        .w(gpui::px(LIST_WIDTH))
        .h_full()
        .p_3()
        .flex()
        .flex_col()
        .gap_3()
        .child(glass::header(
            "Device Detail",
            "Inspector for identity, endpoints, and SSH target reasoning.",
            tokens,
        ))
        .child(selected_device("Synology NAS", "nas-main", tokens))
        .child(selected_device("Office MacBook", "office-macbook", tokens))
        .child(selected_device("HP LaserJet", "printer-hp", tokens))
}

fn selected_device(label: &str, alias: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(label.to_string()),
        )
        .child(
            div()
                .font_family("SF Mono")
                .text_xs()
                .text_color(tokens.colors.text_muted)
                .child(alias.to_string()),
        )
}

fn inspector(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex_1()
        .h_full()
        .flex()
        .gap_4()
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_4()
                .child(identity_card(vm, tokens))
                .child(endpoint_groups(vm, tokens))
                .child(evidence_card(vm, tokens)),
        )
        .child(actions_panel(vm, tokens))
}

fn identity_card(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    glass::panel_strong(tokens)
        .p_4()
        .flex()
        .items_start()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child(vm.identity.label.clone()),
                )
                .child(
                    div()
                        .font_family("SF Mono")
                        .text_xs()
                        .text_color(tokens.colors.text_muted)
                        .child(vm.identity.alias.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(tokens.colors.text_secondary)
                        .child(format!(
                            "{} · {:?} · {:?}",
                            vm.identity.category,
                            vm.identity.tracked_state,
                            vm.identity.endpoint_preference
                        )),
                ),
        )
        .child(buttons::toolbar_button("Edit", tokens))
}

fn endpoint_groups(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    let lan: Vec<_> = vm
        .endpoints
        .iter()
        .filter(|endpoint| endpoint.group == EndpointGroup::Lan)
        .collect();
    let tailscale: Vec<_> = vm
        .endpoints
        .iter()
        .filter(|endpoint| endpoint.group == EndpointGroup::Tailscale)
        .collect();

    div()
        .grid()
        .grid_cols(2)
        .gap_3()
        .child(endpoint_group(EndpointGroup::Lan, &lan, tokens))
        .child(endpoint_group(EndpointGroup::Tailscale, &tailscale, tokens))
}

fn endpoint_group(
    group: EndpointGroup,
    endpoints: &[&EndpointVm],
    tokens: LiquidGlassTokens,
) -> Div {
    glass::panel(tokens)
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(group.label().to_string()),
        )
        .children(
            endpoints
                .iter()
                .map(|endpoint| endpoint_row(endpoint, tokens)),
        )
}

fn endpoint_row(endpoint: &EndpointVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .p_2()
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .font_family("SF Mono")
                        .text_xs()
                        .text_color(tokens.colors.text)
                        .child(endpoint.address.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(tokens.colors.text_muted)
                        .child(format!(
                            "{:?} · checked {}",
                            endpoint.kind, endpoint.last_checked
                        )),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(status::status_dot(endpoint.reachability, tokens))
                .when(endpoint.preferred, |this| {
                    this.child(table_components::source_pill("preferred", tokens))
                }),
        )
}

fn evidence_card(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    glass::panel(tokens)
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child("Identity evidence"),
        )
        .children(vm.evidence.iter().map(|evidence| {
            div()
                .text_xs()
                .text_color(tokens.colors.text_secondary)
                .child(evidence.clone())
        }))
}

fn actions_panel(vm: &DeviceDetailVm, tokens: LiquidGlassTokens) -> Div {
    glass::panel(tokens)
        .w(gpui::px(INSPECTOR_WIDTH))
        .h_full()
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child("SSH Target"),
        )
        .when_some(vm.preferred_target.as_ref(), |this, target| {
            this.child(glass::system_note(
                &target.destination,
                &target.reason,
                tokens,
            ))
        })
        .child(buttons::action_button("Copy SSH", tokens))
        .child(buttons::toolbar_button("Open SSH", tokens))
        .child(glass::system_note(
            "Identity correction",
            "Split or merge is available when automatic matching gets a device identity wrong.",
            tokens,
        ))
        .child(buttons::toolbar_button("Split identity", tokens))
}
