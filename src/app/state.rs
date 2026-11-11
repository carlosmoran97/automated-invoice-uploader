use super::review::InvoiceReview;
use crate::{
    domain::date_range::{DateRange, previous_month_bounds},
    i18n::{Language, messages_for},
    services::{
        authentication::AuthenticationEvent, gmail_search::CandidateEmail,
        invoice_files::SavedInvoiceFiles, settings::AppSettings,
    },
};
use std::collections::HashSet;
use time::Date;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Startup,
    Home,
    Results,
    Review,
    Settings,
}

pub(crate) enum AuthenticationState {
    Checking(AuthenticationEvent),
    Ready,
    Failed(String),
}

#[derive(Clone)]
pub(crate) struct SearchCriteriaInput {
    pub initial_date: Date,
    pub final_date: Date,
    pub dte_query_filter: String,
}

#[derive(Clone)]
pub(crate) struct SettingsInput {
    pub download_dir: String,
    pub drive_root_folder: String,
    pub dte_query_filter: String,
    pub language: Language,
}

impl From<&AppSettings> for SettingsInput {
    fn from(settings: &AppSettings) -> Self {
        Self {
            download_dir: settings.download_dir.display().to_string(),
            drive_root_folder: settings.drive_root_folder.clone(),
            dte_query_filter: settings.dte_query_filter.clone(),
            language: settings.language,
        }
    }
}

#[derive(Default)]
pub(crate) struct ReviewContext {
    pub emails: Vec<CandidateEmail>,
    pub index: usize,
    pub skipped: usize,
    pub processed: Vec<ProcessedInvoice>,
}

pub(crate) struct ProcessedInvoice {
    pub issuer: String,
    pub reference: String,
    pub issue_date: String,
    pub files: SavedInvoiceFiles,
}

pub(crate) enum ReviewState {
    Idle,
    Loading,
    Ready(Box<InvoiceReview>),
    Saving(Box<InvoiceReview>),
    Error(String),
    Complete,
}

pub(crate) struct AppState {
    pub screen: Screen,
    pub authentication: AuthenticationState,
    pub settings: AppSettings,
    pub criteria: SearchCriteriaInput,
    pub searching: bool,
    pub search_period: String,
    pub saving_settings: bool,
    pub error: Option<String>,
    pub settings_status: Option<String>,
    pub results: Vec<CandidateEmail>,
    pub highlighted: usize,
    pub selected_ids: HashSet<String>,
    pub review: ReviewState,
    pub context: ReviewContext,
}

impl AppState {
    pub fn new(settings: AppSettings, today: Date) -> Self {
        let (initial_date, final_date) = previous_month_bounds(today);
        Self {
            screen: Screen::Startup,
            authentication: AuthenticationState::Checking(AuthenticationEvent::Checking),
            criteria: SearchCriteriaInput {
                initial_date,
                final_date,
                dte_query_filter: settings.dte_query_filter.clone(),
            },
            settings,
            searching: false,
            search_period: String::new(),
            saving_settings: false,
            error: None,
            settings_status: None,
            results: Vec::new(),
            highlighted: 0,
            selected_ids: HashSet::new(),
            review: ReviewState::Idle,
            context: ReviewContext::default(),
        }
    }

    pub fn busy(&self) -> bool {
        self.searching
            || self.saving_settings
            || matches!(self.authentication, AuthenticationState::Checking(_))
            || matches!(self.review, ReviewState::Loading | ReviewState::Saving(_))
    }

    pub fn navigate(&mut self, screen: Screen) -> bool {
        if self.busy()
            || !matches!(self.authentication, AuthenticationState::Ready)
            || (self.screen == Screen::Review && !matches!(self.review, ReviewState::Complete))
        {
            return false;
        }
        self.screen = screen;
        self.error = None;
        true
    }

