use crate::{
    domain::invoice::InvoiceSummary,
    services::{drive_upload::UploadedInvoiceFiles, gmail_search::DownloadedAttachment},
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub const DOWNLOAD_DIR: &str = "downloaded_invoices";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedInvoiceFiles {
    pub json_path: PathBuf,
    pub pdf_path: PathBuf,
    pub drive_upload: Option<UploadedInvoiceFiles>,
}

pub fn save_invoice_files(
    invoice: &InvoiceSummary,
    json: &DownloadedAttachment,
    pdf: &DownloadedAttachment,
) -> Result<SavedInvoiceFiles, io::Error> {
    let directory = Path::new(DOWNLOAD_DIR);
    fs::create_dir_all(directory)?;

    let prefix = sanitize_filename(&invoice.file_slug());
    let json_path = unique_path(directory, &prefix, &json.filename);
    let pdf_path = unique_path(directory, &prefix, &pdf.filename);

    fs::write(&json_path, &json.bytes)?;
    fs::write(&pdf_path, &pdf.bytes)?;

    Ok(SavedInvoiceFiles {
        json_path,
        pdf_path,
        drive_upload: None,
    })
}

fn unique_path(directory: &Path, prefix: &str, filename: &str) -> PathBuf {
    let filename = sanitize_filename(filename);
    let filename = if filename.is_empty() {
        "attachment".to_string()
    } else {
        filename
    };
    let mut candidate = directory.join(format!("{prefix}-{filename}"));

    if !candidate.exists() {
        return candidate;
    }

    for index in 2.. {
        candidate = directory.join(format!("{prefix}-{index}-{filename}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("unbounded loop returns before exhausting usize")
}

fn sanitize_filename(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => character,
            _ => '_',
        })
        .collect::<String>()
        .trim_matches(['_', '.'])
        .to_string();

    if sanitized.len() > 120 {
        sanitized[..120].to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_unsafe_filename_characters() {
        assert_eq!(sanitize_filename("../DTE 03:ABC.json"), "DTE_03_ABC.json");
    }
}
