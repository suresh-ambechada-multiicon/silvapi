use anyhow::Result;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, COOKIE, ORIGIN, REFERER, USER_AGENT};
use serde_json::json;

pub const DEFAULT_URL: &str = "https://claude.ai/api/organizations/d161e2d8-44da-4d0d-a388-f4920847a725/chat_conversations/2139e97c-12a2-4e44-ba2b-c9fe848d0b97/completion";

pub const DEFAULT_COOKIE: &str = "anthropic-device-id=6a335f5c-b5e1-478f-8d41-4a05cd093288; anthropic-consent-preferences=%7B%22analytics%22%3Afalse%2C%22marketing%22%3Afalse%7D; g_state={\"i_l\":0,\"i_ll\":1787906701037,\"i_e\":{\"enable_itp_optimization\":24},\"i_et\":1787906701037}; lastActiveOrg=d161e2d8-44da-4d0d-a388-f4920847a725; intercom-device-id-lupk8zyo=c2e75520-d1c2-43c9-baf3-64b069846ec7; sessionKey=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; sessionKeyLC=1791474452632; routingHint=sk-ant-rh-eyJ0eXAiOiAiSldUIiwgImFsZyI6ICJFUzI1NiIsICJraWQiOiAiN0MxcWFPRnhqdWxaUjRFQnNuNk1UeUZGNWdDV2JHbFpNVDR2RklrRFFpbyJ9.eyJzdWIiOiAiN2RiM2ZkMGMtNTU5MS00ODY1LThiMzItMDg1OTg3ZDc1NTdkIiwgInYiOiAyLCAiaXNfdmVyaWZpZWQiOiB0cnVlLCAiaGFzX21lbWJlcnNoaXAiOiB0cnVlLCAib25ib2FyZGluZ19jb21wbGV0ZSI6IHRydWUsICJhZ2VfdmVyaWZpZWQiOiB0cnVlLCAicGVuZGluZ19pbnZpdGUiOiBmYWxzZSwgInBob25lX3ZlcmlmaWVkIjogdHJ1ZSwgIm5hbWUiOiAic3VyZXNoIiwgImZsYWdzIjogWyJlZGdlX2RlZmF1bHRfcm91dGUiXSwgImlhdCI6IDE3OTE0NzQ0NTIsICJpc3MiOiAiY2xhdWRlLWFpLXJvdXRpbmcifQ.vtFpwMnu2HsIM8Q5y00Mg2U4PwjZzSgn7mSZoowkXPl2GZxx1NUWdzfxnYX1KGNxWNFFUM-ccztVGpFHmIxobQ; artifact-app-viewer=1; sessionKeyV3=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; sessionKeyV3LC=1791551365411; CH-prefers-color-scheme=dark; activitySessionId=a0c593cb-7356-4785-9096-cda884d36750; _cfuvid=QXG5adnvFRwiXWKFdr58D5U4YJ3U4nTK0s1jFmrcCgI-1791791907.44955-1.0.1.1-OF4776ZkvSYb68DDXpv47HzIDbbHQrc6lavPktwiyuM; user-sidebar-visible-on-load=true; ion-vk=75c8b176-f024-4397-b898-40c62baa46cd; cf_clearance=3gK4zY5y4EG_7CGF2GGug6Q9mIe4ZjyZ5XliE.8Rl0I-1791797926-1.2.1.1-FNr.nLOj3NwxCXTfAbI2oeE15uOab4zlqVskmsWXfPSLAa4KWdxPFjXFuVxjOs62R4NAOxe2aWleCu6WKZeuei9GQlSsyL3D2vImHGVM3w2RQxONDxujG_EUTefYDi0takeA18pLetffU1v.78rfsxAU9cDQn1DGBKtBhKIwfKhOOWDurqlL2aeEBcPRYfhlEqtX.QguqA2Ti.Feda0Vx.DvbzSM_esSrRRJxspnP3Fnjh2d3n3UhxhWBg9HEsEmZSDonwoqL24W5RfEDk6L.djnkhPT9P5ntXIguCjrTi4StiqeWZa4W_.27IMgGB8o1HcQMRq6QKKFRnlstGgIsdfnZRqgGRdxRhJJGQsc3ERPd2OuJZuKlsBQFzl6qL73Voy3FamjSWssuMomKPikUQtwFJfubUO_JWb35ZtIuzIptii56FYzpvv6KHMgVjfC; __cf_bm=5RxTcIBdTC6U3l.ohOCGdOot.Udj7rLSdvAKWTJQSMA-1791797926.501014-1.0.1.1-lI_ETr3i7HqfMU4gJHpO1Pr_mnBtvbRf9iKW4dy9aZD98gLvcSRRaTR0BjO6U4xEB5eM482xhaEHFCBZRPf7brhpTjB7YL.zfTYjymyAQXJy8IR85SYmUS_HgpiSNhAJ";

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
    mut on_event: F,
) -> Result<()>
where
    F: FnMut(ClaudeStreamEvent) + Send + 'static,
{
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
    headers.insert(
        "anthropic-device-id",
        HeaderValue::from_static("6a335f5c-b5e1-478f-8d41-4a05cd093288"),
    );
    headers.insert(ORIGIN, HeaderValue::from_static("https://claude.ai"));
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://claude.ai/chat/2139e97c-12a2-4e44-ba2b-c9fe848d0b97"),
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/155.0.0.0 Safari/537.36"),
    );

    if let Ok(cookie_val) = HeaderValue::from_str(DEFAULT_COOKIE) {
        headers.insert(COOKIE, cookie_val);
    }

    let parent_id = parent_uuid.unwrap_or("01a12f7c-e7bf-7230-b76b-f26e6a0e0b7c");
    let human_uuid = uuid::Uuid::now_v7().to_string();
    let assistant_uuid = uuid::Uuid::now_v7().to_string();
    let completion_id = uuid::Uuid::new_v4().to_string();

    let body = json!({
        "prompt": prompt,
        "parent_message_uuid": parent_id,
        "timezone": "Asia/Calcutta",
        "locale": "en-US",
        "model": "claude-haiku-5-5",
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
        .post(DEFAULT_URL)
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
