use gpui::{div, prelude::*, px, Context, Div, FontWeight, SharedString};
use network_manager_core::AvailabilityState;

use crate::app::{ActionStatus, NetworkManagerApp};
use crate::components::{
    buttons,
    icons::{self, Icon},
};
use crate::data::SettingsVm;
use crate::layout::app_shell::liquid_titlebar;
use crate::routes::SettingsSection;
use crate::theme::LiquidGlassTokens;

pub fn screen(
    vm: &SettingsVm,
    selected_section: SettingsSection,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .size_full()
        .relative()
        .child(liquid_titlebar(
            Icon::Settings,
            "Settings",
            &[Icon::Dashboard, Icon::RotateCcw],
            tokens,
            cx,
        ))
        .child(settings_sidebar(selected_section, tokens, cx))
        .child(settings_main(
            vm,
            selected_section,
            action_status,
            tokens,
            cx,
        ))
}

fn settings_sidebar(
    selected_section: SettingsSection,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .absolute()
        .left(px(24.0))
        .top(px(80.0))
        .w(px(270.0))
        .h(px(688.0))
        .rounded(px(22.0))
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .p(px(18.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .font_family("Geist")
                .text_size(px(20.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child("Settings"),
        )
        .child(div().h(px(8.0)))
        .children(SettingsSection::ALL.into_iter().map(|section| {
            let icon = match section {
                SettingsSection::Discovery => Icon::Radar,
                SettingsSection::EndpointPreference => Icon::Route,
                SettingsSection::IdentityCorrections => Icon::GitMerge,
                SettingsSection::CliAlias => Icon::Terminal,
                SettingsSection::Notifications => Icon::Bell,
            };
            settings_nav(section, icon, section == selected_section, tokens, cx)
        }))
}

fn settings_nav(
    section: SettingsSection,
    icon: Icon,
    selected: bool,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    let label = section.label();
    div()
        .h(px(38.0))
        .bg(gpui::rgba(0xffffff00))
        .px(px(10.0))
        .py(px(9.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(div().w(px(3.0)).h(px(20.0)).rounded_full().bg(if selected {
            tokens.colors.icy
        } else {
            gpui::rgba(0xffffff00).into()
        }))
        .child(icons::icon(
            icon,
            16.0,
            if selected {
                tokens.colors.text
            } else {
                tokens.colors.text_muted
            },
        ))
        .child(
            div()
                .font_family("Geist")
                .text_size(px(13.0))
                .font_weight(if selected {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(if selected {
                    tokens.colors.text
                } else {
                    tokens.colors.text_secondary
                })
                .child(label.to_string()),
        )
        .cursor_pointer()
        .id(SharedString::from(format!("settings-nav-{label}")))
        .on_click(cx.listener(move |app, _, _, cx| app.select_settings_section(section, cx)))
}

fn settings_main(
    vm: &SettingsVm,
    selected_section: SettingsSection,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    div()
        .absolute()
        .left(px(326.0))
        .top(px(80.0))
        .w(px(908.0))
        .h(px(688.0))
        .id(SharedString::from("settings-main-scroll"))
        .overflow_y_scroll()
        .pr(px(6.0))
        .flex()
        .flex_col()
        .gap(px(18.0))
        .child(settings_header(selected_section, tokens, cx))
        .when_some(action_status, |this, status| {
            this.child(action_status_banner(status, tokens))
        })
        .child(settings_columns(vm, selected_section, tokens, cx))
}

fn action_status_banner(status: &ActionStatus, tokens: LiquidGlassTokens) -> Div {
    let tone = if status.is_error {
        tokens.colors.offline
    } else if status.is_pending {
        tokens.colors.icy
    } else {
        tokens.colors.text_secondary
    };

    div()
        .rounded(px(14.0))
        .bg(tokens.colors.panel)
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .px(px(12.0))
        .py(px(10.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(icons::icon(Icon::Info, 14.0, tone))
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .font_family("Geist")
                .text_size(px(12.0))
                .text_color(tokens.colors.text_secondary)
                .truncate()
                .child(status.message.clone()),
        )
}

fn settings_header(
    selected_section: SettingsSection,
    tokens: LiquidGlassTokens,
    _cx: &mut Context<NetworkManagerApp>,
) -> Div {
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
                        .child(selected_section.label().to_string()),
                )
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(13.0))
                        .text_color(tokens.colors.text_muted)
                        .child(settings_section_description(selected_section)),
                ),
        )
        .child(
            div()
                .id(SharedString::from("settings-saved"))
                .rounded(px(10.0))
                .bg(gpui::rgba(0xffffff0d))
                .px(px(9.0))
                .py(px(6.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(icons::icon(Icon::Check, 13.0, tokens.colors.text_secondary))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text_secondary)
                        .child("Saved"),
                ),
        )
}

fn settings_section_description(section: SettingsSection) -> &'static str {
    match section {
        SettingsSection::Discovery => {
            "Control which local evidence sources are observed by this Mac."
        }
        SettingsSection::EndpointPreference => {
            "Define how LAN, mDNS, DNS, and Tailscale endpoints are ranked for SSH."
        }
        SettingsSection::IdentityCorrections => {
            "Review user-led merges, splits, ignored devices, and correction import/export."
        }
        SettingsSection::CliAlias => {
            "Configure agent-friendly aliases, copied SSH commands, and CLI fallback behavior."
        }
        SettingsSection::Notifications => {
            "Tune local-only feedback, refresh status, and troubleshooting visibility."
        }
    }
}

fn settings_columns(
    vm: &SettingsVm,
    selected_section: SettingsSection,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    match selected_section {
        SettingsSection::Discovery => discovery_settings(vm, tokens, cx),
        SettingsSection::EndpointPreference => endpoint_preference_settings(tokens, cx),
        SettingsSection::IdentityCorrections => identity_correction_settings(vm, tokens, cx),
        SettingsSection::CliAlias => cli_alias_settings(vm, tokens, cx),
        SettingsSection::Notifications => notification_settings(vm, tokens, cx),
    }
}

fn settings_grid() -> Div {
    div().min_h(px(610.0)).grid().grid_cols(2).gap(px(16.0))
}

fn settings_stack() -> Div {
    div().flex().flex_col().gap(px(12.0))
}

fn discovery_settings(
    vm: &SettingsVm,
    tokens: LiquidGlassTokens,
    _cx: &mut Context<NetworkManagerApp>,
) -> Div {
    settings_grid()
        .child(
            settings_stack()
                .child(
                    section(
                        "Discovery Scope",
                        "Discovery observations remain evidence, not stable identity on their own.",
                        tokens,
                    )
                    .child(setting_toggle(
                        "Scan local interfaces",
                        "Observe devices on selected private network interfaces.",
                        true,
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Include Tailscale network",
                        "Use Tailscale presence as discovery source evidence.",
                        vm.tailscale_enabled,
                        tokens,
                    ))
                    .child(setting_menu(
                        "Scan interval",
                        "Controls how often discovery observations are refreshed.",
                        &vm.discovery_interval,
                        tokens,
                    )),
                )
                .child(
                    section(
                        "Passive Signals",
                        "Evidence from caches is kept separate from active probes.",
                        tokens,
                    )
                    .child(setting_toggle(
                        "Read ARP cache",
                        "Use already-observed LAN neighbors without broad scanning.",
                        true,
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Bonjour services",
                        "Use mDNS service records like _ssh._tcp as discovery evidence.",
                        true,
                        tokens,
                    ))
                    .child(setting_menu(
                        "Tailscale service",
                        "Status reported by the local tailscale CLI.",
                        availability_label(vm.tailscale_status),
                        tokens,
                    )),
                ),
        )
        .child(
            settings_stack()
                .child(
                    section(
                        "Refresh Behavior",
                        "Refreshes are serialized by the daemon and safe to trigger often.",
                        tokens,
                    )
                    .child(setting_menu(
                        "Quick refresh",
                        "Refresh active status sources without rebuilding identity evidence.",
                        "Status only",
                        tokens,
                    ))
                    .child(setting_menu(
                        "Full refresh",
                        "Re-read discovery sources and recompute endpoint reachability.",
                        "Discovery + status",
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Passive IPv6",
                        "Record observed IPv6 endpoints, but never actively scan IPv6 ranges.",
                        true,
                        tokens,
                    )),
                )
                .child(
                    section(
                        "Visibility",
                        "Discovery lists every observed identity; Dashboard remains tracked-only.",
                        tokens,
                    )
                    .child(setting_toggle(
                        "Show untracked devices",
                        "Keep observable devices visible until explicitly ignored.",
                        true,
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Collapse duplicate evidence",
                        "Group endpoints that resolve to one device identity.",
                        true,
                        tokens,
                    )),
                ),
        )
}

fn endpoint_preference_settings(
    tokens: LiquidGlassTokens,
    _cx: &mut Context<NetworkManagerApp>,
) -> Div {
    settings_grid()
        .child(
            settings_stack()
                .child(
                    section(
                        "Endpoint Ranking",
                        "Only proven-reachable endpoints are eligible for preferred SSH targets.",
                        tokens,
                    )
                    .child(setting_toggle(
                        "Prefer LAN when reachable",
                        "Choose local network endpoints before Tailscale for SSH actions.",
                        true,
                        tokens,
                    ))
                    .child(setting_menu(
                        "Fallback policy",
                        "Choose the next endpoint when the preferred SSH target is unavailable.",
                        "Tailscale then mDNS",
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Unknown availability",
                        "Treat stale observations as unknown rather than offline.",
                        true,
                        tokens,
                    )),
                )
                .child(
                    section(
                        "SSH Target Assembly",
                        "Destination strings stay deterministic for CLI and agent workflows.",
                        tokens,
                    )
                    .child(setting_menu(
                        "Username source",
                        "Use the device override first, then the current macOS user.",
                        "Device then local user",
                        tokens,
                    ))
                    .child(setting_menu(
                        "Port source",
                        "Use discovered SSH service ports before the default port.",
                        "Discovered then 22",
                        tokens,
                    )),
                ),
        )
        .child(
            settings_stack()
                .child(
                    section(
                        "Reachability Proof",
                        "LAN/Tailscale reachability is measured separately from SSH capability.",
                        tokens,
                    )
                    .child(setting_toggle(
                        "Probe SSH separately",
                        "Do not infer SSH readiness from mDNS service records alone.",
                        true,
                        tokens,
                    ))
                    .child(setting_menu(
                        "Stale timeout",
                        "Mark old endpoint checks unknown instead of offline.",
                        "Daemon managed",
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Prefer stable hostnames",
                        "Use DNS or mDNS names before raw LAN IPs when both are reachable.",
                        true,
                        tokens,
                    )),
                )
                .child(
                    section(
                        "Tailscale Fallback",
                        "Tailscale is a fallback path, not a replacement for local reachability.",
                        tokens,
                    )
                    .child(setting_menu(
                        "Tailnet name",
                        "Resolve MagicDNS names when the local Tailscale service is available.",
                        "MagicDNS if present",
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Use Tailscale SSH",
                        "Reserved for future opt-in; standard ssh remains default.",
                        false,
                        tokens,
                    )),
                ),
        )
}

fn identity_correction_settings(
    vm: &SettingsVm,
    tokens: LiquidGlassTokens,
    _cx: &mut Context<NetworkManagerApp>,
) -> Div {
    settings_grid()
        .child(
            settings_stack()
                .child(
                    section(
                        "Merge and Split",
                        "User corrections override automatic identity matching.",
                        tokens,
                    )
                    .child(setting_menu(
                        "Auto-merge threshold",
                        "Automatically merge only when identity confidence is high.",
                        "High confidence",
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Preserve split decisions",
                        "Prevent rediscovery from recombining identities you split.",
                        true,
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Show correction history",
                        "Expose recent merge/split events in the detail inspector.",
                        true,
                        tokens,
                    )),
                )
                .child(
                    section(
                        "Ignored Devices",
                        "Ignoring hides noise without deleting observed evidence.",
                        tokens,
                    )
                    .child(setting_toggle(
                        "Hide ignored identities",
                        "Remove ignored devices from normal discovery attention.",
                        true,
                        tokens,
                    ))
                    .child(setting_menu(
                        "Restore ignored",
                        "Review and restore ignored identities from local history.",
                        "Open review list",
                        tokens,
                    )),
                ),
        )
        .child(
            settings_stack()
                .child(
                    section(
                        "Portable User Settings",
                        "Export user intent and corrections, not volatile discovery cache.",
                        tokens,
                    )
                    .child(setting_toggle(
                        "Include tracked intent",
                        "Preserve which identities are intentionally tracked.",
                        true,
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Include aliases and labels",
                        "Preserve human labels and CLI-friendly aliases.",
                        true,
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Export SSH config hints",
                        "Include optional SSH helper metadata in portable settings.",
                        vm.ssh_config_export,
                        tokens,
                    )),
                )
                .child(
                    section(
                        "Conflict Handling",
                        "Imported corrections match stable device evidence when available.",
                        tokens,
                    )
                    .child(setting_menu(
                        "Import conflicts",
                        "Choose how to handle local labels that differ from imported labels.",
                        "Keep local first",
                        tokens,
                    ))
                    .child(setting_toggle(
                        "Log correction decisions",
                        "Keep a local audit trail for identity correction troubleshooting.",
                        vm.debug_logging,
                        tokens,
                    )),
                ),
        )
}

fn cli_alias_settings(
    vm: &SettingsVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    settings_grid()
        .child(
            settings_stack()
                .child(section(
                    "Alias Generation",
                    "Aliases are unique, deterministic names for CLI and agent lookup.",
                    tokens,
                )
                .child(setting_menu(
                    "Device alias format",
                    "Generate CLI-friendly names from labels and discovery evidence.",
                    "kebab-case",
                    tokens
                ))
                .child(setting_toggle(
                    "Reserve old aliases",
                    "Keep renamed aliases from being reused by another identity.",
                    true,
                    tokens
                ))
                .child(setting_toggle(
                    "Allow manual alias edits",
                    "Edit aliases for discovered identities without creating devices manually.",
                    true,
                    tokens
                )))
                .child(section(
                    "CLI Fallback",
                    "Commands prefer daemon IPC and fall back to SQLite only on connection failure.",
                    tokens,
                )
                .child(setting_toggle(
                    "Require daemon flag",
                    "Expose --require-daemon for scripts that must avoid SQLite fallback.",
                    true,
                    tokens
                ))
                .child(setting_menu(
                    "Output errors",
                    "Return deterministic machine-readable errors for JSON commands.",
                    "Structured JSON",
                    tokens
                )))
                .child(daemon_launch_agent_settings(tokens, cx)),
        )
        .child(
            settings_stack()
                .child(section(
                    "Copy Commands",
                    "Buttons copy commands; they do not launch shells from the app UI.",
                    tokens,
                )
                .child(setting_menu(
                    "Copy command format",
                    "Controls the command copied from action buttons and Quick Access.",
                    "ssh {target}",
                    tokens
                ))
                .child(setting_toggle(
                    "Include explicit port",
                    "Append -p only when the preferred endpoint uses a non-default port.",
                    true,
                    tokens
                ))
                .child(setting_toggle(
                    "Copy reasoning",
                    "Include why this endpoint was preferred in detail metadata.",
                    vm.debug_logging,
                    tokens
                )))
                .child(section(
                    "Agent Mode",
                    "Keep CLI responses concise, stable, and easy for agents to parse.",
                    tokens,
                )
                .child(setting_toggle(
                    "JSON by default in scripts",
                    "Prefer explicit JSON output when stdout is consumed by automation.",
                    true,
                    tokens
                ))
                .child(setting_toggle(
                    "Suppress color in pipes",
                    "Avoid ANSI escapes when stdout is not an interactive terminal.",
                    true,
                    tokens
                ))),
        )
}

fn daemon_launch_agent_settings(
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    section(
        "Daemon LaunchAgent",
        "Install or control the per-user daemon that keeps local evidence fresh.",
        tokens,
    )
    .child(
        div()
            .px(px(14.0))
            .py(px(12.0))
            .border_t_1()
            .border_color(tokens.colors.edge_soft)
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                buttons::accent_icon_button("Install", Icon::Plus, tokens)
                    .id(SharedString::from("daemon-install-start"))
                    .on_click(cx.listener(|app, _, _, cx| app.install_and_start_daemon(cx))),
            )
            .child(
                buttons::toolbar_icon_button("Start", Icon::Refresh, tokens)
                    .id(SharedString::from("daemon-start"))
                    .on_click(cx.listener(|app, _, _, cx| app.start_daemon(cx))),
            )
            .child(
                buttons::toolbar_icon_button("Stop", Icon::RotateCcw, tokens)
                    .id(SharedString::from("daemon-stop"))
                    .on_click(cx.listener(|app, _, _, cx| app.stop_daemon(cx))),
            ),
    )
}

fn notification_settings(
    vm: &SettingsVm,
    tokens: LiquidGlassTokens,
    _cx: &mut Context<NetworkManagerApp>,
) -> Div {
    settings_grid()
        .child(
            settings_stack()
                .child(section(
                    "Local Feedback",
                    "Network Manager stays local-only; no cloud telemetry is sent.",
                    tokens,
                )
                .child(setting_toggle(
                    "Show compact action status",
                    "Use small state changes instead of large notification panels.",
                    false,
                    tokens
                ))
                .child(setting_toggle(
                    "Battery-aware scans",
                    "Reduce optional refresh work when macOS reports battery mode.",
                    vm.battery_mode,
                    tokens,
                ))
                .child(setting_toggle(
                    "Debug logging",
                    "Write additional local diagnostics for troubleshooting.",
                    vm.debug_logging,
                    tokens,
                )))
                .child(section(
                    "Status Changes",
                    "Keep noisy endpoint churn out of the fixed-size artboards.",
                    tokens,
                )
                .child(setting_menu(
                    "Availability alerts",
                    "Choose when tracked devices should surface status changes.",
                    "Tracked only",
                    tokens,
                ))
                .child(setting_toggle(
                    "Notify SSH target changes",
                    "Surface preferred target changes caused by reachability proof.",
                    false,
                    tokens,
                ))),
        )
        .child(
            settings_stack()
                .child(section(
                    "Menu Bar",
                    "Quick Access mirrors tracked-device reachability for fast SSH copy actions.",
                    tokens,
                )
                .child(setting_toggle(
                    "Show Quick Access summary",
                    "Display LAN, Tailnet, and Unknown counts in the popover.",
                    true,
                    tokens,
                ))
                .child(setting_menu(
                    "Popover refresh",
                    "How the popover refresh button requests daemon work.",
                    "Quick refresh",
                    tokens,
                )))
                .child(section(
                    "Troubleshooting",
                    "Expose local diagnostics without interrupting normal navigation.",
                    tokens,
                )
                .child(setting_menu(
                    "Daemon messages",
                    "Where daemon refresh failures are shown in the UI.",
                    "Status only",
                    tokens,
                ))
                .child(setting_toggle(
                    "Persist transient notices",
                    "Keep action messages in history rather than as large banners.",
                    false,
                    tokens,
                ))),
        )
}

fn availability_label(state: AvailabilityState) -> &'static str {
    match state {
        AvailabilityState::Online => "Online",
        AvailabilityState::Offline => "Offline",
        AvailabilityState::Unknown => "Unknown",
    }
}

fn section(title: &str, footnote: &str, tokens: LiquidGlassTokens) -> Div {
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
                        .child(footnote.to_string()),
                ),
        )
}

fn setting_toggle(label: &str, detail: &str, on: bool, tokens: LiquidGlassTokens) -> Div {
    let value = if on { "On" } else { "Off" };
    setting_row(label, detail, tokens).child(
        setting_value_badge(value, tokens)
            .id(SharedString::from(format!("setting-toggle-{label}"))),
    )
}

fn setting_menu(label: &str, detail: &str, value: &str, tokens: LiquidGlassTokens) -> Div {
    setting_row(label, detail, tokens).child(
        setting_value_badge(value, tokens).id(SharedString::from(format!("setting-menu-{label}"))),
    )
}

fn setting_row(label: &str, detail: &str, tokens: LiquidGlassTokens) -> Div {
    div()
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
                .w(px(240.0))
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
                        .truncate()
                        .child(detail.to_string()),
                ),
        )
}

fn setting_value_badge(value: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(160.0))
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
        .child(value.to_string())
}
