use std::collections::HashMap;
use std::time::Instant;

use gpui::{App, EventEmitter};

use crate::models::{ApiRequest, Collection, CollectionItem, Folder, HttpResponse, Workspace};

#[derive(Clone, Debug)]
pub enum AppEvent {
    RequestSelected,
    ResponseReceived,
    LoadingChanged,
    WorkspaceChanged,
    SaveNeeded,
    PreviewParsed,
    ResponseFormatted,
    ToggleLayout,
}

pub struct AppState {
    pub workspace: Workspace,
    pub active_request: Option<ApiRequest>,
    pub active_request_id: Option<String>,
    pub response: Option<HttpResponse>,
    pub is_loading: bool,
    pub request_started_at: Option<Instant>,
    pub error: Option<String>,
    pub preview_nodes: Option<Vec<crate::ui::html_preview::HtmlNode>>,
    pub formatted_response: Option<String>,
}

impl EventEmitter<AppEvent> for AppState {}

impl AppState {
    pub fn new() -> Self {
        let workspace = crate::storage::load_workspace()
            .ok()
            .flatten()
            .unwrap_or_default();

        Self {
            workspace,
            active_request: None,
            active_request_id: None,
            response: None,
            is_loading: false,
            request_started_at: None,
            error: None,
            preview_nodes: None,
            formatted_response: None,
        }
    }

    pub fn select_request(&mut self, id: &str) -> bool {
        if let Some(req) = self.find_request(id) {
            self.active_request_id = Some(id.to_string());
            self.active_request = Some(req);
            // Restore latest cached response for the newly selected request
            self.response = self
                .workspace
                .response_cache
                .get(id)
                .and_then(|history| history.last().cloned());
            self.preview_nodes = None;
            self.formatted_response = None;
            true
        } else {
            false
        }
    }

    pub fn find_request(&self, id: &str) -> Option<ApiRequest> {
        for col in &self.workspace.collections {
            if let Some(r) = find_in_items(&col.items, id) {
                return Some(r);
            }
        }
        None
    }

    pub fn add_collection(&mut self, name: String) {
        let col = Collection::new(name);
        self.workspace.collections.push(col);
    }

    pub fn add_request_to_collection(&mut self, col_id: &str) -> Option<String> {
        let req = ApiRequest::with_name("New Request");
        let id = req.id.clone();
        for col in &mut self.workspace.collections {
            if col.id == col_id {
                col.items.push(CollectionItem::Request(req));
                return Some(id);
            }
        }
        None
    }

    pub fn add_folder_to_collection(&mut self, col_id: &str) -> Option<String> {
        let folder = Folder::new("New Folder");
        let id = folder.id.clone();
        for col in &mut self.workspace.collections {
            if col.id == col_id {
                col.items.push(CollectionItem::Folder(folder));
                return Some(id);
            }
        }
        None
    }

    pub fn add_request_to_target(&mut self, target_id: Option<&str>) -> Option<String> {
        let req = ApiRequest::with_name("New Request");
        let id = req.id.clone();
        self.add_item_to_target(target_id, CollectionItem::Request(req))?;
        Some(id)
    }

    pub fn add_folder_to_target(&mut self, target_id: Option<&str>) -> Option<String> {
        let folder = Folder::new("New Folder");
        let id = folder.id.clone();
        self.add_item_to_target(target_id, CollectionItem::Folder(folder))?;
        Some(id)
    }

    fn add_item_to_target(&mut self, target_id: Option<&str>, item: CollectionItem) -> Option<()> {
        if self.workspace.collections.is_empty() {
            self.workspace
                .collections
                .push(Collection::new("New Collection"));
        }

        if let Some(id) = target_id {
            for col in &mut self.workspace.collections {
                if col.id == id {
                    col.items.push(item);
                    return Some(());
                }
                if insert_into_folder(&mut col.items, id, item.clone()) {
                    return Some(());
                }
            }
        }

        self.workspace.collections.first_mut()?.items.push(item);
        Some(())
    }

