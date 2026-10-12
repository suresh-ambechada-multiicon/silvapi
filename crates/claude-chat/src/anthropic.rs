use anyhow::Result;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, COOKIE, ORIGIN, REFERER, USER_AGENT};
use serde_json::json;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ClaudeConfig {
    pub cookie: String,
    pub org_id: String,
    pub conv_id: String,
    pub device_id: String,
    pub model: String,
}

impl ClaudeConfig {
    pub fn is_configured(&self) -> bool {
        !self.cookie.trim().is_empty() && !self.org_id.trim().is_empty() && !self.conv_id.trim().is_empty()
    }

    pub fn completion_url(&self) -> String {
        format!(
            "https://claude.ai/api/organizations/{}/chat_conversations/{}/completion",
            self.org_id.trim(),
            self.conv_id.trim()
        )
    }

    pub fn referer_url(&self) -> String {
        format!("https://claude.ai/chat/{}", self.conv_id.trim())
    }
}

#[derive(Clone, Debug, Default)]
pub struct UsageLimitInfo {
    pub five_hour_utilization: Option<f64>,
    pub five_hour_resets_at: Option<i64>,
    pub seven_day_utilization: Option<f64>,
    pub raw_json_summary: String,
}

pub enum ClaudeStreamEvent {
    Delta(String),
    Parent(String),
    Think(String),
    ToolStart(String),
    ToolInput(String),
    ToolResult(String),
    MessageLimit(UsageLimitInfo),
    Done,
    Error(String),
}

