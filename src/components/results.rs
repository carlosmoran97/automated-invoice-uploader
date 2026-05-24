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
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
use std::collections::HashSet;

#[derive(Default)]
pub struct ResultsPage {
    selected: usize,
    scroll_offset: usize,
    details_scroll: u16,
    selected_email_ids: HashSet<String>,
}

pub enum ResultsPageAction {
    Continue,
    Back,
    Quit,
    ReviewSelected,
}

impl ResultsPage {
    pub fn reset(&mut self, emails: &[CandidateEmail]) {
        self.selected = 0;
        self.scroll_offset = 0;
        self.details_scroll = 0;
        self.selected_email_ids = emails.iter().map(|email| email.id.clone()).collect();
    }

    pub fn render(&self, frame: &mut Frame, emails: &[CandidateEmail]) {
        let text = messages();
        let area = centered_rect(frame.area(), 112, 38);
        let block = Block::bordered()
            .title(format!(" {} ", text.results_title))
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);
        let inner = block.inner(area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Percentage(45),
                Constraint::Min(10),
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
            KeyCode::Enter => {
                if !self.selected_email_ids.is_empty() {
                    return ResultsPageAction::ReviewSelected;
                }
            }
            KeyCode::Char(' ') => self.toggle_selected_email(emails),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection_up(emails.len()),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection_down(emails.len()),
            KeyCode::PageUp => self.scroll_details_up(),
            KeyCode::PageDown => self.scroll_details_down(),
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
        let inner = block.inner(area);
        let lines = emails.get(self.selected).map_or_else(
            || vec![Line::from(text.no_email_selected)],
            |email| {
                let message_text = email_message_text(email, text);
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
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        text.snippet_label,
                        Style::default().add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(message_text),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(
                            "Scroll: ",
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("PgUp/PgDn", Style::default().fg(Color::DarkGray)),
                    ]),
                ]
            },
        );
        let scroll = emails
            .get(self.selected)
            .map(|email| {
                self.details_scroll
                    .min(max_details_scroll(email, text, inner))
            })
            .unwrap_or_default();

        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0)),
            area,
        );
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
        self.details_scroll = 0;
    }

    fn move_selection_down(&mut self, email_count: usize) {
        if email_count == 0 {
            return;
        }

        self.selected = (self.selected + 1).min(email_count - 1);
        if self.selected >= self.scroll_offset + 1 {
            self.scroll_offset = self.selected.saturating_sub(1);
        }
        self.details_scroll = 0;
    }

    fn toggle_selected_email(&mut self, emails: &[CandidateEmail]) {
        let Some(email) = emails.get(self.selected) else {
            return;
        };

        if !self.selected_email_ids.remove(&email.id) {
            self.selected_email_ids.insert(email.id.clone());
        }
    }

    fn scroll_details_up(&mut self) {
        self.details_scroll = self.details_scroll.saturating_sub(5);
    }

    fn scroll_details_down(&mut self) {
        self.details_scroll = self.details_scroll.saturating_add(5);
    }

    fn is_email_selected(&self, email: &CandidateEmail) -> bool {
        self.selected_email_ids.contains(&email.id)
    }

    pub fn selected_emails(&self, emails: &[CandidateEmail]) -> Vec<CandidateEmail> {
        emails
            .iter()
            .filter(|email| self.is_email_selected(email))
            .cloned()
            .collect()
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

fn email_message_text<'a>(email: &'a CandidateEmail, text: &'a Messages) -> &'a str {
    display_or(&email.snippet, text.no_snippet)
}

fn max_details_scroll(email: &CandidateEmail, text: &Messages, area: Rect) -> u16 {
    let width = area.width.max(1) as usize;
    let visible_rows = area.height as usize;
    let content_rows = [
        format!(
            "{}{}",
            text.from_label,
            display_or(&email.from, text.unknown_sender)
        ),
        format!(
            "{}{}",
            text.subject_label,
            display_or(&email.subject, text.no_subject)
        ),
        format!(
            "{}{}",
            text.pdf_label,
            attachment_names(&email.pdf_attachments)
        ),
        format!(
            "{}{}",
            text.json_label,
            attachment_names(&email.json_attachments)
        ),
        String::new(),
        text.snippet_label.to_string(),
        email_message_text(email, text).to_string(),
        String::new(),
        "Scroll: PgUp/PgDn".to_string(),
    ]
    .iter()
    .map(|line| wrapped_line_count(line, width))
    .sum::<usize>();

    content_rows
        .saturating_sub(visible_rows)
        .min(u16::MAX as usize) as u16
}

fn wrapped_line_count(value: &str, width: usize) -> usize {
    if value.is_empty() {
        return 1;
    }

    value
        .lines()
        .map(|line| (line.chars().count().max(1) + width - 1) / width)
        .sum()
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
