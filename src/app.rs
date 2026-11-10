mod review;
mod settings;
pub(crate) mod state;

use self::{
    review::{load_invoice_review, review_error_message, save_invoice_review},
    state::*,
};
use crate::{
    components::{
        calendar::{Calendar, CalendarEvent},
        text_input::{Submitted, TextInput},
        widgets::*,
    },
    i18n::{Language, Messages, messages_for},
    services::{
        authentication::{AuthenticationEvent, AuthenticationService},
        gmail_search::{CandidateEmail, GmailSearchService},
        settings::AppSettings,
    },
};
use gpui::{
    Context, Div, Entity, FocusHandle, FontWeight, IntoElement, KeyDownEvent, ScrollStrategy,
    UniformListScrollHandle, Window, actions, div, prelude::*, px, rgb,
};
use time::OffsetDateTime;

#[derive(Clone)]
pub(crate) enum UiAction {
    Navigate(Screen),
    Search,
    ReviewSelected,
    Highlight(usize),
    ToggleEmail(usize),
    ToggleAll,
    Accept,
    Skip,
    SaveSettings,
    Language(Language),
    OpenCalendar(bool),
    RetryAuthentication,
}

actions!(invoice, [Quit, CloseWindow, TabNext, TabPrevious]);

pub struct App {
    pub(crate) state: AppState,
    pub(crate) search_filter: Entity<TextInput>,
    pub(crate) download_dir: Entity<TextInput>,
    pub(crate) drive_folder: Entity<TextInput>,
    pub(crate) settings_filter: Entity<TextInput>,
    pub(crate) settings_language: Language,
    pub(crate) initial_calendar: Entity<Calendar>,
    pub(crate) final_calendar: Entity<Calendar>,
    pub(crate) calendar_open: Option<bool>,
    focus: FocusHandle,
    pub(crate) results_focus: FocusHandle,
    pub(crate) results_scroll: UniformListScrollHandle,
}

impl App {
    pub fn new(settings: AppSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let today = OffsetDateTime::now_local()
            .unwrap_or_else(|_| OffsetDateTime::now_utc())
            .date();
        let state = AppState::new(settings.clone(), today);
        let search_filter = cx.new(|cx| TextInput::new(settings.dte_query_filter.clone(), cx));
        let download_dir =
            cx.new(|cx| TextInput::new(settings.download_dir.display().to_string(), cx));
        let drive_folder = cx.new(|cx| TextInput::new(settings.drive_root_folder.clone(), cx));
        let settings_filter = cx.new(|cx| TextInput::new(settings.dte_query_filter.clone(), cx));
        let initial_calendar =
            cx.new(|cx| Calendar::new(state.criteria.initial_date, settings.language, cx));
        let final_calendar =
            cx.new(|cx| Calendar::new(state.criteria.final_date, settings.language, cx));
        cx.subscribe_in(
            &search_filter,
            window,
            |this, _, _: &Submitted, window, cx| this.dispatch(UiAction::Search, window, cx),
        )
        .detach();
        for input in [&download_dir, &drive_folder, &settings_filter] {
            cx.subscribe_in(input, window, |this, _, _: &Submitted, window, cx| {
                this.dispatch(UiAction::SaveSettings, window, cx)
            })
            .detach();
        }
        for (initial, calendar) in [(true, &initial_calendar), (false, &final_calendar)] {
            cx.subscribe_in(
                calendar,
                window,
                move |this, _, event: &CalendarEvent, window, cx| {
                    if let CalendarEvent::Selected(date) = event {
                        if initial {
                            this.state.criteria.initial_date = *date;
                        } else {
                            this.state.criteria.final_date = *date;
                        }
                        this.state.error = None;
                    }
                    this.calendar_open = None;
                    this.focus.focus(window);
                    cx.notify();
                },
            )
            .detach();
        }
        let focus = cx.focus_handle();
        focus.focus(window);
        let app = Self {
            state,
            search_filter,
            download_dir,
            drive_folder,
            settings_filter,
            settings_language: settings.language,
            initial_calendar,
            final_calendar,
            calendar_open: None,
            focus,
            results_focus: cx.focus_handle().tab_stop(true),
            results_scroll: UniformListScrollHandle::default(),
        };
        app.authenticate(cx);
        app
    }

