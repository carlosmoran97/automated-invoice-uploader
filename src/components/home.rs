use crate::{components::loader::search_loader_widget, i18n::Messages};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{
        Block, BorderType, Borders, Clear, Paragraph,
        calendar::{CalendarEventStore, Monthly},
    },
};
use time::{Date, Duration, Month, OffsetDateTime};

#[derive(Default)]
pub struct HomePage {
    form: SearchCriteriaForm,
}

pub struct SearchCriteriaInput {
    pub initial_date: String,
    pub final_date: String,
}

pub enum HomePageAction {
    Continue,
    OpenSettings,
    Quit,
    Submit(SearchCriteriaInput),
}

pub enum HomePageStatus<'a> {
    Idle,
    Searching {
        initial_date: &'a str,
        final_date: &'a str,
        frame: usize,
    },
    Error(&'a str),
}

impl HomePage {
    pub fn render(&self, frame: &mut Frame, status: HomePageStatus<'_>, text: &'static Messages) {
        let area = centered_rect(frame.area(), 96, 26);
        let block = Block::bordered()
            .title(format!(" {} ", text.app_title))
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);
        let inner = block.inner(area);

        frame.render_widget(block, area);
        self.form.render(frame, inner, status, text);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> HomePageAction {
        match key.code {
            KeyCode::Char('q') => HomePageAction::Quit,
            KeyCode::Char('s') | KeyCode::Char('S') => HomePageAction::OpenSettings,
            _ => self
                .form
                .handle_key(key)
                .map_or(HomePageAction::Continue, HomePageAction::Submit),
        }
    }
}

struct SearchCriteriaForm {
    initial_date: Date,
    final_date: Date,
    focused_field: DateField,
    picker_open: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum DateField {
    #[default]
    Initial,
    Final,
}

impl Default for SearchCriteriaForm {
    fn default() -> Self {
        let today = today();
        let initial_date = first_day_of_month(today);
        let final_date = last_day_of_month(today);

        Self {
            initial_date,
            final_date,
            focused_field: DateField::Initial,
            picker_open: false,
        }
    }
}

impl SearchCriteriaForm {
    fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        status: HomePageStatus<'_>,
        text: &'static Messages,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Length(2),
                Constraint::Min(4),
            ])
            .split(area);

        let title = Paragraph::new(vec![
            Line::from(text.search_criteria_title)
                .style(Style::default().add_modifier(Modifier::BOLD)),
            Line::from(text.search_criteria_subtitle),
        ]);
        frame.render_widget(title, chunks[0]);

        let field_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        self.render_date_field(
            frame,
            field_chunks[0],
            text.initial_date,
            self.initial_date,
            DateField::Initial,
            text,
        );
        self.render_date_field(
            frame,
            field_chunks[1],
            text.final_date,
            self.final_date,
            DateField::Final,
            text,
        );

        let footer = Paragraph::new(vec![
            Line::from(text.form_footer),
            Line::from(text.calendar_footer),
        ])
        .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(footer, chunks[2]);

        frame.render_widget(self.status_widget(status, text), chunks[3]);

        if self.picker_open {
            let anchor = match self.focused_field {
                DateField::Initial => field_chunks[0],
                DateField::Final => field_chunks[1],
            };
            self.render_calendar_dropdown(frame, dropdown_rect(anchor, area, 36, 11), text);
        }
    }

    fn render_date_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        selected_date: Date,
        field: DateField,
        text: &Messages,
    ) {
        let focused = self.focused_field == field;
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {title} "))
            .border_style(if focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let hint = if focused && self.picker_open {
            text.calendar_is_open
        } else if focused {
            text.press_space_to_open
        } else {
            text.press_tab_to_focus
        };
        let field = Paragraph::new(vec![
            Line::from(format_date(selected_date)).style(
                Style::default()
                    .fg(field_color(field))
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(hint).style(Style::default().fg(Color::DarkGray)),
        ])
        .block(block);

        frame.render_widget(field, area);
    }

    fn render_calendar_dropdown(&self, frame: &mut Frame, area: Rect, text: &Messages) {
        let active_date = self.active_date();
        let block = Block::bordered()
            .title(format!(
                " {} ",
                match self.focused_field {
                    DateField::Initial => text.select_initial_date,
                    DateField::Final => text.select_final_date,
                }
            ))
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));
        let calendar = Monthly::new(active_date, self.calendar_events())
            .block(block)
            .show_month_header(Style::default().add_modifier(Modifier::BOLD))
            .show_weekdays_header(Style::default().fg(Color::DarkGray))
            .show_surrounding(Style::default().fg(Color::DarkGray))
            .default_style(Style::default().bg(Color::Black));

        frame.render_widget(Clear, area);
        frame.render_widget(calendar, area);
    }

    fn calendar_events(&self) -> CalendarEventStore {
        let mut events = CalendarEventStore::default();
        let initial_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let final_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD);
        let active_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        events.add(self.initial_date, initial_style);
        events.add(self.final_date, final_style);
        events.add(self.active_date(), active_style);

        events
    }

    fn status_widget<'a>(
        &self,
        status: HomePageStatus<'a>,
        text: &'static Messages,
    ) -> Paragraph<'a> {
        match status {
            HomePageStatus::Idle => {
                Paragraph::new(text.default_period).style(Style::default().fg(Color::DarkGray))
            }
            HomePageStatus::Searching {
                initial_date,
                final_date,
                frame,
            } => search_loader_widget(frame, initial_date, final_date, text),
            HomePageStatus::Error(error) => {
                Paragraph::new(error).style(Style::default().fg(Color::Red))
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<SearchCriteriaInput> {
        if self.picker_open {
            return self.handle_picker_key(key);
        }

        match key.code {
            KeyCode::Tab | KeyCode::BackTab => self.focus_next(),
            KeyCode::Char(' ') => self.picker_open = true,
            KeyCode::Enter | KeyCode::Char('s') => return Some(self.criteria()),
            _ => {}
        }

        None
    }

    fn handle_picker_key(&mut self, key: KeyEvent) -> Option<SearchCriteriaInput> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => self.picker_open = false,
            KeyCode::Tab | KeyCode::BackTab => {
                self.picker_open = false;
                self.focus_next();
            }
            KeyCode::Left | KeyCode::Char('h') => self.move_active_date(-1),
            KeyCode::Right | KeyCode::Char('l') => self.move_active_date(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_active_date(-7),
            KeyCode::Down | KeyCode::Char('j') => self.move_active_date(7),
            KeyCode::PageUp => self.move_active_month(-1),
            KeyCode::PageDown => self.move_active_month(1),
            KeyCode::Home => self.set_active_date_to_first_of_month(),
            KeyCode::End => self.set_active_date_to_last_of_month(),
            _ => {}
        }

        None
    }

    fn criteria(&self) -> SearchCriteriaInput {
        SearchCriteriaInput {
            initial_date: format_date(self.initial_date),
            final_date: format_date(self.final_date),
        }
    }

    fn focus_next(&mut self) {
        self.focused_field = match self.focused_field {
            DateField::Initial => DateField::Final,
            DateField::Final => DateField::Initial,
        };
    }

    fn active_date(&self) -> Date {
        match self.focused_field {
            DateField::Initial => self.initial_date,
            DateField::Final => self.final_date,
        }
    }

    fn active_date_mut(&mut self) -> &mut Date {
        match self.focused_field {
            DateField::Initial => &mut self.initial_date,
            DateField::Final => &mut self.final_date,
        }
    }

    fn move_active_date(&mut self, days: i64) {
        *self.active_date_mut() += Duration::days(days);
    }

    fn move_active_month(&mut self, months: i32) {
        *self.active_date_mut() = add_months(*self.active_date_mut(), months);
    }

    fn set_active_date_to_first_of_month(&mut self) {
        *self.active_date_mut() = first_day_of_month(*self.active_date_mut());
    }

    fn set_active_date_to_last_of_month(&mut self) {
        *self.active_date_mut() = last_day_of_month(*self.active_date_mut());
    }
}

