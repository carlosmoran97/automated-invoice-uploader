use crate::{
    domain::date_range::DateRangeError,
    services::{drive_upload::DriveUploadError, gmail_search::GmailSearchError},
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
    language: Language,
    pub app_title: &'static str,
    pub search_criteria_title: &'static str,
    pub search_criteria_subtitle: &'static str,
    pub initial_date: &'static str,
    pub final_date: &'static str,
    pub form_footer: &'static str,
    pub calendar_footer: &'static str,
    pub calendar_is_open: &'static str,
    pub press_space_to_open: &'static str,
    pub press_tab_to_focus: &'static str,
    pub select_initial_date: &'static str,
    pub select_final_date: &'static str,
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
    pub settings_footer: &'static str,
    pub settings_empty_download_dir: &'static str,
    pub settings_empty_drive_root_folder: &'static str,
    pub searching_gmail: &'static str,
    pub search_loader_title: &'static str,
    pub search_loader_stage_query: &'static str,
    pub search_loader_stage_pdf: &'static str,
    pub search_loader_stage_json: &'static str,
    pub search_loader_stage_match: &'static str,
    pub search_loader_meter: &'static str,
    pub search_loader_lane_pdf: &'static str,
    pub search_loader_lane_json: &'static str,
    pub search_loader_lane_match: &'static str,
    pub search_loader_footer: &'static str,
    pub results_title: &'static str,
    pub results_header: &'static str,
    pub results_list_title: &'static str,
    pub no_matching_emails: &'static str,
    pub selected_email_title: &'static str,
    pub no_email_selected: &'static str,
    pub from_label: &'static str,
    pub subject_label: &'static str,
    pub pdf_label: &'static str,
    pub json_label: &'static str,
    pub snippet_label: &'static str,
    pub results_footer: &'static str,
    pub unknown_sender: &'static str,
    pub no_subject: &'static str,
    pub unknown_date: &'static str,
    pub no_snippet: &'static str,
    pub gmail_search_stopped: &'static str,
    pub review_title: &'static str,
    pub loading_invoice: &'static str,
    pub saving_invoice_files: &'static str,
    pub review_complete: &'static str,
    pub review_error_title: &'static str,
    pub review_prompt: &'static str,
    pub review_busy_footer: &'static str,
    pub review_ready_footer: &'static str,
    pub review_error_footer: &'static str,
    pub review_complete_footer: &'static str,
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
    pub taxes_label: &'static str,
    pub line_items_label: &'static str,
    pub saved_files_label: &'static str,
    pub drive_folder_label: &'static str,
    pub drive_file_ids_label: &'static str,
    pub processed_label: &'static str,
    pub skipped_label: &'static str,
    pub unit_price_label: &'static str,
    pub taxed_short_label: &'static str,
    pub exempt_short_label: &'static str,
}

impl Messages {
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
    language: Language::Spanish,
    app_title: "DTE Uploader",
    search_criteria_title: "Criterios de busqueda",
    search_criteria_subtitle: "Selecciona el periodo de Gmail antes de buscar facturas.",
    initial_date: "Fecha inicial",
    final_date: "Fecha final",
    form_footer: "Tab: cambiar campo  Texto: editar filtro  Enter: buscar  s: configuracion  q: salir",
    calendar_footer: "Fechas: Espacio abre calendario. Calendario: flechas dia/semana  PgUp/PgDn mes",
    calendar_is_open: "Calendario abierto",
    press_space_to_open: "Presiona Espacio para abrir",
    press_tab_to_focus: "Presiona Tab para enfocar",
    select_initial_date: "Seleccionar fecha inicial",
    select_final_date: "Seleccionar fecha final",
    default_period: "El periodo predeterminado va del primer al ultimo dia del mes actual.",
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
    settings_footer: "Tab: cambiar campo  Texto: editar  Espacio: cambiar idioma  Enter: guardar  Esc: volver  Ctrl+q: salir",
    settings_empty_download_dir: "La carpeta de descarga no puede estar vacia.",
    settings_empty_drive_root_folder: "La carpeta base en Drive no puede estar vacia.",
    searching_gmail: "Buscando en Gmail INBOX...",
    search_loader_title: "Escaner de Gmail",
    search_loader_stage_query: "consultando mensajes",
    search_loader_stage_pdf: "rastreando PDF",
    search_loader_stage_json: "rastreando JSON",
    search_loader_stage_match: "cruzando resultados",
    search_loader_meter: "Busqueda",
    search_loader_lane_pdf: "PDF",
    search_loader_lane_json: "JSON",
    search_loader_lane_match: "Cruce",
    search_loader_footer: "Filtrando correos que tienen PDF y JSON al mismo tiempo.",
    results_title: "Correos de factura encontrados",
    results_header: "Correos con adjuntos PDF y JSON",
    results_list_title: "Resultados",
    no_matching_emails: "No se encontraron correos para este periodo.",
    selected_email_title: "Correo seleccionado",
    no_email_selected: "No hay correo seleccionado.",
    from_label: "De: ",
    subject_label: "Asunto: ",
    pdf_label: "PDF: ",
    json_label: "JSON: ",
    snippet_label: "Resumen: ",
    results_footer: "Arriba/Abajo: mover  PgUp/PgDn: texto  Espacio: seleccionar/quitar  Enter: revisar  b/Esc: volver  q: salir",
    unknown_sender: "(remitente desconocido)",
    no_subject: "(sin asunto)",
    unknown_date: "(fecha desconocida)",
    no_snippet: "(sin resumen)",
    gmail_search_stopped: "La busqueda de Gmail se detuvo inesperadamente.",
    review_title: "Revision de facturas",
    loading_invoice: "Descargando JSON de la factura...",
    saving_invoice_files: "Descargando PDF, guardando archivos y subiendo a Drive...",
    review_complete: "Revision completada.",
    review_error_title: "No se pudo revisar este correo",
    review_prompt: "Procesar este correo? Y/n",
    review_busy_footer: "q: salir",
    review_ready_footer: "Y: procesar y subir a Drive  n: omitir  q: salir",
    review_error_footer: "n: omitir  q: salir",
    review_complete_footer: "Enter/Esc: volver a resultados  q: salir",
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
    taxes_label: "Tributos",
    line_items_label: "Lineas",
    saved_files_label: "Archivos guardados",
    drive_folder_label: "Carpeta de Drive",
    drive_file_ids_label: "IDs en Drive",
    processed_label: "Procesados",
    skipped_label: "Omitidos",
    unit_price_label: "Unitario",
    taxed_short_label: "Gravada",
    exempt_short_label: "Exenta",
};

