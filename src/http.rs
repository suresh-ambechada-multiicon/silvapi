use std::time::Instant;

use crate::models::{ApiRequest, AuthType, BodyType, HttpResponse, KeyValue, TimelineEvent};

pub struct HttpClient {
    client: reqwest::blocking::Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn execute<F, G>(
        &self,
        request: &ApiRequest,
        resolved_url: &str,
        mut on_response_start: F,
        mut on_chunk: G,
    ) -> Result<HttpResponse, String>
    where
        F: FnMut(HttpResponse),
        G: FnMut(&[u8]),
    {
        let start = Instant::now();
        let mut timeline = Vec::new();

        let get_time = || chrono::Local::now().format("%H:%M:%S.%3f").to_string();

        timeline.push(crate::models::TimelineEvent {
            name: "validate_certificates = true".to_string(),
            timestamp: get_time(),
            icon: crate::models::TimelineIcon::Setting,
            detail: None,
        });
        timeline.push(crate::models::TimelineEvent {
            name: "redirects = true".to_string(),
            timestamp: get_time(),
            icon: crate::models::TimelineIcon::Setting,
            detail: None,
        });
        timeline.push(crate::models::TimelineEvent {
            name: format!("{} {}", request.method.as_str(), resolved_url),
            timestamp: get_time(),
            icon: crate::models::TimelineIcon::Request,
            detail: None,
        });

        let method = match request.method.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "PATCH" => reqwest::Method::PATCH,
            "DELETE" => reqwest::Method::DELETE,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => reqwest::Method::GET,
        };

        let mut final_url = crate::path_params::normalize_path_params(resolved_url);
        let mut query_params: Vec<(&str, &str)> = Vec::new();

        for p in &request.params {
            if !p.enabled || p.key.is_empty() {
                continue;
            }

            let (next_url, replaced_path_param) =
                crate::path_params::replace_path_param(&final_url, &p.key, &p.value);
            if replaced_path_param {
                final_url = next_url;
            } else {
                query_params.push((p.key.as_str(), p.value.as_str()));
            }
        }

        let mut builder = self.client.request(method, &final_url);

        if !query_params.is_empty() {
            builder = builder.query(&query_params);
        }

        // Headers
        for header in &request.headers {
            if header.enabled && !header.key.is_empty() {
                timeline.push(crate::models::TimelineEvent {
                    name: format!("{}: {}", header.key, header.value),
                    timestamp: get_time(),
                    icon: crate::models::TimelineIcon::Request,
                    detail: Some(format!("Header\n{}\nValue\n{}", header.key, header.value)),
                });
                builder = builder.header(&header.key, &header.value);
            }
        }

        // Auth
        match &request.auth.auth_type {
            AuthType::Bearer => {
                builder = builder.bearer_auth(&request.auth.bearer_token);
            }
            AuthType::Basic => {
                builder = builder.basic_auth(
                    &request.auth.basic_username,
                    Some(&request.auth.basic_password),
                );
            }
            AuthType::ApiKey => {
                if request.auth.api_key_in_header {
                    builder =
                        builder.header(&request.auth.api_key_name, &request.auth.api_key_value);
                }
            }
            AuthType::None => {}
        }

        // Body
        match &request.body.body_type {
            BodyType::Json => {
                timeline.push(crate::models::TimelineEvent {
                    name: "content-type: application/json".to_string(),
                    timestamp: get_time(),
                    icon: crate::models::TimelineIcon::Request,
                    detail: None,
                });
                builder = builder
                    .header("Content-Type", "application/json")
                    .body(request.body.content.clone());
            }
            BodyType::Raw => {
                builder = builder.body(request.body.content.clone());
            }
            BodyType::UrlEncoded => {
                timeline.push(crate::models::TimelineEvent {
                    name: "content-type: application/x-www-form-urlencoded".to_string(),
                    timestamp: get_time(),
                    icon: crate::models::TimelineIcon::Request,
                    detail: None,
                });
                builder = builder
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(request.body.content.clone());
            }
            _ => {}
        }

        timeline.push(crate::models::TimelineEvent {
            name: "Sending request to server".into(),
            timestamp: get_time(),
            icon: crate::models::TimelineIcon::Info,
            detail: None,
        });

        let mut response = builder.send().map_err(|e| e.to_string())?;

        let status = response.status();
        let status_code = status.as_u16();
        let status_text = status.canonical_reason().unwrap_or("").to_string();

        timeline.push(crate::models::TimelineEvent {
            name: format!("HTTP/1.1 {} {}", status_code, status_text),
            timestamp: get_time(),
            icon: crate::models::TimelineIcon::Response,
            detail: None,
        });

        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| {
                let key = k.to_string();
                let val = v.to_str().unwrap_or("").to_string();
                timeline.push(crate::models::TimelineEvent {
                    name: format!("{}: {}", key, val),
                    timestamp: get_time(),
                    icon: crate::models::TimelineIcon::Response,
                    detail: Some(format!("Header\n{}\nValue\n{}", key, val)),
                });
                (key, val)
            })
            .collect();

        // Fire initial response start event
        on_response_start(HttpResponse {
            status: status_code,
            status_text: status_text.clone(),
            headers: headers.clone(),
            body: String::new(),
            time_ms: start.elapsed().as_millis() as u64,
            size_bytes: 0,
            timeline: timeline.clone(),
        });

        let mut body_bytes = Vec::new();
        use std::io::Read;
        let mut buf = [0; 8192];
        loop {
            match response.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    body_bytes.extend_from_slice(&buf[..n]);
                    on_chunk(&buf[..n]);
                }
                Err(e) => {
                    return Err(e.to_string());
                }
            }
        }

        let body = String::from_utf8_lossy(&body_bytes).to_string();
        let size_bytes = body_bytes.len();

        let total_ms = start.elapsed().as_millis() as u64;
        timeline.push(crate::models::TimelineEvent {
            name: "Response completed".into(),
            timestamp: get_time(),
            icon: crate::models::TimelineIcon::Response,
            detail: None,
        });

        Ok(HttpResponse {
            status: status_code,
            status_text,
            headers,
            body,
            time_ms: total_ms,
            size_bytes,
            timeline,
        })
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub fn build_url_with_params(base_url: &str, params: &[KeyValue]) -> String {
    let enabled: Vec<_> = params
        .iter()
        .filter(|p| p.enabled && !p.key.is_empty())
        .collect();
    if enabled.is_empty() {
        return base_url.to_string();
    }
    let query: Vec<String> = enabled
        .iter()
        .map(|p| format!("{}={}", urlencod(&p.key), urlencod(&p.value)))
        .collect();
    if base_url.contains('?') {
        format!("{}&{}", base_url, query.join("&"))
    } else {
        format!("{}?{}", base_url, query.join("&"))
    }
}

fn urlencod(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}
