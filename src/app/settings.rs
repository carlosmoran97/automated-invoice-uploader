use crate::{
    app::state::SettingsInput,
    i18n::Messages,
    services::settings::{AppSettings, save_settings},
};
use std::path::PathBuf;

pub(super) fn save_settings_input(
    input: SettingsInput,
    text: &Messages,
) -> Result<AppSettings, String> {
    let settings = settings_from_input(input, text)?;
    save_settings(&settings).map_err(|error| error.to_string())?;

    Ok(settings)
}

fn settings_from_input(input: SettingsInput, text: &Messages) -> Result<AppSettings, String> {
    let download_dir = input.download_dir.trim();
    if download_dir.is_empty() {
        return Err(text.settings_empty_download_dir.to_string());
    }

    let drive_root_folder = input.drive_root_folder.trim();
    if drive_root_folder.is_empty() {
        return Err(text.settings_empty_drive_root_folder.to_string());
    }

    Ok(AppSettings {
        download_dir: PathBuf::from(download_dir),
        drive_root_folder: drive_root_folder.to_string(),
        dte_query_filter: input.dte_query_filter.trim().to_string(),
        language: input.language,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Language;

    fn input() -> SettingsInput {
        SettingsInput {
            download_dir: " /tmp/invoices ".into(),
            drive_root_folder: " Accountant ".into(),
            dte_query_filter: " DTE-03 ".into(),
            language: Language::English,
        }
    }

    #[test]
    fn validates_and_trims_settings_without_changing_the_schema() {
        let settings =
            settings_from_input(input(), crate::i18n::messages_for(Language::English)).unwrap();
        assert_eq!(settings.download_dir, PathBuf::from("/tmp/invoices"));
        assert_eq!(settings.drive_root_folder, "Accountant");
        assert_eq!(settings.dte_query_filter, "DTE-03");
        assert_eq!(settings.language, Language::English);
    }

    #[test]
    fn empty_folders_are_rejected_and_an_empty_query_filter_is_allowed() {
        let text = crate::i18n::messages_for(Language::Spanish);
        let mut value = input();
        value.download_dir = "  ".into();
        assert_eq!(
            settings_from_input(value, text).unwrap_err(),
            text.settings_empty_download_dir
        );
        let mut value = input();
        value.drive_root_folder = "  ".into();
        assert_eq!(
            settings_from_input(value, text).unwrap_err(),
            text.settings_empty_drive_root_folder
        );
        let mut value = input();
        value.dte_query_filter.clear();
        assert!(
            settings_from_input(value, text)
                .unwrap()
                .dte_query_filter
                .is_empty()
        );
    }
}
