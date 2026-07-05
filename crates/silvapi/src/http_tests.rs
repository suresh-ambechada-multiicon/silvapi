use super::{replace_query_param, replace_query_value_param};

#[test]
fn replaces_existing_query_param() {
    assert_eq!(
        replace_query_param(
            "https://example.com/items?limit=&offset=0#page",
            "limit",
            "25"
        ),
        (
            "https://example.com/items?limit=25&offset=0#page".to_string(),
            true,
        )
    );
}

#[test]
fn leaves_url_unchanged_when_query_param_is_missing() {
    assert_eq!(
        replace_query_param("https://example.com/items?offset=0", "limit", "25"),
        ("https://example.com/items?offset=0".to_string(), false)
    );
}

#[test]
fn replaces_query_value_placeholder() {
    assert_eq!(
        replace_query_value_param(
            "https://example.com/items?page=:limit&offset=:offset#page",
            "limit",
            "25"
        ),
        (
            "https://example.com/items?page=25&offset=:offset#page".to_string(),
            true,
        )
    );
}
