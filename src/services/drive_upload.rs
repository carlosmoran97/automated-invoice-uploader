use crate::{
    domain::invoice::InvoiceSummary,
    services::invoice_files::{SavedInvoiceFiles, sanitize_filename},
};
use md5::{Digest, Md5};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::Command,
};

pub const DEFAULT_ROOT_FOLDER_NAME: &str = "CARLOS ROLANDO MORAN CAMPOS";
const PURCHASES_FOLDER_NAME: &str = "Compras";
const FOLDER_MIME_TYPE: &str = "application/vnd.google-apps.folder";
const GENERATION_CODE_PROPERTY: &str = "dteGenerationCode";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadedInvoiceFiles {
    pub folder_path: String,
    pub folder_id: String,
    pub json_file_id: String,
    pub pdf_file_id: String,
    pub reused_files: usize,
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
    IncompleteSearch,
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

        let prefix = sanitize_filename(&invoice.file_slug());
        let generation_code = invoice.generation_code.to_ascii_uppercase();
        // Drive limits the combined property key and value to 124 UTF-8 bytes.
        let generation_code = (!generation_code.is_empty()
            && generation_code.len() + GENERATION_CODE_PROPERTY.len() <= 124)
            .then_some(generation_code.as_str());
        let (json_file_id, json_reused) = self.find_or_upload_file(
            &files.json_path,
            &purchases_id,
            "application/json",
            &prefix,
            generation_code,
        )?;
        let (pdf_file_id, pdf_reused) = self.find_or_upload_file(
            &files.pdf_path,
            &purchases_id,
            "application/pdf",
            &prefix,
            generation_code,
        )?;

        Ok(UploadedInvoiceFiles {
            folder_path: format!(
                "{root_folder_name}/{}/{}/{}",
                period.year, period.month_name, PURCHASES_FOLDER_NAME
            ),
            folder_id: purchases_id,
            json_file_id,
            pdf_file_id,
            reused_files: usize::from(json_reused) + usize::from(pdf_reused),
        })
    }

    fn find_or_create_folder(
        &self,
        name: &str,
        parent_id: &str,
    ) -> Result<String, DriveUploadError> {
        let query = format!(
            "mimeType='{FOLDER_MIME_TYPE}' and name='{}' and '{}' in parents and trashed=false",
            escape_drive_query_value(name),
            escape_drive_query_value(parent_id),
        );
        if let Some(folder) = self.find_file(&query, "id", |_| true)? {
            return Ok(folder.id);
        }

        self.create_folder(name, parent_id)
    }

    fn find_file(
        &self,
        query: &str,
        fields: &str,
        matches: impl Fn(&DriveFile) -> bool,
    ) -> Result<Option<DriveFile>, DriveUploadError> {
        let mut params = json!({
            "q": query,
            "fields": format!("nextPageToken,incompleteSearch,files({fields})"),
            "pageSize": 100,
            "supportsAllDrives": true,
            "includeItemsFromAllDrives": true,
        });
        loop {
            let args = [
                "drive".to_string(),
                "files".to_string(),
                "list".to_string(),
                "--params".to_string(),
                params.to_string(),
            ];
            let stdout = self.runner.run(&args)?;
            let response: DriveFileList = parse_json(&stdout, "Drive file lookup".into())?;

            if let Some(file) = response.files.into_iter().find(&matches) {
                return Ok(Some(file));
            }
            if response.incomplete_search {
                return Err(DriveUploadError::IncompleteSearch);
            }
            let Some(token) = response.next_page_token else {
                return Ok(None);
            };
            params["pageToken"] = json!(token);
        }
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

    fn find_or_upload_file(
        &self,
        path: &Path,
        parent_id: &str,
        content_type: &str,
        prefix: &str,
        generation_code: Option<&str>,
    ) -> Result<(String, bool), DriveUploadError> {
        let mut checksum = Md5::new();
        io::copy(
            &mut fs::File::open(path).map_err(DriveUploadError::Io)?,
            &mut checksum,
        )
        .map_err(DriveUploadError::Io)?;
        let checksum = format!("{:x}", checksum.finalize());

        // Older uploads have the invoice prefix but no generation code metadata.
        let legacy = format!("name contains '{}-'", escape_drive_query_value(prefix));
        let identity = match generation_code {
            Some(code) => format!(
                "({legacy} or appProperties has {{ key='{GENERATION_CODE_PROPERTY}' and value='{}' }})",
                escape_drive_query_value(code),
            ),
            None => legacy,
        };
        let query = format!(
            "'{}' in parents and trashed=false and mimeType='{content_type}' and {identity}",
            escape_drive_query_value(parent_id),
        );
        let existing = self.find_file(&query, "id,md5Checksum,appProperties", |file| match file
            .app_properties
            .get(GENERATION_CODE_PROPERTY)
        {
            Some(code) => Some(code.as_str()) == generation_code,
            None => file.md5_checksum.as_deref() == Some(checksum.as_str()),
        })?;
        if let Some(file) = existing {
            return Ok((file.id, true));
        }
        let id = self.upload_file(path, parent_id, content_type, generation_code)?;
        Ok((id, false))
    }

    fn upload_file(
        &self,
        path: &Path,
        parent_id: &str,
        content_type: &str,
        generation_code: Option<&str>,
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
        let mut body = json!({
            "name": filename,
            "mimeType": content_type,
            "parents": [parent_id],
        });
        if let Some(code) = generation_code {
            body["appProperties"] = json!({ GENERATION_CODE_PROPERTY: code });
        }
        let args = vec![
            "drive".to_string(),
            "files".to_string(),
            "create".to_string(),
            "--params".to_string(),
            params,
            "--json".to_string(),
            body.to_string(),
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
            Self::IncompleteSearch => {
                write!(formatter, "Drive returned incomplete search results.")
            }
        }
    }
}