static ENGLISH: Messages = Messages {
    language: Language::English,
    app_title: "DTE Uploader",
    search_criteria_title: "Search Criteria",
    search_criteria_subtitle: "Select the Gmail search period before collecting invoice emails.",
    initial_date: "Initial date",
    final_date: "Final date",
    form_footer: "Tab: switch field  Text: edit filter  Enter: search  s: settings  q: quit",
    calendar_footer: "Dates: Space opens calendar. Calendar: arrows move day/week  PgUp/PgDn month",
    calendar_is_open: "Calendar is open",
    press_space_to_open: "Press Space to open",
    press_tab_to_focus: "Press Tab to focus",
    select_initial_date: "Select initial date",
    select_final_date: "Select final date",
    default_period: "The default period is the first through last day of the current month.",
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
    settings_footer: "Tab: switch field  Text: edit  Space: change language  Enter: save  Esc: back  Ctrl+q: quit",
    settings_empty_download_dir: "Download folder cannot be empty.",
    settings_empty_drive_root_folder: "Drive base folder cannot be empty.",
    searching_gmail: "Searching Gmail in INBOX...",
    search_loader_title: "Gmail Scanner",
    search_loader_stage_query: "querying messages",
    search_loader_stage_pdf: "tracking PDF",
    search_loader_stage_json: "tracking JSON",
    search_loader_stage_match: "matching results",
    search_loader_meter: "Search",
    search_loader_lane_pdf: "PDF",
    search_loader_lane_json: "JSON",
    search_loader_lane_match: "Match",
    search_loader_footer: "Filtering emails that have PDF and JSON attachments at the same time.",
    results_title: "Matching invoice emails",
    results_header: "Emails with both PDF and JSON attachments",
    results_list_title: "Results",
    no_matching_emails: "No matching emails found for this period.",
    selected_email_title: "Selected email",
    no_email_selected: "No email selected.",
    from_label: "From: ",
    subject_label: "Subject: ",
    pdf_label: "PDF: ",
    json_label: "JSON: ",
    snippet_label: "Snippet: ",
    results_footer: "Up/Down: move  PgUp/PgDn: text  Space: select/unselect  Enter: review  b/Esc: back  q: quit",
    unknown_sender: "(unknown sender)",
    no_subject: "(no subject)",
    unknown_date: "(unknown date)",
    no_snippet: "(no snippet)",
    gmail_search_stopped: "Gmail search stopped unexpectedly.",
    review_title: "Invoice review",
    loading_invoice: "Downloading invoice JSON...",
    saving_invoice_files: "Downloading PDF, saving files, and uploading to Drive...",
    review_complete: "Review complete.",
    review_error_title: "Could not review this email",
    review_prompt: "Process this email? Y/n",
    review_busy_footer: "q: quit",
    review_ready_footer: "Y: process and upload to Drive  n: skip  q: quit",
    review_error_footer: "n: skip  q: quit",
    review_complete_footer: "Enter/Esc: back to results  q: quit",
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
    taxes_label: "Taxes",
    line_items_label: "Line items",
    saved_files_label: "Saved files",
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
}
