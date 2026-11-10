use crate::app::{App, UiAction};
use gpui::{
    Context, Div, ElementId, FontWeight, IntoElement, KeyDownEvent, SharedString, Stateful, div,
    prelude::*, px, rgb,
};

pub const BLUE: u32 = 0x007bff;
pub const INK: u32 = 0x212529;
pub const MUTED: u32 = 0x6c757d;
pub const BORDER: u32 = 0xdee2e6;
pub const PALE: u32 = 0xe7f1ff;
pub const BACKGROUND: u32 = 0xf5f6fa;

pub fn card() -> Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .bg(rgb(0xffffff))
        .border_1()
        .border_color(rgb(BORDER))
        .rounded_lg()
}

pub fn heading(title: impl Into<SharedString>, subtitle: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(26.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.into()),
        )
        .child(div().text_color(rgb(MUTED)).child(subtitle.into()))
}

pub fn field(label: &'static str, value: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .flex_1()
        .min_w_0()
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .child(label),
        )
        .child(value)
}

pub fn detail(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Div {
    let value = value.into();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .min_w_0()
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(MUTED))
                .child(label.into()),
        )
        .child(div().text_size(px(14.)).child(if value.is_empty() {
            SharedString::from("\u{2014}")
        } else {
            value
        }))
}

pub fn notice(message: impl Into<SharedString>, error: bool) -> Div {
    div()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(rgb(if error { 0xf1b6bc } else { BORDER }))
        .bg(rgb(if error { 0xfff0f1 } else { PALE }))
        .text_color(rgb(if error { 0x9f2431 } else { 0x164f86 }))
        .child(message.into())
}

pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    primary: bool,
    enabled: bool,
    action: UiAction,
    cx: &mut Context<App>,
) -> Stateful<Div> {
    let click_action = action.clone();
    div()
        .id(id)
        .tab_index(0)
        .tab_stop(enabled)
        .flex()
        .items_center()
        .justify_center()
        .px_4()
        .h(px(42.))
        .rounded_md()
        .border_1()
        .border_color(rgb(if primary { BLUE } else { BORDER }))
        .bg(rgb(if primary { BLUE } else { 0xffffff }))
        .text_color(rgb(if primary { 0xffffff } else { INK }))
        .text_size(px(14.))
        .font_weight(FontWeight::MEDIUM)
        .when(!enabled, |s| s.opacity(0.45))
        .when(enabled, |s| {
            s.cursor_pointer()
                .hover(move |s| s.bg(rgb(if primary { 0x0069d9 } else { BACKGROUND })))
        })
        .focus(|s| s.border_color(rgb(INK)))
        .child(label.into())
        .on_click(cx.listener(move |this, _, window, cx| {
            if enabled {
                this.dispatch(click_action.clone(), window, cx);
            }
        }))
        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
            if enabled && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                this.dispatch(action.clone(), window, cx);
                cx.stop_propagation();
            }
        }))
}