pub async fn send_prompt_stream<F>(
    prompt: &str,
    parent_uuid: Option<&str>,
    config: &ClaudeConfig,
    mut on_event: F,
) -> Result<()>
where
    F: FnMut(ClaudeStreamEvent) + Send + 'static,
{
    if !config.is_configured() {
        on_event(ClaudeStreamEvent::Error(
            "Please configure your Claude credentials (Cookie, Organization ID, Conversation ID) in [config] at the top left.".into(),
        ));
        on_event(ClaudeStreamEvent::Done);
        return Ok(());
    }

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        "anthropic-client-platform",
        HeaderValue::from_static("web_claude_ai"),
    );
    headers.insert(
        "anthropic-client-sha",
        HeaderValue::from_static("d40c622ef07c4d318d1504ae40fabc3faa8a85fa"),
    );
    if let Ok(dev_val) = HeaderValue::from_str(config.device_id.trim()) {
        headers.insert("anthropic-device-id", dev_val);
    }
    headers.insert(ORIGIN, HeaderValue::from_static("https://claude.ai"));
    if let Ok(ref_val) = HeaderValue::from_str(&config.referer_url()) {
        headers.insert(REFERER, ref_val);
    }
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/155.0.0.0 Safari/537.36"),
    );

    if let Ok(cookie_val) = HeaderValue::from_str(config.cookie.trim()) {
        headers.insert(COOKIE, cookie_val);
    }

    let default_parent = uuid::Uuid::now_v7().to_string();
    let parent_id = parent_uuid.unwrap_or(&default_parent);
    let human_uuid = uuid::Uuid::now_v7().to_string();
    let assistant_uuid = uuid::Uuid::now_v7().to_string();
    let completion_id = uuid::Uuid::new_v4().to_string();

    let model = if config.model.trim().is_empty() {
        "claude-haiku-5-5"
    } else {
        config.model.trim()
    };

    let body = json!({
        "prompt": prompt,
        "parent_message_uuid": parent_id,
        "timezone": "Asia/Calcutta",
        "locale": "en-US",
        "model": model,
        "effort": "medium",
        "thinking_mode": "auto",
        "tools": [
            {
                "type": "web_search_v0",
                "name": "web_search"
            }
        ],
        "turn_message_uuids": {
            "human_message_uuid": human_uuid,
            "assistant_message_uuid": assistant_uuid
        },
        "attachments": [],
        "files": [],
        "sync_sources": [],
        "completion_request_id": completion_id,
        "rendering_mode": "messages"
    });

    let res = client
        .post(&config.completion_url())
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        let err_text = res.text().await.unwrap_or_default();
        on_event(ClaudeStreamEvent::Error(format!("HTTP {}: {}", status, err_text)));
        return Ok(());
    }

    let mut stream = res.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_res) = stream.next().await {
        let chunk = match chunk_res {
            Ok(c) => c,
            Err(e) => {
                on_event(ClaudeStreamEvent::Error(e.to_string()));
                break;
            }
        };

        let chunk_str = String::from_utf8_lossy(&chunk);
        buffer.push_str(&chunk_str);

        // Normalize CRLF to LF
        let normalized = buffer.replace("\r\n", "\n").replace('\r', "\n");
        let mut parts: Vec<&str> = normalized.split("\n\n").collect();
        let remainder = parts.pop().unwrap_or("").to_string();

        for block in parts {
            for line in block.lines() {
                let trimmed = line.trim();
                let payload = trimmed
                    .strip_prefix("data:")
                    .map(str::trim)
                    .or_else(|| trimmed.starts_with('{').then_some(trimmed));
                if let Some(payload) = payload {
                    if payload == "[DONE]" {
                        on_event(ClaudeStreamEvent::Done);
                        continue;
                    }
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                        // Check for message_limit message
                        if v.get("type").and_then(|t| t.as_str()) == Some("message_limit") {
                            let mut info = UsageLimitInfo::default();
                            if let Some(limit_obj) = v.get("message_limit") {
                                if let Some(windows) = limit_obj.get("windows") {
                                    if let Some(five_h) = windows.get("5h") {
                                        info.five_hour_utilization = five_h.get("utilization").and_then(|u| u.as_f64());
                                        info.five_hour_resets_at = five_h.get("resets_at").and_then(|r| r.as_i64());
                                    }
                                    if let Some(seven_d) = windows.get("7d") {
                                        info.seven_day_utilization = seven_d.get("utilization").and_then(|u| u.as_f64());
                                    }
                                }
                            }
                            info.raw_json_summary = payload.to_string();
                            on_event(ClaudeStreamEvent::MessageLimit(info));
                        }

                        match v.get("type").and_then(|t| t.as_str()) {
                            Some("message_start") => {
                                if let Some(id) = v.pointer("/message/uuid").and_then(|t| t.as_str()) {
                                    on_event(ClaudeStreamEvent::Parent(id.to_string()));
                                }
                            }
                            Some("content_block_start") => {
                                let kind = v.pointer("/content_block/type").and_then(|t| t.as_str());
                                if kind == Some("tool_use") {
                                    let name = v.pointer("/content_block/name").and_then(|t| t.as_str()).unwrap_or("tool");
                                    on_event(ClaudeStreamEvent::ToolStart(name.to_string()));
                                } else if kind == Some("tool_result") {
                                    let mut out = String::new();
                                    if let Some(items) = v.pointer("/content_block/content").and_then(|t| t.as_array()) {
                                        for it in items {
                                            if let Some(t) = it.get("text").and_then(|t| t.as_str()) {
                                                out.push_str(t);
                                                out.push('\n');
                                            } else if let Some(t) = it.get("title").and_then(|t| t.as_str()) {
                                                let url = it.get("url").and_then(|t| t.as_str()).unwrap_or("");
                                                out.push_str(&format!("- {t} {url}\n"));
                                            }
                                        }
                                    }
                                    if out.is_empty() {
                                        if let Some(items) = v.pointer("/content_block/display_content/content").and_then(|t| t.as_array()) {
                                            for it in items {
                                                if let Some(t) = it.get("title").and_then(|t| t.as_str()) {
                                                    out.push_str(&format!("- {t}\n"));
                                                }
                                            }
                                        }
                                    }
                                    on_event(ClaudeStreamEvent::ToolResult(out));
                                }
                            }
                            Some("content_block_delta") => {
                                if let Some(t) = v.pointer("/delta/text").and_then(|t| t.as_str()) {
                                    on_event(ClaudeStreamEvent::Delta(t.to_string()));
                                } else if let Some(t) = v.pointer("/delta/thinking").and_then(|t| t.as_str()) {
                                    on_event(ClaudeStreamEvent::Think(t.to_string()));
                                } else if let Some(t) = v.pointer("/delta/summary/summary").and_then(|t| t.as_str()) {
                                    on_event(ClaudeStreamEvent::Think(format!("{t}\n\n")));
                                } else if let Some(t) = v.pointer("/delta/partial_json").and_then(|t| t.as_str()) {
                                    on_event(ClaudeStreamEvent::ToolInput(t.to_string()));
                                }
                            }
                            Some("error") => {
                                let msg = v.pointer("/error/message").and_then(|t| t.as_str()).unwrap_or("error");
                                on_event(ClaudeStreamEvent::Error(msg.to_string()));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        buffer = remainder;
    }

    on_event(ClaudeStreamEvent::Done);
    Ok(())
}
