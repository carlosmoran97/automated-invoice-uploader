use crate::{
    domain::invoice::{InvoiceParseError, InvoiceSummary},
    i18n::Messages,
    services::{
        drive_upload::{DriveUploadError, DriveUploadService},
        gmail_search::{
            CandidateEmail, DownloadedAttachment, GmailSearchError, GmailSearchService,
        },
        invoice_files::{SavedInvoiceFiles, save_invoice_files},
        pdf_preview::render_page,
        settings::AppSettings,
    },
};
use futures::{FutureExt, future::Shared};
use gpui::{Context, RenderImage, ScrollHandle, Task, Window};
use image::Frame;
use std::sync::Arc;

use super::App;

type PdfDownload = Shared<Task<Result<Arc<DownloadedAttachment>, Arc<ReviewError>>>>;

#[derive(Default)]
pub(crate) struct ReviewPdf {
    download: Option<PdfDownload>,
    task: Option<Task<()>>,
    pub page: Option<PdfPage>,
    pub error: Option<String>,
    pub scroll: ScrollHandle,
}

pub(crate) struct PdfPage {
    pub number: usize,
    pub count: usize,
    pub image: Arc<RenderImage>,
}

impl ReviewPdf {
    pub fn loading(&self) -> bool {
        self.task.is_some()
    }

    pub(super) fn download(&mut self, email: CandidateEmail, cx: &mut Context<App>) {
        self.download = Some(
            cx.background_executor()
                .spawn(async move { download_invoice_pdf(&email).map(Arc::new).map_err(Arc::new) })
                .shared(),
        );
    }

    pub(super) fn attachment(&self) -> Option<PdfDownload> {
        self.download.clone()
    }

    pub(super) fn clear(&mut self, window: Option<&mut Window>, cx: &mut Context<App>) {
        if let Some(page) = self.page.take() {
            cx.drop_image(page.image, window);
        }
        *self = Self::default();
    }
}

impl App {
    pub(super) fn load_pdf_page(&mut self, number: usize, cx: &mut Context<Self>) {
        let preview = &mut self.review_pdf;
        let Some(download) = preview.attachment().filter(|_| !preview.loading()) else {
            return;
        };
        if let Some(page) = &preview.page {
            if number == page.number || !(1..=page.count).contains(&number) {
                return;
            }
        }
        preview.error = None;
        let worker = cx.background_executor().spawn(async move {
            let pdf = download.await?;
            Ok::<_, Arc<ReviewError>>(render_page(&pdf.bytes, number))
        });
        preview.task = Some(cx.spawn(async move |this, cx| {
            let outcome = worker.await;
            let _ = this.update(cx, |this, cx| {
                let text = this.text();
                let preview = &mut this.review_pdf;
                match outcome {
                    Ok(Some(page)) => {
                        let image = Arc::new(RenderImage::new([Frame::new(page.bgra)]));
                        if let Some(old_page) = preview.page.replace(PdfPage {
                            number,
                            count: page.page_count,
                            image,
                        }) {
                            cx.drop_image(old_page.image, None);
                        }
                        preview.scroll.set_offset(Default::default());
                    }
                    Ok(None) => preview.error = Some(text.pdf_preview_error.into()),
                    Err(error) => preview.error = Some(review_error_message(&error, text)),
                }
                preview.task.take();
                cx.notify();
            });
        }));
    }
}

#[derive(Clone)]
pub(crate) struct InvoiceReview {
    pub(crate) email: CandidateEmail,
    pub(crate) json_file: DownloadedAttachment,
    pub(crate) invoice: InvoiceSummary,
}

#[derive(Debug)]
pub(super) enum ReviewError {
    Gmail(GmailSearchError),
    Drive(DriveUploadError),
    Invoice(InvoiceParseError),
    Io(std::io::Error),
    MissingJson,
    MissingPdf,
}

pub(super) fn load_invoice_review(email: CandidateEmail) -> Result<InvoiceReview, ReviewError> {
    let service = GmailSearchService::default();
    let mut last_parse_error = None;

    for json_attachment in &email.json_attachments {
        let json_file = service
            .download_attachment(&email.id, json_attachment)
            .map_err(ReviewError::Gmail)?;
        match InvoiceSummary::from_json_bytes(&json_file.filename, &json_file.bytes) {
            Ok(invoice) => {
                return Ok(InvoiceReview {
                    email,
                    json_file,
                    invoice,
                });
            }
            Err(error) => last_parse_error = Some(error),
        }
    }

    if let Some(error) = last_parse_error {
        Err(ReviewError::Invoice(error))
    } else {
        Err(ReviewError::MissingJson)
    }
}

fn download_invoice_pdf(email: &CandidateEmail) -> Result<DownloadedAttachment, ReviewError> {
    let pdf_attachment = email
        .pdf_attachments
        .first()
        .ok_or(ReviewError::MissingPdf)?;
    GmailSearchService::default()
        .download_attachment(&email.id, pdf_attachment)
        .map_err(ReviewError::Gmail)
}

pub(super) async fn save_invoice_review(
    review: InvoiceReview,
    settings: AppSettings,
    download: Option<PdfDownload>,
) -> Result<SavedInvoiceFiles, ReviewError> {
    let pdf_file = match download {
        Some(download) => download.await.ok(),
        None => None,
    };
    let pdf_file = match pdf_file {
        Some(file) => file,
        None => Arc::new(download_invoice_pdf(&review.email)?),
    };

    let mut saved_files = save_invoice_files(
        &review.invoice,
        &review.json_file,
        &pdf_file,
        &settings.download_dir,
    )
    .map_err(ReviewError::Io)?;
    let upload = DriveUploadService::default()
        .upload_invoice_files(&review.invoice, &saved_files, &settings.drive_root_folder)
        .map_err(ReviewError::Drive)?;
    saved_files.drive_upload = Some(upload);

    Ok(saved_files)
}

pub(super) fn review_error_message(error: &ReviewError, text: &Messages) -> String {
    match error {
        ReviewError::Gmail(error) => text.gmail_search_error(error),
        ReviewError::Drive(error) => text.drive_upload_error(error),
        ReviewError::Invoice(error) => format!("{}: {error}", text.invoice_json_error_prefix),
        ReviewError::Io(error) => format!("{}: {error}", text.invoice_save_error_prefix),
        ReviewError::MissingJson => text.missing_json_error.to_string(),
        ReviewError::MissingPdf => text.missing_pdf_error.to_string(),
    }
}
