use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

const APP_DIR: &str = "desktop-pet";
const APP_CONFIG_FILE: &str = "config.json";
const LEGACY_WINDOW_FILE: &str = "window.json";
const SCALE_MIN: f64 = 0.1;
const SCALE_MAX: f64 = 1.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub asr: AsrConfig,
    #[serde(default)]
    pub pet: PetConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            asr: AsrConfig::default(),
            pet: PetConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_llm_base_url")]
    pub base_url: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: default_llm_model(),
            base_url: default_llm_base_url(),
        }
    }
}

fn default_llm_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_llm_base_url() -> String {
    "https://open.bigmodel.cn/api/paas/v4".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrConfig {
    #[serde(default = "default_asr_provider")]
    pub provider: String,
    // 兼容旧配置文件中遗留字段，运行时不再使用
    #[serde(default)]
    pub sherpa_onnx: LegacySherpaOnnxConfig,
}

impl Default for AsrConfig {
    fn default() -> Self {
        Self {
            provider: default_asr_provider(),
            sherpa_onnx: LegacySherpaOnnxConfig::default(),
        }
    }
}

fn default_asr_provider() -> String {
    "system".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegacySherpaOnnxConfig {
    #[serde(default)]
    pub model_size: String,
    #[serde(default)]
    pub model_dir: String,
    #[serde(default)]
    pub num_threads: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineAsrConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub base_url: String,
}

impl Default for OnlineAsrConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: String::new(),
            base_url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetConfig {
    #[serde(default = "default_pet")]
    pub current: String,
    #[serde(default = "default_pet_scale")]
    pub scale: f64,
    #[serde(default = "default_pet_name")]
    pub name: String,
    #[serde(default = "default_pet_prompt")]
    pub prompt: String,
}

impl Default for PetConfig {
    fn default() -> Self {
        Self {
            current: default_pet(),
            scale: default_pet_scale(),
            name: default_pet_name(),
            prompt: default_pet_prompt(),
        }
    }
}

fn default_pet() -> String {
    "dog".to_string()
}

fn default_pet_scale() -> f64 {
    1.0
}

fn default_pet_name() -> String {
    "小白".to_string()
}

fn default_pet_prompt() -> String {
    "你是一只可爱的桌面宠物，名字叫{name}。你的性格活泼、友好、有点调皮。请用简短、口语化的方式回复，不要太长。回复时要像宠物一样可爱，可以用一些语气词如\"汪\"、\"呀\"、\"呢\"等。".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyWindowConfig {
    #[serde(default = "default_pet_scale")]
    scale: f64,
}

impl AppConfig {
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
            let mut conf = serde_json::from_str::<Self>(&content).map_err(|e| e.to_string())?;
            conf.normalize();
            return Ok(conf);
        }

        let mut conf = Self::default();
        if let Some(legacy) = Self::load_legacy_window_scale() {
            conf.pet.scale = legacy;
        }
        Ok(conf)
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let mut normalized = self.clone();
        normalized.normalize();

        let body = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
        fs::write(path, body).map_err(|e| e.to_string())
    }

    pub fn normalized(mut self) -> Self {
        self.normalize();
        self
    }

    pub fn config_path() -> PathBuf {
        config_dir().join(APP_CONFIG_FILE)
    }

    fn normalize(&mut self) {
        self.pet.scale = self.pet.scale.clamp(SCALE_MIN, SCALE_MAX);
        if self.pet.name.trim().is_empty() {
            self.pet.name = default_pet_name();
        }
        if self.pet.prompt.trim().is_empty() {
            self.pet.prompt = default_pet_prompt();
        }
        if self.llm.base_url.trim().is_empty() {
            self.llm.base_url = default_llm_base_url();
        }
        if self.llm.model.trim().is_empty() {
            self.llm.model = default_llm_model();
        }

        if self.asr.provider.trim().is_empty() {
            self.asr.provider = default_asr_provider();
        }

        if self.asr.provider != "system" {
            self.asr.provider = default_asr_provider();
        }
    }

    fn load_legacy_window_scale() -> Option<f64> {
        let path = config_dir().join(LEGACY_WINDOW_FILE);
        let content = fs::read_to_string(path).ok()?;
        let legacy = serde_json::from_str::<LegacyWindowConfig>(&content).ok()?;
        Some(legacy.scale.clamp(SCALE_MIN, SCALE_MAX))
    }
}

fn config_dir() -> PathBuf {
    if let Some(dir) = dirs::config_dir() {
        return dir.join(APP_DIR);
    }
    PathBuf::from(".").join(APP_DIR)
}

pub fn load_config() -> Result<AppConfig, String> {
    AppConfig::load()
}

pub fn save_config(config: AppConfig) -> Result<AppConfig, String> {
    let normalized = config.normalized();
    normalized.save()?;
    Ok(normalized)
}
