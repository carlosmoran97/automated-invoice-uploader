use crate::{
    domain::date_range::DateRangeError,
    services::{
        authentication::{
            AuthenticationCheck, AuthenticationError, AuthenticationEvent, LoginReason,
        },
        drive_upload::DriveUploadError,
        gmail_search::GmailSearchError,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum Language {
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "en")]
    English,
}

pub const DEFAULT_LANGUAGE: Language = Language::Spanish;

impl Default for Language {
    fn default() -> Self {
        DEFAULT_LANGUAGE
    }
}

pub fn messages_for(language: Language) -> &'static Messages {
    match language {
        Language::Spanish => &SPANISH,
        Language::English => &ENGLISH,
    }
}

pub struct Messages {
    pub search_button: &'static str,
    pub back_button: &'static str,
    pub review_selected_button: &'static str,
    pub accept_upload_button: &'static str,
    pub skip_button: &'static str,
    pub save_button: &'static str,
    pub retry_button: &'static str,
    pub connected_label: &'static str,
    pub connection_title: &'static str,
    pub connection_hint: &'static str,
    pub app_subtitle: &'static str,
    pub period_label: &'static str,
    pub select_all_button: &'static str,
    pub deselect_all_button: &'static str,
    pub email_date_label: &'static str,
    pub quantity_label: &'static str,
    pub discount_label: &'static str,
    pub income_retention_label: &'static str,
    pub vat_retention_label: &'static str,
    pub vat_perception_label: &'static str,
    pub review_manual_hint: &'static str,
    pub months: [&'static str; 12],
    pub weekdays: [&'static str; 7],
    language: Language,
    pub app_title: &'static str,
    pub search_criteria_title: &'static str,
    pub search_criteria_subtitle: &'static str,
    pub initial_date: &'static str,
    pub final_date: &'static str,
    pub default_period: &'static str,
    pub settings_title: &'static str,
    pub settings_header: &'static str,
    pub download_dir_label: &'static str,
    pub drive_root_folder_label: &'static str,
    pub dte_query_filter_label: &'static str,
    pub dte_query_filter_hint: &'static str,
    pub language_label: &'static str,
    pub language_spanish: &'static str,
    pub language_english: &'static str,
    pub settings_saved: &'static str,
    pub settings_empty_download_dir: &'static str,
    pub settings_empty_drive_root_folder: &'static str,
    pub searching_gmail: &'static str,
    pub results_title: &'static str,
    pub results_header: &'static str,
    pub no_matching_emails: &'static str,
    pub selected_email_title: &'static str,
    pub no_email_selected: &'static str,
    pub from_label: &'static str,
    pub subject_label: &'static str,
    pub pdf_label: &'static str,
    pub json_label: &'static str,
    pub snippet_label: &'static str,
    pub unknown_sender: &'static str,
    pub no_subject: &'static str,
    pub review_title: &'static str,
    pub loading_invoice: &'static str,
    pub saving_invoice_files: &'static str,
    pub review_complete: &'static str,
    pub invoice_json_error_prefix: &'static str,
    pub invoice_save_error_prefix: &'static str,
    pub missing_json_error: &'static str,
    pub missing_pdf_error: &'static str,
    pub document_type_label: &'static str,
    pub ccf_warning: &'static str,
    pub control_number_label: &'static str,
    pub generation_code_label: &'static str,
    pub issue_date_label: &'static str,
    pub issuer_label: &'static str,
    pub receiver_label: &'static str,
    pub totals_label: &'static str,
    pub taxed_sales_label: &'static str,
    pub exempt_sales_label: &'static str,
    pub non_subject_sales_label: &'static str,
    pub subtotal_label: &'static str,
    pub operation_total_label: &'static str,
    pub total_to_pay_label: &'static str,
    pub payment_condition_label: &'static str,
    pub line_items_label: &'static str,
    pub drive_folder_label: &'static str,
    pub drive_file_ids_label: &'static str,
    pub processed_label: &'static str,
    pub skipped_label: &'static str,
    pub unit_price_label: &'static str,
    pub taxed_short_label: &'static str,
    pub exempt_short_label: &'static str,
}

impl Messages {
    pub fn authentication_event(&self, event: AuthenticationEvent) -> String {
        match (self.language, event) {
            (Language::Spanish, AuthenticationEvent::Checking) => {
                "Comprobando acceso a Gmail y Google Drive...".to_string()
            }
            (Language::English, AuthenticationEvent::Checking) => {
                "Checking Gmail and Google Drive access...".to_string()
            }
            (Language::Spanish, AuthenticationEvent::LoginRequired(reason)) => format!(
                "{}. Iniciando sesion con gws; completa la autorizacion en el navegador.",
                self.login_reason(reason)
            ),
            (Language::English, AuthenticationEvent::LoginRequired(reason)) => format!(
                "{}. Starting gws login; complete authorization in your browser.",
                self.login_reason(reason)
            ),
            (Language::Spanish, AuthenticationEvent::Verifying) => {
                "Verificando el acceso despues de iniciar sesion...".to_string()
            }
            (Language::English, AuthenticationEvent::Verifying) => {
                "Verifying access after login...".to_string()
            }
            (Language::Spanish, AuthenticationEvent::Ready) => {
                "Acceso a Google confirmado.".to_string()
            }
            (Language::English, AuthenticationEvent::Ready) => {
                "Google access confirmed.".to_string()
            }
        }
    }

    pub fn authentication_error(&self, error: &AuthenticationError) -> String {
        match (self.language, error) {
            (Language::Spanish, AuthenticationError::CliNotFound) => {
                "No se encontro el CLI gws en PATH. Instala gws antes de continuar.".to_string()
            }
            (Language::English, AuthenticationError::CliNotFound) => {
                "gws CLI was not found in PATH. Install gws before continuing.".to_string()
            }
            (Language::Spanish, AuthenticationError::Timeout(check)) => format!(
                "La comprobacion de {} excedio los 30 segundos. Intenta de nuevo.",
                self.authentication_check_name(*check)
            ),
            (Language::English, AuthenticationError::Timeout(check)) => format!(
                "The {} check exceeded 30 seconds. Try again.",
                self.authentication_check_name(*check)
            ),
            (Language::Spanish, AuthenticationError::Io(error)) => {
                format!("No se pudo ejecutar gws: {error}")
            }
            (Language::English, AuthenticationError::Io(error)) => {
                format!("Could not run gws: {error}")
            }
            (Language::Spanish, AuthenticationError::InvalidResponse(check)) => format!(
                "gws devolvio una respuesta JSON invalida al comprobar {}.",
                self.authentication_check_name(*check)
            ),
            (Language::English, AuthenticationError::InvalidResponse(check)) => format!(
                "gws returned an invalid JSON response while checking {}.",
                self.authentication_check_name(*check)
            ),
            (Language::Spanish, AuthenticationError::Configuration) => {
                "La configuracion OAuth de gws es invalida o esta incompleta. Revisa el cliente OAuth o ejecuta gws auth setup.".to_string()
            }
            (Language::English, AuthenticationError::Configuration) => {
                "The gws OAuth configuration is invalid or incomplete. Check the OAuth client or run gws auth setup.".to_string()
            }
            (Language::Spanish, AuthenticationError::Connection(check)) => format!(
                "No se pudo conectar con Google al comprobar {}. Revisa tu conexion e intenta de nuevo.",
                self.authentication_check_name(*check)
            ),
            (Language::English, AuthenticationError::Connection(check)) => format!(
                "Could not connect to Google while checking {}. Check your connection and try again.",
                self.authentication_check_name(*check)
            ),
            (Language::Spanish, AuthenticationError::ApiDisabled(check)) => format!(
                "La API de {} no esta habilitada en el proyecto de Google Cloud. Habilitala antes de continuar.",
                self.authentication_check_name(*check)
            ),
            (Language::English, AuthenticationError::ApiDisabled(check)) => format!(
                "The {} API is not enabled in the Google Cloud project. Enable it before continuing.",
                self.authentication_check_name(*check)
            ),
            (Language::Spanish, AuthenticationError::PermissionDenied(check)) => format!(
                "Google denego el acceso a {}. Revisa los permisos y las politicas de tu cuenta.",
                self.authentication_check_name(*check)
            ),
            (Language::English, AuthenticationError::PermissionDenied(check)) => format!(
                "Google denied access to {}. Check your account permissions and policies.",
                self.authentication_check_name(*check)
            ),
            (Language::Spanish, AuthenticationError::CheckFailed(check)) => format!(
                "No se pudo comprobar {}. Ejecuta gws auth status en tu terminal para revisar el detalle.",
                self.authentication_check_name(*check)
            ),
            (Language::English, AuthenticationError::CheckFailed(check)) => format!(
                "Could not check {}. Run gws auth status in your terminal for details.",
                self.authentication_check_name(*check)
            ),
            (Language::Spanish, AuthenticationError::LoginRequired(reason)) => format!(
                "No se pudo validar el acceso: {}. Revisa gws auth status en tu terminal.",
                self.login_reason(*reason)
            ),
            (Language::English, AuthenticationError::LoginRequired(reason)) => format!(
                "Could not validate access: {}. Check gws auth status in your terminal.",
                self.login_reason(*reason)
            ),
            (Language::Spanish, AuthenticationError::LoginFailed) => {
                "El inicio de sesion se cancelo o fallo. Presiona Reintentar para iniciar de nuevo.".to_string()
            }
            (Language::English, AuthenticationError::LoginFailed) => {
                "Login was cancelled or failed. Press Retry to try again.".to_string()
            }
        }
    }

    fn authentication_check_name(&self, check: AuthenticationCheck) -> &'static str {
        match (self.language, check) {
            (Language::Spanish, AuthenticationCheck::Status) => "la sesion de gws",
            (Language::English, AuthenticationCheck::Status) => "gws session",
            (_, AuthenticationCheck::Gmail) => "Gmail",
            (_, AuthenticationCheck::Drive) => "Google Drive",
        }
    }

    fn login_reason(&self, reason: LoginReason) -> &'static str {
        match (self.language, reason) {
            (Language::Spanish, LoginReason::MissingCredentials) => "No hay una sesion guardada",
            (Language::English, LoginReason::MissingCredentials) => "No saved session was found",
            (Language::Spanish, LoginReason::ExpiredSession) => "La sesion caduco o fue revocada",
            (Language::English, LoginReason::ExpiredSession) => {
                "The session expired or was revoked"
            }
            (Language::Spanish, LoginReason::InsufficientScopes) => {
                "Faltan permisos de Gmail o Drive"
            }
            (Language::English, LoginReason::InsufficientScopes) => {
                "Gmail or Drive permissions are missing"
            }
            (Language::Spanish, LoginReason::UnreadableCredentials) => {
                "gws no pudo descifrar las credenciales guardadas"
            }
            (Language::English, LoginReason::UnreadableCredentials) => {
                "gws could not decrypt the saved credentials"
            }
        }
    }