    pub fn duplicate_item(&mut self, id: &str) -> Option<String> {
        for col in &mut self.workspace.collections {
            if let Some(new_id) = duplicate_in_items(&mut col.items, id) {
                return Some(new_id);
            }
        }
        None
    }

    pub fn rename_item(&mut self, id: &str, new_name: String) {
        for col in &mut self.workspace.collections {
            if col.id == id {
                col.name = new_name;
                return;
            }
            if rename_in_items(&mut col.items, id, &new_name) {
                if self.active_request_id.as_deref() == Some(id) {
                    if let Some(req) = &mut self.active_request {
                        req.name = new_name;
                    }
                }
                return;
            }
        }
    }

    pub fn delete_item(&mut self, id: &str) {
        self.workspace.collections.retain(|c| c.id != id);
        for col in &mut self.workspace.collections {
            delete_from_items(&mut col.items, id);
        }
        if self.active_request_id.as_deref() == Some(id) {
            self.active_request = None;
            self.active_request_id = None;
        }
    }

    pub fn save_active_request(&mut self) {
        if let Some(req) = &self.active_request {
            let id = req.id.clone();
            let req = req.clone();
            for col in &mut self.workspace.collections {
                if update_in_items(&mut col.items, &id, req.clone()) {
                    return;
                }
            }
        }
    }

    pub fn loading_elapsed_ms(&self) -> Option<u64> {
        self.request_started_at
            .filter(|_| self.is_loading)
            .map(|started| started.elapsed().as_millis() as u64)
    }

    pub fn is_active_response_target(&self, request_id: &Option<String>) -> bool {
        match (request_id.as_deref(), self.active_request_id.as_deref()) {
            (Some(target), Some(active)) => target == active,
            (Some(target), None) => self
                .active_request
                .as_ref()
                .map(|req| req.id == target)
                .unwrap_or(false),
            (None, None) => true,
            _ => false,
        }
    }

    pub fn interpolate_variables(&self, text: &str) -> String {
        let mut result = text.to_string();
        for var in &self.workspace.variables {
            if var.enabled {
                let pattern = format!("{{{{{}}}}}", var.name);
                result = result.replace(&pattern, &var.value);
            }
        }
        if let Some(req) = &self.active_request {
            let col_id = self.active_request_id.as_deref().and_then(|_id| {
                self.workspace
                    .collections
                    .iter()
                    .find(|c| find_in_items(&c.items, _id).is_some())
            });
            if let Some(col) = col_id {
                for var in &col.variables {
                    if var.enabled {
                        let pattern = format!("{{{{{}}}}}", var.name);
                        result = result.replace(&pattern, &var.value);
                    }
                }
            }
        }
        // Postman dynamic variables
        if result.contains("{{$guid}}") {
            result = result.replace("{{$guid}}", &uuid::Uuid::new_v4().to_string());
        }
        if result.contains("{{$timestamp}}") {
            result = result.replace(
                "{{$timestamp}}",
                &chrono::Utc::now().timestamp().to_string(),
            );
        }
        if result.contains("{{$randomInt}}") {
            let val = rand::random::<u32>() % 1001;
            result = result.replace("{{$randomInt}}", &val.to_string());
        }

        result
    }

    pub fn import_collection(&mut self, mut col: crate::models::Collection) {
        for var in std::mem::take(&mut col.variables) {
            if !self.workspace.variables.iter().any(|v| v.name == var.name) {
                self.workspace.variables.push(var);
            }
        }
        self.workspace.collections.push(col);
    }

    pub fn get_folder_options(&self) -> Vec<(String, String)> {
        // Returns (folder_id, display_name) for all folders across all collections
        let mut opts = Vec::new();
        for col in &self.workspace.collections {
            opts.push((format!("root:{}", col.id), format!("[{}] (root)", col.name)));
            collect_folder_opts(&col.items, &col.name, &mut opts);
        }
        opts
    }

