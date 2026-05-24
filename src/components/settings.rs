use crate::{
    i18n::{Language, Messages},
    services::settings::AppSettings,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

#[derive(Default)]
pub struct SettingsPage {
    focused_field: SettingsField,
    download_dir: String,
    drive_root_folder: String,
    dte_query_filter: String,
    language: Language,
    status: Option<SettingsStatus>,
}

pub struct SettingsInput {
    pub download_dir: String,
    pub drive_root_folder: String,
    pub dte_query_filter: String,
    pub language: Language,
}

pub enum SettingsPageAction {
    Continue,
    Back,
    Quit,
    Save(SettingsInput),
}

enum SettingsStatus {
    Saved,
    Error(String),
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum SettingsField {
    #[default]
    DownloadDir,
    DriveRootFolder,
    DteQueryFilter,
    Language,
}

impl SettingsPage {
    pub fn reset(&mut self, settings: &AppSettings) {
        self.focused_field = SettingsField::DownloadDir;
        self.download_dir = settings.download_dir.display().to_string();
        self.drive_root_folder = settings.drive_root_folder.clone();
        self.dte_query_filter = settings.dte_query_filter.clone();
        self.language = settings.language;
        self.status = None;
    }

    pub fn mark_saved(&mut self) {
        self.status = Some(SettingsStatus::Saved);
    }

    pub fn mark_error(&mut self, message: String) {
        self.status = Some(SettingsStatus::Error(message));
    }

    pub fn render(&self, frame: &mut Frame, text: &'static Messages) {
        let area = centered_rect(frame.area(), 104, 26);
        let block = Block::bordered()
            .title(format!(" {} ", text.settings_title))
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);
        let inner = block.inner(area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(inner);

        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(
                Line::from(text.settings_header)
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .wrap(Wrap { trim: true }),
            chunks[0],
        );
        self.render_input(
            frame,
            chunks[1],
            text.download_dir_label,
            &self.download_dir,
            SettingsField::DownloadDir,
        );
        self.render_input(
            frame,
            chunks[2],
            text.drive_root_folder_label,
            &self.drive_root_folder,
            SettingsField::DriveRootFolder,
        );
        self.render_input(
            frame,
            chunks[3],
            text.dte_query_filter_label,
            &self.dte_query_filter,
            SettingsField::DteQueryFilter,
        );
        self.render_language(frame, chunks[4], text);
        self.render_status(frame, chunks[5], text);
        frame.render_widget(
            Paragraph::new(text.settings_footer).style(Style::default().fg(Color::DarkGray)),
            chunks[6],
        );
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SettingsPageAction {
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SettingsPageAction::Quit
            }
            KeyCode::Esc => SettingsPageAction::Back,
            KeyCode::Enter => SettingsPageAction::Save(SettingsInput {
                download_dir: self.download_dir.clone(),
                drive_root_folder: self.drive_root_folder.clone(),
                dte_query_filter: self.dte_query_filter.clone(),
                language: self.language,
            }),
            KeyCode::Tab => {
                self.focus_next();
                SettingsPageAction::Continue
            }
            KeyCode::BackTab => {
                self.focus_previous();
                SettingsPageAction::Continue
            }
            KeyCode::Backspace => {
                self.delete_character();
                SettingsPageAction::Continue
            }
            KeyCode::Char(' ') if self.focused_field == SettingsField::Language => {
                self.toggle_language();
                SettingsPageAction::Continue
            }
            KeyCode::Left | KeyCode::Right if self.focused_field == SettingsField::Language => {
                self.toggle_language();
                SettingsPageAction::Continue
            }
            KeyCode::Char(character) => {
                self.push_character(character);
                SettingsPageAction::Continue
            }
            _ => SettingsPageAction::Continue,
        }
    }

    fn render_input(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        field: SettingsField,
    ) {
        let focused = self.focused_field == field;
        let block = field_block(label, focused);
        let value = if value.is_empty() { " " } else { value };

        frame.render_widget(
            Paragraph::new(Line::from(value))
                .block(block)
                .style(Style::default().fg(if focused {
                    Color::LightYellow
                } else {
                    Color::White
                })),
            area,
        );
    }

    fn render_language(&self, frame: &mut Frame, area: Rect, text: &Messages) {
        let focused = self.focused_field == SettingsField::Language;
        let spanish_style = option_style(focused, self.language == Language::Spanish);
        let english_style = option_style(focused, self.language == Language::English);
        let lines = vec![Line::from(vec![
            Span::styled(format!(" {} ", text.language_spanish), spanish_style),
            Span::raw("  "),
            Span::styled(format!(" {} ", text.language_english), english_style),
        ])];

        frame.render_widget(
            Paragraph::new(lines).block(field_block(text.language_label, focused)),
            area,
        );
    }

    fn render_status(&self, frame: &mut Frame, area: Rect, text: &Messages) {
        let line = match &self.status {
            Some(SettingsStatus::Saved) => {
                Line::from(text.settings_saved).style(Style::default().fg(Color::LightGreen))
            }
            Some(SettingsStatus::Error(message)) => {
                Line::from(message.as_str()).style(Style::default().fg(Color::Red))
            }
            None => Line::from(""),
        };

        frame.render_widget(Paragraph::new(line).wrap(Wrap { trim: true }), area);
    }

    fn focus_next(&mut self) {
        self.focused_field = match self.focused_field {
            SettingsField::DownloadDir => SettingsField::DriveRootFolder,
            SettingsField::DriveRootFolder => SettingsField::DteQueryFilter,
            SettingsField::DteQueryFilter => SettingsField::Language,
            SettingsField::Language => SettingsField::DownloadDir,
        };
    }

    fn focus_previous(&mut self) {
        self.focused_field = match self.focused_field {
            SettingsField::DownloadDir => SettingsField::Language,
            SettingsField::DriveRootFolder => SettingsField::DownloadDir,
            SettingsField::DteQueryFilter => SettingsField::DriveRootFolder,
            SettingsField::Language => SettingsField::DteQueryFilter,
        };
    }

    fn push_character(&mut self, character: char) {
        match self.focused_field {
            SettingsField::DownloadDir => {
                self.status = None;
                self.download_dir.push(character);
            }
            SettingsField::DriveRootFolder => {
                self.status = None;
                self.drive_root_folder.push(character);
            }
            SettingsField::DteQueryFilter => {
                self.status = None;
                self.dte_query_filter.push(character);
            }
            SettingsField::Language => {}
        }
    }

    fn delete_character(&mut self) {
        match self.focused_field {
            SettingsField::DownloadDir => {
                self.status = None;
                self.download_dir.pop();
            }
            SettingsField::DriveRootFolder => {
                self.status = None;
                self.drive_root_folder.pop();
            }
            SettingsField::DteQueryFilter => {
                self.status = None;
                self.dte_query_filter.pop();
            }
            SettingsField::Language => {}
        }
    }

    fn toggle_language(&mut self) {
        self.status = None;
        self.language = match self.language {
            Language::Spanish => Language::English,
            Language::English => Language::Spanish,
        };
    }
}

fn field_block(label: &str, focused: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {label} "))
        .border_style(if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        })
}

fn option_style(focused: bool, selected: bool) -> Style {
    match (focused, selected) {
        (true, true) => Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        (_, true) => Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD),
        (true, false) => Style::default().fg(Color::DarkGray),
        (false, false) => Style::default().fg(Color::Gray),
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
