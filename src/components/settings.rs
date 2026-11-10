use super::widgets::*;
use crate::{
    app::{App, UiAction, state::Screen},
    i18n::Language,
};
use gpui::{Context, Div, div, prelude::*, px, rgb};

impl App {
    pub(crate) fn render_settings(&self, cx: &mut Context<Self>) -> Div {
        let text = self.text();
        let enabled = !self.state.busy();
        div()
            .w_full()
            .max_w(px(900.))
            .mx_auto()
            .flex()
            .flex_col()
            .gap_6()
            .child(heading(text.settings_title, text.settings_header))
            .child(
                card()
                    .gap_5()
                    .child(field(text.download_dir_label, self.download_dir.clone()))
                    .child(field(
                        text.drive_root_folder_label,
                        self.drive_folder.clone(),
                    ))
                    .child(field(
                        text.dte_query_filter_label,
                        self.settings_filter.clone(),
                    ))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(MUTED))
                            .child(text.dte_query_filter_hint),
                    )
                    .child(field(
                        text.language_label,
                        div()
                            .flex()
                            .gap_3()
                            .child(button(
                                "spanish",
                                text.language_spanish,
                                self.settings_language == Language::Spanish,
                                enabled,
                                UiAction::Language(Language::Spanish),
                                cx,
                            ))
                            .child(button(
                                "english",
                                text.language_english,
                                self.settings_language == Language::English,
                                enabled,
                                UiAction::Language(Language::English),
                                cx,
                            )),
                    ))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(button(
                                "settings-back",
                                text.back_button,
                                false,
                                enabled,
                                UiAction::Navigate(Screen::Home),
                                cx,
                            ))
                            .child(button(
                                "settings-save",
                                text.save_button,
                                true,
                                enabled,
                                UiAction::SaveSettings,
                                cx,
                            )),
                    ),
            )
            .when_some(self.state.error.clone(), |s, error| {
                s.child(notice(error, true))
            })
            .when_some(self.state.settings_status.clone(), |s, status| {
                s.child(notice(status, false))
            })
    }
}