    pub fn candidate_count(&self, count: usize) -> String {
        match self.language {
            Language::Spanish if count == 1 => {
                format!("{count} correo candidato encontrado en INBOX.")
            }
            Language::Spanish => {
                format!("{count} correos candidatos encontrados en INBOX.")
            }
            Language::English if count == 1 => {
                format!("{count} candidate email found in INBOX.")
            }
            Language::English => {
                format!("{count} candidate emails found in INBOX.")
            }
        }
    }

    pub fn selected_candidate_count(&self, count: usize) -> String {
        match self.language {
            Language::Spanish if count == 1 => format!("{count} correo seleccionado."),
            Language::Spanish => format!("{count} correos seleccionados."),
            Language::English if count == 1 => format!("{count} email selected."),
            Language::English => format!("{count} emails selected."),
        }
    }

    pub fn review_progress(&self, index: usize, total: usize) -> String {
        match self.language {
            Language::Spanish => format!("Correo {} de {total}", index + 1),
            Language::English => format!("Email {} of {total}", index + 1),
        }
    }

    pub fn date_range_error(&self, error: &DateRangeError) -> &'static str {
        match (self.language, error) {
            (Language::Spanish, DateRangeError::InvalidInitialDate) => {
                "La fecha inicial debe ser una fecha valida con formato YYYY-MM-DD."
            }
            (Language::Spanish, DateRangeError::InvalidFinalDate) => {
                "La fecha final debe ser una fecha valida con formato YYYY-MM-DD."
            }
            (Language::Spanish, DateRangeError::InitialAfterFinal) => {
                "La fecha inicial debe ser anterior o igual a la fecha final."
            }
            (Language::English, DateRangeError::InvalidInitialDate) => {
                "Initial date must use a valid YYYY-MM-DD value."
            }
            (Language::English, DateRangeError::InvalidFinalDate) => {
                "Final date must use a valid YYYY-MM-DD value."
            }
            (Language::English, DateRangeError::InitialAfterFinal) => {
                "Initial date must be before or equal to final date."
            }
        }
    }

    pub fn gmail_search_error(&self, error: &GmailSearchError) -> String {
        match self.language {
            Language::Spanish => gmail_search_error_es(error),
            Language::English => gmail_search_error_en(error),
        }
    }

    pub fn drive_upload_error(&self, error: &DriveUploadError) -> String {
        match self.language {
            Language::Spanish => drive_upload_error_es(error),
            Language::English => drive_upload_error_en(error),
        }
    }
}

