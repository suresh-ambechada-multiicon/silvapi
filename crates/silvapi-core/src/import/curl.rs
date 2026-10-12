use crate::models::{
    ApiRequest, AuthType, BodyType, FormDataPart, FormDataPartKind, HttpMethod, KeyValue,
};

pub fn parse_curl(input: &str) -> Result<ApiRequest, String> {
    let input = input.trim();
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return Err("Empty input".into());
    }

    let mut req = ApiRequest::default();
    // `ApiRequest::default()` seeds JSON Content-Type/Accept headers and a blank
    // param row, which suit a hand-authored request but not an import: the
    // result must reflect only what the command says. Keeping the seeded
    // Content-Type would also put a second, conflicting one on the wire next to
    // any `-H` the user passed.
    req.headers.clear();
    req.params.clear();

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
                        let k = k.trim();
                        let v = v.trim();
                        if k.eq_ignore_ascii_case("authorization") {
                            if let Some(token) = v.strip_prefix("Bearer ") {
                                req.auth.auth_type = AuthType::Bearer;
                                req.auth.bearer_token = token.trim().to_string();
                            } else if let Some(basic) = v.strip_prefix("Basic ") {
                                req.auth.auth_type = AuthType::Basic;
                                if let Ok(decoded) = base64::Engine::decode(
                                    &base64::engine::general_purpose::STANDARD,
                                    basic.trim(),
                                ) {
                                    if let Ok(s) = String::from_utf8(decoded) {
                                        if let Some((user, pass)) = s.split_once(':') {
                                            req.auth.basic_username = user.to_string();
                                            req.auth.basic_password = pass.to_string();
                                        }
                                    }
                                }
                            } else {
                                req.headers.push(KeyValue::new(k, v));
                            }
                        } else {
                            req.headers.push(KeyValue::new(k, v));
                        }
                    }
                }
            }
            "--json" => {
                i += 1;
                if let Some(data) = tokens.get(i) {
                    req.body.content = data.to_string();
                    req.body.body_type = BodyType::Json;
                    if req.method == HttpMethod::GET {
                        req.method = HttpMethod::POST;
                    }
                    if !req
                        .headers
                        .iter()
                        .any(|h| h.key.eq_ignore_ascii_case("content-type"))
                    {
                        req.headers
                            .push(KeyValue::new("Content-Type", "application/json"));
                    }
                    if !req.headers.iter().any(|h| h.key.eq_ignore_ascii_case("accept")) {
                        req.headers
                            .push(KeyValue::new("Accept", "application/json"));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
                i += 1;
                if let Some(data) = tokens.get(i) {
                    let data = data.strip_prefix('@').unwrap_or(data);
                    if req.body.content.is_empty() {
                        req.body.content = data.to_string();
                    } else {
                        req.body.content.push('&');
                        req.body.content.push_str(data);
                    }
                    req.body.body_type = BodyType::Raw;
                    // detect json
                    let trimmed = req.body.content.trim();
                    if trimmed.starts_with('{') || trimmed.starts_with('[') {
                        req.body.body_type = BodyType::Json;
                    }
                    if req.method == HttpMethod::GET {
                        req.method = HttpMethod::POST;
                    }
                }
            }
            "--data-urlencode" => {
                i += 1;
                if let Some(data) = tokens.get(i) {
                    req.body.body_type = BodyType::UrlEncoded;
                    push_urlencoded_field(&mut req, data);
                    req.body.content = crate::models::build_urlencoded_body(&req.body.urlencoded);
                    if req.method == HttpMethod::GET {
                        req.method = HttpMethod::POST;
                    }
                }
            }
            "-F" | "--form" | "--form-string" => {
                i += 1;
                if let Some(part) = tokens.get(i) {
                    req.body.body_type = BodyType::FormData;
                    push_form_data_part(&mut req, part, token.as_str() != "--form-string");
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
                    } else {
                        req.auth.auth_type = AuthType::Basic;
                        req.auth.basic_username = creds.to_string();
                    }
                }
            }
            "-b" | "--cookie" => {
                i += 1;
                if let Some(cookie) = tokens.get(i) {
                    req.headers.push(KeyValue::new("Cookie", cookie));
                }
            }
            "-A" | "--user-agent" => {
                i += 1;
                if let Some(agent) = tokens.get(i) {
                    req.headers.push(KeyValue::new("User-Agent", agent));
                }
            }
            "-e" | "--referer" => {
                i += 1;
                if let Some(referer) = tokens.get(i) {
                    req.headers.push(KeyValue::new("Referer", referer));
                }
            }
            "-I" | "--head" => {
                req.method = HttpMethod::HEAD;
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
            // Flags that take an argument to skip
            "-o" | "--output" | "-m" | "--max-time" | "--connect-timeout" | "--cacert"
            | "--capath" | "--cert" | "--key" | "-x" | "--proxy" | "-U" | "--proxy-user"
            | "-c" | "--cookie-jar" | "--resolve" | "--retry" => {
                i += 1;
            }
            // Known boolean flags without arguments
            "-k" | "--insecure" | "-s" | "--silent" | "-S" | "--show-error" | "-L"
            | "--location" | "-v" | "--verbose" | "-i" | "--include" | "--compressed" | "-f"
            | "--fail" | "-N" | "--no-buffer" | "-g" | "--globoff" | "-4" | "-6" => {}
            s if !s.starts_with('-') && req.url.is_empty() => {
                parse_url_into(&mut req, s);
            }
            s if s.starts_with('-') => {
                // If an unknown flag takes an argument, avoid consuming it as URL
                if let Some(next) = tokens.get(i + 1) {
                    if !next.starts_with('-') && !is_likely_url(next) {
                        i += 1;
                    }
                }
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

fn is_likely_url(s: &str) -> bool {
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("localhost")
        || s.starts_with("127.0.0.1")
        || s.starts_with("::1")
        || (s.contains("://"))
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

fn push_urlencoded_field(req: &mut ApiRequest, data: &str) {
    let data = data.strip_prefix('@').unwrap_or(data);
    let (key, value) = data.split_once('=').unwrap_or((data, ""));
    if key.is_empty() {
        return;
    }
    req.body.urlencoded.push(KeyValue::new(key, value));
}

fn push_form_data_part(req: &mut ApiRequest, data: &str, allow_file: bool) {
    let (name, rest) = data.split_once('=').unwrap_or((data, ""));
    if name.is_empty() {
        return;
    }

    let mut value = rest.to_string();
    let mut content_type = String::new();
    let mut kind = FormDataPartKind::Text;

    if allow_file {
        if let Some(path) = rest.strip_prefix('@') {
            kind = FormDataPartKind::File;
            value = path.to_string();
        }
    }

    if let Some((main, opts)) = rest.strip_prefix('@').unwrap_or(rest).split_once(';') {
        value = main.to_string();
        for opt in opts.split(';') {
            if let Some(typ) = opt.strip_prefix("type=") {
                content_type = typ.to_string();
            }
        }
    }

    req.body.form_data.push(FormDataPart {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        value,
        enabled: true,
        kind,
        content_type,
    });
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
    let mut in_ansi_single = false;
    let mut in_double = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '$' if !in_single && !in_ansi_single && !in_double && chars.peek() == Some(&'\'') => {
                chars.next();
                in_ansi_single = true;
            }
            '\'' if in_ansi_single => {
                in_ansi_single = false;
            }
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single && !in_ansi_single => {
                in_double = !in_double;
            }
            '\\' if in_ansi_single => {
                if let Some(next) = chars.next() {
                    match next {
                        'n' => current.push('\n'),
                        'r' => current.push('\r'),
                        't' => current.push('\t'),
                        '\\' => current.push('\\'),
                        '\'' => current.push('\''),
                        '"' => current.push('"'),
                        _ => {
                            current.push(next);
                        }
                    }
                }
            }
            '\\' if in_single => {
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    current.push('\'');
                } else {
                    current.push('\\');
                }
            }
            '\\' if in_double => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            ' ' | '\t' | '\n' | '\r' if !in_single && !in_ansi_single && !in_double => {
                let tok = current.trim().to_string();
                if !tok.is_empty() {
                    tokens.push(tok);
                }
                current.clear();
            }
            '\\' if !in_single && !in_ansi_single && !in_double => {
                // line continuation: consume optional trailing spaces and \r\n or \n
                while chars.peek() == Some(&' ') || chars.peek() == Some(&'\t') {
                    chars.next();
                }
                if chars.peek() == Some(&'\r') {
                    chars.next();
                }
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

#[cfg(test)]
#[path = "curl_tests.rs"]
mod tests;
