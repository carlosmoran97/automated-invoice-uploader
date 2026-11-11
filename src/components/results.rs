use super::widgets::*;
use crate::app::{App, UiAction, state::Screen};
use gpui::{
    Context, Div, FontWeight, ListSizingBehavior, MouseButton, Stateful, div, prelude::*, px, rgb,
    uniform_list,
};

impl App {
    pub(crate) fn render_results(&self, cx: &mut Context<Self>) -> Div {
        let text = self.text();
        let count = self.state.results.len();
        let details = card()
            .id("email-details")
            .min_h_0()
            .overflow_y_scroll()
            .w(px(320.))
            .flex_shrink_0()
            .gap_5()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(text.selected_email_title),
            )
            .map(|panel| {
                if let Some(email) = self.state.results.get(self.state.highlighted) {
                    panel
                        .child(detail(text.from_label, email.from.clone()))
                        .child(detail(text.subject_label, email.subject.clone()))
                        .child(detail(text.email_date_label, email.received_at.clone()))
                        .child(detail(
                            text.pdf_label,
                            email
                                .pdf_attachments
                                .iter()
                                .map(|a| a.filename.as_str())
                                .collect::<Vec<_>>()
                                .join("\n"),
                        ))
                        .child(detail(
                            text.json_label,
                            email
                                .json_attachments
                                .iter()
                                .map(|a| a.filename.as_str())
                                .collect::<Vec<_>>()
                                .join("\n"),
                        ))
                        .child(detail(text.snippet_label, email.snippet.clone()))
                } else {
                    panel.child(text.no_email_selected)
                }
            });
        let list = div()
            .id("email-list")
            .track_focus(&self.results_focus)
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .bg(rgb(0xffffff))
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, _| this.results_focus.focus(window)),
            )
            .on_key_down(cx.listener(Self::results_key))
            .focus(|s| s.border_color(rgb(BLUE)))
            .child(
                div()
                    .flex_shrink_0()
                    .px_4()
                    .py_3()
                    .font_weight(FontWeight::SEMIBOLD)
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(text.results_header),
            )
            .map(|panel| {
                if count == 0 {
                    panel.child(
                        div()
                            .p_6()
                            .text_color(rgb(MUTED))
                            .child(text.no_matching_emails),
                    )
                } else {
                    panel.child(
                        uniform_list(
                            "candidate-emails",
                            count,
                            cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                                range
                                    .map(|index| this.email_row(index, cx))
                                    .collect::<Vec<_>>()
                            }),
                        )
                        .w_full()
                        .flex_1()
                        .min_h_0()
                        .with_sizing_behavior(ListSizingBehavior::Auto)
                        .track_scroll(self.results_scroll.clone()),
                    )
                }
            });
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_6()
            .child(
                heading(
                    text.results_title,
                    format!(
                        "{}  \u{b7}  {}",
                        text.candidate_count(count),
                        self.state.search_period
                    ),
                )
                .flex_shrink_0(),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_5()
                    .child(list)
                    .child(details),
            )
    }

    pub(crate) fn results_actions(&self, cx: &mut Context<Self>) -> Div {
        let text = self.text();
        let count = self.state.results.len();
        let selected = self.state.selected_ids.len();
        action_bar()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(button(
                        "results-back",
                        text.back_button,
                        false,
                        true,
                        UiAction::Navigate(Screen::Home),
                        cx,
                    ))
                    .child(
                        div()
                            .text_color(rgb(MUTED))
                            .child(text.selected_candidate_count(selected)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(button(
                        "toggle-all",
                        if selected == count {
                            text.deselect_all_button
                        } else {
                            text.select_all_button
                        },
                        false,
                        count > 0,
                        UiAction::ToggleAll,
                        cx,
                    ))
                    .child(button(
                        "review",
                        text.review_selected_button,
                        true,
                        selected > 0,
                        UiAction::ReviewSelected,
                        cx,
                    )),
            )
    }

    fn email_row(&self, index: usize, cx: &mut Context<Self>) -> Stateful<Div> {
        let text = self.text();
        let email = &self.state.results[index];
        let checked = self.state.selected_ids.contains(&email.id);
        let highlighted = self.state.highlighted == index;
        let subject = if email.subject.is_empty() {
            text.no_subject.to_owned()
        } else {
            email.subject.clone()
        };
        let from = if email.from.is_empty() {
            text.unknown_sender.to_owned()
        } else {
            email.from.clone()
        };
        div()
            .id(("email", index))
            .w_full()
            .flex()
            .items_center()
            .gap_3()
            .h(px(84.))
            .px_4()
            .border_b_1()
            .border_color(rgb(BORDER))
            .bg(rgb(if highlighted { PALE } else { 0xffffff }))
            .cursor_pointer()
            .hover(|s| s.bg(rgb(BACKGROUND)))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.dispatch(UiAction::Highlight(index), window, cx)
            }))
            .child(
                div()
                    .id(("checkbox", index))
                    .flex_shrink_0()
                    .w(px(22.))
                    .h(px(22.))
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(if checked { BLUE } else { MUTED }))
                    .bg(rgb(if checked { BLUE } else { 0xffffff }))
                    .text_color(rgb(0xffffff))
                    .text_center()
                    .child(if checked { "\u{2713}" } else { "" })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.dispatch(UiAction::ToggleEmail(index), window, cx);
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .truncate()
                            .font_weight(FontWeight::MEDIUM)
                            .child(subject),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(rgb(MUTED))
                            .child(from),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(11.))
                            .text_color(rgb(MUTED))
                            .child(email.received_at.clone()),
                    ),
            )
    }
}
