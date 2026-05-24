use crate::{
    components::{
        home::{HomePage, HomePageAction, HomePageStatus, SearchCriteriaInput},
        results::{ResultsPage, ResultsPageAction},
    },
    domain::date_range::DateRange,
    services::gmail_search::{CandidateEmail, GmailSearchService},
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

type SearchOutcome = Result<Vec<CandidateEmail>, String>;

pub struct App {
    screen: Screen,
    home_page: HomePage,
    results_page: ResultsPage,
    search: SearchState,
    results: Vec<CandidateEmail>,
}

pub enum AppAction {
    Continue,
    Quit,
}

enum Screen {
    Home,
    Results,
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

impl Default for App {
    fn default() -> Self {
        Self {
            screen: Screen::Home,
            home_page: HomePage::default(),
            results_page: ResultsPage::default(),
            search: SearchState::Idle,
            results: Vec::new(),
        }
    }
}

impl App {
    pub fn render(&self, frame: &mut Frame) {
        match self.screen {
            Screen::Home => self.home_page.render(frame, self.home_status()),
            Screen::Results => self.results_page.render(frame, &self.results),
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
        }
    }

    pub fn tick(&mut self) {
        let SearchState::Running { receiver, .. } = &self.search else {
            return;
        };

        match receiver.try_recv() {
            Ok(Ok(results)) => {
                self.results = results;
                self.results_page.reset();
                self.search = SearchState::Idle;
                self.screen = Screen::Results;
            }
            Ok(Err(error)) => {
                self.search = SearchState::Failed(error);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.search = SearchState::Failed("Gmail search stopped unexpectedly.".to_string());
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
        match self.results_page.handle_key(key, self.results.len()) {
            ResultsPageAction::Continue => AppAction::Continue,
            ResultsPageAction::Back => {
                self.screen = Screen::Home;
                AppAction::Continue
            }
            ResultsPageAction::Quit => AppAction::Quit,
        }
    }

    fn start_search(&mut self, input: SearchCriteriaInput) {
        let range = match DateRange::parse(&input.initial_date, &input.final_date) {
            Ok(range) => range,
            Err(error) => {
                self.search = SearchState::Failed(error.to_string());
                return;
            }
        };
        let (sender, receiver) = mpsc::channel();
        let initial_date = range.initial_date();
        let final_date = range.final_date();

        thread::spawn(move || {
            let service = GmailSearchService::default();
            let result = service
                .search_invoice_candidates(&range)
                .map_err(|error| error.to_string());
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
}
