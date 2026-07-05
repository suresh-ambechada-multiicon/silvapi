use super::{
    extract_path_params, extract_query_value_params, normalize_path_params, replace_path_param,
};

#[test]
fn normalizes_single_brace_and_angle_params() {
    assert_eq!(
        normalize_path_params("{{baseUrl}}/api/{user_id}/posts/<post-id>"),
        "{{baseUrl}}/api/:user_id/posts/:post-id"
    );
}

#[test]
fn extracts_only_path_params() {
    assert_eq!(
        extract_path_params("{{baseUrl}}/api/:user_id?q=:query_id"),
        vec!["user_id".to_string()]
    );
}

#[test]
fn extracts_query_value_params() {
    assert_eq!(
        extract_query_value_params("{{baseUrl}}/api/:user_id?limit=:limit&offset=:offset"),
        vec!["limit".to_string(), "offset".to_string()]
    );
}

#[test]
fn replaces_path_param_without_touching_variable() {
    let (url, changed) = replace_path_param("{{baseUrl}}/api/{id}", "id", "42");
    assert!(changed);
    assert_eq!(url, "{{baseUrl}}/api/42");
}
