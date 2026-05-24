use crate::{
    domain::invoice::InvoiceSummary,
    i18n::Messages,
    services::{gmail_search::CandidateEmail, invoice_files::SavedInvoiceFiles},
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

pub enum ReviewView<'a> {
    Loading {
        index: usize,
        total: usize,
        email: &'a CandidateEmail,
    },
    Ready {
        index: usize,
        total: usize,
        email: &'a CandidateEmail,
        invoice: &'a InvoiceSummary,
    },
    Saving {
        index: usize,
        total: usize,
        invoice: &'a InvoiceSummary,
    },
    Error {
        index: usize,
        total: usize,
        email: &'a CandidateEmail,
        message: &'a str,
    },
    Complete {
        processed: usize,
        skipped: usize,
        saved_files: &'a [SavedInvoiceFiles],
    },
}

#[derive(Default)]
pub struct ReviewPage;

impl ReviewPage {
    pub fn render(&self, frame: &mut Frame, view: ReviewView<'_>, text: &'static Messages) {
        let area = centered_rect(frame.area(), 108, 34);
        let block = Block::bordered()
            .title(format!(" {} ", text.review_title))
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);
        let inner = block.inner(area);

        frame.render_widget(block, area);

        match view {
            ReviewView::Loading {
                index,
                total,
                email,
            } => self.render_busy(
                frame,
                inner,
                text,
                text.loading_invoice,
                index,
                total,
                email,
            ),
            ReviewView::Ready {
                index,
                total,
                email,
                invoice,
            } => self.render_invoice(frame, inner, text, index, total, email, invoice),
            ReviewView::Saving {
                index,
                total,
                invoice,
            } => self.render_saving(frame, inner, text, index, total, invoice),
            ReviewView::Error {
                index,
                total,
                email,
                message,
            } => self.render_error(frame, inner, text, index, total, email, message),
            ReviewView::Complete {
                processed,
                skipped,
                saved_files,
            } => self.render_complete(frame, inner, text, processed, skipped, saved_files),
        }
    }

    fn render_busy(
        &self,
        frame: &mut Frame,
        area: Rect,
        text: &Messages,
        message: &str,
        index: usize,
        total: usize,
        email: &CandidateEmail,
    ) {
        let lines = vec![
            Line::from(text.review_progress(index, total))
                .style(Style::default().add_modifier(Modifier::BOLD)),
            Line::from(message),
            Line::from(display_email(text, email)),
            Line::from(""),
            Line::from(text.review_busy_footer).style(Style::default().fg(Color::DarkGray)),
        ];

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_invoice(
        &self,
        frame: &mut Frame,
        area: Rect,
        text: &Messages,
        index: usize,
        total: usize,
        email: &CandidateEmail,
        invoice: &InvoiceSummary,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Length(9),
                Constraint::Length(6),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(area);

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(text.review_progress(index, total))
                    .style(Style::default().add_modifier(Modifier::BOLD)),
                Line::from(display_email(text, email)),
                Line::from(text.review_prompt).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            chunks[0],
        );
        self.render_identity(frame, chunks[1], text, invoice);
        self.render_totals(frame, chunks[2], text, invoice);
        self.render_lines(frame, chunks[3], text, invoice);
        self.render_warning_or_letters(frame, chunks[4], text, invoice);
        frame.render_widget(
            Paragraph::new(text.review_ready_footer).style(Style::default().fg(Color::DarkGray)),
            chunks[5],
        );
    }

    fn render_saving(
        &self,
        frame: &mut Frame,
        area: Rect,
        text: &Messages,
        index: usize,
        total: usize,
        invoice: &InvoiceSummary,
    ) {
        let lines = vec![
            Line::from(text.review_progress(index, total))
                .style(Style::default().add_modifier(Modifier::BOLD)),
            Line::from(text.saving_invoice_files),
            Line::from(format!(
                "{}: {}",
                text.control_number_label,
                display_or(&invoice.control_number, "-")
            )),
            Line::from(""),
            Line::from(text.review_busy_footer).style(Style::default().fg(Color::DarkGray)),
        ];

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_error(
        &self,
        frame: &mut Frame,
        area: Rect,
        text: &Messages,
        index: usize,
        total: usize,
        email: &CandidateEmail,
        message: &str,
    ) {
        let lines = vec![
            Line::from(text.review_progress(index, total))
                .style(Style::default().add_modifier(Modifier::BOLD)),
            Line::from(text.review_error_title).style(Style::default().fg(Color::Red)),
            Line::from(display_email(text, email)),
            Line::from(message),
            Line::from(""),
            Line::from(text.review_error_footer).style(Style::default().fg(Color::DarkGray)),
        ];

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
    }

    fn render_complete(
        &self,
        frame: &mut Frame,
        area: Rect,
        text: &Messages,
        processed: usize,
        skipped: usize,
        saved_files: &[SavedInvoiceFiles],
    ) {
        let mut lines = vec![
            Line::from(text.review_complete).style(Style::default().add_modifier(Modifier::BOLD)),
            Line::from(format!(
                "{}: {processed}    {}: {skipped}",
                text.processed_label, text.skipped_label
            )),
            Line::from(format!("{}:", text.saved_files_label)),
        ];

        for files in saved_files.iter().take(5) {
            lines.push(Line::from(format!("JSON: {}", files.json_path.display())));
            lines.push(Line::from(format!("PDF:  {}", files.pdf_path.display())));
            if let Some(upload) = &files.drive_upload {
                lines.push(Line::from(format!(
                    "{}: {}",
                    text.drive_folder_label, upload.folder_path
                )));
                lines.push(Line::from(format!(
                    "{}: JSON {} | PDF {}",
                    text.drive_file_ids_label, upload.json_file_id, upload.pdf_file_id
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(
            Line::from(text.review_complete_footer).style(Style::default().fg(Color::DarkGray)),
        );

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
    }

    fn render_identity(
        &self,
        frame: &mut Frame,
        area: Rect,
        text: &Messages,
        invoice: &InvoiceSummary,
    ) {
        let document_type = format!(
            "{} ({})",
            invoice.document_type_name,
            display_or(&invoice.document_type_code, "-")
        );
        let lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{}: ", text.document_type_label),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(document_type),
            ]),
            Line::from(format!(
                "{}: {}",
                text.control_number_label,
                display_or(&invoice.control_number, "-")
            )),
            Line::from(format!(
                "{}: {}",
                text.generation_code_label,
                display_or(&invoice.generation_code, "-")
            )),
            Line::from(format!(
                "{}: {} {}",
                text.issue_date_label,
                display_or(&invoice.issue_date, "-"),
                display_or(&invoice.issue_time, "")
            )),
            Line::from(format!(
                "{}: {} | NIT {} | NRC {}",
                text.issuer_label,
                display_or(&invoice.issuer_name, "-"),
                display_or(&invoice.issuer_nit, "-"),
                display_or(&invoice.issuer_nrc, "-")
            )),
            Line::from(format!(
                "{}: {} | NIT {} | NRC {}",
                text.receiver_label,
                display_or(&invoice.receiver_name, "-"),
                display_or(&invoice.receiver_nit, "-"),
                display_or(&invoice.receiver_nrc, "-")
            )),
        ];

        frame.render_widget(section(text.document_type_label, lines), area);
    }

    fn render_totals(
        &self,
        frame: &mut Frame,
        area: Rect,
        text: &Messages,
        invoice: &InvoiceSummary,
    ) {
        let tax_text = if invoice.taxes.is_empty() {
            "-".to_string()
        } else {
            invoice
                .taxes
                .iter()
                .map(|tax| {
                    format!(
                        "{} {} {}",
                        display_or(&tax.code, ""),
                        display_or(&tax.description, ""),
                        tax.value
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        let lines = vec![
            Line::from(format!(
                "{}: {}",
                text.taxed_sales_label, invoice.taxed_sales
            )),
            Line::from(format!(
                "{}: {}",
                text.exempt_sales_label, invoice.exempt_sales
            )),
            Line::from(format!(
                "{}: {}",
                text.non_subject_sales_label, invoice.non_subject_sales
            )),
            Line::from(format!("{}: {}", text.subtotal_label, invoice.subtotal)),
            Line::from(format!(
                "{}: {}",
                text.operation_total_label, invoice.operation_total
            )),
            Line::from(format!(
                "{}: {}",
                text.total_to_pay_label, invoice.total_to_pay
            ))
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(format!(
                "{}: {}",
                text.payment_condition_label, invoice.payment_condition
            )),
            Line::from(format!("{}: {}", text.taxes_label, tax_text)),
        ];

        frame.render_widget(section(text.totals_label, lines), area);
    }

    fn render_lines(
        &self,
        frame: &mut Frame,
        area: Rect,
        text: &Messages,
        invoice: &InvoiceSummary,
    ) {
        let lines = if invoice.line_items.is_empty() {
            vec![Line::from("-")]
        } else {
            invoice
                .line_items
                .iter()
                .map(|item| {
                    Line::from(format!(
                        "{} x {} | {} {} | {} {} | {} {}",
                        display_or(&item.quantity, "-"),
                        display_or(&item.description, "-"),
                        text.unit_price_label,
                        item.unit_price,
                        text.taxed_short_label,
                        item.taxed_sale,
                        text.exempt_short_label,
                        item.exempt_sale
                    ))
                })
                .collect()
        };

        frame.render_widget(section(text.line_items_label, lines), area);
    }

    fn render_warning_or_letters(
        &self,
        frame: &mut Frame,
        area: Rect,
        text: &Messages,
        invoice: &InvoiceSummary,
    ) {
        let mut lines = Vec::new();
        if !invoice.is_ccf {
            lines.push(Line::from(text.ccf_warning).style(Style::default().fg(Color::Red)));
        }
        if !invoice.total_in_words.trim().is_empty() {
            lines.push(Line::from(invoice.total_in_words.as_str()));
        }
        if !invoice.source_filename.trim().is_empty() {
            lines.push(Line::from(format!("JSON: {}", invoice.source_filename)));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
    }
}

fn section<'a>(title: &'a str, lines: Vec<Line<'a>>) -> Paragraph<'a> {
    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {title} ")),
        )
        .wrap(Wrap { trim: true })
}

fn display_email(text: &Messages, email: &CandidateEmail) -> String {
    format!(
        "{} | {}",
        display_or(&email.from, text.unknown_sender),
        display_or(&email.subject, text.no_subject)
    )
}

fn display_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
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
