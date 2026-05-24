use crate::{
    components::settings::SettingsInput,
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
