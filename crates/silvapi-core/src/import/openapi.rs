use serde_json::Value;

use crate::models::{
    ApiRequest, AuthConfig, AuthType, BodyType, Collection, CollectionItem, Folder, FormDataPart,
    FormDataPartKind, HttpMethod, KeyValue, RequestBody,
};

pub fn import_openapi(input: &str) -> Result<Collection, String> {
    // Try JSON first, then YAML
    let v: Value = serde_json::from_str(input)
        .or_else(|_| serde_yaml::from_str::<Value>(input).map_err(|e| e.to_string()))
        .map_err(|e| format!("Failed to parse as JSON or YAML: {}", e))?;

    let title = v["info"]["title"]
        .as_str()
        .unwrap_or("Imported API")
        .to_string();

    let mut col = Collection::new(&title);

    add_server_variables(&v, &mut col);

    let paths = match v["paths"].as_object() {
        Some(p) => p,
        None => return Err("No paths found in OpenAPI spec".into()),
    };

    // Group by first path segment => folder
    let mut folders: std::collections::BTreeMap<String, Folder> = Default::default();

    for (path, path_item) in paths {
        let methods = ["get", "post", "put", "patch", "delete", "head", "options"];

        for method_str in &methods {
            let op = &path_item[*method_str];
            if op.is_null() {
                continue;
            }

            let folder_name = if let Some(tags) = op["tags"].as_array() {
                if let Some(first_tag) = tags.first() {
                    first_tag.as_str().unwrap_or("root").to_string()
                } else {
                    "root".to_string()
                }
            } else {
                path.trim_start_matches('/')
                    .split('/')
                    .next()
                    .unwrap_or("root")
                    .to_string()
            };

            let http_method = match *method_str {
                "get" => HttpMethod::GET,
                "post" => HttpMethod::POST,
                "put" => HttpMethod::PUT,
                "patch" => HttpMethod::PATCH,
                "delete" => HttpMethod::DELETE,
                "head" => HttpMethod::HEAD,
                "options" => HttpMethod::OPTIONS,
                _ => continue,
            };

            let name = op["summary"]
                .as_str()
                .or_else(|| op["operationId"].as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("{} {}", method_str.to_uppercase(), path));

            let mut url = crate::path_params::normalize_path_params(&if col.variables.is_empty() {
                format!("{}{}", "", path)
            } else {
                format!("{{{{baseUrl}}}}{}", path)
            });

            let mut params = Vec::new();
            let mut headers = Vec::new();
            let mut query_param_names = Vec::new();

            add_path_params(&url, &mut params);
            add_openapi_params(
                &v,
                &path_item["parameters"],
                &mut params,
                &mut headers,
                &mut query_param_names,
            );
            if let Some(params_arr) = op["parameters"].as_array() {
                for param in params_arr {
                    let param = resolve_ref(&v, param);
                    let pname = param["name"].as_str().unwrap_or("").to_string();
                    let location = param["in"].as_str().unwrap_or("");
                    match location {
                        "path" => push_unique_param(&mut params, &pname),
                        "query" => {
                            push_unique_query_param(&mut params, &mut query_param_names, &pname)
                        }
                        "header" => headers.push(KeyValue::new(&pname, "")),
                        _ => {}
                    }
                }
            }
            url = append_query_params_to_url(&url, &query_param_names);

            let request_body = resolve_ref(&v, &op["requestBody"]);
            let body = parse_request_body(&v, request_body);

            let mut auth = AuthConfig::default();
            let mut sec_array = op["security"].as_array();
            if sec_array.is_none() {
                sec_array = v["security"].as_array();
            }
            if let Some(secs) = sec_array {
                if let Some(first_sec) = secs.first() {
                    if let Some(first_key) = first_sec.as_object().and_then(|o| o.keys().next()) {
                        let scheme = &v["components"]["securitySchemes"][first_key];
                        let scheme_type = scheme["type"].as_str().unwrap_or("");
                        let scheme_scheme = scheme["scheme"].as_str().unwrap_or("");
                        if scheme_type == "http" && scheme_scheme.to_lowercase() == "bearer" {
                            auth.auth_type = AuthType::Bearer;
                            auth.bearer_token = format!("{{{{{first_key}}}}}");
                            push_unique_variable(&mut col, first_key.as_str(), "");
                        } else if scheme_type == "http" && scheme_scheme.to_lowercase() == "basic" {
                            auth.auth_type = AuthType::Basic;
                            auth.basic_username = format!("{{{{{first_key}Username}}}}");
                            auth.basic_password = format!("{{{{{first_key}Password}}}}");
                            push_unique_variable(&mut col, format!("{first_key}Username"), "");
                            push_unique_variable(&mut col, format!("{first_key}Password"), "");
                        } else if scheme_type == "apiKey" {
                            auth.auth_type = AuthType::ApiKey;
                            auth.api_key_name = scheme["name"].as_str().unwrap_or("").to_string();
                            auth.api_key_value = format!("{{{{{first_key}}}}}");
                            auth.api_key_in_header =
                                scheme["in"].as_str().unwrap_or("") == "header";
                            push_unique_variable(&mut col, first_key.as_str(), "");
                        }
                    }
                }
            }

            let req = ApiRequest {
                name,
                method: http_method,
                url,
                params,
                headers,
                body,
                auth,
                ..Default::default()
            };

            folders
                .entry(folder_name.clone())
                .or_insert_with(|| Folder::new(&folder_name))
                .items
                .push(CollectionItem::Request(req));
        }
    }

    for (_, folder) in folders {
        if folder.items.len() == 1 {
            col.items.push(folder.items.into_iter().next().unwrap());
        } else {
            col.items.push(CollectionItem::Folder(folder));
        }
    }

    Ok(col)
}

