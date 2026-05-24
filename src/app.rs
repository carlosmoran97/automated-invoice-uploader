use crate::{
    components::{
        home::{HomePage, HomePageAction, HomePageStatus, SearchCriteriaInput},
        results::{ResultsPage, ResultsPageAction},
        review::{ReviewPage, ReviewView},
    },
    domain::{
        date_range::DateRange,
        invoice::{InvoiceParseError, InvoiceSummary},
    },
    i18n::messages,
    services::{
        drive_upload::{DriveUploadError, DriveUploadService},
        gmail_search::{
            CandidateEmail, DownloadedAttachment, GmailSearchError, GmailSearchService,
        },
        invoice_files::{SavedInvoiceFiles, save_invoice_files},
    },
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

type SearchOutcome = Result<Vec<CandidateEmail>, crate::services::gmail_search::GmailSearchError>;

pub struct App {
    screen: Screen,
    home_page: HomePage,
    results_page: ResultsPage,
    review_page: ReviewPage,
    search: SearchState,
    review: ReviewState,
    results: Vec<CandidateEmail>,
}

pub enum AppAction {
    Continue,
    Quit,
}

enum Screen {
    Home,
    Results,
    Review,
}

enum SearchState {
    Idle,
    Running {
        receiver: Receiver<SearchOutcome>,
        initial_date: String,
        final_date: String,
    },
    Failed(String),
}

#[derive(Clone)]
struct ReviewContext {
    emails: Vec<CandidateEmail>,
    index: usize,
    processed: usize,
    skipped: usize,
    saved_files: Vec<SavedInvoiceFiles>,
}

#[derive(Clone)]
struct InvoiceReview {
    email: CandidateEmail,
    json_file: DownloadedAttachment,
    invoice: InvoiceSummary,
}

enum ReviewState {
    Idle,
    Loading {
        context: ReviewContext,
        receiver: Receiver<Result<InvoiceReview, ReviewError>>,
    },
    Ready {
        context: ReviewContext,
        review: InvoiceReview,
    },
    Saving {
        context: ReviewContext,
        review: InvoiceReview,
        receiver: Receiver<Result<SavedInvoiceFiles, ReviewError>>,
    },
    Error {
        context: ReviewContext,
        message: String,
    },
    Complete {
        processed: usize,
        skipped: usize,
        saved_files: Vec<SavedInvoiceFiles>,
    },
}

#[derive(Debug)]
enum ReviewError {
    Gmail(GmailSearchError),
    Drive(DriveUploadError),
    Invoice(InvoiceParseError),
    Io(std::io::Error),
    MissingJson,
    MissingPdf,
}

impl Default for App {
    fn default() -> Self {
        Self {
            screen: Screen::Home,
            home_page: HomePage::default(),
            results_page: ResultsPage::default(),
            review_page: ReviewPage,
            search: SearchState::Idle,
            review: ReviewState::Idle,
            results: Vec::new(),
        }
    }
}

impl App {
    pub fn render(&self, frame: &mut Frame) {
        match self.screen {
            Screen::Home => self.home_page.render(frame, self.home_status()),
            Screen::Results => self.results_page.render(frame, &self.results),
            Screen::Review => self.review_page.render(frame, self.review_view()),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        if self.is_searching() {
            return match key.code {
                KeyCode::Char('q') => AppAction::Quit,
                _ => AppAction::Continue,
            };
        }

        match self.screen {
            Screen::Home => self.handle_home_key(key),
            Screen::Results => self.handle_results_key(key),
            Screen::Review => self.handle_review_key(key),
        }
    }

    pub fn tick(&mut self) {
        self.tick_review();

        let SearchState::Running { receiver, .. } = &self.search else {
            return;
        };

        match receiver.try_recv() {
            Ok(Ok(results)) => {
                self.results = results;
                self.results_page.reset(&self.results);
                self.search = SearchState::Idle;
                self.screen = Screen::Results;
            }
            Ok(Err(error)) => {
                self.search = SearchState::Failed(messages().gmail_search_error(&error));
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.search = SearchState::Failed(messages().gmail_search_stopped.to_string());
            }
        }
    }

    fn handle_home_key(&mut self, key: KeyEvent) -> AppAction {
        if matches!(self.search, SearchState::Failed(_)) {
            self.search = SearchState::Idle;
        }

        match self.home_page.handle_key(key) {
            HomePageAction::Continue => AppAction::Continue,
            HomePageAction::Quit => AppAction::Quit,
            HomePageAction::Submit(input) => {
                self.start_search(input);
                AppAction::Continue
            }
        }
    }

    fn handle_results_key(&mut self, key: KeyEvent) -> AppAction {
        match self.results_page.handle_key(key, &self.results) {
            ResultsPageAction::Continue => AppAction::Continue,
            ResultsPageAction::Back => {
                self.screen = Screen::Home;
                AppAction::Continue
            }
            ResultsPageAction::ReviewSelected => {
                let emails = self.results_page.selected_emails(&self.results);
                if !emails.is_empty() {
                    self.start_review(emails);
                }
                AppAction::Continue
            }
            ResultsPageAction::Quit => AppAction::Quit,
        }
    }

    fn handle_review_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Char('q') => return AppAction::Quit,
            _ => {}
        }

        match &self.review {
            ReviewState::Ready { .. } => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => self.process_current_review(),
                KeyCode::Char('n') | KeyCode::Char('N') => self.skip_current_review(),
                _ => {}
            },
            ReviewState::Error { .. } => match key.code {
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter => {
                    self.skip_current_review()
                }
                _ => {}
            },
            ReviewState::Complete { .. } => match key.code {
                KeyCode::Enter | KeyCode::Esc => self.screen = Screen::Results,
                _ => {}
            },
            _ => {}
        }

        AppAction::Continue
    }

    fn start_search(&mut self, input: SearchCriteriaInput) {
        let range = match DateRange::parse(&input.initial_date, &input.final_date) {
            Ok(range) => range,
            Err(error) => {
                self.search = SearchState::Failed(messages().date_range_error(&error).to_string());
                return;
            }
        };
        let (sender, receiver) = mpsc::channel();
        let initial_date = range.initial_date();
        let final_date = range.final_date();

        thread::spawn(move || {
            let service = GmailSearchService::default();
            let result = service.search_invoice_candidates(&range);
            let _ = sender.send(result);
        });

        self.search = SearchState::Running {
            receiver,
            initial_date,
            final_date,
        };
    }

    fn home_status(&self) -> HomePageStatus<'_> {
        match &self.search {
            SearchState::Idle => HomePageStatus::Idle,
            SearchState::Running {
                initial_date,
                final_date,
                ..
            } => HomePageStatus::Searching {
                initial_date,
                final_date,
            },
            SearchState::Failed(error) => HomePageStatus::Error(error),
        }
    }

    fn is_searching(&self) -> bool {
        matches!(self.search, SearchState::Running { .. })
    }

    fn start_review(&mut self, emails: Vec<CandidateEmail>) {
        self.screen = Screen::Review;
        let context = ReviewContext {
            emails,
            index: 0,
            processed: 0,
            skipped: 0,
            saved_files: Vec::new(),
        };
        self.start_loading_review(context);
    }

    fn start_loading_review(&mut self, context: ReviewContext) {
        if context.index >= context.emails.len() {
            self.review = ReviewState::Complete {
                processed: context.processed,
                skipped: context.skipped,
                saved_files: context.saved_files,
            };
            return;
        }

        let email = context.emails[context.index].clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let _ = sender.send(load_invoice_review(email));
        });
        self.review = ReviewState::Loading { context, receiver };
    }

    fn process_current_review(&mut self) {
        let ReviewState::Ready { context, review } =
            std::mem::replace(&mut self.review, ReviewState::Idle)
        else {
            return;
        };
        let (sender, receiver) = mpsc::channel();
        let review_for_thread = review.clone();

        thread::spawn(move || {
            let _ = sender.send(save_invoice_review(review_for_thread));
        });

        self.review = ReviewState::Saving {
            context,
            review,
            receiver,
        };
    }

    fn skip_current_review(&mut self) {
        let mut context = match std::mem::replace(&mut self.review, ReviewState::Idle) {
            ReviewState::Ready { context, .. } | ReviewState::Error { context, .. } => context,
            other => {
                self.review = other;
                return;
            }
        };

        context.skipped += 1;
        context.index += 1;
        self.start_loading_review(context);
    }

    fn tick_review(&mut self) {
        let next_state = match &self.review {
            ReviewState::Loading { context, receiver } => match receiver.try_recv() {
                Ok(Ok(review)) => Some(ReviewState::Ready {
                    context: context.clone(),
                    review,
                }),
                Ok(Err(error)) => Some(ReviewState::Error {
                    context: context.clone(),
                    message: review_error_message(error),
                }),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(ReviewState::Error {
                    context: context.clone(),
                    message: messages().gmail_search_stopped.to_string(),
                }),
            },
            ReviewState::Saving {
                context, receiver, ..
            } => match receiver.try_recv() {
                Ok(Ok(saved_files)) => {
                    let mut context = context.clone();
                    context.processed += 1;
                    context.index += 1;
                    context.saved_files.push(saved_files);
                    self.start_loading_review(context);
                    None
                }
                Ok(Err(error)) => Some(ReviewState::Error {
                    context: context.clone(),
                    message: review_error_message(error),
                }),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(ReviewState::Error {
                    context: context.clone(),
                    message: messages().gmail_search_stopped.to_string(),
                }),
            },
            _ => None,
        };

        if let Some(next_state) = next_state {
            self.review = next_state;
        }
    }

    fn review_view(&self) -> ReviewView<'_> {
        match &self.review {
            ReviewState::Loading { context, .. } => ReviewView::Loading {
                index: context.index,
                total: context.emails.len(),
                email: &context.emails[context.index],
            },
            ReviewState::Ready { context, review } => ReviewView::Ready {
                index: context.index,
                total: context.emails.len(),
                email: &review.email,
                invoice: &review.invoice,
            },
            ReviewState::Saving {
                context, review, ..
            } => ReviewView::Saving {
                index: context.index,
                total: context.emails.len(),
                invoice: &review.invoice,
            },
            ReviewState::Error { context, message } => ReviewView::Error {
                index: context.index,
                total: context.emails.len(),
                email: &context.emails[context.index],
                message,
            },
            ReviewState::Complete {
                processed,
                skipped,
                saved_files,
            } => ReviewView::Complete {
                processed: *processed,
                skipped: *skipped,
                saved_files,
            },
            ReviewState::Idle => ReviewView::Complete {
                processed: 0,
                skipped: 0,
                saved_files: &[],
            },
        }
    }
}

