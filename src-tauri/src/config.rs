use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub scale: f64,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl WindowConfig {
    pub fn load() -> Self {
        let path = Self::path();
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str::<Self>(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let body = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, body).map_err(|e| e.to_string())
    }

    fn path() -> PathBuf {
        if let Some(dir) = dirs::config_dir() {
            return dir.join("desktop-pet").join("window.json");
        }
        PathBuf::from("window.json")
    }
}
