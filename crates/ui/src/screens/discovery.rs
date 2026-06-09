use gpui::{div, prelude::*, px, AnyElement, Context, Div, FontWeight, SharedString};
use network_manager_core::TrackedState;

use crate::app::NetworkManagerApp;
use crate::components::{
    buttons,
    icons::{self, Icon},
    status,
};
use crate::data::{ActionStatus, DiscoveryRowVm, DiscoveryVm};
use crate::layout::app_shell::v4_route_shell;
use crate::routes::{DiscoveryFilter, Route};
use crate::theme::LiquidGlassTokens;

pub fn screen(
    vm: &DiscoveryVm,
    selected_filter: DiscoveryFilter,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let main = discovery_main(vm, selected_filter, action_status, tokens, cx);
    v4_route_shell(
        Route::Discovery,
        Icon::Radar,
        "Discovery",
        &[Icon::Refresh, Icon::SlidersHorizontal, Icon::Settings],
        true,
        main,
        tokens,
        cx,
    )
}

fn discovery_main(
    vm: &DiscoveryVm,
    selected_filter: DiscoveryFilter,
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
        .id(SharedString::from("discovery-main-scroll"))
        .overflow_y_scroll()
        .pr(px(6.0))
        .flex()
        .flex_col()
        .gap(px(18.0))
        .child(header(vm, tokens, cx))
        .child(search_filters(selected_filter, tokens, cx))
        .when_some(action_status, |this, status| {
            this.child(discovery_status_banner(status, tokens))
        })
        .child(discovery_body(vm, selected_filter, tokens, cx))
}

fn header(
    _vm: &DiscoveryVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
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
                        .child("Discovered Devices"),
                )
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(13.0))
                        .text_color(tokens.colors.text_muted)
                        .child("All devices observed inside this Mac's discovery scope."),
                ),
        )
        .child(
            buttons::toolbar_icon_button("Refresh", Icon::Refresh, tokens)
                .id(SharedString::from("discovery-refresh"))
                .on_click(cx.listener(|app, _, _, cx| app.refresh_quick(cx))),
        )
}

fn search_filters(
    selected_filter: DiscoveryFilter,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .h(px(52.0))
        .flex()
        .items_center()
        .gap(px(14.0))
        .child(search_field(tokens))
        .children(
            DiscoveryFilter::ALL
                .into_iter()
                .map(|filter| filter_chip(filter, filter == selected_filter, tokens, cx)),
        )
}

fn search_field(tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(430.0))
        .h_full()
        .rounded(px(16.0))
        .bg(gpui::rgba(0xffffff0d))
        .px(px(16.0))
        .flex()
        .items_center()
        .gap(px(11.0))
        .child(icons::icon(Icon::Search, 18.0, tokens.colors.text_muted))
        .child(
            div()
                .font_family("Geist")
                .text_size(px(14.0))
                .text_color(tokens.colors.text_muted)
                .child("Search labels, aliases, endpoints"),
        )
}

fn filter_chip(
    filter: DiscoveryFilter,
    selected: bool,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!(
            "discovery-filter-{}",
            filter.label()
        )))
        .h_full()
        .rounded(px(14.0))
        .bg(if selected {
            gpui::rgba(0xffffff16)
        } else {
            gpui::rgba(0xffffff0a)
        })
        .px(px(16.0))
        .flex()
        .items_center()
        .font_family("Geist")
        .text_size(px(14.0))
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
        .cursor_pointer()
        .on_click(cx.listener(move |app, _, _, cx| app.select_discovery_filter(filter, cx)))
        .child(filter.label().to_string())
}

