use std::sync::OnceLock;
use instant::Instant;

use silvapi_core::models::{
    ApiRequest, AuthType, BodyType, FormDataPartKind, HttpResponse, KeyValue, RequestSnapshot,
};

/// A single shared async `reqwest::Client` so connection pools / DNS caches are
/// reused across requests instead of rebuilt each time.
fn shared_client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| reqwest::Client::builder().build().unwrap_or_default())
        .clone()
}

pub struct HttpClient {
    client: reqwest::Client,
}

/// Categorize a reqwest error into a concise, human-readable message with
/// the underlying cause detail appended.
fn describe_reqwest_error(e: &reqwest::Error) -> String {
    use std::error::Error;

    let is_connect = {
        #[cfg(not(target_arch = "wasm32"))]
        { e.is_connect() }
        #[cfg(target_arch = "wasm32")]
        { false }
    };

    let category = if e.is_timeout() {
        "Request timed out"
    } else if is_connect {
        "Connection failed"
    } else if e.is_redirect() {
        "Too many redirects"
    } else if e.is_body() || e.is_decode() {
        "Failed to read response body"
    } else if e.is_request() {
        "Invalid request"
    } else {
        "Request failed"
    };

    // Walk the source chain to find the deepest (most specific) cause, since
    // reqwest wraps hyper/TLS/DNS errors.
    let mut detail: Option<String> = None;
    let mut src = e.source();
    while let Some(s) = src {
        detail = Some(s.to_string());
        src = s.source();
    }

    match detail {
        Some(d) => format!("{}: {}", category, d),
        None => category.to_string(),
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: shared_client(),
        }
    }

    /// Execute a request asynchronously on the shared runtime. The body is
    /// streamed chunk-by-chunk (never fully buffered by us before the caller
    /// sees it); `on_chunk` receives each chunk as it arrives. The task can be
    /// aborted mid-flight for true cancellation.
    ///
    /// `on_request_prepared` fires once the outgoing request is fully assembled
    /// and before it is sent, handing back the snapshot that history stores. It
    /// is reported separately so a request that fails in transit still has its
    /// exact wire form recorded.
    pub async fn execute<F, G, H>(
        &self,
        request: &ApiRequest,
        resolved_url: &str,
        mut on_request_prepared: H,
        mut on_response_start: F,
        mut on_chunk: G,
    ) -> Result<HttpResponse, String>
    where
        F: FnMut(HttpResponse),
        G: FnMut(&[u8]),
        H: FnMut(RequestSnapshot),
    {
        let start = Instant::now();
        let mut timeline = Vec::new();

        let get_time = || chrono::Local::now().format("%H:%M:%S.%3f").to_string();

        timeline.push(silvapi_core::models::TimelineEvent {
            name: "validate_certificates = true".to_string(),
            timestamp: get_time(),
            icon: silvapi_core::models::TimelineIcon::Setting,
            detail: None,
        });
        timeline.push(silvapi_core::models::TimelineEvent {
            name: "redirects = true".to_string(),
            timestamp: get_time(),
            icon: silvapi_core::models::TimelineIcon::Setting,
            detail: None,
        });
        timeline.push(silvapi_core::models::TimelineEvent {
            name: format!("{} {}", request.method.as_str(), resolved_url),
            timestamp: get_time(),
            icon: silvapi_core::models::TimelineIcon::Request,
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

        let mut final_url = silvapi_core::path_params::normalize_path_params(resolved_url);
        let mut query_params: Vec<(&str, &str)> = Vec::new();

        for p in &request.params {
            if !p.enabled || p.key.is_empty() {
                continue;
            }

            let (next_url, replaced_path_param) =
                silvapi_core::path_params::replace_path_param(&final_url, &p.key, &p.value);
            if replaced_path_param {
                final_url = next_url;
            } else {
                let (next_url, replaced_query_value_param) =
                    replace_query_value_param(&final_url, &p.key, &p.value);
                if replaced_query_value_param {
                    final_url = next_url;
                    continue;
                }

                let (next_url, replaced_query_param) =
                    replace_query_param(&final_url, &p.key, &p.value);
                if replaced_query_param {
                    final_url = next_url;
                } else {
                    query_params.push((p.key.as_str(), p.value.as_str()));
                }
            }
        }

        let mut builder = self.client.request(method, &final_url);

        if !query_params.is_empty() {
            builder = builder.query(&query_params);
        }

        // Headers
        for header in &request.headers {
            if header.enabled && !header.key.is_empty() {
                timeline.push(silvapi_core::models::TimelineEvent {
                    name: format!("{}: {}", header.key, header.value),
                    timestamp: get_time(),
                    icon: silvapi_core::models::TimelineIcon::Request,
                    detail: Some(format!("Header\n{}\nValue\n{}", header.key, header.value)),
                });
                builder = builder.header(&header.key, &header.value);
            }
        }

        // Auth
        if request.auth.enabled {
            match &request.auth.auth_type {
                AuthType::Bearer => {
                    let prefix = request.auth.bearer_prefix.trim();
                    if prefix.is_empty() || prefix.eq_ignore_ascii_case("bearer") {
                        builder = builder.bearer_auth(&request.auth.bearer_token);
                    } else if !request.auth.bearer_token.is_empty() {
                        builder = builder.header(
                            "Authorization",
                            format!("{} {}", prefix, request.auth.bearer_token),
                        );
                    }
                }
                AuthType::Basic => {
                    builder = builder.basic_auth(
                        &request.auth.basic_username,
                        Some(&request.auth.basic_password),
                    );
                }
                AuthType::ApiKey => {
                    if !request.auth.api_key_name.is_empty() {
                        if request.auth.api_key_in_header {
                            builder = builder
                                .header(&request.auth.api_key_name, &request.auth.api_key_value);
                        } else {
                            builder = builder.query(&[(
                                request.auth.api_key_name.as_str(),
                                request.auth.api_key_value.as_str(),
                            )]);
                        }
                    }
                }
                AuthType::AwsV4 | AuthType::Jwt | AuthType::OAuth2 | AuthType::None => {}
            }
        }

        // Body. Assembled into `wire_body` first so the history snapshot can
        // record exactly what was sent, then moved into the builder.
        let mut wire_body: Option<Vec<u8>> = None;
        let mut content_type: Option<String> = None;

        match &request.body.body_type {
            BodyType::Json => {
                content_type = Some("application/json".to_string());
                wire_body = Some(request.body.content.clone().into_bytes());
                timeline.push(silvapi_core::models::TimelineEvent {
                    name: "content-type: application/json".to_string(),
                    timestamp: get_time(),
                    icon: silvapi_core::models::TimelineIcon::Request,
                    detail: None,
                });
            }
            BodyType::Raw => {
                wire_body = Some(request.body.content.clone().into_bytes());
            }
            BodyType::FormData => {
                let (boundary, body, part_count) = build_multipart_body(request)?;
                content_type = Some(format!("multipart/form-data; boundary={boundary}"));
                timeline.push(silvapi_core::models::TimelineEvent {
                    name: format!("multipart/form-data: {} parts", part_count),
                    timestamp: get_time(),
                    icon: silvapi_core::models::TimelineIcon::Request,
                    detail: None,
                });
                wire_body = Some(body);
            }
            BodyType::UrlEncoded => {
                let body = if request.body.urlencoded.is_empty() {
                    request.body.content.clone()
                } else {
                    build_urlencoded_body(&request.body.urlencoded)
                };
                content_type = Some("application/x-www-form-urlencoded".to_string());
                timeline.push(silvapi_core::models::TimelineEvent {
                    name: "content-type: application/x-www-form-urlencoded".to_string(),
                    timestamp: get_time(),
                    icon: silvapi_core::models::TimelineIcon::Request,
                    detail: None,
                });
                wire_body = Some(body.into_bytes());
            }
            BodyType::BinaryFile => {
                if !request.body.content.is_empty() {
                    let bytes = std::fs::read(&request.body.content).map_err(|e| e.to_string())?;
                    content_type = Some("application/octet-stream".to_string());
                    timeline.push(silvapi_core::models::TimelineEvent {
                        name: format!("file body: {}", request.body.content),
                        timestamp: get_time(),
                        icon: silvapi_core::models::TimelineIcon::Request,
                        detail: None,
                    });
                    wire_body = Some(bytes);
                }
            }
            BodyType::None => {}
        }

        // The URL as reqwest will actually request it, query params included.
        let sent_url = if query_params.is_empty() {
            final_url.clone()
        } else {
            reqwest::Url::parse_with_params(&final_url, &query_params)
                .map(|url| url.to_string())
                .unwrap_or_else(|_| final_url.clone())
        };

        let mut snapshot = RequestSnapshot::from_request(request, sent_url);
        snapshot.set_wire_body(
            content_type.clone(),
            wire_body.as_deref().unwrap_or_default(),
        );
        on_request_prepared(snapshot.clone());

        if let Some(content_type) = content_type {
            builder = builder.header("Content-Type", content_type);
        }
        if let Some(body) = wire_body {
            builder = builder.body(body);
        }

        timeline.push(silvapi_core::models::TimelineEvent {
            name: "Sending request to server".into(),
            timestamp: get_time(),
            icon: silvapi_core::models::TimelineIcon::Info,
            detail: None,
        });

        let response = builder
            .send()
            .await
            .map_err(|e| describe_reqwest_error(&e))?;

        let status = response.status();
        let status_code = status.as_u16();
        let status_text = status.canonical_reason().unwrap_or("").to_string();

        timeline.push(silvapi_core::models::TimelineEvent {
            name: format!("HTTP/1.1 {} {}", status_code, status_text),
            timestamp: get_time(),
            icon: silvapi_core::models::TimelineIcon::Response,
            detail: None,
        });

        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| {
                let key = k.to_string();
                let val = v.to_str().unwrap_or("").to_string();
                timeline.push(silvapi_core::models::TimelineEvent {
                    name: format!("{}: {}", key, val),
                    timestamp: get_time(),
                    icon: silvapi_core::models::TimelineIcon::Response,
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
            body_base64: None,
            time_ms: start.elapsed().as_millis() as u64,
            size_bytes: 0,
            timeline: timeline.clone(),
            request: Some(snapshot.clone()),
            started_at: None,
            error: None,
            formatted_body: None,
        });

        let mut body_bytes = Vec::new();
        use futures::StreamExt;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    body_bytes.extend_from_slice(&bytes);
                    on_chunk(&bytes);
                }
                Err(e) => {
                    return Err(format!("Failed to read response body: {}", e));
                }
            }
        }

        let size_bytes = body_bytes.len();
        let body = if is_textual_response(&headers) {
            String::from_utf8_lossy(&body_bytes).to_string()
        } else {
            format!(
                "[Binary response: {} · {}]",
                response_content_type(&headers).unwrap_or("application/octet-stream"),
                format_response_bytes(size_bytes)
            )
        };

        let total_ms = start.elapsed().as_millis() as u64;
        timeline.push(silvapi_core::models::TimelineEvent {
            name: "Response completed".into(),
            timestamp: get_time(),
            icon: silvapi_core::models::TimelineIcon::Response,
            detail: None,
        });

        Ok(HttpResponse {
            status: status_code,
            status_text,
            headers,
            body,
            body_base64: None,
            time_ms: total_ms,
            size_bytes,
            timeline,
            request: Some(snapshot),
            started_at: None,
            error: None,
            formatted_body: None,
        }
        .with_body_bytes(&body_bytes))
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

fn response_content_type(headers: &[(String, String)]) -> Option<&str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.split(';').next().unwrap_or(value.as_str()).trim())
}

