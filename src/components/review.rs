use super::widgets::*;
use crate::{
    app::{
        App, UiAction,
        state::{ReviewState, Screen},
    },
    domain::invoice::InvoiceSummary,
};
use gpui::{Context, Div, FontWeight, Pixels, Window, div, img, prelude::*, px, rgb};

impl App {
    pub(crate) fn render_review(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        if matches!(self.state.review, ReviewState::Complete | ReviewState::Idle) {
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(self.render_review_summary(cx));
        }
        let text = self.text();
        let context = &self.state.context;
        let email = context.emails.get(context.index);
        let subtitle = text.review_progress(context.index, context.emails.len());
        let details = div()
            .id("review-details")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.review_scroll)
            .child(self.review_details());

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_4()
            .child(heading(text.review_title, subtitle).flex_shrink_0())
            .when_some(email, |s, email| {
                s.child(
                    div()
                        .flex_shrink_0()
                        .truncate()
                        .text_color(rgb(MUTED))
                        .child(format!("{} \u{b7} {}", email.from, email.subject)),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_5()
                    .child(details)
                    .when_some(email, |s, email| {
                        let pdf_width = (window.viewport_size().width - px(84.)) * 0.52;
                        let filename = email
                            .pdf_attachments
                            .first()
                            .map(|file| file.filename.as_str());
                        s.child(self.pdf_panel(pdf_width, filename, cx))
                    }),
            )
    }

    fn review_details(&self) -> Div {
        let text = self.text();
        let body = div().flex().flex_col().gap_4();
        match &self.state.review {
            ReviewState::Loading => body.child(notice(text.loading_invoice, false)),
            ReviewState::Ready(review) | ReviewState::Saving(review) => body
                .when_some(self.state.error.as_ref(), |s, error| {
                    s.child(notice(error.clone(), true))
                })
                .when(matches!(self.state.review, ReviewState::Saving(_)), |s| {
                    s.child(notice(text.saving_invoice_files, false))
                })
                .when(!review.invoice.is_ccf, |s| {
                    s.child(notice(text.ccf_warning, true))
                })
                .child(self.invoice_details(&review.invoice)),
            ReviewState::Error(error) => body.child(notice(error.clone(), true)),
            ReviewState::Complete | ReviewState::Idle => body,
        }
    }

    fn pdf_panel(&self, width: Pixels, filename: Option<&str>, cx: &mut Context<Self>) -> Div {
        let text = self.text();
        let preview = &self.review_pdf;
        let loading = preview.loading();
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .min_h_0()
            .w(width)
            .border_1()
            .border_color(rgb(BORDER))
            .rounded_lg()
            .overflow_hidden()
            .bg(rgb(0xffffff))
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .min_w_0()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("PDF"))
                            .when_some(filename, |s, filename| {
                                s.child(
                                    div()
                                        .truncate()
                                        .text_size(px(12.))
                                        .text_color(rgb(MUTED))
                                        .child(filename.to_owned()),
                                )
                            }),
                    )
                    .when_some(
                        preview.page.as_ref().filter(|page| page.count > 1),
                        |s, page| {
                            s.child(
                                div()
                                    .flex()
                                    .flex_shrink_0()
                                    .items_center()
                                    .gap_2()
                                    .child(button(
                                        "pdf-previous",
                                        text.pdf_previous_page,
                                        false,
                                        !loading && page.number > 1,
                                        UiAction::PdfPage(page.number - 1),
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .text_color(rgb(MUTED))
                                            .child(format!("{} / {}", page.number, page.count)),
                                    )
                                    .child(button(
                                        "pdf-next",
                                        text.pdf_next_page,
                                        false,
                                        !loading && page.number < page.count,
                                        UiAction::PdfPage(page.number.saturating_add(1)),
                                        cx,
                                    )),
                            )
                        },
                    ),
            )
            .child(
                div()
                    .id("review-pdf")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&preview.scroll)
                    .bg(rgb(BACKGROUND))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            .p_4()
                            .when(loading && preview.page.is_none(), |s| {
                                s.child(notice(text.loading_pdf, false))
                            })
                            .when_some(preview.error.as_ref(), |s, error| {
                                s.child(notice(error.clone(), true))
                            })
                            .when_some(preview.page.as_ref(), |s, page| {
                                s.child(img(page.image.clone()).w((width - px(34.)).min(px(850.))))
                            }),
                    ),
            )
    }

    pub(crate) fn review_actions(&self, cx: &mut Context<Self>) -> Div {
        let text = self.text();
        if matches!(self.state.review, ReviewState::Complete | ReviewState::Idle) {
            return action_bar().justify_end().child(button(
                "review-back",
                text.review_back_to_results,
                true,
                true,
                UiAction::Navigate(Screen::Results),
                cx,
            ));
        }
        let ready = matches!(self.state.review, ReviewState::Ready(_));
        let can_skip = ready || matches!(self.state.review, ReviewState::Error(_));
        action_bar()
            .child(
                div()
                    .text_color(rgb(MUTED))
                    .when(ready, |s| s.child(text.review_manual_hint)),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .gap_3()
                    .child(button(
                        "skip",
                        text.skip_button,
                        false,
                        can_skip,
                        UiAction::Skip,
                        cx,
                    ))
                    .child(button(
                        "accept",
                        if self.state.error.is_some() {
                            text.retry_upload_button
                        } else {
                            text.accept_upload_button
                        },
                        true,
                        ready,
                        UiAction::Accept,
                        cx,
                    )),
            )
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
            .child(identity)
            .child(totals)
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
                                    .flex_wrap()
                                    .gap_x_4()
                                    .gap_y_1()
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
