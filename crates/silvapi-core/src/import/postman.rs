use serde_json::Value;

use crate::models::{
    ApiRequest, BodyType, Collection, CollectionItem, Folder, HttpMethod, KeyValue, RequestBody,
};

pub fn import_postman(json: &str) -> Result<Collection, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;

    let name = v["info"]["name"]
        .as_str()
        .unwrap_or("Imported Collection")
        .to_string();

    let mut col = Collection::new(&name);

    let items = v["item"].as_array().ok_or("No items in collection")?;

    for item in items {
        if let Some(ci) = parse_item(item) {
            col.items.push(ci);
        }
    }

    if let Some(vars) = v["variable"].as_array() {
        for var in vars {
            let key = var["key"].as_str().unwrap_or("").to_string();
            let val = var["value"].as_str().unwrap_or("").to_string();
            if !key.is_empty() {
                col.variables.push(crate::models::Variable::new(key, val));
            }
        }
    }

    Ok(col)
}

fn parse_item(item: &Value) -> Option<CollectionItem> {
    if let Some(sub_items) = item["item"].as_array() {
        // It's a folder
        let name = item["name"].as_str().unwrap_or("Folder").to_string();
        let mut folder = Folder::new(name);
        for sub in sub_items {
            if let Some(ci) = parse_item(sub) {
                folder.items.push(ci);
            }
        }
        Some(CollectionItem::Folder(folder))
    } else {
        // It's a request
        parse_postman_request(item).map(CollectionItem::Request)
    }
}

fn parse_postman_request(item: &Value) -> Option<ApiRequest> {
    let req_val = &item["request"];
    let name = item["name"].as_str().unwrap_or("Request").to_string();

    let method_str = req_val["method"].as_str().unwrap_or("GET");
    let method = match method_str.to_uppercase().as_str() {
        "GET" => HttpMethod::GET,
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "PATCH" => HttpMethod::PATCH,
        "DELETE" => HttpMethod::DELETE,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    };

    // URL - can be string or object
    let mut url = if let Some(s) = req_val["url"].as_str() {
        s.to_string()
    } else if let Some(raw) = req_val["url"]["raw"].as_str() {
        raw.to_string()
    } else {
        String::new()
    };
    if let Some(idx) = url.find('?') {
        url.truncate(idx);
    }
    let url = crate::path_params::normalize_path_params(&url);

    // Query params
    let mut params = Vec::new();
    if let Some(query) = req_val["url"]["query"].as_array() {
        for q in query {
            let key = q["key"].as_str().unwrap_or("").to_string();
            let val = q["value"].as_str().unwrap_or("").to_string();
            let disabled = q["disabled"].as_bool().unwrap_or(false);
            params.push(KeyValue {
                id: uuid::Uuid::new_v4().to_string(),
                key,
                value: val,
                enabled: !disabled,
                description: q["description"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    for key in crate::path_params::extract_path_params(&url) {
        if !params.iter().any(|param| param.key == key) {
            params.push(KeyValue::new(key, ""));
        }
    }

    // Headers
    let mut headers = Vec::new();
    if let Some(hdrs) = req_val["header"].as_array() {
        for h in hdrs {
            let key = h["key"].as_str().unwrap_or("").to_string();
            let val = h["value"].as_str().unwrap_or("").to_string();
            let disabled = h["disabled"].as_bool().unwrap_or(false);
            headers.push(KeyValue {
                id: uuid::Uuid::new_v4().to_string(),
                key,
                value: val,
                enabled: !disabled,
                description: h["description"].as_str().unwrap_or("").to_string(),
            });
        }
    }

    // Body
    let body = parse_body(&req_val["body"]);

    Some(ApiRequest {
        name,
        method,
        url,
        params,
        headers,
        body,
        ..Default::default()
    })
}

fn parse_body(body_val: &Value) -> RequestBody {
    let mode = body_val["mode"].as_str().unwrap_or("none");
    match mode {
        "raw" => {
            let content = body_val["raw"].as_str().unwrap_or("").to_string();
            let lang = body_val["options"]["raw"]["language"]
                .as_str()
                .unwrap_or("");
            let body_type = if lang == "json" || content.trim_start().starts_with('{') {
                BodyType::Json
            } else {
                BodyType::Raw
            };
            RequestBody {
                body_type,
                content,
                urlencoded: Vec::new(),
                form_data: Vec::new(),
            }
        }
        "urlencoded" => {
            let pairs: Vec<KeyValue> = body_val["urlencoded"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|p| {
                    let mut kv = KeyValue::new(
                        p["key"].as_str().unwrap_or(""),
                        p["value"].as_str().unwrap_or(""),
                    );
                    kv.enabled = !p["disabled"].as_bool().unwrap_or(false);
                    kv
                })
                .collect();
            RequestBody {
                body_type: BodyType::UrlEncoded,
                content: crate::models::build_urlencoded_body(&pairs),
                urlencoded: pairs,
                form_data: Vec::new(),
            }
        }
        _ => RequestBody::default(),
    }
}