fn is_textual_response(headers: &[(String, String)]) -> bool {
    let Some(content_type) = response_content_type(headers).map(str::to_ascii_lowercase) else {
        return true;
    };

    content_type.starts_with("text/")
        || content_type.contains("json")
        || content_type.contains("xml")
        || content_type.contains("javascript")
        || content_type.contains("x-www-form-urlencoded")
        || content_type.contains("graphql")
        || content_type.contains("event-stream")
}

fn format_response_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn build_multipart_body(request: &ApiRequest) -> Result<(String, Vec<u8>, usize), String> {
    let boundary = format!("silvapi-{}", uuid::Uuid::new_v4());
    let mut body = Vec::new();
    let mut part_count = 0usize;

    for part in &request.body.form_data {
        if !part.enabled || part.name.is_empty() {
            continue;
        }

        match part.kind {
            FormDataPartKind::Text => {
                push_multipart_part_header(
                    &mut body,
                    &boundary,
                    &part.name,
                    None,
                    (!part.content_type.is_empty()).then_some(part.content_type.as_str()),
                );
                body.extend_from_slice(part.value.as_bytes());
                body.extend_from_slice(b"\r\n");
                part_count += 1;
            }
            FormDataPartKind::File => {
                if part.value.is_empty() {
                    continue;
                }
                let path = std::path::Path::new(&part.value);
                let filename = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "file".to_string());
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                push_multipart_part_header(
                    &mut body,
                    &boundary,
                    &part.name,
                    Some(&filename),
                    Some(if part.content_type.is_empty() {
                        "application/octet-stream"
                    } else {
                        part.content_type.as_str()
                    }),
                );
                body.extend_from_slice(&bytes);
                body.extend_from_slice(b"\r\n");
                part_count += 1;
            }
        }
    }

    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
    Ok((boundary, body, part_count))
}