static SPANISH: Messages = Messages {
    search_button: "Buscar facturas",
    back_button: "Volver",
    review_selected_button: "Revisar seleccionados",
    accept_upload_button: "Aceptar y subir",
    skip_button: "Omitir",
    save_button: "Guardar cambios",
    retry_button: "Reintentar",
    connected_label: "Google conectado",
    connection_title: "Conexion con Google",
    connection_hint: "Si se solicita autorizacion, completala en tu navegador.",
    app_subtitle: "Facturas electronicas \u{b7} Gmail y Google Drive",
    period_label: "Periodo de busqueda",
    select_all_button: "Seleccionar todos",
    deselect_all_button: "Quitar seleccion",
    email_date_label: "Fecha de correo",
    quantity_label: "Cantidad",
    discount_label: "Descuentos",
    income_retention_label: "Retencion de renta",
    vat_retention_label: "Retencion de IVA",
    vat_perception_label: "Percepcion de IVA",
    review_manual_hint: "Confirma manualmente si corresponde procesar este documento.",
    months: [
        "Enero",
        "Febrero",
        "Marzo",
        "Abril",
        "Mayo",
        "Junio",
        "Julio",
        "Agosto",
        "Septiembre",
        "Octubre",
        "Noviembre",
        "Diciembre",
    ],
    weekdays: ["Lu", "Ma", "Mi", "Ju", "Vi", "Sa", "Do"],

    language: Language::Spanish,
    app_title: "DTE Uploader",
    search_criteria_title: "Criterios de busqueda",
    search_criteria_subtitle: "Selecciona el periodo de Gmail antes de buscar facturas.",
    initial_date: "Desde",
    final_date: "Hasta",
    default_period: "El periodo predeterminado va del primer al ultimo dia del mes anterior.",
    settings_title: "Configuracion",
    settings_header: "Preferencias de la aplicacion",
    download_dir_label: "Carpeta de descarga",
    drive_root_folder_label: "Carpeta base en Drive",
    dte_query_filter_label: "Filtro DTE en Gmail",
    dte_query_filter_hint: "Vacio desactiva este filtro.",
    language_label: "Idioma",
    language_spanish: "Espanol",
    language_english: "English",
    settings_saved: "Configuracion guardada.",
    settings_empty_download_dir: "La carpeta de descarga no puede estar vacia.",
    settings_empty_drive_root_folder: "La carpeta base en Drive no puede estar vacia.",
    searching_gmail: "Buscando en Gmail INBOX...",
    results_title: "Correos de factura encontrados",
    results_header: "Correos con adjuntos PDF y JSON",
    no_matching_emails: "No se encontraron correos para este periodo.",
    selected_email_title: "Correo seleccionado",
    no_email_selected: "No hay correo seleccionado.",
    from_label: "De: ",
    subject_label: "Asunto: ",
    pdf_label: "PDF: ",
    json_label: "JSON: ",
    snippet_label: "Resumen: ",
    unknown_sender: "(remitente desconocido)",
    no_subject: "(sin asunto)",
    review_title: "Revision de facturas",
    loading_invoice: "Descargando JSON de la factura...",
    saving_invoice_files: "Descargando PDF, guardando archivos y subiendo a Drive...",
    review_complete: "Revision completada.",
    invoice_json_error_prefix: "No se pudo leer el JSON de la factura",
    invoice_save_error_prefix: "No se pudieron guardar los archivos",
    missing_json_error: "Este correo no tiene un adjunto JSON descargable.",
    missing_pdf_error: "Este correo no tiene un adjunto PDF descargable.",
    document_type_label: "Tipo de documento",
    ccf_warning: "Este JSON no parece ser CCF (tipoDte 03).",
    control_number_label: "Numero de control",
    generation_code_label: "Codigo de generacion",
    issue_date_label: "Fecha de emision",
    issuer_label: "Emisor",
    receiver_label: "Receptor",
    totals_label: "Montos",
    taxed_sales_label: "Ventas gravadas",
    exempt_sales_label: "Ventas exentas",
    non_subject_sales_label: "Ventas no sujetas",
    subtotal_label: "Subtotal",
    operation_total_label: "Total operacion",
    total_to_pay_label: "Total a pagar",
    payment_condition_label: "Condicion",
    line_items_label: "Lineas",
    drive_folder_label: "Carpeta de Drive",
    drive_file_ids_label: "IDs en Drive",
    processed_label: "Procesados",
    skipped_label: "Omitidos",
    unit_price_label: "Unitario",
    taxed_short_label: "Gravada",
    exempt_short_label: "Exenta",
};

