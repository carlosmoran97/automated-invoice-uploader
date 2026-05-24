use crate::domain::date_range::DateRange;
use base64::{
    Engine,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashSet, fmt, io, process::Command};

const PAGE_LIMIT: usize = 1000;
pub const DEFAULT_DTE_QUERY_FILTER: &str = "DTE-03";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateEmail {
    pub id: String,
    pub thread_id: String,
    pub from: String,
    pub subject: String,
    pub received_at: String,
    pub snippet: String,
    pub internal_date_ms: Option<u64>,
    pub pdf_attachments: Vec<AttachmentSummary>,
    pub json_attachments: Vec<AttachmentSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentSummary {
    pub filename: String,
    pub attachment_id: Option<String>,
    pub mime_type: String,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadedAttachment {
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum GmailSearchError {
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
    PageLimitReached {
        query: String,
        page_limit: usize,
    },
    AttachmentNotDownloadable {
        filename: String,
    },
    AttachmentMissingData {
        filename: String,
    },
    Base64 {
        filename: String,
        source: base64::DecodeError,
    },
}

pub trait CommandRunner {
    fn run(&self, args: &[String]) -> Result<String, GmailSearchError>;
}

#[derive(Default)]
pub struct SystemCommandRunner;

pub struct GmailSearchService<R = SystemCommandRunner> {
    runner: R,
    dte_query_filter: String,
}

impl Default for GmailSearchService<SystemCommandRunner> {
    fn default() -> Self {
        Self {
            runner: SystemCommandRunner,
            dte_query_filter: DEFAULT_DTE_QUERY_FILTER.to_string(),
        }
    }
}

impl<R: CommandRunner> GmailSearchService<R> {
    #[cfg(test)]
    pub fn new(runner: R) -> Self {
        Self {
            runner,
            dte_query_filter: DEFAULT_DTE_QUERY_FILTER.to_string(),
        }
    }

    pub fn with_dte_query_filter(mut self, dte_query_filter: impl Into<String>) -> Self {
        self.dte_query_filter = dte_query_filter.into();
        self
    }

    pub fn search_invoice_candidates(
        &self,
        range: &DateRange,
    ) -> Result<Vec<CandidateEmail>, GmailSearchError> {
        let pdf_query = build_attachment_query(range, "pdf", &self.dte_query_filter);
        let json_query = build_attachment_query(range, "json", &self.dte_query_filter);
        let pdf_message_ids = self.list_message_ids(&pdf_query)?;
        let json_message_ids = self.list_message_ids(&json_query)?;
        let intersected_ids = intersect_ids(&pdf_message_ids, &json_message_ids);
        let mut candidates = Vec::new();

        for message_id in intersected_ids {
            if let Some(candidate) = self.fetch_candidate(&message_id)? {
                candidates.push(candidate);
            }
        }

        candidates.sort_by(|left, right| right.internal_date_ms.cmp(&left.internal_date_ms));

        Ok(candidates)
    }

    pub fn download_attachment(
        &self,
        message_id: &str,
        attachment: &AttachmentSummary,
    ) -> Result<DownloadedAttachment, GmailSearchError> {
        let attachment_id = attachment.attachment_id.as_deref().ok_or_else(|| {
            GmailSearchError::AttachmentNotDownloadable {
                filename: attachment.filename.clone(),
            }
        })?;
        let params = json!({
            "userId": "me",
            "messageId": message_id,
            "id": attachment_id,
        })
        .to_string();
        let args = vec![
            "gmail".to_string(),
            "users".to_string(),
            "messages".to_string(),
            "attachments".to_string(),
            "get".to_string(),
            "--params".to_string(),
            params,
        ];
        let stdout = self.runner.run(&args)?;
        let body: MessagePartBody =
            parse_json(&stdout, format!("attachment {}", attachment.filename))?;
        let data = body
            .data
            .ok_or_else(|| GmailSearchError::AttachmentMissingData {
                filename: attachment.filename.clone(),
            })?;

        Ok(DownloadedAttachment {
            filename: attachment.filename.clone(),
            bytes: decode_base64_url(&attachment.filename, &data)?,
        })
    }

    fn list_message_ids(&self, query: &str) -> Result<Vec<String>, GmailSearchError> {
        let params = json!({
            "userId": "me",
            "q": query,
            "labelIds": ["INBOX"],
            "maxResults": 500,
        })
        .to_string();
        let args = vec![
            "gmail".to_string(),
            "users".to_string(),
            "messages".to_string(),
            "list".to_string(),
            "--params".to_string(),
            params,
            "--page-all".to_string(),
            "--page-limit".to_string(),
            PAGE_LIMIT.to_string(),
        ];
        let stdout = self.runner.run(&args)?;
        let output = parse_list_output(&stdout, query)?;

        if output.hit_page_limit {
            return Err(GmailSearchError::PageLimitReached {
                query: query.to_string(),
                page_limit: PAGE_LIMIT,
            });
        }

        Ok(output.message_ids)
    }

    fn fetch_candidate(
        &self,
        message_id: &str,
    ) -> Result<Option<CandidateEmail>, GmailSearchError> {
        let params = json!({
            "userId": "me",
            "id": message_id,
            "format": "full",
        })
        .to_string();
        let args = vec![
            "gmail".to_string(),
            "users".to_string(),
            "messages".to_string(),
            "get".to_string(),
            "--params".to_string(),
            params,
        ];
        let stdout = self.runner.run(&args)?;
        let message: GmailMessage =
            parse_json(&stdout, format!("message details for {message_id}"))?;

        Ok(message_to_candidate(message))
    }
}

impl CommandRunner for SystemCommandRunner {
    fn run(&self, args: &[String]) -> Result<String, GmailSearchError> {
        let output = Command::new("gws").args(args).output().map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                GmailSearchError::CliNotFound
            } else {
                GmailSearchError::Io(error)
            }
        })?;

        if !output.status.success() {
            let status = output.status.code().map_or_else(
                || "terminated by signal".to_string(),
                |code| code.to_string(),
            );
            return Err(GmailSearchError::CommandFailed {
                command: format!("gws {}", args.join(" ")),
                status,
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl fmt::Display for GmailSearchError {
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
            Self::PageLimitReached { query, page_limit } => write!(
                formatter,
                "Gmail search reached the {page_limit}-page limit for query `{query}`. Narrow the date range and try again."
            ),
            Self::AttachmentNotDownloadable { filename } => {
                write!(formatter, "Attachment `{filename}` is not downloadable.")
            }
            Self::AttachmentMissingData { filename } => {
                write!(
                    formatter,
                    "Attachment `{filename}` did not include downloadable data."
                )
            }
            Self::Base64 { filename, source } => {
                write!(
                    formatter,
                    "Failed to decode attachment `{filename}`: {source}"
                )
            }
        }
    }
}

impl std::error::Error for GmailSearchError {}

#[derive(Deserialize)]
struct ListMessagesResponse {
    #[serde(default)]
    messages: Vec<MessageReference>,
    #[serde(default, rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct MessageReference {
    id: String,
}

#[derive(Deserialize)]
struct GmailMessage {
    id: String,
    #[serde(default, rename = "threadId")]
    thread_id: Option<String>,
    #[serde(default, rename = "internalDate")]
    internal_date: Option<String>,
    #[serde(default)]
    snippet: Option<String>,
    #[serde(default)]
    payload: Option<MessagePart>,
}

#[derive(Deserialize)]
struct MessagePart {
    #[serde(default, rename = "mimeType")]
    mime_type: Option<String>,
    #[serde(default)]
    filename: Option<String>,
    #[serde(default)]
    headers: Vec<MessageHeader>,
    #[serde(default)]
    body: Option<MessagePartBody>,
    #[serde(default)]
    parts: Vec<MessagePart>,
}

#[derive(Deserialize)]
struct MessageHeader {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct MessagePartBody {
    #[serde(default, rename = "attachmentId")]
    attachment_id: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    data: Option<String>,
}

struct ListOutput {
    message_ids: Vec<String>,
    hit_page_limit: bool,
}

fn build_attachment_query(range: &DateRange, extension: &str, dte_query_filter: &str) -> String {
    let mut query = format!(
        "after:{} before:{} filename:{}",
        range.gmail_after_date(),
        range.gmail_before_date(),
        extension
    );
    let dte_query_filter = dte_query_filter.trim();

    if !dte_query_filter.is_empty() {
        query.push(' ');
        query.push_str(dte_query_filter);
    }

    query
}

fn parse_list_output(stdout: &str, query: &str) -> Result<ListOutput, GmailSearchError> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(ListOutput {
            message_ids: Vec::new(),
            hit_page_limit: false,
        });
    }

    if let Ok(page) = serde_json::from_str::<ListMessagesResponse>(trimmed) {
        return Ok(list_output_from_pages(vec![page], query));
    }

    let mut pages = Vec::new();
    for line in trimmed.lines().filter(|line| !line.trim().is_empty()) {
        pages.push(parse_json::<ListMessagesResponse>(
            line,
            format!("message list page for query `{query}`"),
        )?);
    }

    Ok(list_output_from_pages(pages, query))
}

fn list_output_from_pages(pages: Vec<ListMessagesResponse>, _query: &str) -> ListOutput {
    let hit_page_limit = pages.len() >= PAGE_LIMIT
        && pages
            .last()
            .and_then(|page| page.next_page_token.as_ref())
            .is_some();
    let mut seen = HashSet::new();
    let mut message_ids = Vec::new();

    for page in pages {
        for message in page.messages {
            if seen.insert(message.id.clone()) {
                message_ids.push(message.id);
            }
        }
    }

    ListOutput {
        message_ids,
        hit_page_limit,
    }
}

fn parse_json<T>(input: &str, context: String) -> Result<T, GmailSearchError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(input).map_err(|source| GmailSearchError::Json { context, source })
}

fn decode_base64_url(filename: &str, data: &str) -> Result<Vec<u8>, GmailSearchError> {
    URL_SAFE_NO_PAD
        .decode(data)
        .or_else(|_| URL_SAFE.decode(data))
        .map_err(|source| GmailSearchError::Base64 {
            filename: filename.to_string(),
            source,
        })
}

fn intersect_ids(pdf_message_ids: &[String], json_message_ids: &[String]) -> Vec<String> {
    let json_message_ids = json_message_ids.iter().collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut intersected_ids = Vec::new();

    for message_id in pdf_message_ids {
        if json_message_ids.contains(message_id) && seen.insert(message_id) {
            intersected_ids.push(message_id.clone());
        }
    }

    intersected_ids
}

fn message_to_candidate(message: GmailMessage) -> Option<CandidateEmail> {
    let payload = message.payload.as_ref()?;
    let mut pdf_attachments = Vec::new();
    let mut json_attachments = Vec::new();

    extract_attachments(payload, &mut pdf_attachments, &mut json_attachments);

    if pdf_attachments.is_empty() || json_attachments.is_empty() {
        return None;
    }

    let from = header_value(&payload.headers, "From").unwrap_or_default();
    let subject = header_value(&payload.headers, "Subject").unwrap_or_default();
    let received_at = header_value(&payload.headers, "Date")
        .or_else(|| message.internal_date.clone())
        .unwrap_or_default();
    let internal_date_ms = message
        .internal_date
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok());

    Some(CandidateEmail {
        id: message.id,
        thread_id: message.thread_id.unwrap_or_default(),
        from,
        subject,
        received_at,
        snippet: message.snippet.unwrap_or_default(),
        internal_date_ms,
        pdf_attachments,
        json_attachments,
    })
}