fn push_multipart_part_header(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    filename: Option<&str>,
    content_type: Option<&str>,
) {
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"",
            escape_multipart_header_value(name)
        )
        .as_bytes(),
    );
    if let Some(filename) = filename {
        body.extend_from_slice(
            format!("; filename=\"{}\"", escape_multipart_header_value(filename)).as_bytes(),
        );
    }
    body.extend_from_slice(b"\r\n");
    if let Some(content_type) = content_type {
        body.extend_from_slice(format!("Content-Type: {}\r\n", content_type).as_bytes());
    }
    body.extend_from_slice(b"\r\n");
}

fn escape_multipart_header_value(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch != '\r' && *ch != '\n')
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}

#[allow(dead_code)]
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

fn replace_query_param(url: &str, key: &str, value: &str) -> (String, bool) {
    let (without_fragment, fragment) = match url.split_once('#') {
        Some((base, fragment)) => (base, Some(fragment)),
        None => (url, None),
    };

    let Some((base, query)) = without_fragment.split_once('?') else {
        return (url.to_string(), false);
    };

    let encoded_key = urlencod(key);
    let encoded_value = urlencod(value);
    let mut changed = false;
    let mut pairs = Vec::new();

    for pair in query.split('&') {
        let (pair_key, _) = pair.split_once('=').unwrap_or((pair, ""));
        if pair_key == key || pair_key == encoded_key {
            pairs.push(format!("{encoded_key}={encoded_value}"));
            changed = true;
        } else {
            pairs.push(pair.to_string());
        }
    }

    if !changed {
        return (url.to_string(), false);
    }

    let mut out = format!("{base}?{}", pairs.join("&"));
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }

    (out, true)
}

fn replace_query_value_param(url: &str, key: &str, value: &str) -> (String, bool) {
    let (without_fragment, fragment) = match url.split_once('#') {
        Some((base, fragment)) => (base, Some(fragment)),
        None => (url, None),
    };

    let Some((base, query)) = without_fragment.split_once('?') else {
        return (url.to_string(), false);
    };

    let pattern = format!(":{key}");
    let encoded_value = urlencod(value);
    let mut changed = false;
    let mut pairs = Vec::new();

    for pair in query.split('&') {
        let Some((pair_key, pair_value)) = pair.split_once('=') else {
            pairs.push(pair.to_string());
            continue;
        };
        if pair_value == pattern {
            pairs.push(format!("{pair_key}={encoded_value}"));
            changed = true;
        } else {
            pairs.push(pair.to_string());
        }
    }

    if !changed {
        return (url.to_string(), false);
    }

    let mut out = format!("{base}?{}", pairs.join("&"));
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }

    (out, true)
}

pub use silvapi_core::models::build_urlencoded_body;
use silvapi_core::models::urlencod;

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