fn today() -> Date {
    OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .date()
}

fn first_day_of_month(date: Date) -> Date {
    date.replace_day(1).expect("day 1 is valid for every month")
}

fn last_day_of_month(date: Date) -> Date {
    date.replace_day(date.month().length(date.year()))
        .expect("month length is always a valid day")
}

fn add_months(date: Date, months: i32) -> Date {
    let month_index = date.year() * 12 + i32::from(u8::from(date.month())) - 1 + months;
    let year = month_index.div_euclid(12);
    let month_number = month_index.rem_euclid(12) as u8 + 1;
    let month = Month::try_from(month_number).expect("month number is normalized to 1..=12");
    let day = date.day().min(month.length(year));

    Date::from_calendar_date(year, month, day).expect("normalized date must be valid")
}

fn format_date(date: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

fn field_color(field: DateField) -> Color {
    match field {
        DateField::Initial => Color::Cyan,
        DateField::Final => Color::Green,
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let horizontal_margin = area.width.saturating_sub(width) / 2;
    let vertical_margin = area.height.saturating_sub(height) / 2;

    Rect {
        x: area.x + horizontal_margin,
        y: area.y + vertical_margin,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn dropdown_rect(anchor: Rect, bounds: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(bounds.width);
    let height = height.min(bounds.height);
    let max_x = bounds.right().saturating_sub(width);
    let x = anchor.x.min(max_x);
    let preferred_y = anchor.bottom().saturating_add(1);
    let max_y = bounds.bottom().saturating_sub(height);
    let y = if preferred_y <= max_y {
        preferred_y
    } else {
        anchor
            .y
            .saturating_sub(height.saturating_add(1))
            .max(bounds.y)
    };

    Rect {
        x,
        y,
        width,
        height,
    }
}
