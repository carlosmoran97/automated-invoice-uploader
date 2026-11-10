use super::widgets::*;
use crate::{
    app::{
        App, UiAction,
        state::{ReviewState, Screen},
    },
    domain::invoice::InvoiceSummary,
};
use gpui::{Context, Div, FontWeight, div, prelude::*, px, rgb};

impl App {
    pub(crate) fn render_review(&self, cx: &mut Context<Self>) -> Div {
        let text = self.text();
        let context = &self.state.context;
        let complete = matches!(self.state.review, ReviewState::Complete);
        let subtitle = if complete {
            format!(
                "{}: {}  \u{b7}  {}: {}",
                text.processed_label, context.processed, text.skipped_label, context.skipped
            )
        } else {
            text.review_progress(context.index, context.emails.len())
        };
        let mut body = div()
            .flex()
            .flex_col()
            .gap_5()
            .child(heading(text.review_title, subtitle));
        if let Some(email) = context.emails.get(context.index).filter(|_| !complete) {
            body = body.child(
                div()
                    .text_color(rgb(MUTED))
                    .child(format!("{} \u{b7} {}", email.from, email.subject)),
            );
        }
        match &self.state.review {
            ReviewState::Loading => body.child(notice(text.loading_invoice, false)),
            ReviewState::Ready(review) | ReviewState::Saving(review) => {
                let saving = matches!(self.state.review, ReviewState::Saving(_));
                body.when(saving, |s| {
                    s.child(notice(text.saving_invoice_files, false))
                })
                .when(!review.invoice.is_ccf, |s| {
                    s.child(notice(text.ccf_warning, true))
                })
                .child(self.invoice_details(&review.invoice))
                .child(
                    card()
                        .child(div().text_color(rgb(MUTED)).child(text.review_manual_hint))
                        .child(
                            div()
                                .flex()
                                .gap_3()
                                .justify_end()
                                .child(button(
                                    "skip",
                                    text.skip_button,
                                    false,
                                    !saving,
                                    UiAction::Skip,
                                    cx,
                                ))
                                .child(button(
                                    "accept",
                                    text.accept_upload_button,
                                    true,
                                    !saving,
                                    UiAction::Accept,
                                    cx,
                                )),
                        ),
                )
            }
            ReviewState::Error(error) => {
                body.child(notice(error.clone(), true))
                    .child(div().flex().justify_end().child(button(
                        "skip-error",
                        text.skip_button,
                        false,
                        true,
                        UiAction::Skip,
                        cx,
                    )))
            }
            ReviewState::Complete | ReviewState::Idle => body
                .child(notice(text.review_complete, false))
                .children(context.saved_files.iter().map(|files| {
                    card()
                        .child(detail("JSON", files.json_path.display().to_string()))
                        .child(detail("PDF", files.pdf_path.display().to_string()))
                        .when_some(files.drive_upload.as_ref(), |s, upload| {
                            s.child(detail(text.drive_folder_label, upload.folder_path.clone()))
                                .child(detail(
                                    text.drive_file_ids_label,
                                    format!(
                                        "JSON: {}\nPDF: {}",
                                        upload.json_file_id, upload.pdf_file_id
                                    ),
                                ))
                        })
                }))
                .child(div().flex().child(button(
                    "review-back",
                    text.back_button,
                    true,
                    true,
                    UiAction::Navigate(Screen::Results),
                    cx,
                ))),
        }
    }

    fn invoice_details(&self, invoice: &InvoiceSummary) -> Div {
        let text = self.text();
        let identity = card()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(text.document_type_label),
            )
            .child(detail(
                text.document_type_label,
                format!(
                    "{} ({})",
                    invoice.document_type_name, invoice.document_type_code
                ),
            ))
            .child(detail(
                text.control_number_label,
                invoice.control_number.clone(),
            ))
            .child(detail(
                text.generation_code_label,
                invoice.generation_code.clone(),
            ))
            .child(
                div()
                    .flex()
                    .gap_6()
                    .child(detail(
                        text.issue_date_label,
                        format!("{} {}", invoice.issue_date, invoice.issue_time),
                    ))
                    .child(detail(
                        text.payment_condition_label,
                        invoice.payment_condition.clone(),
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap_6()
                    .child(div().flex_1().min_w_0().child(detail(
                        text.issuer_label,
                        format!(
                            "{}\nNIT: {}  NRC: {}",
                            invoice.issuer_name, invoice.issuer_nit, invoice.issuer_nrc
                        ),
                    )))
                    .child(div().flex_1().min_w_0().child(detail(
                        text.receiver_label,
                        format!(
                            "{}\nNIT: {}  NRC: {}",
                            invoice.receiver_name, invoice.receiver_nit, invoice.receiver_nrc
                        ),
                    ))),
            );
        let amounts = [
            (text.taxed_sales_label, &invoice.taxed_sales),
            (text.exempt_sales_label, &invoice.exempt_sales),
            (text.non_subject_sales_label, &invoice.non_subject_sales),
            (text.subtotal_label, &invoice.subtotal_sales),
            (text.discount_label, &invoice.discount_total),
            (text.subtotal_label, &invoice.subtotal),
            (text.income_retention_label, &invoice.income_tax_retention),
            (text.vat_retention_label, &invoice.vat_retention),
            (text.vat_perception_label, &invoice.vat_perception),
            (text.operation_total_label, &invoice.operation_total),
        ];
        let totals = card()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(text.totals_label),
            )
            .children(amounts.iter().map(|(label, value)| {
                div()
                    .flex()
                    .justify_between()
                    .gap_4()
                    .child(div().text_color(rgb(MUTED)).child(*label))
                    .child((*value).clone())
            }))
            .children(invoice.taxes.iter().map(|tax| {
                div()
                    .flex()
                    .justify_between()
                    .gap_4()
                    .child(format!("{} ({})", tax.description, tax.code))
                    .child(tax.value.clone())
            }))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .pt_4()
                    .text_size(px(20.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(BLUE))
                    .child(text.total_to_pay_label)
                    .child(format!("{} {}", invoice.total_to_pay, invoice.currency)),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(MUTED))
                    .child(invoice.total_in_words.clone()),
            );
        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_5()
                    .child(div().flex_1().min_w_0().child(identity))
                    .child(div().w(px(340.)).flex_shrink_0().child(totals)),
            )
            .child(
                card()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(text.line_items_label),
                    )
                    .children(invoice.line_items.iter().map(|item| {
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .border_t_1()
                            .border_color(rgb(BORDER))
                            .pt_3()
                            .child(item.description.clone())
                            .child(
                                div()
                                    .flex()
                                    .gap_6()
                                    .text_size(px(12.))
                                    .text_color(rgb(MUTED))
                                    .child(format!("{}: {}", text.quantity_label, item.quantity))
                                    .child(format!(
                                        "{}: {}",
                                        text.unit_price_label, item.unit_price
                                    ))
                                    .child(format!(
                                        "{}: {}",
                                        text.taxed_short_label, item.taxed_sale
                                    ))
                                    .child(format!(
                                        "{}: {}",
                                        text.exempt_short_label, item.exempt_sale
                                    ))
                                    .child(format!(
                                        "{}: {}",
                                        text.non_subject_sales_label, item.non_subject_sale
                                    )),
                            )
                    })),
            )
    }
}
