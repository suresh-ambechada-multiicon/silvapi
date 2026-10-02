use crate::model::DownloadConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};

const DEFAULT_CONFIG: &str = include_str!("../resources/default-config.json");
const DESIGN_TOKENS: &str = include_str!("../resources/design-tokens.json");

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub download: DownloadConfig,
    pub video_containers: Vec<String>,
    pub audio_formats: Vec<String>,
    pub resolutions: Vec<u16>,
    pub worker_options: Vec<usize>,
    pub fragment_options: Vec<usize>,
    pub retry_options: Vec<usize>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DesignTokens {
    pub window_width: f32,
    pub window_height: f32,
    pub content_max_width: f32,
    pub page_padding: f32,
    pub section_padding: f32,
    pub control_gap: f32,
    pub section_gap: f32,
    pub corner_radius: f32,
    pub title_size: f32,
    pub body_size: f32,
    pub caption_size: f32,
    pub log_height: f32,
}

impl AppConfig {
    pub fn defaults() -> Result<Self> { serde_json::from_str(DEFAULT_CONFIG).context("invalid built-in application config") }
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() { return Self::defaults(); }
        serde_json::from_slice(&fs::read(path).with_context(|| format!("read {}", path.display()))?)
            .with_context(|| format!("parse {}", path.display()))
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
        fs::write(path, serde_json::to_vec_pretty(self)?).with_context(|| format!("write {}", path.display()))
    }
    pub fn user_path() -> PathBuf {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|v| PathBuf::from(v).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("silvadl").join("config.json")
    }
}

impl DesignTokens {
    pub fn load() -> Result<Self> { serde_json::from_str(DESIGN_TOKENS).context("invalid built-in design tokens") }
}
