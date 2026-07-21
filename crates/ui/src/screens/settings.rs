use gpui::{div, prelude::*, px, Context, Div, FontWeight, SharedString};
use network_manager_core::AvailabilityState;

use crate::app::NetworkManagerApp;
use crate::components::{buttons, icons::Icon, status};
use crate::data::{ActionStatus, SettingsVm};
use crate::layout::app_shell::{route_shell, TitlebarAction};
use crate::routes::Route;
use crate::theme::LiquidGlassTokens;

pub fn screen(
    vm: &SettingsVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let main = sync_diagnostics_main(vm, action_status, tokens, cx);
    route_shell(
        Route::Settings,
        Icon::Settings,
        "Sync & Diagnostics",
        &[TitlebarAction::Refresh],
        main,
        tokens,
        cx,
    )
}

fn sync_diagnostics_main(
    vm: &SettingsVm,
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
        .id(SharedString::from("sync-diagnostics-main-scroll"))
        .overflow_y_scroll()
        .pr(px(6.0))
        .flex()
        .flex_col()
        .gap(px(18.0))
        .child(page_header(vm, tokens))
        .when_some(action_status, |this, action_status| {
            this.child(status::action_banner(action_status, tokens))
        })
        .child(sync_diagnostics_grid(vm, tokens, cx))
}

fn page_header(vm: &SettingsVm, tokens: LiquidGlassTokens) -> Div {
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
                        .child("Sync & Diagnostics"),
                )
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(13.0))
                        .text_color(tokens.colors.text_muted)
                        .child(
                            "Inspect local sync health, refresh discovery, and recover the daemon.",
                        ),
                ),
        )
        .child(
            div()
                .rounded(px(10.0))
                .bg(gpui::rgba(0xffffff0d))
                .px(px(10.0))
                .py(px(7.0))
                .flex()
                .items_center()
                .gap(px(7.0))
                .child(status::status_mini_dot(vm.daemon.state, tokens))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text_secondary)
                        .child(status::status_text(vm.daemon.state)),
                ),
        )
}

fn sync_diagnostics_grid(
    vm: &SettingsVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .min_h(px(560.0))
        .grid()
        .grid_cols(2)
        .gap(px(16.0))
        .child(
            section_stack()
                .child(sync_health(vm, tokens))
                .child(sync_actions(tokens, cx)),
        )
        .child(
            section_stack()
                .child(daemon_recovery(tokens, cx))
                .child(diagnostics(tokens, cx)),
        )
}

fn sync_health(vm: &SettingsVm, tokens: LiquidGlassTokens) -> Div {
    section(
        "Sync Health",
        "Current local daemon and network-source status.",
        tokens,
    )
    .child(status_row(
        "Daemon",
        daemon_hint(vm),
        &format!(
            "{} via {}",
            status::status_text(vm.daemon.state),
            vm.daemon.source
        ),
        tokens,
    ))
    .child(status_row(
        "Last sync",
        if vm.daemon.stale {
            "Status is stale; refresh or restart the daemon."
        } else {
            "Latest daemon update recorded locally."
        },
        &vm.daemon.last_scan,
        tokens,
    ))
    .child(status_row(
        "This Mac",
        "LAN address used for local network checks.",
        &vm.daemon.local_ip_address,
        tokens,
    ))
    .child(status_row(
        "Tailscale",
        "Fallback presence and SSH evidence source.",
        status::status_text(vm.daemon.tailscale_service),
        tokens,
    ))
}

fn sync_actions(tokens: LiquidGlassTokens, cx: &mut Context<NetworkManagerApp>) -> Div {
    section(
        "Refresh",
        "Request bounded refresh work from the local daemon.",
        tokens,
    )
    .child(action_row(
        "Sync now",
        "Quick refresh updates status; full refresh also rebuilds discovery evidence.",
        tokens,
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                buttons::accent_icon_button("Quick", Icon::Refresh, tokens)
                    .id(SharedString::from("settings-sync-quick-scan"))
                    .on_click(cx.listener(|app, _, _, cx| app.refresh_quick(cx))),
            )
            .child(
                buttons::toolbar_icon_button("Full", Icon::Radar, tokens)
                    .id(SharedString::from("settings-sync-full-scan"))
                    .on_click(cx.listener(|app, _, _, cx| app.refresh_full(cx))),
            ),
    ))
}

