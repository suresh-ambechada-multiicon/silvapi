//! `localStorage`-backed persistence for the web build.
//!
//! Mirrors the SQLite backend's API exactly. Differences worth knowing:
//!
//! - `localStorage` is per-origin and typically capped around 5 MB, so response
//!   history is trimmed harder than on the desktop.
//! - Values are JSON rather than rows, so there is no query layer; history is
//!   stored as one array per request.
//! - Every call is synchronous, matching the native signatures.

use anyhow::{Context as _, Result, anyhow};
use silvapi_core::models::{HttpResponse, Workspace};
use std::collections::HashMap;

const WORKSPACE_KEY: &str = "silvapi.workspace";
const SETTING_PREFIX: &str = "silvapi.setting.";
const HISTORY_PREFIX: &str = "silvapi.history.";

/// Kept lower than the desktop's 20: `localStorage` is a few megabytes for the
/// whole origin, and response bodies are the largest thing we store.
const MAX_RESPONSE_HISTORY_PER_REQUEST: usize = 10;

fn store() -> Result<web_sys::Storage> {
    web_sys::window()
        .ok_or_else(|| anyhow!("no browser window"))?
        .local_storage()
        .map_err(|_| anyhow!("localStorage is blocked for this origin"))?
        .ok_or_else(|| anyhow!("localStorage is unavailable"))
}

fn get(key: &str) -> Result<Option<String>> {
    store()?
        .get_item(key)
        .map_err(|_| anyhow!("could not read {key} from localStorage"))
}

fn set(key: &str, value: &str) -> Result<()> {
    store()?
        .set_item(key, value)
        // Almost always the quota being exceeded, which is worth saying plainly.
        .map_err(|_| anyhow!("could not write {key}; localStorage may be full"))
}

pub fn load_workspace() -> Result<Option<Workspace>> {
    let Some(json) = get(WORKSPACE_KEY)? else {
        return Ok(None);
    };
    let mut workspace: Workspace =
        serde_json::from_str(&json).context("failed to parse the saved workspace")?;
    workspace.response_cache = load_response_cache()?;
    Ok(Some(workspace))
}

pub fn save_workspace(workspace: &Workspace) -> Result<()> {
    // History lives under its own keys, so it is not written twice.
    let mut trimmed = workspace.clone();
    trimmed.response_cache = Default::default();
    set(WORKSPACE_KEY, &serde_json::to_string(&trimmed)?)
}

fn load_response_cache() -> Result<HashMap<String, Vec<HttpResponse>>> {
    let store = store()?;
    let mut cache: HashMap<String, Vec<HttpResponse>> = HashMap::new();

    let length = store.length().unwrap_or(0);
    for index in 0..length {
        let Ok(Some(key)) = store.key(index) else {
            continue;
        };
        let Some(request_id) = key.strip_prefix(HISTORY_PREFIX) else {
            continue;
        };
        let Ok(Some(json)) = store.get_item(&key) else {
            continue;
        };
        // A single unreadable entry must not cost the user the rest of it.
        if let Ok(history) = serde_json::from_str::<Vec<HttpResponse>>(&json) {
            cache.insert(request_id.to_string(), history);
        }
    }
    Ok(cache)
}

pub fn append_response(request_id: &str, response: &HttpResponse) -> Result<()> {
    let key = format!("{HISTORY_PREFIX}{request_id}");
    let mut history: Vec<HttpResponse> = get(&key)?
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();

    history.push(response.clone());
    let overflow = history.len().saturating_sub(MAX_RESPONSE_HISTORY_PER_REQUEST);
    if overflow > 0 {
        history.drain(0..overflow);
    }
    set(&key, &serde_json::to_string(&history)?)
}

pub fn delete_response_history(request_id: &str) -> Result<()> {
    store()?
        .remove_item(&format!("{HISTORY_PREFIX}{request_id}"))
        .map_err(|_| anyhow!("could not clear history for {request_id}"))
}

pub fn load_theme_name() -> Result<Option<String>> {
    load_setting("theme")
}

pub fn save_theme_name(theme_name: &str) -> Result<()> {
    save_setting("theme", theme_name)
}

pub fn load_setting(key: &str) -> Result<Option<String>> {
    get(&format!("{SETTING_PREFIX}{key}"))
}

pub fn save_setting(key: &str, value: &str) -> Result<()> {
    set(&format!("{SETTING_PREFIX}{key}"), value)
}
