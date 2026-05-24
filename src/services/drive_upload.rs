use crate::{domain::invoice::InvoiceSummary, services::invoice_files::SavedInvoiceFiles};
use serde::Deserialize;
use serde_json::json;
use std::{
    fmt, io,
    path::{Path, PathBuf},
    process::Command,
};

pub const DEFAULT_ROOT_FOLDER_NAME: &str = "CARLOS ROLANDO MORAN CAMPOS";
const PURCHASES_FOLDER_NAME: &str = "Compras";
const FOLDER_MIME_TYPE: &str = "application/vnd.google-apps.folder";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadedInvoiceFiles {
    pub folder_path: String,
    pub folder_id: String,
    pub json_file_id: String,
    pub pdf_file_id: String,
}

#[derive(Debug)]
pub enum DriveUploadError {
    CliNotFound,
    CommandFailed {
        command: String,
        status: String,
        stderr: String,
    },
    Io(io::Error),
    Json {
        context: String,
        source: serde_json::Error,
    },
    InvalidIssueDate {
        value: String,
    },
    MissingFileName {
        path: PathBuf,
    },
}

pub trait CommandRunner {
    fn run(&self, args: &[String]) -> Result<String, DriveUploadError>;
}

#[derive(Default)]
pub struct SystemCommandRunner;

pub struct DriveUploadService<R = SystemCommandRunner> {
    runner: R,
}

impl Default for DriveUploadService<SystemCommandRunner> {
    fn default() -> Self {
        Self {
            runner: SystemCommandRunner,
        }
    }
}

impl<R: CommandRunner> DriveUploadService<R> {
    #[cfg(test)]
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    pub fn upload_invoice_files(
        &self,
        invoice: &InvoiceSummary,
        files: &SavedInvoiceFiles,
        root_folder_name: &str,
    ) -> Result<UploadedInvoiceFiles, DriveUploadError> {
        let period = DrivePeriod::from_issue_date(&invoice.issue_date)?;
        let root_id = self.find_or_create_folder(root_folder_name, "root")?;
        let year_id = self.find_or_create_folder(&period.year, &root_id)?;
        let month_id = self.find_or_create_folder(&period.month_name, &year_id)?;
        let purchases_id = self.find_or_create_folder(PURCHASES_FOLDER_NAME, &month_id)?;

        let json_file_id = self.upload_file(&files.json_path, &purchases_id, "application/json")?;
        let pdf_file_id = self.upload_file(&files.pdf_path, &purchases_id, "application/pdf")?;

        Ok(UploadedInvoiceFiles {
            folder_path: format!(
                "{root_folder_name}/{}/{}/{}",
                period.year, period.month_name, PURCHASES_FOLDER_NAME
            ),
            folder_id: purchases_id,
            json_file_id,
            pdf_file_id,
        })
    }

    fn find_or_create_folder(
        &self,
        name: &str,
        parent_id: &str,
    ) -> Result<String, DriveUploadError> {
        if let Some(folder) = self.find_folder(name, parent_id)? {
            return Ok(folder.id);
        }

        self.create_folder(name, parent_id)
    }

    fn find_folder(
        &self,
        name: &str,
        parent_id: &str,
    ) -> Result<Option<DriveFile>, DriveUploadError> {
        let params = json!({
            "q": format!(
                "mimeType='{}' and name='{}' and '{}' in parents and trashed=false",
                FOLDER_MIME_TYPE,
                escape_drive_query_value(name),
                escape_drive_query_value(parent_id)
            ),
            "fields": "files(id,name,mimeType)",
            "pageSize": 10,
            "supportsAllDrives": true,
            "includeItemsFromAllDrives": true,
        })
        .to_string();
        let args = vec![
            "drive".to_string(),
            "files".to_string(),
            "list".to_string(),
            "--params".to_string(),
            params,
        ];
        let stdout = self.runner.run(&args)?;
        let response: DriveFileList = parse_json(&stdout, format!("folder lookup `{name}`"))?;

        Ok(response.files.into_iter().next())
    }

