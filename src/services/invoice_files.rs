use crate::{
    domain::invoice::InvoiceSummary,
    services::{drive_upload::UploadedInvoiceFiles, gmail_search::DownloadedAttachment},
};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub const DEFAULT_DOWNLOAD_DIR: &str = "downloaded_invoices";

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
    directory: &Path,
) -> Result<SavedInvoiceFiles, io::Error> {
    fs::create_dir_all(directory)?;

    let prefix = sanitize_filename(&invoice.file_slug());
    let json_path = save_attachment(directory, &prefix, json)?;
    let pdf_path = save_attachment(directory, &prefix, pdf)?;

    Ok(SavedInvoiceFiles {
        json_path,
        pdf_path,
        drive_upload: None,
    })
}

fn save_attachment(
    directory: &Path,
    prefix: &str,
    attachment: &DownloadedAttachment,
) -> io::Result<PathBuf> {
    let filename = sanitize_filename(&attachment.filename);
    let filename = if filename.is_empty() {
        "attachment".to_string()
    } else {
        filename
    };
    let mut index = 1;
    loop {
        let path = if index == 1 {
            directory.join(format!("{prefix}-{filename}"))
        } else {
            directory.join(format!("{prefix}-{index}-{filename}"))
        };
        match fs::read(&path) {
            Ok(existing) if existing == attachment.bytes => return Ok(path),
            Ok(_) => {
                index += 1;
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }

        let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
        temporary.write_all(&attachment.bytes)?;
        match temporary.persist_noclobber(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.error),
        }
    }
}

pub(super) fn sanitize_filename(value: &str) -> String {
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

    #[test]
    fn reuses_identical_attachments_but_preserves_different_contents() {
        let directory = tempfile::tempdir().unwrap();
        let mut attachment = DownloadedAttachment {
            filename: "invoice.pdf".into(),
            bytes: b"original".to_vec(),
        };
        let original = save_attachment(directory.path(), "DTE-TEST", &attachment).unwrap();
        assert_eq!(
            save_attachment(directory.path(), "DTE-TEST", &attachment).unwrap(),
            original
        );

        attachment.bytes = b"different PDF".to_vec();
        let other = save_attachment(directory.path(), "DTE-TEST", &attachment).unwrap();
        assert_ne!(other, original);
        assert_eq!(fs::read(&original).unwrap(), b"original");
        assert_eq!(fs::read(&other).unwrap(), attachment.bytes);
        assert_eq!(
            save_attachment(directory.path(), "DTE-TEST", &attachment).unwrap(),
            other
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}