static ENGLISH: Messages = Messages {
    search_button: "Search invoices",
    back_button: "Back",
    review_selected_button: "Review selected",
    accept_upload_button: "Accept and upload",
    skip_button: "Skip",
    save_button: "Save changes",
    retry_button: "Retry",
    connected_label: "Google connected",
    connection_title: "Connect to Google",
    connection_hint: "If authorization is requested, complete it in your browser.",
    app_subtitle: "Electronic invoices \u{b7} Gmail and Google Drive",
    period_label: "Search period",
    select_all_button: "Select all",
    deselect_all_button: "Deselect all",
    email_date_label: "Email date",
    quantity_label: "Quantity",
    discount_label: "Discounts",
    income_retention_label: "Income tax withholding",
    vat_retention_label: "VAT withholding",
    vat_perception_label: "VAT perception",
    review_manual_hint: "Manually confirm whether this document should be processed.",
    months: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
    weekdays: ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"],

    language: Language::English,
    app_title: "DTE Uploader",
    search_criteria_title: "Search Criteria",
    search_criteria_subtitle: "Select the Gmail search period before collecting invoice emails.",
    initial_date: "Initial date",
    final_date: "Final date",
    default_period: "The default period is the first through last day of the previous month.",
    settings_title: "Settings",
    settings_header: "Application preferences",
    download_dir_label: "Download folder",
    drive_root_folder_label: "Drive base folder",
    dte_query_filter_label: "Gmail DTE filter",
    dte_query_filter_hint: "Empty disables this filter.",
    language_label: "Language",
    language_spanish: "Espanol",
    language_english: "English",
    settings_saved: "Settings saved.",
    settings_empty_download_dir: "Download folder cannot be empty.",
    settings_empty_drive_root_folder: "Drive base folder cannot be empty.",
    searching_gmail: "Searching Gmail in INBOX...",
    results_title: "Matching invoice emails",
    results_header: "Emails with both PDF and JSON attachments",
    no_matching_emails: "No matching emails found for this period.",
    selected_email_title: "Selected email",
    no_email_selected: "No email selected.",
    from_label: "From: ",
    subject_label: "Subject: ",
    pdf_label: "PDF: ",
    json_label: "JSON: ",
    snippet_label: "Snippet: ",
    unknown_sender: "(unknown sender)",
    no_subject: "(no subject)",
    review_title: "Invoice review",
    loading_invoice: "Downloading invoice JSON...",
    saving_invoice_files: "Downloading PDF, saving files, and uploading to Drive...",
    review_complete: "Review complete.",
    invoice_json_error_prefix: "Could not read the invoice JSON",
    invoice_save_error_prefix: "Could not save the files",
    missing_json_error: "This email does not have a downloadable JSON attachment.",
    missing_pdf_error: "This email does not have a downloadable PDF attachment.",
    document_type_label: "Document type",
    ccf_warning: "This JSON does not look like CCF (tipoDte 03).",
    control_number_label: "Control number",
    generation_code_label: "Generation code",
    issue_date_label: "Issue date",
    issuer_label: "Issuer",
    receiver_label: "Receiver",
    totals_label: "Amounts",
    taxed_sales_label: "Taxed sales",
    exempt_sales_label: "Exempt sales",
    non_subject_sales_label: "Non-subject sales",
    subtotal_label: "Subtotal",
    operation_total_label: "Operation total",
    total_to_pay_label: "Total to pay",
    payment_condition_label: "Condition",
    line_items_label: "Line items",
    drive_folder_label: "Drive folder",
    drive_file_ids_label: "Drive IDs",
    processed_label: "Processed",
    skipped_label: "Skipped",
    unit_price_label: "Unit",
    taxed_short_label: "Taxed",
    exempt_short_label: "Exempt",
};