fn discovery_body(
    vm: &DiscoveryVm,
    selected_filter: DiscoveryFilter,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    let rows = filtered_rows(vm, selected_filter);
    let has_visible_rows = !rows.is_empty();
    let grouped_rows = grouped_rows_by_type(rows, tokens, cx);
    div()
        .id(SharedString::from("discovery-list-scroll"))
        .h(px(550.0))
        .w_full()
        .overflow_y_scroll()
        .pr(px(6.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .children(grouped_rows)
        .when(!has_visible_rows, |this| {
            this.child(empty_discovery_state(vm.rows.is_empty(), tokens))
        })
}

fn filtered_rows(vm: &DiscoveryVm, selected_filter: DiscoveryFilter) -> Vec<&DiscoveryRowVm> {
    vm.rows
        .iter()
        .filter(|row| match selected_filter {
            DiscoveryFilter::AllSources => true,
            DiscoveryFilter::Lan => row.sources.iter().any(|source| source == "LAN"),
            DiscoveryFilter::Tailscale => row.sources.iter().any(|source| source == "Tailscale"),
            DiscoveryFilter::SshCapable => row.ssh_capable,
            DiscoveryFilter::Untracked => row.tracked_state == TrackedState::Untracked,
        })
        .collect()
}

fn grouped_rows_by_type(
    mut rows: Vec<&DiscoveryRowVm>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Vec<AnyElement> {
    rows.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });

    let mut elements = Vec::new();
    let mut current_category: Option<&str> = None;
    for row in rows {
        if current_category != Some(row.category.as_str()) {
            current_category = Some(row.category.as_str());
            elements.push(type_group_header(&row.category, tokens).into_any_element());
        }
        elements.push(discovery_row(row, tokens, cx).into_any_element());
    }
    elements
}

fn type_group_header(category: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .h(px(28.0))
        .px(px(4.0))
        .flex()
        .items_end()
        .font_family("Geist Mono")
        .text_size(px(10.0))
        .font_weight(FontWeight::BOLD)
        .text_color(tokens.colors.text_muted)
        .child(category.to_ascii_uppercase())
}

fn discovery_row(
    row: &DiscoveryRowVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let display_name = row.display_name.replace('\n', " ");
    let endpoint_text = endpoint_summary(row);
    div()
        .h(px(88.0))
        .rounded(px(16.0))
        .bg(if row.tracked_state == TrackedState::Tracked {
            gpui::rgba(0xffffff10)
        } else {
            gpui::rgba(0xffffff0a)
        })
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .hover(|style| style.bg(gpui::rgba(0xffffff14)))
        .px(px(16.0))
        .flex()
        .items_center()
        .gap(px(14.0))
        .child(availability_badge(row, tokens))
        .child(
            div()
                .w(px(330.0))
                .overflow_hidden()
                .flex()
                .flex_col()
                .gap(px(5.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .truncate()
                        .child(display_name),
                )
                .child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(11.0))
                        .text_color(tokens.colors.text_muted)
                        .truncate()
                        .child(endpoint_text),
                ),
        )
        .child(
            div()
                .w(px(244.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .children(
                    row.sources
                        .iter()
                        .take(2)
                        .map(|source| source_badge(source, tokens)),
                )
                .when(row.ssh_capable, |this| {
                    this.child(source_badge("SSH", tokens))
                }),
        )
        .child(
            div()
                .w(px(122.0))
                .overflow_hidden()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(tokens.colors.text_secondary)
                        .truncate()
                        .child(row.category.clone()),
                )
                .child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(10.0))
                        .text_color(tokens.colors.text_muted)
                        .child(row.last_seen.clone()),
                ),
        )
        .child(track_button(row, tokens, cx))
}

fn track_button(
    row: &DiscoveryRowVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> AnyElement {
    let next_state = if row.tracked_state == TrackedState::Tracked {
        TrackedState::Untracked
    } else {
        TrackedState::Tracked
    };
    match row.identity_id.clone() {
        Some(identity_id) => if row.tracked_state == TrackedState::Tracked {
            buttons::action_icon_button("Untrack", Icon::Check, tokens)
        } else {
            buttons::accent_icon_button("Track", Icon::Plus, tokens)
        }
        .w(px(86.0))
        .flex_none()
        .id(SharedString::from(format!("discovery-{}-track", row.id)))
        .on_click(cx.listener(move |app, _, _, cx| {
            app.set_discovery_identity_state(Some(identity_id.clone()), next_state, cx)
        }))
        .into_any_element(),
        None => buttons::disabled_icon_button("No ID", Icon::Info, tokens)
            .w(px(86.0))
            .flex_none()
            .into_any_element(),
    }
}

fn discovery_status_banner(status: &ActionStatus, tokens: LiquidGlassTokens) -> Div {
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

fn empty_discovery_state(is_empty: bool, tokens: LiquidGlassTokens) -> Div {
    let (title, subtitle) = if is_empty {
        (
            "No discovered devices",
            "Refresh discovery to populate devices from LAN and Tailscale sources.",
        )
    } else {
        (
            "No matching devices",
            "Change the active filter to show a broader set of discovered devices.",
        )
    };
    div()
        .h_full()
        .rounded(px(22.0))
        .bg(gpui::rgba(0xffffff0a))
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(10.0))
                .child(icons::icon(Icon::Radar, 26.0, tokens.colors.text_muted))
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(15.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens.colors.text)
                        .child(title),
                )
                .child(
                    div()
                        .font_family("Geist")
                        .text_size(px(12.0))
                        .text_color(tokens.colors.text_muted)
                        .child(subtitle),
                ),
        )
}

fn availability_badge(row: &DiscoveryRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(88.0))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(status::status_dot(row.availability, tokens))
        .child(
            div()
                .font_family("Geist")
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(tokens.colors.text_secondary)
                .child(status::status_text(row.availability)),
        )
}

fn endpoint_summary(row: &DiscoveryRowVm) -> String {
    let hostname = clean_endpoint_part(&row.hostname);
    let ip_address = clean_endpoint_part(&row.ip_address);
    match (hostname.as_deref(), ip_address.as_deref()) {
        (Some(hostname), Some(ip_address)) => format!("{hostname} · {ip_address}"),
        (Some(hostname), None) => hostname.to_string(),
        (None, Some(ip_address)) => ip_address.to_string(),
        (None, None) => "No endpoint recorded".to_string(),
    }
}

fn clean_endpoint_part(value: &str) -> Option<String> {
    let value = value.replace('\n', " ");
    let value = value.trim();
    if value.is_empty() || value == "—" {
        None
    } else {
        Some(value.to_string())
    }
}

fn source_badge(source: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .rounded(px(10.0))
        .bg(gpui::rgba(0xffffff0d))
        .px(px(8.0))
        .py(px(4.0))
        .font_family("Geist Mono")
        .text_size(px(10.0))
        .font_weight(FontWeight::BOLD)
        .text_color(tokens.colors.text_secondary)
        .child(source.to_string())
}
