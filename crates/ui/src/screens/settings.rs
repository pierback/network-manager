use gpui::{div, prelude::*, Div, FontWeight};

use crate::components::{forms, glass, status};
use crate::data::SettingsVm;
use crate::theme::LiquidGlassTokens;

pub fn screen(vm: &SettingsVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap_4()
        .child(glass::header(
            "Settings",
            "Discovery scope, endpoint preference, CLI aliases, and privacy controls.",
            tokens,
        ))
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_4()
                .child(discovery_group(vm, tokens))
                .child(integration_group(vm, tokens)),
        )
}

fn discovery_group(vm: &SettingsVm, tokens: LiquidGlassTokens) -> Div {
    group("Discovery", tokens)
        .child(forms::setting_row(
            "Scan cadence",
            "Refresh observed devices without treating all discoveries as tracked.",
            &vm.discovery_interval,
            tokens,
        ))
        .child(setting_toggle(
            "Battery mode",
            "Reduce broad LAN scans while on battery power.",
            vm.battery_mode,
            tokens,
        ))
        .child(setting_toggle(
            "Include Tailscale",
            "Track Tailscale presence separately from LAN reachability.",
            vm.tailscale_enabled,
            tokens,
        ))
}

fn integration_group(vm: &SettingsVm, tokens: LiquidGlassTokens) -> Div {
    group("CLI and privacy", tokens)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
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
                                .child("Tailscale service"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(tokens.colors.text_muted)
                                .child("Local Tailscale service status on this Mac."),
                        ),
                )
                .child(status::status_pill(
                    vm.tailscale_status.as_str(),
                    vm.tailscale_status,
                    tokens,
                )),
        )
        .child(setting_toggle(
            "SSH config export",
            "Export aliases explicitly; never modify ~/.ssh/config automatically.",
            vm.ssh_config_export,
            tokens,
        ))
        .child(setting_toggle(
            "Debug logging",
            "Include names and endpoints in diagnostic logs only when enabled.",
            vm.debug_logging,
            tokens,
        ))
}

fn group(title: &str, tokens: LiquidGlassTokens) -> Div {
    glass::panel(tokens).p_4().flex().flex_col().gap_1().child(
        div()
            .text_lg()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(tokens.colors.text)
            .child(title.to_string()),
    )
}

fn setting_toggle(label: &str, description: &str, on: bool, tokens: LiquidGlassTokens) -> Div {
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
        .child(forms::toggle(on, tokens))
}
