# Automated Invoice Uploader

This project is a Rust terminal application for finding invoice emails in Gmail,
reviewing them manually, and uploading the selected invoice files to a shared
Google Drive folder.

The intended workflow uses the Google Workspace CLI installed in the local
environment. The app will call that CLI to search Gmail, find emails with both
PDF and JSON attachments, and help decide which invoices should be uploaded for
accounting purposes.

## Purpose

As a contribuyente in El Salvador, not every invoice email should be uploaded to
the accountant's shared Google Drive folder.

The important manual distinction is:

- Upload: Credito fiscal invoices.
- Skip: Consumidor final invoices.

The app is meant to reduce the repetitive work around finding, downloading, and
organizing invoice attachments, while still keeping the invoice-type validation
manual for now.

## Planned Workflow

1. Ask for a period of time to search.
2. Use the Google Workspace CLI to search Gmail for matching emails.
3. Filter emails that contain both a PDF attachment and a JSON attachment.
4. Show a summary of the matching emails.
5. Open a TUI review interface.
6. Iterate through each email one by one.
7. Download the JSON attachment for manual inspection.
8. Decide whether the email corresponds to a Credito fiscal invoice.
9. Upload the selected PDF and JSON files to the configured Google Drive folder.
10. Skip emails that are not valid for upload.

## Current State

The project currently contains a Ratatui home page with dropdown-style calendar
pickers for selecting the initial and final dates. After selection, the app
validates the range and searches Gmail through the Google Workspace CLI.

The current search flow:

- Searches only `INBOX`.
- Runs separate Gmail searches for `filename:pdf` and `filename:json`.
- Intersects the message IDs from both searches.
- Fetches full Gmail message payloads for the intersected messages.
- Keeps only messages that really contain both PDF and JSON attachments.
- Shows a summary list with sender, date, subject, and attachment names.

Run it with:

```bash
cargo run
```

Use `Tab` to switch between the initial and final date fields. Press `Enter` or
`Space` to open the calendar for the focused field. Inside the calendar, use the
arrow keys to move by day or week, `PgUp` and `PgDn` to move by month, `Home`
and `End` to select the first or last day of the visible month, and `Enter` or
`Esc` to close the picker. Press `s` from the form to search and `q` to quit. On
the results screen, use `Up` and `Down` to select an email and `b` or `Esc` to
go back to the search form.

## Dependencies

The Rust app currently uses:

- `ratatui` for the terminal user interface.
- `crossterm` for terminal input and control.
- `serde` and `serde_json` for parsing Google Workspace CLI JSON output.
- `time` for calendar date handling.

The runtime environment is expected to have:

- Rust and Cargo.
- The Google Workspace CLI configured with access to Gmail and Google Drive.
- Access to the target Google Drive folder shared with the accountant.

## Notes

The app should not automatically decide whether an invoice is a Credito fiscal
or Consumidor final yet. That decision remains part of the manual TUI review
flow.
