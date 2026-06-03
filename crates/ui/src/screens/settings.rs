use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, InteractiveElement, SharedString,
    StatefulInteractiveElement,
};

use crate::app::{ActionStatus, NetworkManagerApp};
use crate::components::{buttons, glass, status};
use crate::data::SettingsVm;
use crate::theme::LiquidGlassTokens;

pub fn screen(
    vm: &SettingsVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(px(28.0))
        .pt(px(60.0))
        .pr(px(40.0))
        .pb(px(40.0))
        .pl(px(40.0))
        .child(header(tokens))
        .when_some(action_status, |this, status| {
            this.child(action_note(status, tokens))
        })
        .child(discovery_section(vm, tokens))
        .child(tailscale_section(vm, tokens))
        .child(cli_section(vm, tokens, cx))
        .child(privacy_section(vm, tokens))
}

fn header(tokens: LiquidGlassTokens) -> Div {
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
                .child("Settings"),
        )
        .child(
            div()
                .font_family("Inter")
                .text_size(px(13.0))
                .text_color(tokens.colors.text_secondary)
                .child("Discovery, daemon, CLI, and privacy preferences"),
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

fn discovery_section(vm: &SettingsVm, tokens: LiquidGlassTokens) -> Div {
    section("DISCOVERY", tokens)
        .child(setting_value_row(
            "Scan Interval",
            "Daemon-managed automatic quick refresh cadence.",
            &vm.discovery_interval,
            tokens,
        ))
        .child(setting_toggle_row(
            "Battery Mode",
            "Reduce broad LAN scans while on battery power.",
            vm.battery_mode,
            tokens,
        ))
        .child(setting_toggle_row(
            "ARP Scanning",
            "Use local neighbour tables and bounded LAN probes.",
            true,
            tokens,
        ))
}

fn tailscale_section(vm: &SettingsVm, tokens: LiquidGlassTokens) -> Div {
    section("TAILSCALE", tokens)
        .child(setting_toggle_row(
            "Tailscale Integration",
            "Read local tailscale status without managing Tailscale config.",
            vm.tailscale_enabled,
            tokens,
        ))
        .child(
            setting_base_row(
                "Tailscale Status",
                "Local service state on this Mac.",
                tokens,
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(status::status_mini_dot(vm.tailscale_status, tokens))
                    .child(
                        div()
                            .font_family("Inter")
                            .text_size(px(12.0))
                            .text_color(tokens.status_color(vm.tailscale_status))
                            .child(status::status_text(vm.tailscale_status)),
                    ),
            ),
        )
}

fn cli_section(
    vm: &SettingsVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    section("CLI", tokens)
        .child(setting_value_row(
            "CLI Path",
            "Agent-friendly local command interface.",
            "network-manager",
            tokens,
        ))
        .child(setting_toggle_row(
            "SSH Config Export",
            "Generate reviewed Host entries; never modify ~/.ssh/config automatically.",
            vm.ssh_config_export,
            tokens,
        ))
        .child(
            setting_base_row(
                "Daemon LaunchAgent",
                "Install, start, or stop the per-user background daemon.",
                tokens,
            )
            .child(
                div()
                    .flex()
                    .gap(px(6.0))
                    .child(
                        buttons::toolbar_button("Install", tokens)
                            .id(SharedString::from("settings-daemon-install-start"))
                            .on_click(
                                cx.listener(|app, _, _, cx| app.install_and_start_daemon(cx)),
                            ),
                    )
                    .child(
                        buttons::toolbar_button("Start", tokens)
                            .id(SharedString::from("settings-daemon-start"))
                            .on_click(cx.listener(|app, _, _, cx| app.start_daemon(cx))),
                    )
                    .child(
                        buttons::toolbar_button("Stop", tokens)
                            .id(SharedString::from("settings-daemon-stop"))
                            .on_click(cx.listener(|app, _, _, cx| app.stop_daemon(cx))),
                    ),
            ),
        )
}

fn privacy_section(vm: &SettingsVm, tokens: LiquidGlassTokens) -> Div {
    section("PRIVACY", tokens)
        .child(setting_toggle_row(
            "Debug Logging",
            "Include names and endpoints in diagnostic logs only when enabled.",
            vm.debug_logging,
            tokens,
        ))
        .child(setting_value_row(
            "Data Storage",
            "All discovery, preferences, and corrections stay local in SQLite.",
            "Local only",
            tokens,
        ))
}

fn section(title: &str, tokens: LiquidGlassTokens) -> Div {
    div().flex().flex_col().gap(px(8.0)).child(
        div()
            .font_family("Inter")
            .text_size(px(11.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(tokens.colors.text_muted)
            .child(title.to_string()),
    )
}

fn setting_base_row(label: &str, description: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .w_full()
        .rounded(px(6.0))
        .bg(tokens.colors.panel)
        .px(px(16.0))
        .py(px(14.0))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .font_family("Inter")
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(tokens.colors.text)
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .font_family("Inter")
                        .text_size(px(12.0))
                        .text_color(tokens.colors.text_muted)
                        .child(description.to_string()),
                ),
        )
}

fn setting_value_row(
    label: &str,
    description: &str,
    value: &str,
    tokens: LiquidGlassTokens,
) -> Div {
    setting_base_row(label, description, tokens).child(
        div()
            .rounded(px(5.0))
            .bg(tokens.colors.panel_strong)
            .px(px(12.0))
            .py(px(5.0))
            .font_family("Inter")
            .text_size(px(12.0))
            .text_color(tokens.colors.text)
            .child(value.to_string()),
    )
}

fn setting_toggle_row(label: &str, description: &str, on: bool, tokens: LiquidGlassTokens) -> Div {
    setting_base_row(label, description, tokens).child(toggle(on, tokens))
}

fn toggle(on: bool, tokens: LiquidGlassTokens) -> Div {
    div()
        .relative()
        .w(px(40.0))
        .h(px(22.0))
        .rounded_full()
        .bg(if on {
            tokens.colors.accent
        } else {
            tokens.colors.panel_strong
        })
        .child(
            div()
                .absolute()
                .left(px(if on { 20.0 } else { 2.0 }))
                .top(px(2.0))
                .w(px(18.0))
                .h(px(18.0))
                .rounded_full()
                .bg(gpui::rgba(0xffffffff)),
        )
}
