mod review;
mod settings;
mod state;

use self::{
    review::{load_invoice_review, review_error_message, save_invoice_review},
    settings::save_settings_input as persist_settings_input,
    state::{ReviewContext, ReviewState, Screen, SearchState},
};
use crate::{
    components::{
        home::{HomePage, HomePageAction, HomePageStatus, SearchCriteriaInput},
        results::{ResultsPage, ResultsPageAction},
        review::{ReviewPage, ReviewView},
        settings::{SettingsInput, SettingsPage, SettingsPageAction},
    },
    domain::date_range::DateRange,
    i18n::{Messages, messages_for},
    services::{
        gmail_search::{CandidateEmail, GmailSearchService},
        settings::{AppSettings, load_settings},
    },
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::{
    sync::mpsc::{self, TryRecvError},
    thread,
};

pub struct App {
    screen: Screen,
    home_page: HomePage,
    results_page: ResultsPage,
    review_page: ReviewPage,
    settings_page: SettingsPage,
    search: SearchState,
    review: ReviewState,
    results: Vec<CandidateEmail>,
    search_frame: usize,
    settings: AppSettings,
}

pub enum AppAction {
    Continue,
    Quit,
}

impl Default for App {
    fn default() -> Self {
        let settings = load_settings().unwrap_or_default();
        let home_page =
            HomePage::default().with_dte_query_filter(settings.dte_query_filter.clone());
        Self {
            screen: Screen::Home,
            home_page,
            results_page: ResultsPage::default(),
            review_page: ReviewPage,
            settings_page: SettingsPage::default(),
            search: SearchState::Idle,
            review: ReviewState::Idle,
            results: Vec::new(),
            search_frame: 0,
            settings,
        }
    }
}

impl App {
    fn text(&self) -> &'static Messages {
        messages_for(self.settings.language)
    }

    pub fn render(&self, frame: &mut Frame) {
        let text = self.text();
        match self.screen {
            Screen::Home => self.home_page.render(frame, self.home_status(), text),
            Screen::Results => self.results_page.render(frame, &self.results, text),
            Screen::Review => self.review_page.render(frame, self.review_view(), text),
            Screen::Settings => self.settings_page.render(frame, text),
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
            Screen::Settings => self.handle_settings_key(key),
        }
    }

    pub fn tick(&mut self) {
        self.tick_review();
        let text = self.text();

        let SearchState::Running { receiver, .. } = &self.search else {
            return;
        };
        self.search_frame = self.search_frame.wrapping_add(1);

        match receiver.try_recv() {
            Ok(Ok(results)) => {
                self.results = results;
                self.results_page.reset(&self.results);
                self.search = SearchState::Idle;
                self.screen = Screen::Results;
            }
            Ok(Err(error)) => {
                self.search = SearchState::Failed(text.gmail_search_error(&error));
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.search = SearchState::Failed(text.gmail_search_stopped.to_string());
            }
        }
    }

    fn handle_home_key(&mut self, key: KeyEvent) -> AppAction {
        if matches!(self.search, SearchState::Failed(_)) {
            self.search = SearchState::Idle;
        }

        match self.home_page.handle_key(key) {
            HomePageAction::Continue => AppAction::Continue,
            HomePageAction::OpenSettings => {
                self.settings_page.reset(&self.settings);
                self.screen = Screen::Settings;
                AppAction::Continue
            }
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

    fn handle_settings_key(&mut self, key: KeyEvent) -> AppAction {
        match self.settings_page.handle_key(key) {
            SettingsPageAction::Continue => AppAction::Continue,
            SettingsPageAction::Back => {
                self.screen = Screen::Home;
                AppAction::Continue
            }
            SettingsPageAction::Quit => AppAction::Quit,
            SettingsPageAction::Save(input) => {
                self.save_settings_input(input);
                AppAction::Continue
            }
        }
    }

    fn save_settings_input(&mut self, input: SettingsInput) {
        let text = messages_for(input.language);
        let settings = match persist_settings_input(input, text) {
            Ok(settings) => settings,
            Err(message) => {
                self.settings_page.mark_error(message);
                return;
            }
        };

        self.settings = settings;
        self.home_page
            .set_dte_query_filter(self.settings.dte_query_filter.clone());
        self.settings_page.reset(&self.settings);
        self.settings_page.mark_saved();
    }

    fn start_search(&mut self, input: SearchCriteriaInput) {
        let range = match DateRange::parse(&input.initial_date, &input.final_date) {
            Ok(range) => range,
            Err(error) => {
                self.search = SearchState::Failed(self.text().date_range_error(&error).to_string());
                return;
            }
        };
        let (sender, receiver) = mpsc::channel();
        let initial_date = range.initial_date();
        let final_date = range.final_date();
        let dte_query_filter = input.dte_query_filter.trim().to_string();
        self.search_frame = 0;

        thread::spawn(move || {
            let service = GmailSearchService::default().with_dte_query_filter(dte_query_filter);
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
                frame: self.search_frame,
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
        let settings = self.settings.clone();

        thread::spawn(move || {
            let _ = sender.send(save_invoice_review(review_for_thread, settings));
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
        let text = self.text();
        let next_state = match &self.review {
            ReviewState::Loading { context, receiver } => match receiver.try_recv() {
                Ok(Ok(review)) => Some(ReviewState::Ready {
                    context: context.clone(),
                    review,
                }),
                Ok(Err(error)) => Some(ReviewState::Error {
                    context: context.clone(),
                    message: review_error_message(error, text),
                }),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(ReviewState::Error {
                    context: context.clone(),
                    message: text.gmail_search_stopped.to_string(),
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
                    message: review_error_message(error, text),
                }),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(ReviewState::Error {
                    context: context.clone(),
                    message: text.gmail_search_stopped.to_string(),
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