fn gmail_search_error_es(error: &GmailSearchError) -> String {
    match error {
        GmailSearchError::CliNotFound => "No se encontro el CLI gws en PATH.".to_string(),
        GmailSearchError::CommandFailed {
            command,
            status,
            stderr,
        } if stderr.is_empty() => {
            format!("`{command}` fallo con codigo de salida {status}.")
        }
        GmailSearchError::CommandFailed {
            command,
            status,
            stderr,
        } => {
            format!("`{command}` fallo con codigo de salida {status}: {stderr}")
        }
        GmailSearchError::Io(error) => format!("No se pudo ejecutar gws: {error}"),
        GmailSearchError::Json { context, source } => {
            format!("No se pudo leer el JSON de gws para {context}: {source}")
        }
        GmailSearchError::PageLimitReached { query, page_limit } => format!(
            "La busqueda llego al limite de {page_limit} paginas para `{query}`. Reduce el rango de fechas e intenta otra vez."
        ),
        GmailSearchError::AttachmentNotDownloadable { filename } => {
            format!("El adjunto `{filename}` no se puede descargar.")
        }
        GmailSearchError::AttachmentMissingData { filename } => {
            format!("El adjunto `{filename}` no incluyo datos descargables.")
        }
        GmailSearchError::Base64 { filename, source } => {
            format!("No se pudo decodificar el adjunto `{filename}`: {source}")
        }
    }
}

