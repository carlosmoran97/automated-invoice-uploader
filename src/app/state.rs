use super::review::{InvoiceReview, ReviewError};
use crate::services::{
    gmail_search::{CandidateEmail, GmailSearchError},
    invoice_files::SavedInvoiceFiles,
};
use std::sync::mpsc::Receiver;

pub(super) type SearchOutcome = Result<Vec<CandidateEmail>, GmailSearchError>;

pub(super) enum Screen {
    Home,
    Results,
    Review,
    Settings,
}

pub(super) enum SearchState {
    Idle,
    Running {
        receiver: Receiver<SearchOutcome>,
        initial_date: String,
        final_date: String,
    },
    Failed(String),
}

#[derive(Clone)]
pub(super) struct ReviewContext {
    pub(super) emails: Vec<CandidateEmail>,
    pub(super) index: usize,
    pub(super) processed: usize,
    pub(super) skipped: usize,
    pub(super) saved_files: Vec<SavedInvoiceFiles>,
}

pub(super) enum ReviewState {
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
