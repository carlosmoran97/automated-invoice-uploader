use super::widgets::*;
use crate::app::{App, UiAction};
use gpui::{
    AnchoredPositionMode, Context, Div, FontWeight, anchored, deferred, div, point, prelude::*, px,
    rgb,
};

impl App {
    fn date_field(
        &self,
        initial: bool,
        label: &'static str,
        value: &str,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        field(
            label,
            button(
                if initial {
                    "initial-date"
                } else {
                    "final-date"
                },
                format!("{value}   \u{25be}"),
                false,
                enabled,
                UiAction::OpenCalendar(initial),
                cx,
            ),
        )
        .relative()
        .when(self.calendar_open == Some(initial), |s| {
            s.child(
                deferred(
                    anchored()
                        .position_mode(AnchoredPositionMode::Local)
                        .position(point(px(0.), px(76.)))
                        .snap_to_window_with_margin(px(16.))
                        .child(if initial {
                            self.initial_calendar.clone()
                        } else {
                            self.final_calendar.clone()
                        }),
                )
                .with_priority(10),
            )
        })
    }

    pub(crate) fn render_home(&self, cx: &mut Context<Self>) -> Div {
        let text = self.text();
        let enabled = !self.state.busy();
        let initial = self.state.criteria.initial_date.to_string();
        let final_date = self.state.criteria.final_date.to_string();
        div()
            .w_full()
            .max_w(px(900.))
            .mx_auto()
            .flex()
            .flex_col()
            .gap_6()
            .child(heading(
                text.search_criteria_title,
                text.search_criteria_subtitle,
            ))
            .child(
                card()
                    .gap_5()
                    .child(
                        div()
                            .text_color(rgb(BLUE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(text.period_label),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_5()
                            .child(self.date_field(true, text.initial_date, &initial, enabled, cx))
                            .child(self.date_field(
                                false,
                                text.final_date,
                                &final_date,
                                enabled,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(MUTED))
                            .child(text.default_period),
                    )
                    .child(field(
                        text.dte_query_filter_label,
                        self.search_filter.clone(),
                    ))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(MUTED))
                            .child(text.dte_query_filter_hint),
                    )
                    .child(div().flex().justify_end().child(button(
                        "search",
                        text.search_button,
                        true,
                        enabled,
                        UiAction::Search,
                        cx,
                    ))),
            )
            .when(self.state.searching, |s| {
                s.child(notice(
                    format!("{}  {}", text.searching_gmail, self.state.search_period),
                    false,
                ))
            })
            .when_some(self.state.error.clone(), |s, error| {
                s.child(notice(error, true))
            })
    }
}
