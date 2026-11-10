mod app;
mod components;
mod domain;
mod i18n;
mod services;

use app::{App as InvoiceApp, CloseWindow, Quit, TabNext, TabPrevious};
use gpui::{
    App, AppContext, Application, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions,
    px, size,
};
use services::settings::load_settings;

fn main() {
    Application::new().run(|cx: &mut App| {
        components::text_input::bind_keys(cx);
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-w", CloseWindow, None),
            KeyBinding::new("tab", TabNext, None),
            KeyBinding::new("shift-tab", TabPrevious, None),
        ]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &CloseWindow, cx| {
            if let Some(window) = cx
                .active_window()
                .or_else(|| cx.windows().into_iter().next())
            {
                let _ = window.update(cx, |_, window, _| window.remove_window());
            }
        });
        cx.on_action(|_: &TabNext, cx| {
            if let Some(window) = cx
                .active_window()
                .or_else(|| cx.windows().into_iter().next())
            {
                let _ = window.update(cx, |_, window, _| window.focus_next());
            }
        });
        cx.on_action(|_: &TabPrevious, cx| {
            if let Some(window) = cx
                .active_window()
                .or_else(|| cx.windows().into_iter().next())
            {
                let _ = window.update(cx, |_, window, _| window.focus_prev());
            }
        });
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let settings = load_settings().unwrap_or_default();
        let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(960.), px(640.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("DTE Uploader".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| InvoiceApp::new(settings, window, cx)),
        )
        .expect("could not open the application window");
        cx.activate(true);
    });
}