    pub fn begin_search(&mut self) -> Option<(DateRange, String)> {
        if self.busy()
            || self.screen != Screen::Home
            || !matches!(self.authentication, AuthenticationState::Ready)
        {
            return None;
        }
        let range = match DateRange::parse(
            &self.criteria.initial_date.to_string(),
            &self.criteria.final_date.to_string(),
        ) {
            Ok(range) => range,
            Err(error) => {
                self.error = Some(
                    messages_for(self.settings.language)
                        .date_range_error(&error)
                        .into(),
                );
                return None;
            }
        };
        self.error = None;
        self.search_period = format!("{} \u{2014} {}", range.initial_date(), range.final_date());
        self.searching = true;
        Some((range, self.criteria.dte_query_filter.trim().to_owned()))
    }

    pub fn finish_search(&mut self, results: Vec<CandidateEmail>) {
        self.searching = false;
        self.selected_ids = results.iter().map(|email| email.id.clone()).collect();
        self.results = results;
        self.highlighted = 0;
        self.screen = Screen::Results;
    }

    pub fn toggle_email(&mut self, index: usize) {
        if self.busy() || self.screen != Screen::Results {
            return;
        }
        if let Some(email) = self.results.get(index) {
            if !self.selected_ids.remove(&email.id) {
                self.selected_ids.insert(email.id.clone());
            }
        }
    }

    pub fn begin_review(&mut self) -> Option<CandidateEmail> {
        if self.busy() || self.screen != Screen::Results {
            return None;
        }
        let emails: Vec<_> = self
            .results
            .iter()
            .filter(|email| self.selected_ids.contains(&email.id))
            .cloned()
            .collect();
        if emails.is_empty() {
            return None;
        }
        self.context = ReviewContext {
            emails,
            ..ReviewContext::default()
        };
        self.screen = Screen::Review;
        self.next_review()
    }

    fn next_review(&mut self) -> Option<CandidateEmail> {
        self.error = None;
        match self.context.emails.get(self.context.index) {
            Some(email) => {
                self.review = ReviewState::Loading;
                Some(email.clone())
            }
            None => {
                self.review = ReviewState::Complete;
                None
            }
        }
    }

    pub fn begin_save(&mut self) -> Option<InvoiceReview> {
        if self.screen != Screen::Review {
            return None;
        }
        let ReviewState::Ready(review) = &self.review else {
            return None;
        };
        let review = review.clone();
        self.error = None;
        self.review = ReviewState::Saving(review.clone());
        Some(*review)
    }

    pub fn fail_save(&mut self, error: String) {
        if let ReviewState::Saving(review) = &self.review {
            self.review = ReviewState::Ready(review.clone());
            self.error = Some(error);
        }
    }

    pub fn skip_review(&mut self) -> Option<CandidateEmail> {
        if self.screen != Screen::Review
            || !matches!(self.review, ReviewState::Ready(_) | ReviewState::Error(_))
        {
            return None;
        }
        self.context.skipped += 1;
        self.context.index += 1;
        self.next_review()
    }

