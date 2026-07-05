use std::collections::HashMap;

use super::AppState;
use silvapi_core::models::{ApiRequest, Collection, CollectionItem, Folder, Variable, Workspace};

#[test]
fn interpolate_variables_resolves_nested_collection_variables() {
    let request = ApiRequest::with_name("Nested variable");
    let request_id = request.id.clone();

    let mut collection = Collection::new("Imported API");
    collection
        .variables
        .push(Variable::new("environment", "api"));
    collection.variables.push(Variable::new(
        "baseUrl",
        "https://{{environment}}.example.com",
    ));
    collection.items.push(CollectionItem::Request(request));

    let mut workspace = Workspace::new("Test");
    workspace.collections.push(collection);

    let state = AppState {
        workspace,
        active_request: None,
        active_request_id: Some(request_id),
        response: None,
        is_loading: false,
        request_started_at: None,
        error: None,
        request_activity: HashMap::new(),
        next_request_run_id: 0,
        formatted_response: None,
        cache_order: Vec::new(),
        run_abort: std::collections::HashMap::new(),
    };

    assert_eq!(
        state.interpolate_variables("{{baseUrl}}/users"),
        "https://api.example.com/users"
    );
}

#[test]
fn interpolate_variables_resolves_parent_folder_variables() {
    let request = ApiRequest::with_name("Folder variable");
    let request_id = request.id.clone();

    let mut folder = Folder::new("api");
    folder
        .variables
        .push(Variable::new("baseUrl", "https://folder.example.com"));
    folder.items.push(CollectionItem::Request(request));

    let mut collection = Collection::new("Imported API");
    collection
        .variables
        .push(Variable::new("baseUrl", "https://collection.example.com"));
    collection.items.push(CollectionItem::Folder(folder));

    let mut workspace = Workspace::new("Test");
    workspace.collections.push(collection);

    let state = AppState {
        workspace,
        active_request: None,
        active_request_id: Some(request_id),
        response: None,
        is_loading: false,
        request_started_at: None,
        error: None,
        request_activity: HashMap::new(),
        next_request_run_id: 0,
        formatted_response: None,
        cache_order: Vec::new(),
        run_abort: std::collections::HashMap::new(),
    };

    assert_eq!(
        state.interpolate_variables("{{baseUrl}}/users"),
        "https://folder.example.com/users"
    );
}

#[test]
fn loading_state_is_scoped_to_selected_request() {
    let request_a = ApiRequest::with_name("Request A");
    let request_a_id = request_a.id.clone();
    let request_b = ApiRequest::with_name("Request B");
    let request_b_id = request_b.id.clone();

    let mut collection = Collection::new("Collection");
    collection.items.push(CollectionItem::Request(request_a));
    collection.items.push(CollectionItem::Request(request_b));

    let mut state = AppState {
        workspace: Workspace::new("Test"),
        active_request: None,
        active_request_id: None,
        response: None,
        is_loading: false,
        request_started_at: None,
        error: None,
        request_activity: HashMap::new(),
        next_request_run_id: 0,
        formatted_response: None,
        cache_order: Vec::new(),
        run_abort: std::collections::HashMap::new(),
    };
    state.workspace.collections.push(collection);

    assert!(state.select_request(&request_a_id));
    state.request_started(&request_a_id);
    assert!(state.is_loading);

    assert!(state.select_request(&request_b_id));
    assert!(!state.is_loading);
    assert!(state.request_is_loading(&request_a_id));

    assert!(state.select_request(&request_a_id));
    assert!(state.is_loading);
}