    fn create_folder(&self, name: &str, parent_id: &str) -> Result<String, DriveUploadError> {
        let params = json!({
            "fields": "id,name,mimeType",
            "supportsAllDrives": true,
        })
        .to_string();
        let body = json!({
            "name": name,
            "mimeType": FOLDER_MIME_TYPE,
            "parents": [parent_id],
        })
        .to_string();
        let args = vec![
            "drive".to_string(),
            "files".to_string(),
            "create".to_string(),
            "--params".to_string(),
            params,
            "--json".to_string(),
            body,
        ];
        let stdout = self.runner.run(&args)?;
        let file: DriveFile = parse_json(&stdout, format!("create folder `{name}`"))?;

        Ok(file.id)
    }

    fn upload_file(
        &self,
        path: &Path,
        parent_id: &str,
        content_type: &str,
    ) -> Result<String, DriveUploadError> {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| DriveUploadError::MissingFileName {
                path: path.to_path_buf(),
            })?;
        let params = json!({
            "fields": "id,name,mimeType",
            "supportsAllDrives": true,
        })
        .to_string();
        let body = json!({
            "name": filename,
            "parents": [parent_id],
        })
        .to_string();
        let args = vec![
            "drive".to_string(),
            "files".to_string(),
            "create".to_string(),
            "--params".to_string(),
            params,
            "--json".to_string(),
            body,
            "--upload".to_string(),
            path.display().to_string(),
            "--upload-content-type".to_string(),
            content_type.to_string(),
        ];
        let stdout = self.runner.run(&args)?;
        let file: DriveFile = parse_json(&stdout, format!("upload `{filename}`"))?;

        Ok(file.id)
    }
}

