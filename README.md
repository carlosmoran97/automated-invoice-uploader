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

Before opening the terminal interface, the app checks `gws auth status` and makes
small read-only Gmail and Drive requests. Each check has a 30-second timeout.
`gws` renews access tokens using its saved credentials when possible. If login is
required, the app runs the following command once, with the terminal available:

```bash
gws auth login --scopes https://www.googleapis.com/auth/gmail.readonly,https://www.googleapis.com/auth/drive
```

This skips the `gws` scope picker. Complete account selection and consent in the
browser when Google requests them. Gmail access is read-only; Drive access allows
the app to find existing accounting folders, create subfolders, and upload files.
After login, the app verifies access again before opening the interface. A
cancelled login or failed verification stops startup. Connection failures,
disabled APIs, and invalid OAuth configuration stop startup without starting a
login. Startup messages use the saved language preference.

Credentials, refresh tokens, and encryption remain managed by `gws` with its
existing storage configuration. The app does not export or store credentials.

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

## Repeated Google login

If login works for a few days and then requires authorization again, check the
OAuth project's publishing status in
[Google Auth Platform > Audience](https://console.cloud.google.com/auth/audience).
Select the project associated with your `gws` OAuth client. For external apps in
**Testing**, Google issues refresh tokens that expire after seven days when
requesting Gmail and Drive scopes. See
[Google's refresh token expiration documentation](https://developers.google.com/identity/protocols/oauth2#expiration).

If Testing is the cause, use **Publish app** to change the publishing status to
**In production**, following any requirements shown in the console. Then run the
login command above once to obtain a new authorization. This removes the
Testing-specific seven-day lifetime; revoked permissions and other Google
policies can still require login. The app never changes the Cloud project itself.

If login is required on every execution, run `gws auth status` from the same
terminal used to start the app. An `encryption_valid: false` result indicates that
`gws` could not decrypt its saved credentials. Check access to the macOS keychain
and confirm that login and subsequent runs use the same `gws` configuration and
keyring backend. A decryption error observed from a restricted environment needs
confirmation in your normal terminal; it does not by itself prove that the saved
credentials are broken. Keep the full status output private.

To validate persistence manually, start the app, complete authorization if
requested, quit, and immediately start it again. The second launch should verify
access and open the interface without another browser login.

## Notes

The app should not automatically decide whether an invoice is a Credito fiscal
or Consumidor final yet. That decision remains part of the manual TUI review
flow.