fn extract_attachments(
    part: &MessagePart,
    pdf_attachments: &mut Vec<AttachmentSummary>,
    json_attachments: &mut Vec<AttachmentSummary>,
) {
    if let Some(summary) = attachment_summary(part) {
        if is_pdf_attachment(&summary) {
            pdf_attachments.push(summary);
        } else if is_json_attachment(&summary) {
            json_attachments.push(summary);
        }
    }

    for child in &part.parts {
        extract_attachments(child, pdf_attachments, json_attachments);
    }
}

fn attachment_summary(part: &MessagePart) -> Option<AttachmentSummary> {
    let filename = part.filename.as_deref()?.trim();
    if filename.is_empty() {
        return None;
    }

    Some(AttachmentSummary {
        filename: filename.to_string(),
        attachment_id: part
            .body
            .as_ref()
            .and_then(|body| body.attachment_id.clone()),
        mime_type: part.mime_type.clone().unwrap_or_default(),
        size: part.body.as_ref().and_then(|body| body.size),
    })
}

fn is_pdf_attachment(attachment: &AttachmentSummary) -> bool {
    let filename = attachment.filename.to_ascii_lowercase();
    let mime_type = attachment.mime_type.to_ascii_lowercase();

    filename.ends_with(".pdf") || mime_type == "application/pdf"
}

