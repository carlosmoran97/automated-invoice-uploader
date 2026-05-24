use crate::{domain::date_range::DateRangeError, services::gmail_search::GmailSearchError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Spanish,
    English,
}

pub const DEFAULT_LANGUAGE: Language = Language::Spanish;

pub fn messages() -> &'static Messages {
    messages_for(DEFAULT_LANGUAGE)
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
    pub searching_gmail: &'static str,
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
}

static SPANISH: Messages = Messages {
    language: Language::Spanish,
    app_title: "Automated Invoice Uploader",
    search_criteria_title: "Criterios de busqueda",
    search_criteria_subtitle: "Selecciona el periodo de Gmail antes de buscar facturas.",
    initial_date: "Fecha inicial",
    final_date: "Fecha final",
    form_footer: "Tab: cambiar campo  Espacio: abrir calendario  Enter: buscar  q: salir",
    calendar_footer: "En calendario: flechas dia/semana  PgUp/PgDn mes  Enter/Esc cerrar",
    calendar_is_open: "Calendario abierto",
    press_space_to_open: "Presiona Espacio para abrir",
    press_tab_to_focus: "Presiona Tab para enfocar",
    select_initial_date: "Seleccionar fecha inicial",
    select_final_date: "Seleccionar fecha final",
    default_period: "El periodo predeterminado va del primer al ultimo dia del mes actual.",
    searching_gmail: "Buscando en Gmail INBOX...",
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
    results_footer: "Arriba/Abajo: mover  Espacio: seleccionar/quitar  b/Esc: volver  q: salir",
    unknown_sender: "(remitente desconocido)",
    no_subject: "(sin asunto)",
    unknown_date: "(fecha desconocida)",
    no_snippet: "(sin resumen)",
    gmail_search_stopped: "La busqueda de Gmail se detuvo inesperadamente.",
};

static ENGLISH: Messages = Messages {
    language: Language::English,
    app_title: "Automated Invoice Uploader",
    search_criteria_title: "Search Criteria",
    search_criteria_subtitle: "Select the Gmail search period before collecting invoice emails.",
    initial_date: "Initial date",
    final_date: "Final date",
    form_footer: "Tab: switch field  Space: open calendar  Enter: search  q: quit",
    calendar_footer: "In calendar: arrows move day/week  PgUp/PgDn month  Enter/Esc close",
    calendar_is_open: "Calendar is open",
    press_space_to_open: "Press Space to open",
    press_tab_to_focus: "Press Tab to focus",
    select_initial_date: "Select initial date",
    select_final_date: "Select final date",
    default_period: "The default period is the first through last day of the current month.",
    searching_gmail: "Searching Gmail in INBOX...",
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
    results_footer: "Up/Down: move  Space: select/unselect  b/Esc: back  q: quit",
    unknown_sender: "(unknown sender)",
    no_subject: "(no subject)",
    unknown_date: "(unknown date)",
    no_snippet: "(no snippet)",
    gmail_search_stopped: "Gmail search stopped unexpectedly.",
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spanish_is_the_default_language() {
        assert_eq!(DEFAULT_LANGUAGE, Language::Spanish);
        assert_eq!(messages().search_criteria_title, "Criterios de busqueda");
    }

    #[test]
    fn english_messages_are_available() {
        assert_eq!(
            messages_for(Language::English).search_criteria_title,
            "Search Criteria"
        );
    }
}
