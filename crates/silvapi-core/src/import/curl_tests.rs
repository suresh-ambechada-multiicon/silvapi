use super::*;

#[test]
fn imports_only_the_headers_the_command_specifies() {
    let req = parse_curl("curl https://example.com/users").unwrap();
    assert!(
        req.headers.is_empty(),
        "expected no headers, got {:?}",
        req.headers
    );
    assert!(
        req.params.is_empty(),
        "expected no params, got {:?}",
        req.params
    );
}

#[test]
fn does_not_add_a_second_content_type() {
    let req = parse_curl(
        "curl -X POST https://example.com/users -H 'Content-Type: application/xml' -d '<user/>'",
    )
    .unwrap();

    let content_types: Vec<&str> = req
        .headers
        .iter()
        .filter(|header| header.key.eq_ignore_ascii_case("content-type"))
        .map(|header| header.value.as_str())
        .collect();
    assert_eq!(content_types, vec!["application/xml"]);
}

#[test]
fn keeps_explicit_json_headers() {
    let req =
        parse_curl("curl https://example.com -H 'Accept: application/json' -H 'X-Trace: abc'")
            .unwrap();

    let pairs: Vec<(&str, &str)> = req
        .headers
        .iter()
        .map(|header| (header.key.as_str(), header.value.as_str()))
        .collect();
    assert_eq!(
        pairs,
        vec![("Accept", "application/json"), ("X-Trace", "abc")]
    );
}

#[test]
fn query_params_come_only_from_the_url() {
    let req = parse_curl("curl 'https://example.com/search?q=rust&page=2'").unwrap();

    let pairs: Vec<(&str, &str)> = req
        .params
        .iter()
        .map(|param| (param.key.as_str(), param.value.as_str()))
        .collect();
    assert_eq!(pairs, vec![("q", "rust"), ("page", "2")]);
    assert_eq!(req.url, "https://example.com/search");
}

#[test]
fn imports_urlencoded_rows() {
    let req = parse_curl("curl https://example.com/login --data-urlencode 'name=suresh' --data-urlencode 'token={{apiToken}}'").unwrap();

    assert_eq!(req.body.body_type, crate::models::BodyType::UrlEncoded);
    let pairs: Vec<(&str, &str)> = req
        .body
        .urlencoded
        .iter()
        .map(|field| (field.key.as_str(), field.value.as_str()))
        .collect();
    assert_eq!(pairs, vec![("name", "suresh"), ("token", "{{apiToken}}")]);
}

#[test]
fn imports_multipart_text_and_file_parts() {
    let req = parse_curl("curl https://example.com/upload -F 'title=Report' -F 'media=@/tmp/report.pdf;type=application/pdf'").unwrap();

    assert_eq!(req.body.body_type, crate::models::BodyType::FormData);
    assert_eq!(req.body.form_data.len(), 2);
    assert_eq!(req.body.form_data[0].name, "title");
    assert_eq!(req.body.form_data[0].value, "Report");
    assert_eq!(
        req.body.form_data[0].kind,
        crate::models::FormDataPartKind::Text
    );
    assert_eq!(req.body.form_data[1].name, "media");
    assert_eq!(req.body.form_data[1].value, "/tmp/report.pdf");
    assert_eq!(
        req.body.form_data[1].kind,
        crate::models::FormDataPartKind::File
    );
    assert_eq!(req.body.form_data[1].content_type, "application/pdf");
}

#[test]
fn imports_curl_json_flag() {
    let req = parse_curl("curl --json '{\"hello\":\"world\"}' https://example.com/api").unwrap();
    assert_eq!(req.method, HttpMethod::POST);
    assert_eq!(req.body.body_type, crate::models::BodyType::Json);
    assert_eq!(req.body.content, "{\"hello\":\"world\"}");
    assert!(req.headers.iter().any(|h| h.key == "Content-Type" && h.value == "application/json"));
    assert!(req.headers.iter().any(|h| h.key == "Accept" && h.value == "application/json"));
}

#[test]
fn imports_cookie_and_user_agent() {
    let req = parse_curl("curl -A 'Silvapi/1.0' -b 'token=secret123' https://example.com/data").unwrap();
    assert!(req.headers.iter().any(|h| h.key == "User-Agent" && h.value == "Silvapi/1.0"));
    assert!(req.headers.iter().any(|h| h.key == "Cookie" && h.value == "token=secret123"));
}

#[test]
fn imports_basic_auth_header() {
    // Basic dXNlcjpwYXNz is base64 of "user:pass"
    let req = parse_curl("curl -H 'Authorization: Basic dXNlcjpwYXNz' https://example.com").unwrap();
    assert_eq!(req.auth.auth_type, AuthType::Basic);
    assert_eq!(req.auth.basic_username, "user");
    assert_eq!(req.auth.basic_password, "pass");
}

#[test]
fn handles_line_continuations_with_whitespace() {
    let cmd = "curl \\\n  -X POST \\\r\n  https://example.com/api \\\n  -d '{\"ok\":true}'";
    let req = parse_curl(cmd).unwrap();
    assert_eq!(req.method, HttpMethod::POST);
    assert_eq!(req.url, "https://example.com/api");
    assert_eq!(req.body.content, "{\"ok\":true}");
}

#[test]
fn handles_ansi_c_single_quotes_and_escapes() {
    let cmd = r#"curl 'https://example.com/api' --data-raw $'{"prompt":"what\'s this","model":"haiku"}'"#;
    let req = parse_curl(cmd).unwrap();
    assert_eq!(req.method, HttpMethod::POST);
    assert_eq!(req.body.body_type, BodyType::Json);
    assert_eq!(req.body.content, r#"{"prompt":"what's this","model":"haiku"}"#);
}