fn is_json_attachment(attachment: &AttachmentSummary) -> bool {
    let filename = attachment.filename.to_ascii_lowercase();
    let mime_type = attachment.mime_type.to_ascii_lowercase();

    filename.ends_with(".json") || mime_type.contains("json")
}

fn header_value(headers: &[MessageHeader], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    #[test]
    fn builds_gmail_attachment_query_with_credito_fiscal_filter() {
        let range = DateRange::parse("2026-05-01", "2026-05-31").unwrap();

        assert_eq!(
            build_attachment_query(&range, "pdf", DEFAULT_DTE_QUERY_FILTER),
            "after:2026/05/01 before:2026/06/01 filename:pdf DTE-03"
        );
    }

    #[test]
    fn omits_credito_fiscal_filter_when_empty() {
        let range = DateRange::parse("2026-05-01", "2026-05-31").unwrap();

        assert_eq!(
            build_attachment_query(&range, "json", " "),
            "after:2026/05/01 before:2026/06/01 filename:json"
        );
    }

    #[test]
    fn parses_ndjson_message_list_pages() {
        let stdout = r#"{"messages":[{"id":"one"},{"id":"two"}]}
{"messages":[{"id":"two"},{"id":"three"}]}"#;
        let output = parse_list_output(stdout, "filename:pdf").unwrap();

        assert_eq!(output.message_ids, vec!["one", "two", "three"]);
        assert!(!output.hit_page_limit);
    }

    #[test]
    fn intersects_message_ids_in_pdf_search_order() {
        let pdf_ids = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let json_ids = vec!["three".to_string(), "one".to_string()];

        assert_eq!(intersect_ids(&pdf_ids, &json_ids), vec!["one", "three"]);
    }

    #[test]
    fn extracts_nested_pdf_and_json_attachments() {
        let message: GmailMessage = serde_json::from_str(
            r#"{
                "id": "m1",
                "threadId": "t1",
                "internalDate": "1770000000000",
                "snippet": "invoice",
                "payload": {
                    "headers": [
                        {"name": "From", "value": "Vendor <vendor@example.com>"},
                        {"name": "Subject", "value": "Factura"},
                        {"name": "Date", "value": "Fri, 01 May 2026 09:00:00 -0600"}
                    ],
                    "parts": [
                        {
                            "mimeType": "multipart/mixed",
                            "parts": [
                                {
                                    "filename": "invoice.PDF",
                                    "mimeType": "application/pdf",
                                    "body": {"attachmentId": "pdf-1", "size": 12}
                                },
                                {
                                    "filename": "invoice.json",
                                    "mimeType": "application/json",
                                    "body": {"attachmentId": "json-1", "size": 24}
                                }
                            ]
                        }
                    ]
                }
            }"#,
        )
        .unwrap();

        let candidate = message_to_candidate(message).unwrap();

        assert_eq!(candidate.pdf_attachments[0].filename, "invoice.PDF");
        assert_eq!(candidate.json_attachments[0].filename, "invoice.json");
        assert_eq!(candidate.from, "Vendor <vendor@example.com>");
    }

    #[test]
    fn searches_with_fake_runner_and_fetches_intersection_only() {
        let runner = FakeRunner::new(vec![
            r#"{"messages":[{"id":"pdf-only"},{"id":"both"}]}"#,
            r#"{"messages":[{"id":"both"},{"id":"json-only"}]}"#,
            r#"{
                "id": "both",
                "threadId": "thread",
                "internalDate": "1770000000000",
                "payload": {
                    "headers": [
                        {"name": "From", "value": "Vendor <vendor@example.com>"},
                        {"name": "Subject", "value": "Invoice"}
                    ],
                    "parts": [
                        {
                            "filename": "invoice.pdf",
                            "mimeType": "application/pdf",
                            "body": {"attachmentId": "pdf-id"}
                        },
                        {
                            "filename": "invoice.json",
                            "mimeType": "application/json",
                            "body": {"attachmentId": "json-id"}
                        }
                    ]
                }
            }"#,
        ]);
        let calls = runner.calls.clone();
        let service = GmailSearchService::new(runner);
        let range = DateRange::parse("2026-05-01", "2026-05-31").unwrap();

        let candidates = service.search_invoice_candidates(&range).unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "both");
        assert_eq!(calls.lock().unwrap().len(), 3);
    }

    #[test]
    fn search_uses_configured_dte_query_filter() {
        let runner = FakeRunner::new(vec![
            r#"{"messages":[{"id":"both"}]}"#,
            r#"{"messages":[{"id":"both"}]}"#,
            r#"{
                "id": "both",
                "threadId": "thread",
                "internalDate": "1770000000000",
                "payload": {
                    "headers": [
                        {"name": "From", "value": "Vendor <vendor@example.com>"},
                        {"name": "Subject", "value": "Invoice"}
                    ],
                    "parts": [
                        {
                            "filename": "invoice.pdf",
                            "mimeType": "application/pdf",
                            "body": {"attachmentId": "pdf-id"}
                        },
                        {
                            "filename": "invoice.json",
                            "mimeType": "application/json",
                            "body": {"attachmentId": "json-id"}
                        }
                    ]
                }
            }"#,
        ]);
        let calls = runner.calls.clone();
        let service = GmailSearchService::new(runner).with_dte_query_filter("custom-filter");
        let range = DateRange::parse("2026-05-01", "2026-05-31").unwrap();

        let candidates = service.search_invoice_candidates(&range).unwrap();

        let calls = calls.lock().unwrap();
        assert_eq!(candidates.len(), 1);
        assert!(calls[0][5].contains("custom-filter"));
        assert!(calls[1][5].contains("custom-filter"));
    }

    #[test]
    fn downloads_attachment_data() {
        let runner = FakeRunner::new(vec![r#"{"data":"SGVsbG8"}"#]);
        let service = GmailSearchService::new(runner);
        let attachment = AttachmentSummary {
            filename: "invoice.json".to_string(),
            attachment_id: Some("attachment-id".to_string()),
            mime_type: "application/json".to_string(),
            size: Some(5),
        };

        let downloaded = service
            .download_attachment("message-id", &attachment)
            .unwrap();

        assert_eq!(downloaded.filename, "invoice.json");
        assert_eq!(downloaded.bytes, b"Hello");
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
        fn run(&self, args: &[String]) -> Result<String, GmailSearchError> {
            self.calls.lock().unwrap().push(args.to_vec());
            self.outputs
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| GmailSearchError::Io(io::Error::other("missing fake output")))
        }
    }
}