fn daemon_recovery(tokens: LiquidGlassTokens, cx: &mut Context<NetworkManagerApp>) -> Div {
    section(
        "Daemon Recovery",
        "Control the per-user LaunchAgent that keeps local state fresh.",
        tokens,
    )
    .child(action_row(
        "Daemon",
        "Start, restart, or stop the installed LaunchAgent.",
        tokens,
        div()
            .flex()
            .items_center()
            .gap(px(7.0))
            .child(
                buttons::accent_icon_button("Start", Icon::Refresh, tokens)
                    .id(SharedString::from("settings-daemon-start"))
                    .on_click(cx.listener(|app, _, _, cx| app.start_daemon(cx))),
            )
            .child(
                buttons::toolbar_icon_button("Restart", Icon::RotateCcw, tokens)
                    .id(SharedString::from("settings-daemon-restart"))
                    .on_click(cx.listener(|app, _, _, cx| app.restart_daemon(cx))),
            )
            .child(
                buttons::toolbar_icon_button("Stop", Icon::RotateCcw, tokens)
                    .id(SharedString::from("settings-daemon-stop"))
                    .on_click(cx.listener(|app, _, _, cx| app.stop_daemon(cx))),
            ),
    ))
    .child(action_row(
        "Repair",
        "Reinstall the LaunchAgent with the bundled daemon and load it.",
        tokens,
        buttons::toolbar_icon_button("Repair daemon", Icon::ShieldCheck, tokens)
            .id(SharedString::from("settings-daemon-repair"))
            .on_click(cx.listener(|app, _, _, cx| app.install_and_start_daemon(cx))),
    ))
}

fn diagnostics(tokens: LiquidGlassTokens, cx: &mut Context<NetworkManagerApp>) -> Div {
    section(
        "Diagnostics",
        "Open the local daemon logs when sync or pairing fails.",
        tokens,
    )
    .child(action_row(
        "Logs",
        "Open ~/Library/Logs/Network Manager in Finder.",
        tokens,
        buttons::toolbar_icon_button("Open logs", Icon::Folder, tokens)
            .id(SharedString::from("settings-open-logs"))
            .on_click(cx.listener(|app, _, _, cx| app.open_diagnostics_folder(cx))),
    ))
}

fn daemon_hint(vm: &SettingsVm) -> &'static str {
    if vm.daemon.stale {
        "The daemon has not reported recently."
    } else {
        match vm.daemon.state {
            AvailabilityState::Online => "Local daemon is answering sync requests.",
            AvailabilityState::Offline => "Start or repair the LaunchAgent.",
            AvailabilityState::Unknown => "Run a refresh or restart the daemon.",
        }
    }
}

fn section_stack() -> Div {
    div().flex().flex_col().gap(px(12.0))
}

fn section(title: &str, description: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .rounded(px(20.0))
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(14.0))
                .pt(px(14.0))
                .pb(px(10.0))
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(11.0))
                        .text_color(tokens.colors.text_muted)
                        .child(description.to_string()),
                ),
        )
}

fn status_row(label: &str, description: &str, value: &str, tokens: LiquidGlassTokens) -> Div {
    row(label, description, tokens).child(
        div()
            .w(px(150.0))
            .flex_none()
            .rounded(px(10.0))
            .bg(gpui::rgba(0xffffff0c))
            .px(px(10.0))
            .py(px(6.0))
            .font_family("Geist")
            .text_size(px(11.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(tokens.colors.text_secondary)
            .truncate()
            .child(value.to_string()),
    )
}

fn action_row(
    label: &str,
    description: &str,
    tokens: LiquidGlassTokens,
    controls: impl IntoElement,
) -> Div {
    row(label, description, tokens).child(controls)
}

fn row(label: &str, description: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .min_h(px(64.0))
        .px(px(14.0))
        .py(px(12.0))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .border_t_1()
        .border_color(tokens.colors.edge_soft)
        .child(
            div()
                .w(px(220.0))
                .flex_shrink()
                .overflow_hidden()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(11.0))
                        .text_color(tokens.colors.text_muted)
                        .child(description.to_string()),
                ),
        )
}
