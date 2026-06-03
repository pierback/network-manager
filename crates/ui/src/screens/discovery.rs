use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, InteractiveElement, SharedString,
    StatefulInteractiveElement,
};
use network_manager_core::{AvailabilityState, TrackedState};

use crate::app::{ActionStatus, NetworkManagerApp};
use crate::components::{buttons, glass, status, table as table_components};
use crate::data::{DiscoveryRowVm, DiscoveryVm};
use crate::layout::table::DISCOVERY_COLUMNS;
use crate::theme::LiquidGlassTokens;

pub fn screen(
    vm: &DiscoveryVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(px(20.0))
        .pt(px(60.0))
        .pr(px(32.0))
        .pb(px(32.0))
        .pl(px(32.0))
        .child(toolbar(vm, action_status, tokens, cx))
        .when_some(action_status, |this, status| {
            this.child(action_note(status, tokens))
        })
        .child(filters(tokens))
        .when_some(vm.possible_match.as_ref(), |this, note| {
            this.child(glass::system_note("Possible match", note, tokens))
        })
        .child(discovery_table(&vm.rows, tokens, cx))
}

fn toolbar(
    vm: &DiscoveryVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let scan_label = if action_status.is_some_and(|status| status.is_pending) {
        "Scanning"
    } else {
        "Full scan"
    };
    div()
        .w_full()
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
                        .font_family("Inter")
                        .text_size(px(22.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens.colors.text)
                        .child("Discovery"),
                )
                .child(
                    div()
                        .font_family("Inter")
                        .text_size(px(13.0))
                        .text_color(tokens.colors.text_secondary)
                        .child(format!("{} devices in discovery scope", vm.rows.len())),
                ),
        )
        .child(
            buttons::toolbar_button(scan_label, tokens)
                .id(SharedString::from("discovery-full-scan"))
                .on_click(cx.listener(|app, _, _, cx| app.refresh_full(cx))),
        )
}

fn filters(tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(search_field(tokens))
        .child(filter_button("All Sources", tokens))
        .child(filter_button("Status", tokens))
        .child(filter_button("Category", tokens))
        .child(filter_button("Tracked", tokens))
}

fn search_field(tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(280.0))
        .h(px(34.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .child(
            div()
                .text_size(px(14.0))
                .text_color(tokens.colors.text_muted)
                .child("⌕"),
        )
        .child(
            div()
                .font_family("Inter")
                .text_size(px(12.0))
                .text_color(tokens.colors.text_muted)
                .child("Search devices..."),
        )
}

fn filter_button(label: &str, tokens: LiquidGlassTokens) -> Div {
    div()
        .h(px(34.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(12.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .font_family("Inter")
        .text_size(px(12.0))
        .text_color(tokens.colors.text_secondary)
        .child(label.to_string())
        .child(
            div()
                .text_size(px(12.0))
                .text_color(tokens.colors.text_muted)
                .child("⌄"),
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

fn discovery_table(
    rows: &[DiscoveryRowVm],
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(table_header(tokens))
        .when(rows.is_empty(), |this| this.child(empty_state(tokens, cx)))
        .children(rows.iter().map(|row| discovery_row(row, tokens, cx)))
}

fn table_header(tokens: LiquidGlassTokens) -> Div {
    div()
        .h(px(32.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(16.0))
        .border_b_1()
        .border_color(tokens.colors.edge_soft)
        .children(
            DISCOVERY_COLUMNS
                .into_iter()
                .map(|(label, width)| table_components::header_cell(label, width, tokens)),
        )
}

fn empty_state(tokens: LiquidGlassTokens, cx: &mut Context<NetworkManagerApp>) -> Div {
    div()
        .h(px(48.0))
        .flex()
        .items_center()
        .justify_between()
        .rounded(px(6.0))
        .bg(tokens.colors.panel)
        .px(px(16.0))
        .child(
            div()
                .font_family("Inter")
                .text_size(px(13.0))
                .text_color(tokens.colors.text_secondary)
                .child("No devices discovered yet"),
        )
        .child(
            buttons::accent_button("Track after scan", tokens)
                .id(SharedString::from("discovery-empty-scan"))
                .on_click(cx.listener(|app, _, _, cx| app.refresh_full(cx))),
        )
}

fn discovery_row(
    row: &DiscoveryRowVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .h(px(48.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(16.0))
        .rounded(px(6.0))
        .bg(tokens.colors.panel)
        .child(name_cell(row, tokens))
        .child(mono_cell(
            &row.hostname,
            DISCOVERY_COLUMNS[1].1,
            tokens.colors.text_muted,
        ))
        .child(mono_cell(
            &row.ip_address,
            DISCOVERY_COLUMNS[2].1,
            tokens.colors.text_secondary,
        ))
        .child(source_cell(row, tokens))
        .child(status_text_cell(row.availability, tokens))
        .child(action_cell(row, tokens, cx))
}

fn name_cell(row: &DiscoveryRowVm, tokens: LiquidGlassTokens) -> Div {
    let tracked = row.tracked_state == TrackedState::Tracked;
    div()
        .w(px(DISCOVERY_COLUMNS[0].1))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(status::status_mini_dot(row.availability, tokens))
        .child(
            div()
                .font_family("Inter")
                .text_size(px(13.0))
                .font_weight(if tracked {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if tracked {
                    tokens.colors.text
                } else {
                    tokens.colors.text_secondary
                })
                .child(row.display_name.clone()),
        )
}

fn source_cell(row: &DiscoveryRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(DISCOVERY_COLUMNS[3].1))
        .flex()
        .gap(px(4.0))
        .children(
            row.sources
                .iter()
                .take(3)
                .map(|source| table_components::source_pill(source, tokens)),
        )
}

fn status_text_cell(state: AvailabilityState, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(px(DISCOVERY_COLUMNS[4].1))
        .font_family("Inter")
        .text_size(px(12.0))
        .text_color(tokens.status_color(state))
        .child(status::status_text(state))
}

fn action_cell(
    row: &DiscoveryRowVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let cell = div().w(px(DISCOVERY_COLUMNS[5].1)).flex().justify_end();
    match row.tracked_state {
        TrackedState::Tracked => cell.child(buttons::disabled_button("✓ Tracked", tokens)),
        TrackedState::Ignored | TrackedState::Untracked => {
            let Some(identity_id) = row.identity_id.clone() else {
                return cell.child(buttons::disabled_button("No identity", tokens));
            };
            cell.child(
                buttons::accent_button("+ Track", tokens)
                    .id(SharedString::from(format!("discovery-track-{}", row.id)))
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.set_discovery_identity_state(
                            Some(identity_id.clone()),
                            TrackedState::Tracked,
                            cx,
                        )
                    })),
            )
        }
    }
}

fn mono_cell(text: &str, width: f32, color: gpui::Hsla) -> Div {
    div()
        .w(px(width))
        .font_family("Geist Mono")
        .text_size(px(11.0))
        .text_color(color)
        .child(if text.is_empty() { "—" } else { text }.to_string())
}
