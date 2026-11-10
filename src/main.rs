mod app;
mod components;
mod domain;
mod i18n;
mod services;

use app::{App, AppAction};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;
use services::{authentication::AuthenticationService, settings::load_settings};
use std::{process::ExitCode, time::Duration};

fn main() -> ExitCode {
    let settings = load_settings().unwrap_or_default();
    let text = i18n::messages_for(settings.language);
    if let Err(error) = AuthenticationService::default()
        .ensure_authenticated(|event| eprintln!("{}", text.authentication_event(event)))
    {
        eprintln!("{}", text.authentication_error(&error));
        return ExitCode::FAILURE;
    }

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, App::new(settings));
    ratatui::restore();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(terminal: &mut DefaultTerminal, mut app: App) -> std::io::Result<()> {
    loop {
        app.tick();
        terminal.draw(|frame| app.render(frame))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.handle_key(key) {
                        AppAction::Continue => {}
                        AppAction::Quit => break,
                    }
                }
            }
        }
    }

    Ok(())
}