impl CommandRunner for SystemCommandRunner {
    fn run(&self, args: &[String]) -> Result<String, DriveUploadError> {
        let output = Command::new("gws").args(args).output().map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                DriveUploadError::CliNotFound
            } else {
                DriveUploadError::Io(error)
            }
        })?;

        if !output.status.success() {
            let status = output.status.code().map_or_else(
                || "terminated by signal".to_string(),
                |code| code.to_string(),
            );
            return Err(DriveUploadError::CommandFailed {
                command: format!("gws {}", args.join(" ")),
                status,
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl fmt::Display for DriveUploadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CliNotFound => write!(formatter, "gws CLI was not found in PATH."),
            Self::CommandFailed {
                command,
                status,
                stderr,
            } => {
                if stderr.is_empty() {
                    write!(formatter, "`{command}` failed with exit status {status}.")
                } else {
                    write!(
                        formatter,
                        "`{command}` failed with exit status {status}: {stderr}"
                    )
                }
            }
            Self::Io(error) => write!(formatter, "Failed to run gws: {error}"),
            Self::Json { context, source } => {
                write!(
                    formatter,
                    "Failed to parse gws JSON for {context}: {source}"
                )
            }
            Self::InvalidIssueDate { value } => {
                write!(formatter, "Invalid invoice issue date `{value}`.")
            }
            Self::MissingFileName { path } => {
                write!(
                    formatter,
                    "Could not determine file name for `{}`.",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for DriveUploadError {}

#[derive(Deserialize)]
struct DriveFileList {
    #[serde(default)]
    files: Vec<DriveFile>,
}

#[derive(Deserialize)]
struct DriveFile {
    id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DrivePeriod {
    year: String,
    month_name: String,
}

impl DrivePeriod {
    fn from_issue_date(issue_date: &str) -> Result<Self, DriveUploadError> {
        let mut parts = issue_date.split('-');
        let year = parts.next().unwrap_or_default();
        let month_name = parts
            .next()
            .and_then(|value| value.parse::<u8>().ok())
            .and_then(spanish_month_name);
        let day = parts.next().and_then(|value| value.parse::<u8>().ok());

        if year.len() != 4
            || !year.chars().all(|character| character.is_ascii_digit())
            || !matches!(day, Some(1..=31))
            || parts.next().is_some()
            || month_name.is_none()
        {
            return Err(DriveUploadError::InvalidIssueDate {
                value: issue_date.to_string(),
            });
        }

        Ok(Self {
            year: year.to_string(),
            month_name: month_name.unwrap().to_string(),
        })
    }
}

fn spanish_month_name(month: u8) -> Option<&'static str> {
    match month {
        1 => Some("Enero"),
        2 => Some("Febrero"),
        3 => Some("Marzo"),
        4 => Some("Abril"),
        5 => Some("Mayo"),
        6 => Some("Junio"),
        7 => Some("Julio"),
        8 => Some("Agosto"),
        9 => Some("Septiembre"),
        10 => Some("Octubre"),
        11 => Some("Noviembre"),
        12 => Some("Diciembre"),
        _ => None,
    }
}

fn escape_drive_query_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn parse_json<T>(input: &str, context: String) -> Result<T, DriveUploadError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(input).map_err(|source| DriveUploadError::Json { context, source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::invoice::InvoiceSummary;
    use std::{
        collections::VecDeque,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    #[test]
    fn maps_issue_date_to_drive_period() {
        assert_eq!(
            DrivePeriod::from_issue_date("2026-05-24").unwrap(),
            DrivePeriod {
                year: "2026".to_string(),
                month_name: "Mayo".to_string()
            }
        );
    }

    #[test]
    fn uploads_to_created_year_month_purchases_path() {
        let runner = FakeRunner::new(vec![
            r#"{"files":[]}"#,
            r#"{"id":"root-folder"}"#,
            r#"{"files":[]}"#,
            r#"{"id":"year-folder"}"#,
            r#"{"files":[]}"#,
            r#"{"id":"month-folder"}"#,
            r#"{"files":[]}"#,
            r#"{"id":"purchases-folder"}"#,
            r#"{"id":"json-file"}"#,
            r#"{"id":"pdf-file"}"#,
        ]);
        let calls = runner.calls.clone();
        let service = DriveUploadService::new(runner);
        let mut invoice = empty_invoice();
        invoice.issue_date = "2026-05-24".to_string();
        let files = SavedInvoiceFiles {
            json_path: PathBuf::from("downloaded_invoices/invoice.json"),
            pdf_path: PathBuf::from("downloaded_invoices/invoice.pdf"),
            drive_upload: None,
        };

        let upload = service
            .upload_invoice_files(&invoice, &files, DEFAULT_ROOT_FOLDER_NAME)
            .unwrap();

        assert_eq!(
            upload.folder_path,
            "CARLOS ROLANDO MORAN CAMPOS/2026/Mayo/Compras"
        );
        assert_eq!(upload.json_file_id, "json-file");
        assert_eq!(upload.pdf_file_id, "pdf-file");
        assert_eq!(calls.lock().unwrap().len(), 10);
    }

    struct FakeRunner {
        calls: Arc<Mutex<Vec<Vec<String>>>>,
        outputs: Mutex<VecDeque<String>>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<&str>) -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                outputs: Mutex::new(
                    outputs
                        .into_iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                ),
            }
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, args: &[String]) -> Result<String, DriveUploadError> {
            self.calls.lock().unwrap().push(args.to_vec());
            self.outputs
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| DriveUploadError::Io(io::Error::other("missing fake output")))
        }
    }

    fn empty_invoice() -> InvoiceSummary {
        InvoiceSummary {
            source_filename: String::new(),
            document_type_code: String::new(),
            document_type_name: String::new(),
            is_ccf: true,
            control_number: String::new(),
            generation_code: String::new(),
            issue_date: String::new(),
            issue_time: String::new(),
            currency: String::new(),
            issuer_name: String::new(),
            issuer_nit: String::new(),
            issuer_nrc: String::new(),
            receiver_name: String::new(),
            receiver_nit: String::new(),
            receiver_nrc: String::new(),
            taxed_sales: String::new(),
            exempt_sales: String::new(),
            non_subject_sales: String::new(),
            subtotal_sales: String::new(),
            discount_total: String::new(),
            subtotal: String::new(),
            taxes: Vec::new(),
            income_tax_retention: String::new(),
            vat_retention: String::new(),
            vat_perception: String::new(),
            operation_total: String::new(),
            total_to_pay: String::new(),
            total_in_words: String::new(),
            payment_condition: String::new(),
            line_items: Vec::new(),
        }
    }
}