impl std::error::Error for DriveUploadError {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveFileList {
    #[serde(default)]
    files: Vec<DriveFile>,
    next_page_token: Option<String>,
    #[serde(default)]
    incomplete_search: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveFile {
    id: String,
    md5_checksum: Option<String>,
    #[serde(default)]
    app_properties: HashMap<String, String>,
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
    use std::{
        collections::VecDeque,
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
            r#"{"files":[]}"#,
            r#"{"id":"json-file"}"#,
            r#"{"files":[]}"#,
            r#"{"id":"pdf-file"}"#,
        ]);
        let calls = runner.calls.clone();
        let service = DriveUploadService::new(runner);
        let (invoice, files, _directory) = invoice_files();

        let upload = service
            .upload_invoice_files(&invoice, &files, "Test account")
            .unwrap();

        assert_eq!(upload.folder_path, "Test account/2026/Mayo/Compras");
        assert_eq!(upload.json_file_id, "json-file");
        assert_eq!(upload.pdf_file_id, "pdf-file");
        assert_eq!(upload.reused_files, 0);
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 12);
        for (index, mime_type) in [(9, "application/json"), (11, "application/pdf")] {
            let body: serde_json::Value = serde_json::from_str(&calls[index][6]).unwrap();
            assert_eq!(body["mimeType"], mime_type);
            assert_eq!(body["appProperties"][GENERATION_CODE_PROPERTY], CODE);
        }
    }

    #[test]
    fn reuses_both_tagged_files_for_a_resent_invoice() {
        let json = format!(
            r#"{{"files":[{{"id":"existing-json","appProperties":{{"dteGenerationCode":"{CODE}"}}}}]}}"#
        );
        let pdf = format!(
            r#"{{"files":[{{"id":"existing-pdf","appProperties":{{"dteGenerationCode":"{CODE}"}}}}]}}"#
        );
        let (service, calls) = existing_folder_service([json.as_str(), pdf.as_str()]);
        let (invoice, files, _directory) = invoice_files();
        let result = service
            .upload_invoice_files(&invoice, &files, "Test account")
            .unwrap();

        assert_eq!(result.json_file_id, "existing-json");
        assert_eq!(result.pdf_file_id, "existing-pdf");
        assert_eq!(result.reused_files, 2);
        let calls = calls.lock().unwrap();
        assert!(calls.iter().all(|call| call[2] == "list"));
        for (index, mime_type) in [(4, "application/json"), (5, "application/pdf")] {
            let params: serde_json::Value = serde_json::from_str(&calls[index][4]).unwrap();
            let query = params["q"].as_str().unwrap();
            assert!(query.contains("'purchases-folder' in parents and trashed=false"));
            assert!(query.contains(&format!("mimeType='{mime_type}'")));
            assert!(query.contains(&format!(
                "appProperties has {{ key='dteGenerationCode' and value='{CODE}' }}"
            )));
        }
    }

    #[test]
    fn uploads_only_the_missing_pdf_when_a_legacy_json_already_exists() {
        let checksum = format!("{:x}", Md5::digest(JSON_BYTES));
        let first_page = r#"{"files":[{"id":"different-json","md5Checksum":"different"}],"nextPageToken":"next"}"#;
        let next_page =
            format!(r#"{{"files":[{{"id":"legacy-json","md5Checksum":"{checksum}"}}]}}"#);
        let (service, calls) = existing_folder_service([
            first_page,
            next_page.as_str(),
            r#"{"files":[]}"#,
            r#"{"id":"new-pdf"}"#,
        ]);
        let (invoice, files, _directory) = invoice_files();
        let result = service
            .upload_invoice_files(&invoice, &files, "Test account")
            .unwrap();

        assert_eq!(result.json_file_id, "legacy-json");
        assert_eq!(result.pdf_file_id, "new-pdf");
        assert_eq!(result.reused_files, 1);
        let calls = calls.lock().unwrap();
        let next: serde_json::Value = serde_json::from_str(&calls[5][4]).unwrap();
        assert_eq!(next["pageToken"], "next");
        assert!(
            next["q"]
                .as_str()
                .unwrap()
                .contains("name contains 'DTE-TEST-'")
        );
        let creates: Vec<_> = calls.iter().filter(|call| call[2] == "create").collect();
        assert_eq!(creates.len(), 1);
        assert_eq!(creates[0][10], "application/pdf");
    }

    #[test]
    fn does_not_create_files_when_drive_cannot_complete_the_lookup() {
        let (service, calls) = existing_folder_service([r#"{"incompleteSearch":true}"#]);
        let (invoice, files, _directory) = invoice_files();
        let result = service.upload_invoice_files(&invoice, &files, "Test account");

        assert!(matches!(result, Err(DriveUploadError::IncompleteSearch)));
        assert!(calls.lock().unwrap().iter().all(|call| call[2] == "list"));
    }

    type Calls = Arc<Mutex<Vec<Vec<String>>>>;

    struct FakeRunner {
        calls: Calls,
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

    fn existing_folder_service<'a>(
        file_responses: impl IntoIterator<Item = &'a str>,
    ) -> (DriveUploadService<FakeRunner>, Calls) {
        let outputs = [
            r#"{"files":[{"id":"root-folder"}]}"#,
            r#"{"files":[{"id":"year-folder"}]}"#,
            r#"{"files":[{"id":"month-folder"}]}"#,
            r#"{"files":[{"id":"purchases-folder"}]}"#,
        ];
        let runner = FakeRunner::new(outputs.into_iter().chain(file_responses).collect());
        let calls = runner.calls.clone();
        (DriveUploadService::new(runner), calls)
    }

    const CODE: &str = "01234567-89AB-CDEF-0123-456789ABCDEF";
    const JSON_BYTES: &[u8] = br#"{"identificacion":{"numeroControl":"DTE-TEST","codigoGeneracion":"01234567-89ab-cdef-0123-456789abcdef","fecEmi":"2026-05-24"}}"#;

    fn invoice_files() -> (InvoiceSummary, SavedInvoiceFiles, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let json_path = directory.path().join("DTE-TEST-invoice.json");
        let pdf_path = directory.path().join("DTE-TEST-invoice.pdf");
        fs::write(&json_path, JSON_BYTES).unwrap();
        fs::write(&pdf_path, b"test PDF bytes").unwrap();
        let invoice = InvoiceSummary::from_json_bytes("invoice.json", JSON_BYTES).unwrap();
        let files = SavedInvoiceFiles {
            json_path,
            pdf_path,
            drive_upload: None,
        };
        (invoice, files, directory)
    }
}