fn gmail_search_error_en(error: &GmailSearchError) -> String {
    match error {
        GmailSearchError::CliNotFound => "gws CLI was not found in PATH.".to_string(),
        GmailSearchError::CommandFailed {
            command,
            status,
            stderr,
        } if stderr.is_empty() => {
            format!("`{command}` failed with exit status {status}.")
        }
        GmailSearchError::CommandFailed {
            command,
            status,
            stderr,
        } => {
            format!("`{command}` failed with exit status {status}: {stderr}")
        }
        GmailSearchError::Io(error) => format!("Failed to run gws: {error}"),
        GmailSearchError::Json { context, source } => {
            format!("Failed to parse gws JSON for {context}: {source}")
        }
        GmailSearchError::PageLimitReached { query, page_limit } => format!(
            "Gmail search reached the {page_limit}-page limit for query `{query}`. Narrow the date range and try again."
        ),
        GmailSearchError::AttachmentNotDownloadable { filename } => {
            format!("Attachment `{filename}` is not downloadable.")
        }
        GmailSearchError::AttachmentMissingData { filename } => {
            format!("Attachment `{filename}` did not include downloadable data.")
        }
        GmailSearchError::Base64 { filename, source } => {
            format!("Failed to decode attachment `{filename}`: {source}")
        }
    }
}

