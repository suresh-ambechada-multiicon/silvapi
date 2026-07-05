use super::RequestPanel;
use silvapi_core::models::KeyValue;

#[test]
fn stale_path_param_prefix_rows_are_pruned() {
    let extracted = vec!["paramssksjska".to_string()];
    let previous = Vec::new();

    assert!(RequestPanel::is_stale_path_param_row(
        "params", "", &extracted, &previous
    ));
    assert!(!RequestPanel::is_stale_path_param_row(
        "paramssksjska",
        "",
        &extracted,
        &previous
    ));
    assert!(!RequestPanel::is_stale_path_param_row(
        "limit", "", &extracted, &previous
    ));
    assert!(!RequestPanel::is_stale_path_param_row(
        "params", "manual", &extracted, &previous
    ));
}

#[test]
fn blank_query_params_with_rows_become_placeholders() {
    let params = vec![KeyValue::new("limit", "10"), KeyValue::new("offset", "")];

    assert_eq!(
        RequestPanel::normalize_query_param_placeholders(
            "{{baseUrl}}/api/v2/berry/?limit=&offset=&q=",
            &params
        ),
        "{{baseUrl}}/api/v2/berry/?limit=:limit&offset=:offset&q="
    );
}