    // Move item_id into target: "root:col_id" or "folder:folder_id"
    pub fn move_item_to(&mut self, item_id: &str, target: &str) {
        if item_id.is_empty() {
            return;
        }
        if let Some(col_id) = target.strip_prefix("root:") {
            if !self
                .workspace
                .collections
                .iter()
                .any(|col| col.id == col_id)
            {
                return;
            }
        } else if let Some(folder_id) = target.strip_prefix("folder:") {
            if folder_id == item_id
                || folder_contains_id_in_collections(
                    &self.workspace.collections,
                    item_id,
                    folder_id,
                )
                || !folder_exists_in_collections(&self.workspace.collections, folder_id)
            {
                return;
            }
        } else {
            return;
        }

        // Extract the item first
        let item = match extract_item_from_all(&mut self.workspace.collections, item_id) {
            Some(i) => i,
            None => return,
        };
        if let Some(col_id) = target.strip_prefix("root:") {
            for col in &mut self.workspace.collections {
                if col.id == col_id {
                    col.items.push(item);
                    return;
                }
            }
        } else if let Some(folder_id) = target.strip_prefix("folder:") {
            for col in &mut self.workspace.collections {
                if insert_into_folder(&mut col.items, folder_id, item.clone()) {
                    return;
                }
            }
        }
    }

    pub fn move_item_up(&mut self, id: &str) {
        for col in &mut self.workspace.collections {
            if move_in_items(&mut col.items, id, true) {
                return;
            }
        }
    }

    pub fn move_item_down(&mut self, id: &str) {
        for col in &mut self.workspace.collections {
            if move_in_items(&mut col.items, id, false) {
                return;
            }
        }
    }
}

fn find_in_items(items: &[CollectionItem], id: &str) -> Option<ApiRequest> {
    for item in items {
        match item {
            CollectionItem::Request(r) if r.id == id => return Some(r.clone()),
            CollectionItem::Folder(f) => {
                if let Some(r) = find_in_items(&f.items, id) {
                    return Some(r);
                }
            }
            _ => {}
        }
    }
    None
}