fn load_invoice_review(email: CandidateEmail) -> Result<InvoiceReview, ReviewError> {
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

fn save_invoice_review(review: InvoiceReview) -> Result<SavedInvoiceFiles, ReviewError> {
    let pdf_attachment = review
        .email
        .pdf_attachments
        .first()
        .ok_or(ReviewError::MissingPdf)?;
    let service = GmailSearchService::default();
    let pdf_file = service
        .download_attachment(&review.email.id, pdf_attachment)
        .map_err(ReviewError::Gmail)?;

    let mut saved_files = save_invoice_files(&review.invoice, &review.json_file, &pdf_file)
        .map_err(ReviewError::Io)?;
    let upload = DriveUploadService::default()
        .upload_invoice_files(&review.invoice, &saved_files)
        .map_err(ReviewError::Drive)?;
    saved_files.drive_upload = Some(upload);

    Ok(saved_files)
}

fn review_error_message(error: ReviewError) -> String {
    match error {
        ReviewError::Gmail(error) => messages().gmail_search_error(&error),
        ReviewError::Drive(error) => messages().drive_upload_error(&error),
        ReviewError::Invoice(error) => format!("No se pudo leer el JSON de la factura: {error}"),
        ReviewError::Io(error) => format!("No se pudieron guardar los archivos: {error}"),
        ReviewError::MissingJson => "Este correo no tiene un adjunto JSON descargable.".to_string(),
        ReviewError::MissingPdf => "Este correo no tiene un adjunto PDF descargable.".to_string(),
    }
}
