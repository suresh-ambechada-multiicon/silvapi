use super::import_openapi;
use crate::models::CollectionItem;

#[test]
fn imports_query_params_into_url() {
    let spec = r#"
openapi: 3.0.3
info:
  title: PokeAPI
  version: "1.0"
servers:
  - url: https://pokeapi.co
paths:
  /api/v2/berry/:
get:
  operationId: berry_list
  summary: List berries
  parameters:
    - name: limit
      required: false
      in: query
      schema:
        type: integer
    - name: offset
      required: false
      in: query
      schema:
        type: integer
    - name: q
      required: false
      in: query
      schema:
        type: string
"#;

    let collection = import_openapi(spec).expect("imports openapi yaml");
    let request = match &collection.items[0] {
        CollectionItem::Request(request) => request,
        CollectionItem::Folder(folder) => match &folder.items[0] {
            CollectionItem::Request(request) => request,
            CollectionItem::Folder(_) => panic!("unexpected nested folder"),
        },
    };

    assert_eq!(
        request.url,
        "{{baseUrl}}/api/v2/berry/?limit=:limit&offset=:offset&q=:q"
    );
    assert_eq!(
        request
            .params
            .iter()
            .map(|param| param.key.as_str())
            .collect::<Vec<_>>(),
        vec!["limit", "offset", "q"]
    );
}

#[test]
fn imports_openapi_server_variables() {
    let spec = r#"
openapi: 3.0.3
info:
  title: Variable API
  version: "1.0"
servers:
  - url: https://{environment}.example.com/{version}
variables:
  environment:
    default: api
  version:
    default: v1
paths:
  /users/{id}:
get:
  summary: Get user
  parameters:
    - name: id
      in: path
      required: true
      schema:
        type: string
"#;

    let collection = import_openapi(spec).expect("imports server variables");
    let variables = collection
        .variables
        .iter()
        .map(|var| (var.name.as_str(), var.value.as_str()))
        .collect::<Vec<_>>();

    assert!(variables.contains(&("environment", "api")));
    assert!(variables.contains(&("version", "v1")));
    assert!(
        variables.contains(&("baseUrl", "https://{{environment}}.example.com/{{version}}"))
    );
}
