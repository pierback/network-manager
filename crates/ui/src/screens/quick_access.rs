use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, InteractiveElement, SharedString,
    StatefulInteractiveElement,
};

use crate::app::{ActionStatus, NetworkManagerApp};
use crate::components::{buttons, status};
use crate::data::{QuickAccessVm, TrackedDeviceRowVm};
use crate::routes::Route;
use crate::theme::LiquidGlassTokens;

pub fn screen(
    vm: &QuickAccessVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(popover(vm, action_status, tokens, cx))
}

fn popover(
    vm: &QuickAccessVm,
    action_status: Option<&ActionStatus>,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> Div {
    let refresh_label = if action_status.is_some_and(|status| status.is_pending) {
        "…"
    } else {
        "↻"
    };

    div()
        .w(px(320.0))
        .h(px(460.0))
        .rounded(px(12.0))
        .border_1()
        .border_color(tokens.colors.edge)
        .bg(tokens.colors.popover)
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(16.0))
                .pt(px(14.0))
                .pb(px(10.0))
                .flex()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(tokens.colors.edge_soft)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .w(px(14.0))
                                .h(px(14.0))
                                .rounded(px(4.0))
                                .bg(tokens.colors.accent),
                        )
                        .child(
                            div()
                                .font_family("Inter")
                                .text_size(px(13.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(tokens.colors.text)
                                .child("Network Manager"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            buttons::small_icon_button(refresh_label, false, tokens)
                                .id(SharedString::from("quick-access-refresh"))
                                .on_click(cx.listener(|app, _, _, cx| app.refresh_quick(cx))),
                        )
                        .child(
                            buttons::small_icon_button("↗", false, tokens)
                                .id(SharedString::from("quick-access-open-dashboard-top"))
                                .on_click(
                                    cx.listener(|app, _, _, cx| {
                                        app.set_route(Route::Dashboard, cx)
                                    }),
                                ),
                        ),
                ),
        )
        .child(
            div()
                .flex_1()
                .px(px(8.0))
                .py(px(6.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .when(vm.rows.is_empty(), |this| this.child(empty_row(tokens)))
                .children(vm.rows.iter().map(|row| popover_row(row, tokens, cx))),
        )
        .child(
            div()
                .px(px(16.0))
                .pt(px(8.0))
                .pb(px(10.0))
                .flex()
                .items_center()
                .justify_between()
                .border_t_1()
                .border_color(tokens.colors.edge_soft)
                .child(
                    div()
                        .font_family("Inter")
                        .text_size(px(11.0))
                        .text_color(tokens.colors.text_muted)
                        .child(format!("Last scan: {}", vm.last_scan)),
                )
                .child(
                    div()
                        .id(SharedString::from("quick-access-open-dashboard"))
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .font_family("Inter")
                        .text_size(px(11.0))
                        .text_color(tokens.colors.accent)
                        .cursor_pointer()
                        .child("Open Dashboard")
                        .child("→")
                        .on_click(cx.listener(|app, _, _, cx| app.set_route(Route::Dashboard, cx))),
                ),
        )
}

fn empty_row(tokens: LiquidGlassTokens) -> Div {
    div()
        .px(px(10.0))
        .py(px(8.0))
        .font_family("Inter")
        .text_size(px(12.0))
        .text_color(tokens.colors.text_muted)
        .child("No tracked devices")
}

fn popover_row(
    row: &TrackedDeviceRowVm,
    tokens: LiquidGlassTokens,
    cx: &mut Context<NetworkManagerApp>,
) -> impl IntoElement {
    let identity_id = row.id.clone();
    div()
        .id(SharedString::from(format!("quick-row-{}", row.id)))
        .on_click(
            cx.listener(move |app, _, _, cx| app.select_device_detail(identity_id.clone(), cx)),
        )
        .rounded(px(6.0))
        .hover(|style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .px(px(10.0))
        .py(px(8.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(status::status_dot(row.overall, tokens))
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
                        .child(row.label.clone()),
                )
                .child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(10.0))
                        .text_color(tokens.colors.text_muted)
                        .child(row.preferred_target.clone()),
                ),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(buttons::small_icon_button("⌘", true, tokens))
                .child(buttons::small_icon_button("⧉", false, tokens)),
        )
}
