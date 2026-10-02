pub mod curl;
pub mod openapi;
pub mod postman;

pub use curl::parse_curl;
pub use openapi::import_openapi;
pub use postman::import_postman;
use crate::models::Collection;

/// Auto-detects whether the input is a Postman collection or OpenAPI specification,
/// parses it accordingly, and returns the Collection along with the detected format name.
pub fn import_collection(input: &str) -> Result<(Collection, &'static str), String> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(input) {
        let is_postman = v.get("item").map_or(false, |it| it.is_array())
            || v.get("info")
                .and_then(|i| i.get("schema"))
                .and_then(|s| s.as_str())
                .map_or(false, |s| s.contains("postman"));

        let is_openapi = v.get("openapi").is_some()
            || v.get("swagger").is_some()
            || v.get("paths").is_some();

        if is_postman && !is_openapi {
            return postman::import_postman(input).map(|c| (c, "Postman"));
        }
        if is_openapi {
            return openapi::import_openapi(input).map(|c| (c, "OpenAPI"));
        }
        if let Ok(c) = postman::import_postman(input) {
            return Ok((c, "Postman"));
        }
        if let Ok(c) = openapi::import_openapi(input) {
            return Ok((c, "OpenAPI"));
        }
    }

    // Try OpenAPI for YAML content
    if let Ok(c) = openapi::import_openapi(input) {
        return Ok((c, "OpenAPI"));
    }

    // Return the clearest error message
    postman::import_postman(input)
        .map(|c| (c, "Postman"))
        .or_else(|_| openapi::import_openapi(input).map(|c| (c, "OpenAPI")))
}