fn resolve_ref<'a>(root: &'a Value, mut val: &'a Value) -> &'a Value {
    if let Some(ref_str) = val["$ref"].as_str() {
        if ref_str.starts_with("#/") {
            let mut current = root;
            for part in ref_str.trim_start_matches("#/").split('/') {
                current = &current[part];
            }
            val = current;
        }
    }
    val
}

fn parse_request_body(root: &Value, request_body: &Value) -> RequestBody {
    let Some(content) = request_body["content"].as_object() else {
        return RequestBody::default();
    };

    if content.contains_key("application/json") {
        return json_request_body(root, &content["application/json"]);
    }

    if content.contains_key("application/x-www-form-urlencoded") {
        let fields = schema_fields(
            root,
            &content["application/x-www-form-urlencoded"]["schema"],
        );
        return RequestBody {
            body_type: BodyType::UrlEncoded,
            content: crate::models::build_urlencoded_body(&fields),
            urlencoded: fields,
            form_data: Vec::new(),
        };
    }

    if content.contains_key("multipart/form-data") {
        let parts = schema_fields(root, &content["multipart/form-data"]["schema"])
            .into_iter()
            .map(|field| FormDataPart {
                id: uuid::Uuid::new_v4().to_string(),
                name: field.key,
                value: field.value,
                enabled: field.enabled,
                kind: FormDataPartKind::Text,
                content_type: String::new(),
            })
            .collect();
        return RequestBody {
            body_type: BodyType::FormData,
            content: String::new(),
            urlencoded: Vec::new(),
            form_data: parts,
        };
    }

    RequestBody::default()
}

fn json_request_body(root: &Value, json_content: &Value) -> RequestBody {
    let example = &json_content["example"];
    let schema_example = &json_content["schema"]["example"];
    let examples = &json_content["examples"];

    let mut body_str = String::new();
    if !example.is_null() {
        body_str = serde_json::to_string_pretty(example).unwrap_or_default();
    } else if !schema_example.is_null() {
        body_str = serde_json::to_string_pretty(schema_example).unwrap_or_default();
    } else if let Some(ex_obj) = examples.as_object() {
        if let Some((_, ex_val)) = ex_obj.iter().next() {
            if !ex_val["value"].is_null() {
                body_str = serde_json::to_string_pretty(&ex_val["value"]).unwrap_or_default();
            }
        }
    }

    if body_str.is_empty() {
        let schema = resolve_ref(root, &json_content["schema"]);
        let default_body = generate_default_from_schema(root, schema);
        if default_body.is_object() && default_body.as_object().unwrap().is_empty() {
            body_str = "{\n  \n}".to_string();
        } else {
            body_str = serde_json::to_string_pretty(&default_body)
                .unwrap_or_else(|_| "{\n  \n}".to_string());
        }
    }

    RequestBody {
        body_type: BodyType::Json,
        content: body_str,
        urlencoded: Vec::new(),
        form_data: Vec::new(),
    }
}

fn schema_fields(root: &Value, schema: &Value) -> Vec<KeyValue> {
    let schema = resolve_ref(root, schema);
    schema["properties"]
        .as_object()
        .map(|props| {
            props
                .iter()
                .map(|(name, prop)| KeyValue::new(name, schema_default_value(root, prop)))
                .collect()
        })
        .unwrap_or_default()
}

