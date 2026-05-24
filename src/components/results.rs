use crate::services::gmail_search::{AttachmentSummary, CandidateEmail};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(Default)]
pub struct ResultsPage {
    selected: usize,
    scroll_offset: usize,
}

pub enum ResultsPageAction {
    Continue,
    Back,
    Quit,
}

impl ResultsPage {
    pub fn reset(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn render(&self, frame: &mut Frame, emails: &[CandidateEmail]) {
        let area = centered_rect(frame.area(), 100, 30);
        let block = Block::bordered()
            .title(" Matching invoice emails ")
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
        self.render_header(frame, chunks[0], emails.len());
        self.render_list(frame, chunks[1], emails);
        self.render_details(frame, chunks[2], emails);
        self.render_footer(frame, chunks[3]);
    }

    pub fn handle_key(&mut self, key: KeyEvent, email_count: usize) -> ResultsPageAction {
        match key.code {
            KeyCode::Char('q') => return ResultsPageAction::Quit,
            KeyCode::Char('b') | KeyCode::Esc => return ResultsPageAction::Back,
            KeyCode::Up | KeyCode::Char('k') => self.move_selection_up(email_count),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection_down(email_count),
            _ => {}
        }

        ResultsPageAction::Continue
    }

    fn render_header(&self, frame: &mut Frame, area: Rect, count: usize) {
        let plural = if count == 1 { "" } else { "s" };
        let header = Paragraph::new(vec![
            Line::from("Emails with both PDF and JSON attachments")
                .style(Style::default().add_modifier(Modifier::BOLD)),
            Line::from(format!("{count} candidate email{plural} found in INBOX.")),
        ]);

        frame.render_widget(header, area);
    }

    fn render_list(&self, frame: &mut Frame, area: Rect, emails: &[CandidateEmail]) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(Style::default().fg(Color::DarkGray));
        let visible_rows = block.inner(area).height as usize;
        let end = (self.scroll_offset + visible_rows).min(emails.len());
        let mut lines = Vec::new();

        if emails.is_empty() {
            lines.push(Line::from("No matching emails found for this period."));
        } else {
            for (index, email) in emails[self.scroll_offset..end].iter().enumerate() {
                let absolute_index = self.scroll_offset + index;
                let selected = absolute_index == self.selected;
                let marker = if selected { ">" } else { " " };
                let row = format!(
                    "{marker} {}  {}  {}",
                    truncate(&email.received_at, 24),
                    truncate(&email.from, 30),
                    truncate(&email.subject, 36)
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

    fn render_details(&self, frame: &mut Frame, area: Rect, emails: &[CandidateEmail]) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Selected email ")
            .border_style(Style::default().fg(Color::DarkGray));
        let lines = emails.get(self.selected).map_or_else(
            || vec![Line::from("No email selected.")],
            |email| {
                vec![
                    Line::from(vec![
                        Span::styled("From: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(&email.from),
                    ]),
                    Line::from(vec![
                        Span::styled("Subject: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(&email.subject),
                    ]),
                    Line::from(vec![
                        Span::styled("PDF: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(attachment_names(&email.pdf_attachments)),
                    ]),
                    Line::from(vec![
                        Span::styled("JSON: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(attachment_names(&email.json_attachments)),
                    ]),
                    Line::from(vec![
                        Span::styled("Snippet: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(truncate(&email.snippet, 86)),
                    ]),
                ]
            },
        );

        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let footer = Paragraph::new("Up/Down: select  b/Esc: back to search  q: quit")
            .style(Style::default().fg(Color::DarkGray));

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
}

fn attachment_names(attachments: &[AttachmentSummary]) -> String {
    attachments
        .iter()
        .map(|attachment| attachment.filename.as_str())
        .collect::<Vec<_>>()
        .join(", ")
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
