use serde_json::Value;

use crate::models::{
    ApiRequest, BodyType, Collection, CollectionItem, Folder, HttpMethod, KeyValue, RequestBody,
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

    // Detect server base URL
    let mut base_url = v["servers"][0]["url"]
        .as_str()
        .unwrap_or("")
        .trim_end_matches('/')
        .to_string();

    if let Some(vars) = v["servers"][0]["variables"].as_object() {
        for (k, val) in vars {
            let default_val = val["default"].as_str().unwrap_or("").to_string();
            base_url = base_url.replace(&format!("{{{}}}", k), &default_val);
        }
    }

    if !base_url.is_empty() {
        col.variables
            .push(crate::models::Variable::new("baseUrl", base_url));
    }

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

            let url = crate::path_params::normalize_path_params(&if col.variables.is_empty() {
                format!("{}{}", "", path)
            } else {
                format!("{{{{baseUrl}}}}{}", path)
            });

            let mut params = Vec::new();
            let mut headers = Vec::new();

            add_path_params(&url, &mut params);
            add_openapi_params(&v, &path_item["parameters"], &mut params, &mut headers);
            if let Some(params_arr) = op["parameters"].as_array() {
                for param in params_arr {
                    let param = resolve_ref(&v, param);
                    let pname = param["name"].as_str().unwrap_or("").to_string();
                    let location = param["in"].as_str().unwrap_or("");
                    match location {
                        "path" | "query" => push_unique_param(&mut params, &pname),
                        "header" => headers.push(KeyValue::new(&pname, "")),
                        _ => {}
                    }
                }
            }

            let body = if let Some(content) = op["requestBody"]["content"].as_object() {
                if content.contains_key("application/json") {
                    let json_content = &content["application/json"];
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
                                body_str = serde_json::to_string_pretty(&ex_val["value"])
                                    .unwrap_or_default();
                            }
                        }
                    }

                    if body_str.is_empty() {
                        let schema = resolve_ref(&v, &json_content["schema"]);
                        let default_body = generate_default_from_schema(&v, schema);
                        if default_body.is_object() && default_body.as_object().unwrap().is_empty()
                        {
                            body_str = "{\n  \n}".to_string();
                        } else {
                            body_str = serde_json::to_string_pretty(&default_body)
                                .unwrap_or_else(|_| "{\n  \n}".to_string());
                        }
                    }

                    RequestBody {
                        body_type: BodyType::Json,
                        content: body_str,
                    }
                } else {
                    RequestBody::default()
                }
            } else {
                RequestBody::default()
            };

            let mut auth = crate::models::AuthConfig::default();
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
                            auth.auth_type = crate::models::AuthType::Bearer;
                        } else if scheme_type == "http" && scheme_scheme.to_lowercase() == "basic" {
                            auth.auth_type = crate::models::AuthType::Basic;
                        } else if scheme_type == "apiKey" {
                            auth.auth_type = crate::models::AuthType::ApiKey;
                            auth.api_key_name = scheme["name"].as_str().unwrap_or("").to_string();
                            auth.api_key_in_header =
                                scheme["in"].as_str().unwrap_or("") == "header";
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

fn add_openapi_params(
    root: &Value,
    value: &Value,
    params: &mut Vec<KeyValue>,
    headers: &mut Vec<KeyValue>,
) {
    if let Some(params_arr) = value.as_array() {
        for param in params_arr {
            let param = resolve_ref(root, param);
            let pname = param["name"].as_str().unwrap_or("").to_string();
            match param["in"].as_str().unwrap_or("") {
                "path" | "query" => push_unique_param(params, &pname),
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