fn rename_in_items(items: &mut Vec<CollectionItem>, id: &str, name: &str) -> bool {
    for item in items.iter_mut() {
        match item {
            CollectionItem::Request(r) if r.id == id => {
                r.name = name.to_string();
                return true;
            }
            CollectionItem::Folder(f) if f.id == id => {
                f.name = name.to_string();
                return true;
            }
            CollectionItem::Folder(f) => {
                if rename_in_items(&mut f.items, id, name) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn collect_folder_opts(items: &[CollectionItem], prefix: &str, opts: &mut Vec<(String, String)>) {
    for item in items {
        if let CollectionItem::Folder(f) = item {
            let display = format!("{} / {}", prefix, f.name);
            opts.push((format!("folder:{}", f.id), display.clone()));
            collect_folder_opts(&f.items, &display, opts);
        }
    }
}

fn folder_exists_in_collections(
    collections: &[crate::models::Collection],
    folder_id: &str,
) -> bool {
    collections
        .iter()
        .any(|col| contains_folder_id(&col.items, folder_id))
}

fn contains_folder_id(items: &[CollectionItem], folder_id: &str) -> bool {
    items.iter().any(|item| match item {
        CollectionItem::Folder(f) if f.id == folder_id => true,
        CollectionItem::Folder(f) => contains_folder_id(&f.items, folder_id),
        CollectionItem::Request(_) => false,
    })
}

fn folder_contains_id_in_collections(
    collections: &[crate::models::Collection],
    folder_id: &str,
    target_id: &str,
) -> bool {
    collections
        .iter()
        .any(|col| folder_contains_id(&col.items, folder_id, target_id))
}

fn folder_contains_id(items: &[CollectionItem], folder_id: &str, target_id: &str) -> bool {
    for item in items {
        if let CollectionItem::Folder(f) = item {
            if f.id == folder_id {
                return contains_item_id(&f.items, target_id);
            }
            if folder_contains_id(&f.items, folder_id, target_id) {
                return true;
            }
        }
    }
    false
}

fn contains_item_id(items: &[CollectionItem], target_id: &str) -> bool {
    items.iter().any(|item| match item {
        CollectionItem::Request(r) => r.id == target_id,
        CollectionItem::Folder(f) => f.id == target_id || contains_item_id(&f.items, target_id),
    })
}

fn extract_item_from_all(
    collections: &mut Vec<crate::models::Collection>,
    id: &str,
) -> Option<CollectionItem> {
    for col in collections.iter_mut() {
        if let Some(item) = extract_from_items(&mut col.items, id) {
            return Some(item);
        }
    }
    None
}

fn extract_from_items(items: &mut Vec<CollectionItem>, id: &str) -> Option<CollectionItem> {
    if let Some(pos) = items.iter().position(|i| i.id() == id) {
        return Some(items.remove(pos));
    }
    for item in items.iter_mut() {
        if let CollectionItem::Folder(f) = item {
            if let Some(found) = extract_from_items(&mut f.items, id) {
                return Some(found);
            }
        }
    }
    None
}

fn insert_into_folder(
    items: &mut Vec<CollectionItem>,
    folder_id: &str,
    new_item: CollectionItem,
) -> bool {
    for item in items.iter_mut() {
        if let CollectionItem::Folder(f) = item {
            if f.id == folder_id {
                f.items.push(new_item);
                return true;
            }
            if insert_into_folder(&mut f.items, folder_id, new_item.clone()) {
                return true;
            }
        }
    }
    false
}

fn duplicate_in_items(items: &mut Vec<CollectionItem>, id: &str) -> Option<String> {
    if let Some(pos) = items.iter().position(|i| i.id() == id) {
        let mut copy = clone_with_new_ids(&items[pos]);
        let new_id = copy.id().to_string();
        rename_copy(&mut copy);
        items.insert(pos + 1, copy);
        return Some(new_id);
    }

    for item in items.iter_mut() {
        if let CollectionItem::Folder(f) = item {
            if let Some(new_id) = duplicate_in_items(&mut f.items, id) {
                return Some(new_id);
            }
        }
    }
    None
}

fn clone_with_new_ids(item: &CollectionItem) -> CollectionItem {
    match item {
        CollectionItem::Request(req) => {
            let mut req = req.clone();
            req.id = uuid::Uuid::new_v4().to_string();
            for param in &mut req.params {
                param.id = uuid::Uuid::new_v4().to_string();
            }
            for header in &mut req.headers {
                header.id = uuid::Uuid::new_v4().to_string();
            }
            CollectionItem::Request(req)
        }
        CollectionItem::Folder(folder) => {
            let mut folder = folder.clone();
            folder.id = uuid::Uuid::new_v4().to_string();
            folder.items = folder.items.iter().map(clone_with_new_ids).collect();
            CollectionItem::Folder(folder)
        }
    }
}

fn rename_copy(item: &mut CollectionItem) {
    match item {
        CollectionItem::Request(req) => req.name = format!("{} Copy", req.name),
        CollectionItem::Folder(folder) => folder.name = format!("{} Copy", folder.name),
    }
}

fn move_in_items(items: &mut Vec<CollectionItem>, id: &str, up: bool) -> bool {
    if let Some(pos) = items.iter().position(|i| i.id() == id) {
        if up && pos > 0 {
            items.swap(pos, pos - 1);
        } else if !up && pos + 1 < items.len() {
            items.swap(pos, pos + 1);
        }
        return true;
    }
    for item in items.iter_mut() {
        if let CollectionItem::Folder(f) = item {
            if move_in_items(&mut f.items, id, up) {
                return true;
            }
        }
    }
    false
}

fn delete_from_items(items: &mut Vec<CollectionItem>, id: &str) {
    items.retain(|i| i.id() != id);
    for item in items.iter_mut() {
        if let CollectionItem::Folder(f) = item {
            delete_from_items(&mut f.items, id);
        }
    }
}

fn update_in_items(items: &mut Vec<CollectionItem>, id: &str, new_req: ApiRequest) -> bool {
    for item in items.iter_mut() {
        match item {
            CollectionItem::Request(r) if r.id == id => {
                *r = new_req;
                return true;
            }
            CollectionItem::Folder(f) => {
                if update_in_items(&mut f.items, id, new_req.clone()) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

pub fn init(_cx: &mut App) {}
