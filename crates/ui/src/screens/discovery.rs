use gpui::{div, prelude::*, Div, FontWeight};
use network_manager_core::TrackedState;

use crate::components::{buttons, forms, glass, status, table as table_components};
use crate::data::{DiscoveryRowVm, DiscoveryVm};
use crate::layout::table::DISCOVERY_COLUMNS;
use crate::theme::LiquidGlassTokens;

pub fn screen(vm: &DiscoveryVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap_4()
        .child(toolbar(tokens))
        .child(filters(vm, tokens))
        .when_some(vm.possible_match.as_ref(), |this, note| {
            this.child(glass::system_note("Possible match", note, tokens))
        })
        .child(discovery_table(&vm.rows, tokens))
}

fn toolbar(tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_start()
        .justify_between()
        .child(glass::header(
            "Discovery",
            "All devices observable from this Mac. Select what should be tracked.",
            tokens,
        ))
        .child(buttons::toolbar_button("Full scan", tokens))
}

fn filters(vm: &DiscoveryVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(forms::search_field(
            "Search devices, aliases, endpoints",
            tokens,
        ))
        .child(
            div().flex().gap_2().children(
                vm.filters
                    .iter()
                    .enumerate()
                    .map(|(index, filter)| forms::filter_chip(filter, index == 1, tokens)),
            ),
        )
}

fn discovery_table(rows: &[DiscoveryRowVm], tokens: LiquidGlassTokens) -> Div {
    glass::panel(tokens)
        .p_3()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div().flex().gap_3().items_center().h_8().px_2().children(
                DISCOVERY_COLUMNS
                    .into_iter()
                    .map(|(label, width)| table_components::header_cell(label, width, tokens)),
            ),
        )
        .children(rows.iter().map(|row| discovery_row(row, tokens)))
}

fn discovery_row(row: &DiscoveryRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .px_2()
        .h(gpui::px(58.0))
        .rounded_lg()
        .border_1()
        .border_color(tokens.colors.edge_soft)
        .bg(tokens.colors.panel)
        .child(device_cell(row, tokens))
        .child(source_cell(row, tokens))
        .child(text_cell(&row.category, DISCOVERY_COLUMNS[2].1, tokens))
        .child(
            div()
                .w(gpui::px(DISCOVERY_COLUMNS[3].1))
                .child(status::status_label(row.availability, tokens)),
        )
        .child(text_cell(&row.last_seen, DISCOVERY_COLUMNS[4].1, tokens))
        .child(
            div()
                .w(gpui::px(DISCOVERY_COLUMNS[5].1))
                .child(buttons::action_button(
                    action_label(row.tracked_state),
                    tokens,
                )),
        )
}

fn device_cell(row: &DiscoveryRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(gpui::px(DISCOVERY_COLUMNS[0].1))
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens.colors.text)
                .child(row.display_name.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(tokens.colors.text_muted)
                .child(row.source.clone()),
        )
}

fn source_cell(row: &DiscoveryRowVm, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(gpui::px(DISCOVERY_COLUMNS[1].1))
        .flex()
        .gap_1()
        .children(
            row.sources
                .iter()
                .map(|source| table_components::source_pill(source, tokens)),
        )
}

fn text_cell(text: &str, width: f32, tokens: LiquidGlassTokens) -> Div {
    div()
        .w(gpui::px(width))
        .text_xs()
        .text_color(tokens.colors.text_secondary)
        .child(text.to_string())
}

fn action_label(state: TrackedState) -> &'static str {
    match state {
        TrackedState::Tracked => "Tracked",
        TrackedState::Ignored => "Ignored",
        TrackedState::Untracked => "Track",
    }
}
