use super::widgets::*;
use crate::app::{App, UiAction, state::ProcessedInvoice};
use gpui::{Context, Div, ElementId, FontWeight, Stateful, div, prelude::*, px, rgb};
use std::collections::BTreeMap;

impl App {
    pub(crate) fn render_review_summary(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let text = self.text();
        let context = &self.state.context;
        let mut folders = BTreeMap::<_, Vec<_>>::new();
        let mut uploaded = 0;
        for (index, invoice) in context.processed.iter().enumerate() {
            let folder = invoice
                .files
                .drive_upload
                .as_ref()
                .map(|upload| (upload.folder_path.as_str(), upload.folder_id.as_str()));
            uploaded += usize::from(folder.is_some());
            folders.entry(folder).or_default().push((index, invoice));
        }

        div()
            .id("review-summary")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(
                div().flex().justify_center().child(
                    div()
                        .w_full()
                        .max_w(px(1040.))
                        .flex()
                        .flex_col()
                        .gap_6()
                        .child(
                            div()
                                .text_size(px(28.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(text.review_complete),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_4()
                                .child(summary_count(text.uploaded_invoices_label, uploaded, true))
                                .child(summary_count(
                                    text.skipped_invoices_label,
                                    context.skipped,
                                    false,
                                )),
                        )
                        .when(folders.is_empty(), |s| {
                            s.child(card().text_color(rgb(MUTED)).child(text.review_no_uploads))
                        })
                        .when(!folders.is_empty(), |s| {
                            s.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .child(
                                        div()
                                            .text_size(px(17.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(text.review_saved_invoices),
                                    )
                                    .children(folders.into_iter().enumerate().map(
                                        |(index, (folder, invoices))| {
                                            self.invoice_group(index, folder, invoices, cx)
                                        },
                                    )),
                            )
                        }),
                ),
            )
    }

    fn invoice_group(
        &self,
        index: usize,
        folder: Option<(&str, &str)>,
        invoices: Vec<(usize, &ProcessedInvoice)>,
        cx: &mut Context<Self>,
    ) -> Div {
        let text = self.text();
        card()
            .gap_0()
            .p_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .px_4()
                    .py_3()
                    .bg(rgb(0xf8fafc))
                    .rounded_t_lg()
                    .child(
                        div().min_w_0().font_weight(FontWeight::MEDIUM).child(
                            folder
                                .map_or(text.review_local_files, |(path, _)| path)
                                .to_owned(),
                        ),
                    )
                    .when_some(folder, |s, (_, id)| {
                        s.child(drive_button(
                            ("summary-folder", index),
                            text.review_open_folder,
                            id,
                            format!("https://drive.google.com/drive/folders/{id}"),
                            cx,
                        ))
                    }),
            )
            .children(
                invoices
                    .into_iter()
                    .map(|(index, invoice)| self.invoice_row(index, invoice, cx)),
            )
    }

    fn invoice_row(&self, index: usize, invoice: &ProcessedInvoice, cx: &mut Context<Self>) -> Div {
        let text = self.text();
        let files = &invoice.files;
        let expanded = self.expanded_invoices.contains(&index);
        let issuer = if invoice.issuer.trim().is_empty() {
            text.review_unknown_issuer
        } else {
            &invoice.issuer
        };
        let reference = if invoice.issue_date.is_empty() {
            invoice.reference.clone()
        } else {
            format!("{} \u{b7} {}", invoice.reference, invoice.issue_date)
        };

        div()
            .border_t_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .px_4()
                    .py_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(issuer.to_owned()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(MUTED))
                                    .child(reference),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .flex_shrink_0()
                            .gap_2()
                            .when_some(files.drive_upload.as_ref(), |s, upload| {
                                s.children(
                                    [
                                        ("summary-pdf", "PDF", &upload.pdf_file_id),
                                        ("summary-json", "JSON", &upload.json_file_id),
                                    ]
                                    .into_iter()
                                    .map(|(key, label, id)| {
                                        drive_button(
                                            (key, index),
                                            label,
                                            id,
                                            format!("https://drive.google.com/file/d/{id}/view"),
                                            cx,
                                        )
                                    }),
                                )
                            })
                            .child(compact_button(
                                ("summary-details", index),
                                if expanded {
                                    text.review_hide_details
                                } else {
                                    text.review_show_details
                                },
                                true,
                                UiAction::ToggleInvoiceDetails(index),
                                cx,
                            )),
                    ),
            )
            .when(expanded, |s| {
                s.child(
                    div()
                        .mx_4()
                        .mb_3()
                        .rounded_md()
                        .bg(rgb(BACKGROUND))
                        .p_4()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(detail("PDF", files.pdf_path.display().to_string()))
                        .child(detail("JSON", files.json_path.display().to_string()))
                        .when_some(files.drive_upload.as_ref(), |s, upload| {
                            s.child(detail(
                                text.drive_file_ids_label,
                                format!(
                                    "PDF: {}\nJSON: {}",
                                    upload.pdf_file_id, upload.json_file_id
                                ),
                            ))
                        }),
                )
            })
    }
}

fn summary_count(label: &'static str, count: usize, uploaded: bool) -> Div {
    let success = uploaded && count > 0;
    card()
        .flex_1()
        .gap_1()
        .p_4()
        .when(success, |s| s.bg(rgb(0xf0faf4)).border_color(rgb(0xcde7d7)))
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(if success { 0x256243 } else { MUTED }))
                .child(label),
        )
        .child(
            div()
                .text_size(px(32.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(count.to_string()),
        )
}

fn drive_button(
    element: impl Into<ElementId>,
    label: &'static str,
    id: &str,
    url: String,
    cx: &mut Context<App>,
) -> Stateful<Div> {
    let valid_id = !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'));
    compact_button(element, label, valid_id, UiAction::OpenDriveUrl(url), cx)
}

fn compact_button(
    id: impl Into<ElementId>,
    label: &'static str,
    enabled: bool,
    action: UiAction,
    cx: &mut Context<App>,
) -> Stateful<Div> {
    button(id, label, false, enabled, action, cx)
        .flex_shrink_0()
        .h(px(32.))
        .px_3()
        .text_size(px(12.))
}
