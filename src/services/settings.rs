use crate::{
    i18n::{DEFAULT_LANGUAGE, Language},
    services::{drive_upload::DEFAULT_ROOT_FOLDER_NAME, invoice_files::DEFAULT_DOWNLOAD_DIR},
};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct AppSettings {
    pub download_dir: PathBuf,
    pub drive_root_folder: String,
    pub language: Language,
}

#[derive(Debug)]
pub enum SettingsError {
    ConfigDirectoryUnavailable,
    Io(io::Error),
    Json(serde_json::Error),
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::from(DEFAULT_DOWNLOAD_DIR),
            drive_root_folder: DEFAULT_ROOT_FOLDER_NAME.to_string(),
            language: DEFAULT_LANGUAGE,
        }
    }
}

pub fn load_settings() -> Result<AppSettings, SettingsError> {
    let path = settings_path()?;
    let input = match fs::read_to_string(&path) {
        Ok(input) => input,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(AppSettings::default()),
        Err(error) => return Err(SettingsError::Io(error)),
    };

    serde_json::from_str(&input).map_err(SettingsError::Json)
}

pub fn save_settings(settings: &AppSettings) -> Result<(), SettingsError> {
    let path = settings_path()?;
    let directory = path
        .parent()
        .ok_or(SettingsError::ConfigDirectoryUnavailable)?;
    fs::create_dir_all(directory).map_err(SettingsError::Io)?;

    let temp_path = temporary_settings_path(&path);
    let output = serde_json::to_string_pretty(settings).map_err(SettingsError::Json)?;
    fs::write(&temp_path, output).map_err(SettingsError::Io)?;
    fs::rename(temp_path, path).map_err(SettingsError::Io)
}

fn settings_path() -> Result<PathBuf, SettingsError> {
    ProjectDirs::from("com", "carlosmoran", "automated-invoice-uploader")
        .map(|directories| directories.config_dir().join(SETTINGS_FILE))
        .ok_or(SettingsError::ConfigDirectoryUnavailable)
}

fn temporary_settings_path(path: &Path) -> PathBuf {
    path.with_extension("json.tmp")
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigDirectoryUnavailable => write!(
                formatter,
                "Could not determine the user configuration directory."
            ),
            Self::Io(error) => write!(formatter, "Could not read or write settings: {error}"),
            Self::Json(error) => write!(formatter, "Could not parse settings JSON: {error}"),
        }
    }
}

impl std::error::Error for SettingsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_preserve_current_workflow() {
        let settings = AppSettings::default();

        assert_eq!(settings.download_dir, PathBuf::from("downloaded_invoices"));
        assert_eq!(settings.drive_root_folder, "CARLOS ROLANDO MORAN CAMPOS");
        assert_eq!(settings.language, Language::Spanish);
    }

    #[test]
    fn settings_serialize_language_as_short_code() {
        let settings = AppSettings {
            language: Language::English,
            ..AppSettings::default()
        };

        let json = serde_json::to_string(&settings).unwrap();

        assert!(json.contains(r#""language":"en""#));
    }
}