fn drive_upload_error_es(error: &DriveUploadError) -> String {
    match error {
        DriveUploadError::CliNotFound => "No se encontro el CLI gws en PATH.".to_string(),
        DriveUploadError::CommandFailed {
            command,
            status,
            stderr,
        } if stderr.is_empty() => {
            format!("`{command}` fallo con codigo de salida {status}.")
        }
        DriveUploadError::CommandFailed {
            command,
            status,
            stderr,
        } => {
            format!("`{command}` fallo con codigo de salida {status}: {stderr}")
        }
        DriveUploadError::Io(error) => format!("No se pudo ejecutar gws Drive: {error}"),
        DriveUploadError::Json { context, source } => {
            format!("No se pudo leer el JSON de gws Drive para {context}: {source}")
        }
        DriveUploadError::InvalidIssueDate { value } => {
            format!("La fecha de emision `{value}` no permite resolver la carpeta de Drive.")
        }
        DriveUploadError::MissingFileName { path } => {
            format!(
                "No se pudo determinar el nombre del archivo `{}`.",
                path.display()
            )
        }
    }
}

fn drive_upload_error_en(error: &DriveUploadError) -> String {
    match error {
        DriveUploadError::CliNotFound => "gws CLI was not found in PATH.".to_string(),
        DriveUploadError::CommandFailed {
            command,
            status,
            stderr,
        } if stderr.is_empty() => {
            format!("`{command}` failed with exit status {status}.")
        }
        DriveUploadError::CommandFailed {
            command,
            status,
            stderr,
        } => {
            format!("`{command}` failed with exit status {status}: {stderr}")
        }
        DriveUploadError::Io(error) => format!("Failed to run gws Drive: {error}"),
        DriveUploadError::Json { context, source } => {
            format!("Failed to parse gws Drive JSON for {context}: {source}")
        }
        DriveUploadError::InvalidIssueDate { value } => {
            format!("Invoice issue date `{value}` cannot be mapped to a Drive folder.")
        }
        DriveUploadError::MissingFileName { path } => {
            format!(
                "Could not determine the file name for `{}`.",
                path.display()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spanish_is_the_default_language() {
        assert_eq!(DEFAULT_LANGUAGE, Language::Spanish);
        assert_eq!(
            messages_for(DEFAULT_LANGUAGE).search_criteria_title,
            "Criterios de busqueda"
        );
    }

    #[test]
    fn english_messages_are_available() {
        assert_eq!(
            messages_for(Language::English).search_criteria_title,
            "Search Criteria"
        );
    }

    #[test]
    fn startup_messages_use_the_configured_language() {
        let spanish = messages_for(Language::Spanish);
        let english = messages_for(Language::English);
        assert_eq!(
            spanish.authentication_event(AuthenticationEvent::Checking),
            "Comprobando acceso a Gmail y Google Drive..."
        );
        assert_eq!(
            english.authentication_event(AuthenticationEvent::Checking),
            "Checking Gmail and Google Drive access..."
        );
        assert!(
            spanish
                .authentication_error(&AuthenticationError::Timeout(AuthenticationCheck::Drive))
                .contains("Google Drive excedio los 30 segundos")
        );
        assert!(
            english
                .authentication_error(&AuthenticationError::Timeout(AuthenticationCheck::Gmail))
                .contains("Gmail check exceeded 30 seconds")
        );
    }
}
