# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 terminal app for finding invoice emails, reviewing DTE JSON, downloading invoice files, and uploading accepted files to Google Drive with `gws`.

- `src/main.rs` starts the TUI runtime.
- `src/app.rs` owns app state, screen transitions, and workers.
- `src/components/` contains Ratatui UI only: `home`, `results`, and `review`.
- `src/services/` contains external and filesystem logic: Gmail search/download, Drive upload, and local invoice file saving.
- `src/domain/` contains parsing and validation, including date ranges and invoice summaries.
- `src/i18n.rs` contains Spanish-default and English UI strings.
- `docs/svfe-json-schemas/` stores reference DTE schemas. Do not move these unless the parser contract changes.

## Build, Test, and Development Commands

- `cargo run` starts the local TUI.
- `cargo fmt` formats Rust code with `rustfmt`.
- `cargo check` type-checks quickly without producing a final binary.
- `cargo test` runs all unit tests.

Before testing Gmail or Drive flows manually, make sure the Google Workspace CLI (`gws`) is installed, authenticated, and available in `PATH`.

## Coding Style & Naming Conventions

Use standard Rust formatting via `cargo fmt`; keep files ASCII unless existing content requires otherwise. Preserve module boundaries: UI components render and handle local input, while service modules perform Gmail, Drive, and file operations. Use `snake_case` for functions, modules, and variables; `PascalCase` for structs/enums; and descriptive enum variants such as `ReviewState::Saving`.

Keep comments sparse and useful. Add comments only for non-obvious parsing, command construction, or workflow behavior.

## Testing Guidelines

Tests use Rust’s built-in test framework and usually live beside the code under `#[cfg(test)]`. Name tests after behavior, for example `maps_issue_date_to_drive_period` or `rejects_invalid_dates`.

Run `cargo test` before handing off changes. Add focused unit tests when changing parsing, date calculations, Gmail/Drive command construction, selection behavior, or file naming.

## Commit & Pull Request Guidelines

Recent commits are short and action-oriented, for example `review invoices and download files` and `upload to google drive`. Keep commit messages concise, specific, and in the imperative or verb-first style.

Pull requests should describe the user-facing workflow change, list verification commands run, and note any `gws` behavior that needs manual validation. Include terminal screenshots only when UI layout changes materially.

## Security & Configuration Tips

Never commit downloaded invoice files or private credentials. `/downloaded_invoices/` is ignored and should remain local. Treat Gmail message content, DTE JSON, PDFs, Drive folder IDs, and accountant folder paths as sensitive operational data.