fn schema_default_value(root: &Value, schema: &Value) -> String {
    let value = generate_default_from_schema(root, schema);
    match value {
        Value::Null => String::new(),
        Value::String(value) => value,
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(&value).unwrap_or_default(),
    }
}

fn add_server_variables(root: &Value, col: &mut Collection) {
    let Some(server) = root["servers"]
        .as_array()
        .and_then(|servers| servers.first())
    else {
        return;
    };

    let Some(raw_url) = server["url"]
        .as_str()
        .map(str::trim)
        .filter(|url| !url.is_empty())
    else {
        return;
    };

    let mut base_url = raw_url.trim_end_matches('/').to_string();
    if let Some(vars) = server["variables"].as_object() {
        for (name, val) in vars {
            if name.trim().is_empty() {
                continue;
            }

            let default_val = val["default"].as_str().unwrap_or("").to_string();
            push_unique_variable(col, name.as_str(), default_val);
            base_url = base_url.replace(&format!("{{{name}}}"), &format!("{{{{{name}}}}}"));
        }
    }

    push_unique_variable(col, "baseUrl", base_url);
}

fn push_unique_variable(col: &mut Collection, name: impl Into<String>, value: impl Into<String>) {
    let name = name.into();
    if !col.variables.iter().any(|var| var.name == name) {
        col.variables
            .push(crate::models::Variable::new(name, value.into()));
    }
}

fn add_openapi_params(
    root: &Value,
    value: &Value,
    params: &mut Vec<KeyValue>,
    headers: &mut Vec<KeyValue>,
    query_param_names: &mut Vec<String>,
) {
    if let Some(params_arr) = value.as_array() {
        for param in params_arr {
            let param = resolve_ref(root, param);
            let pname = param["name"].as_str().unwrap_or("").to_string();
            match param["in"].as_str().unwrap_or("") {
                "path" => push_unique_param(params, &pname),
                "query" => push_unique_query_param(params, query_param_names, &pname),
                "header" => headers.push(KeyValue::new(&pname, "")),
                _ => {}
            }
        }
    }
}

fn add_path_params(url: &str, params: &mut Vec<KeyValue>) {
    for key in crate::path_params::extract_path_params(url) {
        push_unique_param(params, &key);
    }
}

fn push_unique_param(params: &mut Vec<KeyValue>, key: &str) {
    if !key.is_empty() && !params.iter().any(|param| param.key == key) {
        params.push(KeyValue::new(key, ""));
    }
}

fn push_unique_query_param(
    params: &mut Vec<KeyValue>,
    query_param_names: &mut Vec<String>,
    key: &str,
) {
    push_unique_param(params, key);
    if !key.is_empty() && !query_param_names.iter().any(|name| name == key) {
        query_param_names.push(key.to_string());
    }
}

fn append_query_params_to_url(url: &str, query_param_names: &[String]) -> String {
    if query_param_names.is_empty() {
        return url.to_string();
    }

    let query = query_param_names
        .iter()
        .map(|name| {
            let value = if is_query_placeholder_name(name) {
                format!(":{name}")
            } else {
                String::new()
            };
            format!("{}={value}", urlencod(name))
        })
        .collect::<Vec<_>>()
        .join("&");

    if url.contains('?') {
        format!("{url}&{query}")
    } else {
        format!("{url}?{query}")
    }
}

fn urlencod(s: &str) -> String {
    s.bytes()
        .flat_map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![b as char]
            }
            b' ' => vec!['+'],
            _ => {
                let hex = format!("%{:02X}", b);
                hex.chars().collect()
            }
        })
        .collect()
}

fn is_query_placeholder_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn generate_default_from_schema<'a>(root: &'a Value, mut schema: &'a Value) -> Value {
    schema = resolve_ref(root, schema);
    let schema_type = schema["type"].as_str().unwrap_or("object");
    match schema_type {
        "object" => {
            let mut map = serde_json::Map::new();
            if let Some(props) = schema["properties"].as_object() {
                for (k, v) in props {
                    map.insert(k.clone(), generate_default_from_schema(root, v));
                }
            }
            Value::Object(map)
        }
        "array" => {
            let mut arr = Vec::new();
            if !schema["items"].is_null() {
                arr.push(generate_default_from_schema(root, &schema["items"]));
            }
            Value::Array(arr)
        }
        "string" => Value::String("".to_string()),
        "integer" | "number" => serde_json::json!(0),
        "boolean" => Value::Bool(false),
        _ => Value::Null,
    }
}

#[cfg(test)]
#[path = "openapi_tests.rs"]
mod tests;