    pub fn finish_save(&mut self, files: SavedInvoiceFiles) -> Option<CandidateEmail> {
        let ReviewState::Saving(review) = &self.review else {
            return None;
        };
        self.context.processed.push(ProcessedInvoice {
            issuer: review.invoice.issuer_name.clone(),
            reference: review.invoice.file_slug(),
            issue_date: review.invoice.issue_date.clone(),
            files,
        });
        self.context.index += 1;
        self.next_review()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn state() -> AppState {
        let mut state = AppState::new(
            AppSettings::default(),
            Date::from_calendar_date(2026, Month::November, 10).unwrap(),
        );
        state.authentication = AuthenticationState::Ready;
        state.screen = Screen::Home;
        state
    }

    fn email(id: &str) -> CandidateEmail {
        CandidateEmail {
            id: id.into(),
            thread_id: String::new(),
            from: String::new(),
            subject: String::new(),
            received_at: String::new(),
            snippet: String::new(),
            internal_date_ms: None,
            pdf_attachments: Vec::new(),
            json_attachments: Vec::new(),
        }
    }

    #[test]
    fn navigation_preserves_the_selected_period() {
        let mut state = state();
        assert_eq!(state.criteria.initial_date.to_string(), "2026-10-01");
        assert_eq!(state.criteria.final_date.to_string(), "2026-10-31");
        state.criteria.initial_date = state.criteria.initial_date.next_day().unwrap();
        assert!(state.navigate(Screen::Settings));
        assert!(state.navigate(Screen::Home));
        assert_eq!(state.criteria.initial_date.to_string(), "2026-10-02");
    }

    #[test]
    fn search_validates_dates_and_blocks_duplicate_work_and_navigation() {
        let mut state = state();
        state.criteria.initial_date = state.criteria.final_date.next_day().unwrap();
        assert!(state.begin_search().is_none());
        assert!(state.error.is_some());
        state.criteria.initial_date = state.criteria.final_date;
        assert!(state.begin_search().is_some());
        assert!(state.begin_search().is_none());
        assert!(!state.navigate(Screen::Settings));
    }

    #[test]
    fn results_select_all_by_default_and_review_preserves_order() {
        let mut state = state();
        state.finish_search(vec![email("a"), email("b"), email("c")]);
        assert_eq!(state.selected_ids.len(), 3);
        state.toggle_email(1);
        assert_eq!(state.begin_review().unwrap().id, "a");
        assert_eq!(
            state
                .context
                .emails
                .iter()
                .map(|email| email.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "c"]
        );
        assert!(state.begin_review().is_none());
        assert!(state.skip_review().is_none());
        assert!(!state.navigate(Screen::Home));
        state.review = ReviewState::Error("broken JSON".into());
        assert_eq!(state.skip_review().unwrap().id, "c");
        state.review = ReviewState::Error("broken JSON".into());
        assert!(state.skip_review().is_none());
        assert_eq!(state.context.skipped, 2);
        assert!(matches!(state.review, ReviewState::Complete));
        assert!(state.navigate(Screen::Results));
    }

    #[test]
    fn empty_selection_cannot_start_a_review_and_new_search_resets_selection() {
        let mut state = state();
        state.finish_search(vec![email("a")]);
        state.toggle_email(0);
        assert!(state.begin_review().is_none());
        state.finish_search(vec![email("b")]);
        assert_eq!(state.selected_ids, HashSet::from(["b".into()]));
        state.finish_search(Vec::new());
        assert!(state.begin_review().is_none());
    }

    #[test]
    fn accepting_an_invoice_blocks_double_upload_and_skip_until_finished() {
        let mut state = state();
        state.finish_search(vec![email("a")]);
        state.begin_review();
        let json = crate::services::gmail_search::DownloadedAttachment {
            filename: "test.json".into(),
            bytes: br#"{"identificacion":{"numeroControl":"DTE-TEST","fecEmi":"2026-10-15"},"emisor":{"nombre":"Test supplier"}}"#.to_vec(),
        };
        state.review = ReviewState::Ready(Box::new(InvoiceReview {
            email: email("a"),
            invoice: crate::domain::invoice::InvoiceSummary::from_json_bytes(
                &json.filename,
                &json.bytes,
            )
            .unwrap(),
            json_file: json,
        }));
        assert!(state.begin_save().is_some());
        assert!(state.begin_save().is_none());
        assert!(state.skip_review().is_none());
        assert!(!state.navigate(Screen::Home));
        state.fail_save("PDF upload failed".into());
        assert_eq!(state.error.as_deref(), Some("PDF upload failed"));
        assert_eq!(state.begin_save().unwrap().email.id, "a");
        assert!(state.error.is_none());
        state.finish_save(SavedInvoiceFiles {
            json_path: "test.json".into(),
            pdf_path: "test.pdf".into(),
            drive_upload: None,
        });
        assert_eq!(state.context.processed.len(), 1);
        let processed = &state.context.processed[0];
        assert_eq!(processed.issuer, "Test supplier");
        assert_eq!(processed.reference, "DTE-TEST");
        assert_eq!(processed.issue_date, "2026-10-15");
        assert_eq!(processed.files.pdf_path, std::path::Path::new("test.pdf"));
        assert!(matches!(state.review, ReviewState::Complete));
    }
}
