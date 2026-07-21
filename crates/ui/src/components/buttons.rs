use gpui::{div, prelude::*, px, Div, FontWeight};

use crate::components::icons::{self, Icon};
use crate::theme::LiquidGlassTokens;

pub fn toolbar_icon_button(label: &str, icon: Icon, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(9.0))
        .rounded(px(12.0))
        .bg(gpui::rgba(0xffffff14))
        .hover(|style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .font_family("Geist")
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens.colors.text)
        .child(icons::icon(icon, 14.0, tokens.colors.text_secondary))
        .child(label.to_string())
}

pub fn small_icon_button(icon: Icon, accent: bool, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(28.0))
        .h(px(28.0))
        .rounded(px(10.0))
        .bg(gpui::rgba(0xffffff0b))
        .hover(|style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .child(icons::icon(
            icon,
            14.0,
            if accent {
                tokens.colors.icy
            } else {
                tokens.colors.text_secondary
            },
        ))
}

pub fn action_button(label: &str, tokens: LiquidGlassTokens) -> Div {
    action_button_base(label, None, false, tokens)
}

pub fn action_icon_button(label: &str, icon: Icon, tokens: LiquidGlassTokens) -> Div {
    action_button_base(label, Some(icon), false, tokens)
}

pub fn accent_icon_button(label: &str, icon: Icon, tokens: LiquidGlassTokens) -> Div {
    action_button_base(label, Some(icon), true, tokens)
}

pub fn disabled_icon_button(label: &str, icon: Icon, tokens: LiquidGlassTokens) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .px(px(9.0))
        .py(px(6.0))
        .rounded(px(10.0))
        .bg(gpui::rgba(0xffffff0a))
        .font_family("Geist")
        .text_size(px(11.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens.colors.text_secondary)
        .child(icons::icon(icon, 13.0, tokens.colors.text_secondary))
        .child(label.to_string())
}

fn action_button_base(
    label: &str,
    icon: Option<Icon>,
    accent: bool,
    tokens: LiquidGlassTokens,
) -> Div {
    let fg = if accent {
        tokens.colors.text
    } else {
        tokens.colors.text_secondary
    };
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .px(px(9.0))
        .py(px(6.0))
        .rounded(px(10.0))
        .bg(if accent {
            gpui::rgba(0xffffff18)
        } else {
            gpui::rgba(0xffffff0d)
        })
        .hover(|style| style.bg(tokens.colors.selected))
        .cursor_pointer()
        .font_family("Geist")
        .text_size(px(11.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(fg)
        .when_some(icon, |this, icon| this.child(icons::icon(icon, 13.0, fg)))
        .child(label.to_string())
}
