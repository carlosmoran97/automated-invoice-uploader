use super::widgets::{BLUE, BORDER, INK, MUTED, PALE};
use crate::i18n::{Language, messages_for};
use gpui::{
    Context, EventEmitter, FocusHandle, FontWeight, IntoElement, KeyDownEvent, Window, div,
    prelude::*, px, rgb,
};
use time::{Date, Duration, Month};

pub enum CalendarEvent {
    Selected(Date),
    Dismiss,
}

pub struct Calendar {
    selected: Date,
    cursor: Date,
    language: Language,
    focus: FocusHandle,
}

impl EventEmitter<CalendarEvent> for Calendar {}

impl Calendar {
    pub fn new(date: Date, language: Language, cx: &mut Context<Self>) -> Self {
        Self {
            selected: date,
            cursor: date,
            language,
            focus: cx.focus_handle().tab_stop(true),
        }
    }

    pub fn open(
        &mut self,
        date: Date,
        language: Language,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected = date;
        self.cursor = date;
        self.language = language;
        self.focus.focus(window);
        cx.notify();
    }

    fn move_month(&mut self, delta: i32, cx: &mut Context<Self>) {
        self.cursor = add_months(self.cursor, delta);
        cx.notify();
    }
}

impl Render for Calendar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text = messages_for(self.language);
        let first = self.cursor.replace_day(1).unwrap();
        let offset = first.weekday().number_days_from_monday() as i64;
        let title = format!(
            "{} {}",
            text.months[u8::from(self.cursor.month()) as usize - 1],
            self.cursor.year()
        );
        let mut weeks = Vec::new();
        for week in 0..6 {
            let mut row = div().flex().gap_1();
            for day in 0..7 {
                let date = first
                    .checked_add(Duration::days(week * 7 + day - offset))
                    .unwrap_or(first);
                let focused = date == self.cursor;
                let selected = date == self.selected;
                row = row.child(
                    div()
                        .id(("calendar-day", (week * 7 + day) as usize))
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(38.))
                        .h(px(34.))
                        .rounded_md()
                        .bg(if selected {
                            rgb(BLUE)
                        } else if focused {
                            rgb(PALE)
                        } else {
                            rgb(0xffffff)
                        })
                        .border_1()
                        .border_color(if focused { rgb(BLUE) } else { rgb(0xffffff) })
                        .text_color(if selected {
                            rgb(0xffffff)
                        } else if date.month() != self.cursor.month() {
                            rgb(MUTED)
                        } else {
                            rgb(INK)
                        })
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(PALE)).text_color(rgb(INK)))
                        .child(date.day().to_string())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selected = date;
                            this.cursor = date;
                            cx.emit(CalendarEvent::Selected(date));
                        })),
                );
            }
            weeks.push(row);
        }
        div()
            .id("calendar")
            .track_focus(&self.focus)
            .key_context("Calendar")
            .w(px(328.))
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(rgb(0xffffff))
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .shadow_lg()
            .text_size(px(13.))
            .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(CalendarEvent::Dismiss)))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let days = match event.keystroke.key.as_str() {
                    "left" => -1,
                    "right" => 1,
                    "up" => -7,
                    "down" => 7,
                    _ => 0,
                };
                if days != 0 {
                    this.cursor = this
                        .cursor
                        .checked_add(Duration::days(days))
                        .unwrap_or(this.cursor);
                }
                match event.keystroke.key.as_str() {
                    "pageup" => this.cursor = add_months(this.cursor, -1),
                    "pagedown" => this.cursor = add_months(this.cursor, 1),
                    "home" => this.cursor = this.cursor.replace_day(1).unwrap(),
                    "end" => {
                        this.cursor = this
                            .cursor
                            .replace_day(this.cursor.month().length(this.cursor.year()))
                            .unwrap()
                    }
                    "enter" | "space" => cx.emit(CalendarEvent::Selected(this.cursor)),
                    "escape" => cx.emit(CalendarEvent::Dismiss),
                    _ if days == 0 => return,
                    _ => {}
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .id("previous-month")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .hover(|s| s.bg(rgb(PALE)))
                            .child("\u{2039}")
                            .on_click(cx.listener(|this, _, _, cx| this.move_month(-1, cx))),
                    )
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(
                        div()
                            .id("next-month")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .hover(|s| s.bg(rgb(PALE)))
                            .child("\u{203a}")
                            .on_click(cx.listener(|this, _, _, cx| this.move_month(1, cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .children(text.weekdays.iter().map(|day| {
                        div()
                            .w(px(38.))
                            .text_center()
                            .text_color(rgb(MUTED))
                            .child(*day)
                    })),
            )
            .children(weeks)
    }
}

pub fn add_months(date: Date, delta: i32) -> Date {
    let index = date.year() * 12 + i32::from(u8::from(date.month())) - 1 + delta;
    let year = index.div_euclid(12);
    let month = Month::try_from((index.rem_euclid(12) + 1) as u8).unwrap();
    Date::from_calendar_date(year, month, date.day().min(month.length(year))).unwrap_or(date)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn month_navigation_clamps_days_and_crosses_years() {
        let january = Date::from_calendar_date(2024, Month::January, 31).unwrap();
        assert_eq!(add_months(january, 1).to_string(), "2024-02-29");
        assert_eq!(add_months(january, -1).to_string(), "2023-12-31");
        let march = Date::from_calendar_date(2026, Month::March, 31).unwrap();
        assert_eq!(add_months(march, -1).to_string(), "2026-02-28");
    }
}
