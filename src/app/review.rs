use crate::{
    domain::invoice::{InvoiceParseError, InvoiceSummary},
    i18n::Messages,
    services::{
        drive_upload::{DriveUploadError, DriveUploadService},
        gmail_search::{
            CandidateEmail, DownloadedAttachment, GmailSearchError, GmailSearchService,
        },
        invoice_files::{SavedInvoiceFiles, save_invoice_files},
        settings::AppSettings,
    },
};

#[derive(Clone)]
pub(super) struct InvoiceReview {
    pub(super) email: CandidateEmail,
    pub(super) json_file: DownloadedAttachment,
    pub(super) invoice: InvoiceSummary,
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

pub(super) fn save_invoice_review(
    review: InvoiceReview,
    settings: AppSettings,
) -> Result<SavedInvoiceFiles, ReviewError> {
    let pdf_attachment = review
        .email
        .pdf_attachments
        .first()
        .ok_or(ReviewError::MissingPdf)?;
    let service = GmailSearchService::default();
    let pdf_file = service
        .download_attachment(&review.email.id, pdf_attachment)
        .map_err(ReviewError::Gmail)?;

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

pub(super) fn review_error_message(error: ReviewError, text: &Messages) -> String {
    match error {
        ReviewError::Gmail(error) => text.gmail_search_error(&error),
        ReviewError::Drive(error) => text.drive_upload_error(&error),
        ReviewError::Invoice(error) => format!("{}: {error}", text.invoice_json_error_prefix),
        ReviewError::Io(error) => format!("{}: {error}", text.invoice_save_error_prefix),
        ReviewError::MissingJson => text.missing_json_error.to_string(),
        ReviewError::MissingPdf => text.missing_pdf_error.to_string(),
    }
}
