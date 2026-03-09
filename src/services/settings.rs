use std::{env, fs, io, path::PathBuf};

use crate::shortcuts::ShortcutSettings;

const APP_CONFIG_DIR: &str = "MLVInspector";
const SHORTCUTS_FILE_NAME: &str = "shortcuts.json";

pub fn load_shortcut_settings() -> io::Result<ShortcutSettings> {
    let path = shortcut_settings_path()?;

    match fs::read_to_string(path) {
        Ok(json) => serde_json::from_str(&json)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(ShortcutSettings::default()),
        Err(err) => Err(err),
    }
}

pub fn save_shortcut_settings(settings: &ShortcutSettings) -> io::Result<()> {
    let path = shortcut_settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(settings)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, json)
}

fn shortcut_settings_path() -> io::Result<PathBuf> {
    Ok(config_root_dir()?
        .join(APP_CONFIG_DIR)
        .join(SHORTCUTS_FILE_NAME))
}

fn config_root_dir() -> io::Result<PathBuf> {
    if let Ok(value) = env::var("APPDATA") {
        return Ok(PathBuf::from(value));
    }

    if let Ok(value) = env::var("LOCALAPPDATA") {
        return Ok(PathBuf::from(value));
    }

    if let Ok(value) = env::var("HOME") {
        return Ok(PathBuf::from(value).join(".config"));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Could not determine config directory for shortcut settings",
    ))
}