    pub(crate) fn text(&self) -> &'static Messages {
        messages_for(self.state.settings.language)
    }

    fn authenticate(&self, cx: &mut Context<Self>) {
        enum Update {
            Progress(AuthenticationEvent),
            Finished(Result<(), crate::services::authentication::AuthenticationError>),
        }
        let (sender, receiver) = async_channel::unbounded();
        let worker = cx.background_executor().spawn(async move {
            let outcome = AuthenticationService::default().ensure_authenticated(|event| {
                let _ = sender.try_send(Update::Progress(event));
            });
            let _ = sender.try_send(Update::Finished(outcome));
        });
        cx.spawn(async move |this, cx| {
            while let Ok(update) = receiver.recv().await {
                let done = matches!(update, Update::Finished(_));
                if this
                    .update(cx, |this, cx| {
                        match update {
                            Update::Progress(event) => {
                                this.state.authentication = AuthenticationState::Checking(event)
                            }
                            Update::Finished(Ok(())) => {
                                this.state.authentication = AuthenticationState::Ready;
                                this.state.screen = Screen::Home;
                            }
                            Update::Finished(Err(error)) => {
                                this.state.authentication = AuthenticationState::Failed(
                                    this.text().authentication_error(&error),
                                )
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
                if done {
                    break;
                }
            }
            worker.await;
        })
        .detach();
    }

    pub(crate) fn dispatch(
        &mut self,
        action: UiAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            UiAction::Navigate(screen) => {
                if self.state.navigate(screen) {
                    self.calendar_open = None;
                    if screen == Screen::Settings {
                        let input = SettingsInput::from(&self.state.settings);
                        self.download_dir
                            .update(cx, |view, cx| view.set_value(input.download_dir, cx));
                        self.drive_folder
                            .update(cx, |view, cx| view.set_value(input.drive_root_folder, cx));
                        self.settings_filter
                            .update(cx, |view, cx| view.set_value(input.dte_query_filter, cx));
                        self.settings_language = input.language;
                        self.state.settings_status = None;
                    }
                    self.focus.focus(window);
                }
            }
            UiAction::Search => {
                self.state.criteria.dte_query_filter = self.search_filter.read(cx).value();
                if let Some((range, filter)) = self.state.begin_search() {
                    self.calendar_open = None;
                    let worker = cx.background_executor().spawn(async move {
                        GmailSearchService::default()
                            .with_dte_query_filter(filter)
                            .search_invoice_candidates(&range)
                    });
                    cx.spawn(async move |this, cx| {
                        let result = worker.await;
                        let _ = this.update(cx, |this, cx| {
                            match result {
                                Ok(results) => {
                                    this.state.finish_search(results);
                                    this.results_scroll.scroll_to_item(0, ScrollStrategy::Top);
                                }
                                Err(error) => {
                                    this.state.searching = false;
                                    this.state.error = Some(this.text().gmail_search_error(&error));
                                }
                            }
                            cx.notify();
                        });
                    })
                    .detach();
                }
            }
            UiAction::ReviewSelected => {
                if let Some(email) = self.state.begin_review() {
                    self.load_review(email, cx);
                }
            }
            UiAction::Highlight(index) => {
                if !self.state.busy() && index < self.state.results.len() {
                    self.state.highlighted = index;
                    self.results_focus.focus(window);
                }
            }
            UiAction::ToggleEmail(index) => self.state.toggle_email(index),
            UiAction::ToggleAll => {
                if !self.state.busy() && self.state.screen == Screen::Results {
                    if self.state.selected_ids.len() == self.state.results.len() {
                        self.state.selected_ids.clear();
                    } else {
                        self.state.selected_ids = self
                            .state
                            .results
                            .iter()
                            .map(|email| email.id.clone())
                            .collect();
                    }
                }
            }
            UiAction::Accept => {
                if let Some(review) = self.state.begin_save() {
                    let settings = self.state.settings.clone();
                    let worker = cx
                        .background_executor()
                        .spawn(async move { save_invoice_review(review, settings) });
                    cx.spawn(async move |this, cx| {
                        let result = worker.await;
                        let _ = this.update(cx, |this, cx| {
                            match result {
                                Ok(files) => {
                                    if let Some(email) = this.state.finish_save(files) {
                                        this.load_review(email, cx);
                                    }
                                }
                                Err(error) => {
                                    this.state.review =
                                        ReviewState::Error(review_error_message(error, this.text()))
                                }
                            }
                            cx.notify();
                        });
                    })
                    .detach();
                }
            }
            UiAction::Skip => {
                if let Some(email) = self.state.skip_review() {
                    self.load_review(email, cx);
                }
            }
            UiAction::SaveSettings => {
                if self.state.screen == Screen::Settings && !self.state.busy() {
                    let input = SettingsInput {
                        download_dir: self.download_dir.read(cx).value(),
                        drive_root_folder: self.drive_folder.read(cx).value(),
                        dte_query_filter: self.settings_filter.read(cx).value(),
                        language: self.settings_language,
                    };
                    self.state.saving_settings = true;
                    self.state.error = None;
                    self.state.settings_status = None;
                    let worker = cx.background_executor().spawn(async move {
                        settings::save_settings_input(input.clone(), messages_for(input.language))
                    });
                    cx.spawn(async move |this, cx| {
                        let outcome = worker.await;
                        let _ = this.update(cx, |this, cx| {
                            this.state.saving_settings = false;
                            match outcome {
                                Ok(settings) => {
                                    this.search_filter.update(cx, |view, cx| {
                                        view.set_value(settings.dte_query_filter.clone(), cx)
                                    });
                                    this.state.criteria.dte_query_filter =
                                        settings.dte_query_filter.clone();
                                    this.state.settings = settings;
                                    this.state.settings_status =
                                        Some(this.text().settings_saved.into());
                                }
                                Err(error) => this.state.error = Some(error),
                            }
                            cx.notify();
                        });
                    })
                    .detach();
                }
            }
            UiAction::Language(language) => {
                if !self.state.busy() {
                    self.settings_language = language;
                }
            }
            UiAction::OpenCalendar(initial) => {
                if !self.state.busy() && self.state.screen == Screen::Home {
                    if self.calendar_open == Some(initial) {
                        self.calendar_open = None;
                    } else {
                        self.calendar_open = Some(initial);
                        let date = if initial {
                            self.state.criteria.initial_date
                        } else {
                            self.state.criteria.final_date
                        };
                        let calendar = if initial {
                            &self.initial_calendar
                        } else {
                            &self.final_calendar
                        };
                        let language = self.state.settings.language;
                        calendar
                            .update(cx, |calendar, cx| calendar.open(date, language, window, cx));
                    }
                }
            }
            UiAction::RetryAuthentication => {
                if matches!(self.state.authentication, AuthenticationState::Failed(_)) {
                    self.state.authentication =
                        AuthenticationState::Checking(AuthenticationEvent::Checking);
                    self.authenticate(cx);
                }
            }
        }
        cx.notify();
    }

    fn load_review(&self, email: CandidateEmail, cx: &mut Context<Self>) {
        let worker = cx
            .background_executor()
            .spawn(async move { load_invoice_review(email) });
        cx.spawn(async move |this, cx| {
            let outcome = worker.await;
            let _ = this.update(cx, |this, cx| {
                this.state.review = match outcome {
                    Ok(review) => ReviewState::Ready(Box::new(review)),
                    Err(error) => ReviewState::Error(review_error_message(error, this.text())),
                };
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn results_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.keystroke.key.as_str() {
            "space" => self.dispatch(UiAction::ToggleEmail(self.state.highlighted), window, cx),
            "enter" => self.dispatch(UiAction::ReviewSelected, window, cx),
            "up" => self.state.highlighted = self.state.highlighted.saturating_sub(1),
            "down" => {
                self.state.highlighted =
                    (self.state.highlighted + 1).min(self.state.results.len().saturating_sub(1))
            }
            _ => return,
        }
        self.results_scroll
            .scroll_to_item(self.state.highlighted, ScrollStrategy::Center);
        cx.stop_propagation();
        cx.notify();
    }

    fn render_startup(&self, cx: &mut Context<Self>) -> Div {
        let text = self.text();
        let failed = matches!(self.state.authentication, AuthenticationState::Failed(_));
        let message = match &self.state.authentication {
            AuthenticationState::Checking(event) => text.authentication_event(*event),
            AuthenticationState::Failed(message) => message.clone(),
            AuthenticationState::Ready => String::new(),
        };
        div().flex_1().flex().items_center().justify_center().child(
            card()
                .w(px(620.))
                .child(heading(text.connection_title, text.connection_hint))
                .child(notice(message, failed))
                .when(failed, |s| {
                    s.child(button(
                        "retry",
                        text.retry_button,
                        true,
                        true,
                        UiAction::RetryAuthentication,
                        cx,
                    ))
                }),
        )
    }
}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text = self.text();
        let can_navigate = !self.state.busy()
            && matches!(self.state.authentication, AuthenticationState::Ready)
            && (self.state.screen != Screen::Review
                || matches!(self.state.review, ReviewState::Complete));
        let body = match self.state.screen {
            Screen::Startup => self.render_startup(cx).into_any_element(),
            Screen::Home => self.render_home(cx).into_any_element(),
            Screen::Results => self.render_results(window, cx).into_any_element(),
            Screen::Review => self.render_review(cx).into_any_element(),
            Screen::Settings => self.render_settings(cx).into_any_element(),
        };
        div()
            .id("invoice-app")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .font_family(".SystemUIFont")
            .text_size(px(14.))
            .text_color(rgb(INK))
            .bg(rgb(BACKGROUND))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .on_action(|_: &TabNext, window, _| window.focus_next())
            .on_action(|_: &TabPrevious, window, _| window.focus_prev())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_8()
                    .py_5()
                    .bg(rgb(0xffffff))
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(div().w(px(5.)).h(px(44.)).rounded_full().bg(rgb(BLUE)))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_size(px(20.))
                                            .font_weight(FontWeight::BOLD)
                                            .child(text.app_title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(rgb(MUTED))
                                            .child(text.app_subtitle),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(button(
                                "nav-search",
                                text.search_button,
                                self.state.screen == Screen::Home,
                                can_navigate,
                                UiAction::Navigate(Screen::Home),
                                cx,
                            ))
                            .child(button(
                                "nav-settings",
                                text.settings_title,
                                self.state.screen == Screen::Settings,
                                can_navigate,
                                UiAction::Navigate(Screen::Settings),
                                cx,
                            )),
                    ),
            )
            .child(
                div()
                    .id(("page", self.state.screen as usize))
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_8()
                    .child(body),
            )
            .child(
                div()
                    .px_8()
                    .py_3()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .text_size(px(12.))
                    .text_color(rgb(MUTED))
                    .child(
                        if matches!(self.state.authentication, AuthenticationState::Ready) {
                            text.connected_label
                        } else {
                            text.connection_title
                        },
                    ),
            )
    }
}
