use crate::{
    i18n::{Messages, messages},
    services::gmail_search::{AttachmentSummary, CandidateEmail},
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use std::collections::HashSet;

#[derive(Default)]
pub struct ResultsPage {
    selected: usize,
    scroll_offset: usize,
    selected_email_ids: HashSet<String>,
}

pub enum ResultsPageAction {
    Continue,
    Back,
    Quit,
}

impl ResultsPage {
    pub fn reset(&mut self, emails: &[CandidateEmail]) {
        self.selected = 0;
        self.scroll_offset = 0;
        self.selected_email_ids = emails.iter().map(|email| email.id.clone()).collect();
    }

    pub fn render(&self, frame: &mut Frame, emails: &[CandidateEmail]) {
        let text = messages();
        let area = centered_rect(frame.area(), 100, 30);
        let block = Block::bordered()
            .title(format!(" {} ", text.results_title))
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);
        let inner = block.inner(area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(6),
                Constraint::Length(7),
                Constraint::Length(1),
            ])
            .split(inner);

        frame.render_widget(block, area);
        self.render_header(frame, chunks[0], emails.len(), text);
        self.render_list(frame, chunks[1], emails, text);
        self.render_details(frame, chunks[2], emails, text);
        self.render_footer(frame, chunks[3], text);
    }

    pub fn handle_key(&mut self, key: KeyEvent, emails: &[CandidateEmail]) -> ResultsPageAction {
        match key.code {
            KeyCode::Char('q') => return ResultsPageAction::Quit,
            KeyCode::Char('b') | KeyCode::Esc => return ResultsPageAction::Back,
            KeyCode::Char(' ') => self.toggle_selected_email(emails),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection_up(emails.len()),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection_down(emails.len()),
            _ => {}
        }

        ResultsPageAction::Continue
    }

    fn render_header(&self, frame: &mut Frame, area: Rect, count: usize, text: &Messages) {
        let header = Paragraph::new(vec![
            Line::from(text.results_header).style(Style::default().add_modifier(Modifier::BOLD)),
            Line::from(text.candidate_count(count)),
            Line::from(text.selected_candidate_count(self.selected_email_ids.len())),
        ]);

        frame.render_widget(header, area);
    }

    fn render_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        emails: &[CandidateEmail],
        text: &Messages,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", text.results_list_title))
            .border_style(Style::default().fg(Color::DarkGray));
        let visible_rows = block.inner(area).height as usize;
        let end = (self.scroll_offset + visible_rows).min(emails.len());
        let mut lines = Vec::new();

        if emails.is_empty() {
            lines.push(Line::from(text.no_matching_emails));
        } else {
            for (index, email) in emails[self.scroll_offset..end].iter().enumerate() {
                let absolute_index = self.scroll_offset + index;
                let selected = absolute_index == self.selected;
                let marker = if selected { ">" } else { " " };
                let checkbox = if self.is_email_selected(email) {
                    "[x]"
                } else {
                    "[ ]"
                };
                let row = format!(
                    "{marker} {checkbox} {}  {}  {}",
                    truncate(display_or(&email.received_at, text.unknown_date), 22),
                    truncate(display_or(&email.from, text.unknown_sender), 28),
                    truncate(display_or(&email.subject, text.no_subject), 33)
                );
                let style = if selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                lines.push(Line::from(row).style(style));
            }
        }

        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_details(
        &self,
        frame: &mut Frame,
        area: Rect,
        emails: &[CandidateEmail],
        text: &Messages,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", text.selected_email_title))
            .border_style(Style::default().fg(Color::DarkGray));
        let lines = emails.get(self.selected).map_or_else(
            || vec![Line::from(text.no_email_selected)],
            |email| {
                vec![
                    Line::from(vec![
                        Span::styled(
                            text.from_label,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(display_or(&email.from, text.unknown_sender)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            text.subject_label,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(display_or(&email.subject, text.no_subject)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            text.pdf_label,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(attachment_names(&email.pdf_attachments)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            text.json_label,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(attachment_names(&email.json_attachments)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            text.snippet_label,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(truncate(display_or(&email.snippet, text.no_snippet), 86)),
                    ]),
                ]
            },
        );

        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, text: &Messages) {
        let footer =
            Paragraph::new(text.results_footer).style(Style::default().fg(Color::DarkGray));

        frame.render_widget(footer, area);
    }

    fn move_selection_up(&mut self, email_count: usize) {
        if email_count == 0 {
            return;
        }

        self.selected = self.selected.saturating_sub(1);
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
    }

    fn move_selection_down(&mut self, email_count: usize) {
        if email_count == 0 {
            return;
        }

        self.selected = (self.selected + 1).min(email_count - 1);
        if self.selected >= self.scroll_offset + 1 {
            self.scroll_offset = self.selected.saturating_sub(1);
        }
    }

    fn toggle_selected_email(&mut self, emails: &[CandidateEmail]) {
        let Some(email) = emails.get(self.selected) else {
            return;
        };

        if !self.selected_email_ids.remove(&email.id) {
            self.selected_email_ids.insert(email.id.clone());
        }
    }

    fn is_email_selected(&self, email: &CandidateEmail) -> bool {
        self.selected_email_ids.contains(&email.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_selects_all_emails_by_default() {
        let emails = vec![candidate("one"), candidate("two")];
        let mut page = ResultsPage::default();

        page.reset(&emails);

        assert!(page.is_email_selected(&emails[0]));
        assert!(page.is_email_selected(&emails[1]));
    }

    #[test]
    fn space_toggles_highlighted_email_selection() {
        let emails = vec![candidate("one"), candidate("two")];
        let mut page = ResultsPage::default();
        page.reset(&emails);

        page.toggle_selected_email(&emails);
        assert!(!page.is_email_selected(&emails[0]));

        page.toggle_selected_email(&emails);
        assert!(page.is_email_selected(&emails[0]));
    }

    fn candidate(id: &str) -> CandidateEmail {
        CandidateEmail {
            id: id.to_string(),
            thread_id: String::new(),
            from: String::new(),
            subject: String::new(),
            received_at: String::new(),
            snippet: String::new(),
            internal_date_ms: None,
            pdf_attachments: Vec::new(),
            json_attachments: Vec::new(),
        }
    }
}

fn attachment_names(attachments: &[AttachmentSummary]) -> String {
    attachments
        .iter()
        .map(|attachment| attachment.filename.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn display_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();

    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
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
