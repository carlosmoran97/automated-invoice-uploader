# DTE Uploader

A Rust 2024 macOS desktop application built with GPUI for finding invoice emails
in Gmail, reviewing Salvadoran DTE documents, and uploading accepted PDF and JSON
files to the accountant's Google Drive folder through `gws`.

## Run

Install Rust, Xcode with its command line tools, and the
[Google Workspace CLI](https://github.com/googleworkspace/cli). Configure the
OAuth client for `gws`, then start the desktop window:

```bash
cargo run
```

GPUI is pinned to 0.2.2. The build enables its `font-kit` and `runtime_shaders`
features: Metal shaders compile when the window opens, so Xcode's separate Metal
Toolchain download is not required. This version targets macOS and starts from
Cargo; a Finder `.app` bundle is not included.

The light theme uses blue, white, and gray inspired by the official
[El Salvador electronic invoicing portal](https://admin.factura.gob.sv/login).

## Workflow

1. The window opens immediately and checks Google access in the background.
2. If authorization is needed, complete the Google login in your browser. Errors
   appear in the window with a Retry button.
3. Choose **From** and **To**. They start at the first and last day of the previous
   month, using the Mac's local date (UTC if unavailable). For example, opening on
   November 10, 2026 selects October 1–31, 2026.
4. Edit the optional Gmail DTE filter, initially `DTE-03`, and click Search.
5. Browse matching emails and their details. Every result starts selected;
   toggle individual checkboxes or select/deselect all, then Review selected.
6. Review each invoice's identity, issuer, receiver, totals, taxes, and line items.
   **Accept and upload** downloads the PDF, saves both files locally, and uploads
   them to Drive. **Skip** advances without processing the invoice.
7. The completion screen lists processed/skipped counts, saved paths, and Drive
   destinations.

The decision to process an invoice remains manual. A warning identifies a JSON
that is not CCF (`tipoDte` 03), without deciding on the user's behalf. The existing
DTE parser and its summary of up to five line items remain unchanged.

Search is limited to `INBOX`: the app runs separate PDF and JSON searches,
intersects message IDs, and checks that each email really has both attachment
types. The selected final date is inclusive. Search, download, and upload work
runs outside the UI thread. Actions that could duplicate or interrupt the active
workflow are disabled while it is working.

## Controls and preferences

Use the mouse or Tab / Shift-Tab to move between controls, and Enter / Space to
activate buttons. Date buttons open independent calendars at their selected
month. Inside a calendar, arrow keys move by day/week, Page Up / Page Down change
months, Home / End move to the month's first/last day, Enter / Space select, and
Escape closes without changing the date. Clicking outside also closes it.
Returning from results or settings preserves edited dates.

Text fields support Unicode, cursor movement, selection with Shift-arrows,
Command-A, Command-C/X/V, and macOS text composition. Enter submits the search or
settings form. In the results list, arrows move the highlighted email, Space
toggles it, and Enter starts review. Command-W closes the window and Command-Q
quits.

Settings contains the download folder, Drive base folder, default Gmail filter,
and Spanish/English language. Spanish is the default. Existing settings and their
location remain compatible; there is no configuration migration. Empty download
and Drive folders are rejected in the window.

Downloaded invoices remain local in the configured folder (initially
`downloaded_invoices/`, ignored by Git). Drive organization and file naming are
unchanged: the issue date determines the accounting year/month destination.

## Google authentication

The app checks `gws auth status`, then makes small read-only Gmail and Drive
requests. Each check has a 30-second timeout. `gws` refreshes saved credentials
when possible. If login is required, the app runs this once per attempt:

```bash
gws auth login --scopes https://www.googleapis.com/auth/gmail.readonly,https://www.googleapis.com/auth/drive
```

Explicit scopes skip the terminal scope picker. Login opens the browser and does
not require terminal input. After login, both services are verified again. A
cancelled login or failed verification shows an error; Retry starts a new attempt.
Connection, API, policy, and OAuth configuration failures do not trigger login.
Credentials and encryption remain managed by `gws`; the application does not
export or store credentials.

If authorization expires after seven days, check your OAuth project's publishing
status in [Google Auth Platform > Audience](https://console.cloud.google.com/auth/audience).
External apps in **Testing** have a seven-day refresh token lifetime for these
scopes. See [Google's documentation](https://developers.google.com/identity/protocols/oauth2#expiration).
If appropriate for your project, publish it to **In production**, then authorize
again. The app does not change your Google Cloud configuration.

If login is required every time, inspect `gws auth status` privately from the same
terminal used to run the app. Verify access to the macOS keychain and consistent
`gws` configuration/keyring settings. A decryption error from a restricted
environment does not by itself prove that saved credentials are broken.

## Development

```bash
cargo fmt --check
cargo check
cargo test
```

Unit tests cover dates (including leap years and year boundaries), workflow and
selection, settings validation, parsing, command construction, and filesystem
behavior. Service tests use fake command runners. For desktop validation, use
fictitious messages and attachments; screenshots must not expose real invoices,
credentials, or Drive folder identifiers. Real Gmail/Drive validation requires
an authenticated `gws` with access to the target folder.
