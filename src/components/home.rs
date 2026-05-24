use crate::{components::loader::search_loader_widget, i18n::Messages};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
    pub dte_query_filter: String,
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
    pub fn with_dte_query_filter(mut self, dte_query_filter: impl Into<String>) -> Self {
        self.form.dte_query_filter = dte_query_filter.into();
        self
    }

    pub fn set_dte_query_filter(&mut self, dte_query_filter: impl Into<String>) {
        self.form.dte_query_filter = dte_query_filter.into();
    }

    pub fn render(&self, frame: &mut Frame, status: HomePageStatus<'_>, text: &'static Messages) {
        let area = centered_rect(frame.area(), 100, 30);
        let block = Block::bordered()
            .title(format!(" {} ", text.app_title))
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);
        let inner = block.inner(area);

        frame.render_widget(block, area);
        self.form.render(frame, inner, status, text);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> HomePageAction {
        if self.form.is_text_edit_key(key) {
            return self
                .form
                .handle_key(key)
                .map_or(HomePageAction::Continue, HomePageAction::Submit);
        }

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
    dte_query_filter: String,
    focused_field: SearchField,
    picker_open: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum SearchField {
    #[default]
    InitialDate,
    FinalDate,
    DteQueryFilter,
}

impl Default for SearchCriteriaForm {
    fn default() -> Self {
        let today = today();
        let initial_date = first_day_of_month(today);
        let final_date = last_day_of_month(today);

        Self {
            initial_date,
            final_date,
            dte_query_filter: String::new(),
            focused_field: SearchField::InitialDate,
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
            SearchField::InitialDate,
            text,
        );
        self.render_date_field(
            frame,
            field_chunks[1],
            text.final_date,
            self.final_date,
            SearchField::FinalDate,
            text,
        );
        self.render_filter_field(frame, chunks[2], text);

        let footer = Paragraph::new(vec![
            Line::from(text.form_footer),
            Line::from(text.calendar_footer),
        ])
        .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(footer, chunks[3]);

        frame.render_widget(self.status_widget(status, text), chunks[4]);

        if self.picker_open {
            let anchor = match self.focused_field {
                SearchField::InitialDate => field_chunks[0],
                SearchField::FinalDate => field_chunks[1],
                SearchField::DteQueryFilter => return,
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
        field: SearchField,
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

    fn render_filter_field(&self, frame: &mut Frame, area: Rect, text: &Messages) {
        let focused = self.focused_field == SearchField::DteQueryFilter;
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", text.dte_query_filter_label))
            .border_style(if focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            });
        let value = if self.dte_query_filter.is_empty() {
            " "
        } else {
            &self.dte_query_filter
        };
        let hint = if focused {
            text.dte_query_filter_hint
        } else {
            text.press_tab_to_focus
        };
        let field = Paragraph::new(vec![
            Line::from(value).style(Style::default().fg(if focused {
                Color::LightYellow
            } else {
                Color::White
            })),
            Line::from(hint).style(Style::default().fg(Color::DarkGray)),
        ])
        .block(block);

        frame.render_widget(field, area);
    }

    fn render_calendar_dropdown(&self, frame: &mut Frame, area: Rect, text: &Messages) {
        let Some(active_date) = self.active_date() else {
            return;
        };
        let block = Block::bordered()
            .title(format!(
                " {} ",
                match self.focused_field {
                    SearchField::InitialDate => text.select_initial_date,
                    SearchField::FinalDate => text.select_final_date,
                    SearchField::DteQueryFilter => return,
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

        events.add(self.initial_date, initial_style);
        events.add(self.final_date, final_style);
        if let Some(active_date) = self.active_date() {
            let active_style = Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD);
            events.add(active_date, active_style);
        }

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

    fn is_text_edit_key(&self, key: KeyEvent) -> bool {
        self.focused_field == SearchField::DteQueryFilter
            && match key.code {
                KeyCode::Char(_) => {
                    !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT)
                }
                KeyCode::Backspace => true,
                _ => false,
            }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<SearchCriteriaInput> {
        if self.picker_open {
            return self.handle_picker_key(key);
        }

        match key.code {
            KeyCode::Tab => self.focus_next(),
            KeyCode::BackTab => self.focus_previous(),
            KeyCode::Char(' ') if self.is_date_field_focused() => self.picker_open = true,
            KeyCode::Backspace if self.focused_field == SearchField::DteQueryFilter => {
                self.dte_query_filter.pop();
            }
            KeyCode::Char(character) if self.focused_field == SearchField::DteQueryFilter => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    self.dte_query_filter.push(character);
                }
            }
            KeyCode::Enter => return Some(self.criteria()),
            _ => {}
        }

        None
    }

    fn handle_picker_key(&mut self, key: KeyEvent) -> Option<SearchCriteriaInput> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => self.picker_open = false,
            KeyCode::Tab => {
                self.picker_open = false;
                self.focus_next();
            }
            KeyCode::BackTab => {
                self.picker_open = false;
                self.focus_previous();
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
            dte_query_filter: self.dte_query_filter.clone(),
        }
    }

    fn focus_next(&mut self) {
        self.focused_field = match self.focused_field {
            SearchField::InitialDate => SearchField::FinalDate,
            SearchField::FinalDate => SearchField::DteQueryFilter,
            SearchField::DteQueryFilter => SearchField::InitialDate,
        };
    }

    fn focus_previous(&mut self) {
        self.focused_field = match self.focused_field {
            SearchField::InitialDate => SearchField::DteQueryFilter,
            SearchField::FinalDate => SearchField::InitialDate,
            SearchField::DteQueryFilter => SearchField::FinalDate,
        };
    }

    fn is_date_field_focused(&self) -> bool {
        matches!(
            self.focused_field,
            SearchField::InitialDate | SearchField::FinalDate
        )
    }

    fn active_date(&self) -> Option<Date> {
        match self.focused_field {
            SearchField::InitialDate => Some(self.initial_date),
            SearchField::FinalDate => Some(self.final_date),
            SearchField::DteQueryFilter => None,
        }
    }

    fn active_date_mut(&mut self) -> Option<&mut Date> {
        match self.focused_field {
            SearchField::InitialDate => Some(&mut self.initial_date),
            SearchField::FinalDate => Some(&mut self.final_date),
            SearchField::DteQueryFilter => None,
        }
    }

    fn move_active_date(&mut self, days: i64) {
        if let Some(active_date) = self.active_date_mut() {
            *active_date += Duration::days(days);
        }
    }

    fn move_active_month(&mut self, months: i32) {
        if let Some(active_date) = self.active_date_mut() {
            *active_date = add_months(*active_date, months);
        }
    }

    fn set_active_date_to_first_of_month(&mut self) {
        if let Some(active_date) = self.active_date_mut() {
            *active_date = first_day_of_month(*active_date);
        }
    }

    fn set_active_date_to_last_of_month(&mut self) {
        if let Some(active_date) = self.active_date_mut() {
            *active_date = last_day_of_month(*active_date);
        }
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

fn field_color(field: SearchField) -> Color {
    match field {
        SearchField::InitialDate => Color::Cyan,
        SearchField::FinalDate => Color::Green,
        SearchField::DteQueryFilter => Color::White,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_criteria_include_dte_query_filter() {
        let mut form = SearchCriteriaForm::default();
        form.dte_query_filter = "DTE-03".to_string();

        let criteria = form.criteria();

        assert_eq!(criteria.dte_query_filter, "DTE-03");
    }

    #[test]
    fn dte_query_filter_captures_shortcut_letters_as_text() {
        let mut page = HomePage::default().with_dte_query_filter("");
        page.form.focused_field = SearchField::DteQueryFilter;

        let action = page.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(matches!(action, HomePageAction::Continue));

        let action = page.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert!(matches!(action, HomePageAction::Continue));
        assert_eq!(page.form.dte_query_filter, "qs");
    }
}
