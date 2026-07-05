use crate::models::{ApiRequest, AuthType, BodyType, HttpMethod, KeyValue};

pub fn parse_curl(input: &str) -> Result<ApiRequest, String> {
    let input = input.trim();
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return Err("Empty input".into());
    }

    let mut req = ApiRequest::default();
    let mut i = 0;

    // Skip "curl"
    if tokens.get(0).map(|s| s.as_str()) == Some("curl") {
        i += 1;
    }

    while i < tokens.len() {
        let token = &tokens[i];
        match token.as_str() {
            "-X" | "--request" => {
                i += 1;
                if let Some(method) = tokens.get(i) {
                    req.method = parse_method(method);
                }
            }
            "-H" | "--header" => {
                i += 1;
                if let Some(header) = tokens.get(i) {
                    if let Some((k, v)) = header.split_once(':') {
                        let kv = KeyValue::new(k.trim(), v.trim());
                        if k.trim().to_lowercase() == "authorization" {
                            let v = v.trim();
                            if let Some(token) = v.strip_prefix("Bearer ") {
                                req.auth.auth_type = AuthType::Bearer;
                                req.auth.bearer_token = token.to_string();
                            }
                        } else {
                            req.headers.push(kv);
                        }
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                i += 1;
                if let Some(data) = tokens.get(i) {
                    let data = data.strip_prefix('@').unwrap_or(data);
                    req.body.content = data.to_string();
                    req.body.body_type = BodyType::Raw;
                    // detect json
                    let trimmed = data.trim();
                    if trimmed.starts_with('{') || trimmed.starts_with('[') {
                        req.body.body_type = BodyType::Json;
                        if req.method == HttpMethod::GET {
                            req.method = HttpMethod::POST;
                        }
                    }
                }
            }
            "--data-urlencode" => {
                i += 1;
                if let Some(data) = tokens.get(i) {
                    req.body.content = data.to_string();
                    req.body.body_type = BodyType::UrlEncoded;
                    if req.method == HttpMethod::GET {
                        req.method = HttpMethod::POST;
                    }
                }
            }
            "-u" | "--user" => {
                i += 1;
                if let Some(creds) = tokens.get(i) {
                    if let Some((user, pass)) = creds.split_once(':') {
                        req.auth.auth_type = AuthType::Basic;
                        req.auth.basic_username = user.to_string();
                        req.auth.basic_password = pass.to_string();
                    }
                }
            }
            "--url" => {
                i += 1;
                if let Some(url) = tokens.get(i) {
                    parse_url_into(&mut req, url);
                }
            }
            "-G" | "--get" => {
                req.method = HttpMethod::GET;
            }
            s if !s.starts_with('-') && req.url.is_empty() => {
                parse_url_into(&mut req, s);
            }
            _ => {}
        }
        i += 1;
    }

    if req.url.is_empty() {
        return Err("No URL found in curl command".into());
    }

    // Derive name from URL
    req.name = url_to_name(&req.url);

    Ok(req)
}

fn parse_url_into(req: &mut ApiRequest, url: &str) {
    if let Some((base, query)) = url.split_once('?') {
        req.url = base.to_string();
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                req.params.push(KeyValue::new(urldeced(k), urldeced(v)));
            }
        }
    } else {
        req.url = url.to_string();
    }
}

fn urldeced(s: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                out.push(hex as char);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn parse_method(s: &str) -> HttpMethod {
    match s.to_uppercase().as_str() {
        "GET" => HttpMethod::GET,
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "PATCH" => HttpMethod::PATCH,
        "DELETE" => HttpMethod::DELETE,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    }
}

fn url_to_name(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('?')
        .next()
        .unwrap_or(url)
        .to_string()
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '\\' if in_double => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            ' ' | '\t' | '\n' | '\r' if !in_single && !in_double => {
                let tok = current.trim().to_string();
                if !tok.is_empty() {
                    tokens.push(tok);
                }
                current.clear();
            }
            '\\' if !in_single && !in_double => {
                // line continuation
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }
    tokens
}
