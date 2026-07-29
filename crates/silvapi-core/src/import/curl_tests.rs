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
    let req = parse_curl(
        "curl https://example.com -H 'Accept: application/json' -H 'X-Trace: abc'",
    )
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
