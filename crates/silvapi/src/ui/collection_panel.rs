use std::{collections::HashSet, ops::Range};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt as _, PopupMenuItem},
    spinner::Spinner,
    v_flex,
};

use crate::{
    models::{AuthConfig, AuthType, CollectionItem, Folder, KeyValue, Variable},
    state::{AppEvent, AppState},
};

use super::actions::{FocusActiveRequest, RenameSelected, SendRequest};

// ── Drag payload ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct DragItem {
    id: String,
    display: String,
}

struct DragPreview(String);
impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .text_sm()
            .text_color(cx.theme().foreground)
            .child(self.0.clone())
    }
}

// ── Flat list item ───────────────────────────────────────────────────────────

#[derive(Clone)]
struct FlatItem {
    id: String,
    display: String,
    method_str: String,
    method_color: Hsla,
    depth: usize,
    is_folder: bool,
    is_collection_root: bool,
    is_expanded: bool,
}

#[derive(Clone, Copy)]
enum RequestRowStatus {
    Loading,
    Status(u16),
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FolderSettingsTab {
    General,
    Headers,
    Auth,
    Variables,
}

struct KvRow {
    key: Entity<InputState>,
    value: Entity<InputState>,
    enabled: bool,
}

struct FolderAuthInputs {
    bearer_token: Entity<InputState>,
    bearer_prefix: Entity<InputState>,
    basic_username: Entity<InputState>,
    basic_password: Entity<InputState>,
    api_key_name: Entity<InputState>,
    api_key_value: Entity<InputState>,
    aws_access_key_id: Entity<InputState>,
    aws_secret_access_key: Entity<InputState>,
    aws_service: Entity<InputState>,
    aws_region: Entity<InputState>,
    aws_session_token: Entity<InputState>,
    jwt_secret: Entity<InputState>,
    jwt_payload: Entity<InputState>,
    oauth_client_id: Entity<InputState>,
    oauth_client_secret: Entity<InputState>,
    oauth_authorization_url: Entity<InputState>,
    oauth_access_token_url: Entity<InputState>,
    oauth_redirect_uri: Entity<InputState>,
    oauth_state: Entity<InputState>,
    oauth_audience: Entity<InputState>,
    oauth_username: Entity<InputState>,
    oauth_password: Entity<InputState>,
    oauth_scope: Entity<InputState>,
    oauth_header_name: Entity<InputState>,
    oauth_header_prefix: Entity<InputState>,
}

impl FolderAuthInputs {
    fn all(&self) -> Vec<Entity<InputState>> {
        vec![
            self.bearer_token.clone(),
            self.bearer_prefix.clone(),
            self.basic_username.clone(),
            self.basic_password.clone(),
            self.api_key_name.clone(),
            self.api_key_value.clone(),
            self.aws_access_key_id.clone(),
            self.aws_secret_access_key.clone(),
            self.aws_service.clone(),
            self.aws_region.clone(),
            self.aws_session_token.clone(),
            self.jwt_secret.clone(),
            self.jwt_payload.clone(),
            self.oauth_client_id.clone(),
            self.oauth_client_secret.clone(),
            self.oauth_authorization_url.clone(),
            self.oauth_access_token_url.clone(),
            self.oauth_redirect_uri.clone(),
            self.oauth_state.clone(),
            self.oauth_audience.clone(),
            self.oauth_username.clone(),
            self.oauth_password.clone(),
            self.oauth_scope.clone(),
            self.oauth_header_name.clone(),
            self.oauth_header_prefix.clone(),
        ]
    }
}

// ── Panel ────────────────────────────────────────────────────────────────────

pub struct CollectionPanel {
    focus_handle: FocusHandle,
    app_state: Entity<AppState>,
    expanded: HashSet<String>,
    selected_id: Option<String>,
    renaming_id: Option<String>,
    rename_input: Entity<InputState>,
    search_input: Entity<InputState>,
    flat_cache: Vec<FlatItem>,
    flat_dirty: bool,
    list_scroll: gpui::UniformListScrollHandle,
    folder_settings_id: Option<String>,
    folder_settings_tab: FolderSettingsTab,
    folder_name_input: Entity<InputState>,
    folder_description_input: Entity<InputState>,
    folder_header_rows: Vec<KvRow>,
    folder_variable_rows: Vec<KvRow>,
    folder_auth_enabled: bool,
    folder_auth_type: AuthType,
    folder_auth_inputs: FolderAuthInputs,

    _subs: Vec<Subscription>,
    _folder_auth_subs: Vec<Subscription>,
}

impl Focusable for CollectionPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl CollectionPanel {
    pub fn new(app_state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let rename_input = cx.new(|cx| InputState::new(window, cx).placeholder("Rename..."));
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let folder_name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Folder name"));
        let folder_description_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .soft_wrap(true)
                .placeholder("Folder description")
        });

        let folder_auth_inputs = Self::make_folder_auth_inputs(window, cx);
        let folder_auth_subs = folder_auth_inputs
            .all()
            .into_iter()
            .map(|input| {
                cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, _, cx| {
                    if matches!(ev, InputEvent::Change) {
                        this.save_folder_settings(cx);
                    }
                })
            })
            .collect::<Vec<_>>();

        // Expand all collections by default
        let mut expanded = HashSet::new();
        for col in &app_state.read(cx).workspace.collections {
            expanded.insert(col.id.clone());
        }

        let ws_sub = cx.subscribe_in(
            &app_state,
            window,
            |this, app_state, ev: &AppEvent, window, cx| {
                match ev {
                    AppEvent::WorkspaceChanged => {
                        // Auto-expand new collections
                        for col in &app_state.read(cx).workspace.collections {
                            this.expanded.insert(col.id.clone());
                        }
                        this.flat_dirty = true;
                        cx.notify();
                    }
                    AppEvent::RequestSelected => {
                        this.reveal_active_request(false, window, cx);
                    }
                    _ => {}
                }
            },
        );

        let rename_sub = cx.subscribe_in(
            &rename_input,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { .. } => this.commit_rename(window, cx),
                InputEvent::Blur => this.commit_rename(window, cx),
                _ => {}
            },
        );

        let search_sub = cx.subscribe_in(
            &search_input,
            window,
            |this, _, ev: &InputEvent, _, cx| match ev {
                InputEvent::Change => {
                    this.flat_dirty = true;
                    cx.notify();
                }
                _ => {}
            },
        );

        Self {
            focus_handle,
            app_state,
            expanded,
            selected_id: None,
            renaming_id: None,
            rename_input,
            search_input,
            flat_cache: Vec::new(),
            flat_dirty: true,
            list_scroll: gpui::UniformListScrollHandle::new(),
            folder_settings_id: None,
            folder_settings_tab: FolderSettingsTab::General,
            folder_name_input,
            folder_description_input,
            folder_header_rows: Vec::new(),
            folder_variable_rows: Vec::new(),
            folder_auth_enabled: false,
            folder_auth_type: AuthType::None,
            folder_auth_inputs,
            _subs: vec![ws_sub, rename_sub, search_sub],
            _folder_auth_subs: folder_auth_subs,
        }
    }

    fn new_auth_input(
        window: &mut Window,
        cx: &mut Context<Self>,
        placeholder: &'static str,
    ) -> Entity<InputState> {
        cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
    }

    fn make_folder_auth_inputs(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> FolderAuthInputs {
        FolderAuthInputs {
            bearer_token: Self::new_auth_input(window, cx, "Token"),
            bearer_prefix: Self::new_auth_input(window, cx, "Bearer"),
            basic_username: Self::new_auth_input(window, cx, "Username"),
            basic_password: Self::new_auth_input(window, cx, "Password"),
            api_key_name: Self::new_auth_input(window, cx, "Header name"),
            api_key_value: Self::new_auth_input(window, cx, "API key"),
            aws_access_key_id: Self::new_auth_input(window, cx, "Access key ID"),
            aws_secret_access_key: Self::new_auth_input(window, cx, "Secret access key"),
            aws_service: Self::new_auth_input(window, cx, "sts"),
            aws_region: Self::new_auth_input(window, cx, "us-east-1"),
            aws_session_token: Self::new_auth_input(window, cx, "Session token"),
            jwt_secret: Self::new_auth_input(window, cx, "Secret or private key"),
            jwt_payload: cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .code_editor("json")
                    .placeholder("Payload")
            }),
            oauth_client_id: Self::new_auth_input(window, cx, "Client ID"),
            oauth_client_secret: Self::new_auth_input(window, cx, "Client secret"),
            oauth_authorization_url: Self::new_auth_input(window, cx, "Authorization URL"),
            oauth_access_token_url: Self::new_auth_input(window, cx, "Access token URL"),
            oauth_redirect_uri: Self::new_auth_input(window, cx, "Redirect URI"),
            oauth_state: Self::new_auth_input(window, cx, "State"),
            oauth_audience: Self::new_auth_input(window, cx, "Audience"),
            oauth_username: Self::new_auth_input(window, cx, "Username"),
            oauth_password: Self::new_auth_input(window, cx, "Password"),
            oauth_scope: Self::new_auth_input(window, cx, "Scope"),
            oauth_header_name: Self::new_auth_input(window, cx, "Authorization"),
            oauth_header_prefix: Self::new_auth_input(window, cx, "Bearer"),
        }
    }

    fn current_folder_auth(&self, cx: &App) -> AuthConfig {
        let Some(folder_id) = self.folder_settings_id.clone() else {
            return AuthConfig::default();
        };
        self.app_state
            .read(cx)
            .workspace
            .collections
            .iter()
            .find_map(|collection| find_folder_in_items(&collection.items, &folder_id))
            .map(|folder| folder.auth.clone())
            .unwrap_or_default()
    }

    fn update_folder_auth(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut AuthConfig)) {
        let Some(folder_id) = self.folder_settings_id.clone() else {
            return;
        };
        self.app_state.update(cx, |state, cx| {
            for collection in &mut state.workspace.collections {
                if let Some(folder) = find_folder_in_items_mut(&mut collection.items, &folder_id) {
                    f(&mut folder.auth);
                    Self::ensure_folder_auth_defaults(&mut folder.auth);
                    break;
                }
            }
            cx.emit(AppEvent::WorkspaceChanged);
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    fn set_folder_auth(
        &mut self,
        enabled: bool,
        auth_type: AuthType,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.folder_auth_enabled = enabled;
        self.folder_auth_type = auth_type.clone();
        self.folder_settings_tab = FolderSettingsTab::Auth;
        self.update_folder_auth(cx, |auth| {
            auth.enabled = enabled;
            auth.auth_type = auth_type.clone();
        });
        let auth = self.current_folder_auth(cx);
        self.load_folder_auth_inputs(&auth, window, cx);
        cx.notify();
    }

    fn ensure_folder_auth_defaults(auth: &mut AuthConfig) {
        if auth.bearer_prefix.is_empty() {
            auth.bearer_prefix = "Bearer".to_string();
        }
        if auth.aws_service.is_empty() {
            auth.aws_service = "sts".to_string();
        }
        if auth.aws_region.is_empty() {
            auth.aws_region = "us-east-1".to_string();
        }
        if auth.jwt_algorithm.is_empty() {
            auth.jwt_algorithm = "HS256".to_string();
        }
        if auth.jwt_payload.is_empty() {
            auth.jwt_payload = "{\n  \"foo\": \"bar\"\n}".to_string();
        }
        if auth.oauth_grant_type.is_empty() {
            auth.oauth_grant_type = "Authorization Code".to_string();
        } else if auth.oauth_grant_type == "Password" {
            auth.oauth_grant_type = "Resource Owner Password Credential".to_string();
        }
        if auth.oauth_authorization_url.is_empty() {
            auth.oauth_authorization_url = "https://github.com/login/oauth/authorize".to_string();
        }
        if auth.oauth_access_token_url.is_empty() {
            auth.oauth_access_token_url =
                "https://github.com/login/oauth/access_token".to_string();
        }
        if auth.oauth_token_target.is_empty() {
            auth.oauth_token_target = "access_token".to_string();
        }
        if auth.oauth_response_type.is_empty() {
            auth.oauth_response_type = "Access Token".to_string();
        }
        if auth.oauth_header_name.is_empty() {
            auth.oauth_header_name = "Authorization".to_string();
        }
        if auth.oauth_header_prefix.is_empty() {
            auth.oauth_header_prefix = "Bearer".to_string();
        }
        if auth.oauth_send_credentials.is_empty() {
            auth.oauth_send_credentials = "In Request Body".to_string();
        } else if auth.oauth_send_credentials == "As Basic Auth Header" {
            auth.oauth_send_credentials = "As Basic Authentication".to_string();
        }
    }

    fn load_folder_auth_inputs(
        &self,
        auth: &AuthConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let i = &self.folder_auth_inputs;
        i.bearer_token
            .update(cx, |s, cx| s.set_value(auth.bearer_token.clone(), window, cx));
        i.bearer_prefix.update(cx, |s, cx| {
            s.set_value(folder_default_if_empty(&auth.bearer_prefix, "Bearer"), window, cx)
        });
        i.basic_username.update(cx, |s, cx| {
            s.set_value(auth.basic_username.clone(), window, cx)
        });
        i.basic_password.update(cx, |s, cx| {
            s.set_value(auth.basic_password.clone(), window, cx)
        });
        i.api_key_name
            .update(cx, |s, cx| s.set_value(auth.api_key_name.clone(), window, cx));
        i.api_key_value
            .update(cx, |s, cx| s.set_value(auth.api_key_value.clone(), window, cx));
        i.aws_access_key_id.update(cx, |s, cx| {
            s.set_value(auth.aws_access_key_id.clone(), window, cx)
        });
        i.aws_secret_access_key.update(cx, |s, cx| {
            s.set_value(auth.aws_secret_access_key.clone(), window, cx)
        });
        i.aws_service.update(cx, |s, cx| {
            s.set_value(folder_default_if_empty(&auth.aws_service, "sts"), window, cx)
        });
        i.aws_region.update(cx, |s, cx| {
            s.set_value(folder_default_if_empty(&auth.aws_region, "us-east-1"), window, cx)
        });
        i.aws_session_token.update(cx, |s, cx| {
            s.set_value(auth.aws_session_token.clone(), window, cx)
        });
        i.jwt_secret
            .update(cx, |s, cx| s.set_value(auth.jwt_secret.clone(), window, cx));
        i.jwt_payload.update(cx, |s, cx| {
            s.set_value(
                folder_default_if_empty(&auth.jwt_payload, "{\n  \"foo\": \"bar\"\n}"),
                window,
                cx,
            )
        });
        i.oauth_client_id.update(cx, |s, cx| {
            s.set_value(auth.oauth_client_id.clone(), window, cx)
        });
        i.oauth_client_secret.update(cx, |s, cx| {
            s.set_value(auth.oauth_client_secret.clone(), window, cx)
        });
        i.oauth_authorization_url.update(cx, |s, cx| {
            s.set_value(
                folder_default_if_empty(
                    &auth.oauth_authorization_url,
                    "https://github.com/login/oauth/authorize",
                ),
                window,
                cx,
            )
        });
        i.oauth_access_token_url.update(cx, |s, cx| {
            s.set_value(
                folder_default_if_empty(
                    &auth.oauth_access_token_url,
                    "https://github.com/login/oauth/access_token",
                ),
                window,
                cx,
            )
        });
        i.oauth_redirect_uri.update(cx, |s, cx| {
            s.set_value(auth.oauth_redirect_uri.clone(), window, cx)
        });
        i.oauth_state
            .update(cx, |s, cx| s.set_value(auth.oauth_state.clone(), window, cx));
        i.oauth_audience.update(cx, |s, cx| {
            s.set_value(auth.oauth_audience.clone(), window, cx)
        });
        i.oauth_username.update(cx, |s, cx| {
            s.set_value(auth.oauth_username.clone(), window, cx)
        });
        i.oauth_password.update(cx, |s, cx| {
            s.set_value(auth.oauth_password.clone(), window, cx)
        });
        i.oauth_scope
            .update(cx, |s, cx| s.set_value(auth.oauth_scope.clone(), window, cx));
        i.oauth_header_name.update(cx, |s, cx| {
            s.set_value(
                folder_default_if_empty(&auth.oauth_header_name, "Authorization"),
                window,
                cx,
            )
        });
        i.oauth_header_prefix.update(cx, |s, cx| {
            s.set_value(
                folder_default_if_empty(&auth.oauth_header_prefix, "Bearer"),
                window,
                cx,
            )
        });
    }

    fn sync_folder_auth_inputs_to_config(&self, auth: &mut AuthConfig, cx: &App) {
        let i = &self.folder_auth_inputs;
        auth.bearer_token = i.bearer_token.read(cx).value().to_string();
        auth.bearer_prefix = i.bearer_prefix.read(cx).value().to_string();
        auth.basic_username = i.basic_username.read(cx).value().to_string();
        auth.basic_password = i.basic_password.read(cx).value().to_string();
        auth.api_key_name = i.api_key_name.read(cx).value().to_string();
        auth.api_key_value = i.api_key_value.read(cx).value().to_string();
        auth.aws_access_key_id = i.aws_access_key_id.read(cx).value().to_string();
        auth.aws_secret_access_key = i.aws_secret_access_key.read(cx).value().to_string();
        auth.aws_service = i.aws_service.read(cx).value().to_string();
        auth.aws_region = i.aws_region.read(cx).value().to_string();
        auth.aws_session_token = i.aws_session_token.read(cx).value().to_string();
        auth.jwt_secret = i.jwt_secret.read(cx).value().to_string();
        auth.jwt_payload = i.jwt_payload.read(cx).value().to_string();
        auth.oauth_client_id = i.oauth_client_id.read(cx).value().to_string();
        auth.oauth_client_secret = i.oauth_client_secret.read(cx).value().to_string();
        auth.oauth_authorization_url = i.oauth_authorization_url.read(cx).value().to_string();
        auth.oauth_access_token_url = i.oauth_access_token_url.read(cx).value().to_string();
        auth.oauth_redirect_uri = i.oauth_redirect_uri.read(cx).value().to_string();
        auth.oauth_state = i.oauth_state.read(cx).value().to_string();
        auth.oauth_audience = i.oauth_audience.read(cx).value().to_string();
        auth.oauth_username = i.oauth_username.read(cx).value().to_string();
        auth.oauth_password = i.oauth_password.read(cx).value().to_string();
        auth.oauth_scope = i.oauth_scope.read(cx).value().to_string();
        auth.oauth_header_name = i.oauth_header_name.read(cx).value().to_string();
        auth.oauth_header_prefix = i.oauth_header_prefix.read(cx).value().to_string();
    }

    fn commit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.renaming_id.take() {
            let name = self.rename_input.read(cx).value().to_string();
            if !name.is_empty() {
                self.app_state.update(cx, |state, cx| {
                    state.rename_item(&id, name);
                    cx.emit(AppEvent::WorkspaceChanged);
                });
            }

            let fh = self.focus_handle.clone();
            cx.defer_in(window, move |_, window, cx| {
                fh.focus(window, cx);
            });
        }
        cx.notify();
    }

    fn flat_items(&mut self, cx: &App) -> Vec<FlatItem> {
        if self.flat_dirty {
            self.flat_cache = self.build_flat(cx);
            self.flat_dirty = false;
        }
        self.flat_cache.clone()
    }

    fn build_flat(&self, cx: &App) -> Vec<FlatItem> {
        let state = self.app_state.read(cx);
        let mut result = Vec::new();
        let query = self.search_input.read(cx).value().to_lowercase();

        for col in &state.workspace.collections {
            if !query.is_empty() && !col.matches_query(&query) {
                continue;
            }

            let is_exp = self.expanded.contains(&col.id) || !query.is_empty();
            result.push(FlatItem {
                id: col.id.clone(),
                display: col.name.clone(),
                method_str: String::new(),
                method_color: cx.theme().muted_foreground,
                depth: 0,
                is_folder: true,
                is_collection_root: true,
                is_expanded: is_exp,
            });
            if is_exp {
                self.add_items(&col.items, 1, cx, &query, &mut result);
            }
        }
        result
    }

    fn add_items(
        &self,
        items: &[CollectionItem],
        depth: usize,
        cx: &App,
        query: &str,
        result: &mut Vec<FlatItem>,
    ) {
        for item in items {
            if !query.is_empty() && !item.matches_query(query) {
                continue;
            }
            match item {
                CollectionItem::Folder(f) => {
                    let is_exp = self.expanded.contains(&f.id) || !query.is_empty();
                    result.push(FlatItem {
                        id: f.id.clone(),
                        display: f.name.clone(),
                        method_str: String::new(),
                        method_color: cx.theme().muted_foreground,
                        depth,
                        is_folder: true,
                        is_collection_root: false,
                        is_expanded: is_exp,
                    });
                    if is_exp {
                        self.add_items(&f.items, depth + 1, cx, query, result);
                    }
                }
                CollectionItem::Request(r) => {
                    let method = r.method.as_str().to_string();
                    let color = method_color(&method);
                    result.push(FlatItem {
                        id: r.id.clone(),
                        display: r.name.clone(),
                        method_str: method,
                        method_color: color,
                        depth,
                        is_folder: false,
                        is_collection_root: false,
                        is_expanded: false,
                    });
                }
            }
        }
    }

    fn make_kv_row(
        &self,
        key: &str,
        value: &str,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> KvRow {
        let key_input = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let value_input = cx.new(|cx| InputState::new(window, cx).placeholder("Value"));
        key_input.update(cx, |state, cx| {
            state.set_value(key.to_string(), window, cx);
        });
        value_input.update(cx, |state, cx| {
            state.set_value(value.to_string(), window, cx);
        });
        KvRow {
            key: key_input,
            value: value_input,
            enabled,
        }
    }

    fn open_folder_settings(
        &mut self,
        folder_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let folder = self
            .app_state
            .read(cx)
            .workspace
            .collections
            .iter()
            .find_map(|collection| find_folder_in_items(&collection.items, &folder_id))
            .cloned();

        let Some(folder) = folder else {
            return;
        };

        self.folder_settings_id = Some(folder.id.clone());
        self.folder_settings_tab = FolderSettingsTab::General;
        self.folder_auth_enabled = folder.auth.enabled;
        self.folder_auth_type = folder.auth.auth_type.clone();
        self.load_folder_auth_inputs(&folder.auth, window, cx);
        self.folder_name_input.update(cx, |state, cx| {
            state.set_value(folder.name.clone(), window, cx);
        });
        self.folder_description_input.update(cx, |state, cx| {
            state.set_value(folder.description.clone(), window, cx);
        });
        self.folder_header_rows = folder
            .headers
            .iter()
            .map(|header| self.make_kv_row(&header.key, &header.value, header.enabled, window, cx))
            .collect();
        self.folder_variable_rows = folder
            .variables
            .iter()
            .map(|var| self.make_kv_row(&var.name, &var.value, var.enabled, window, cx))
            .collect();
        cx.notify();
    }

    fn close_folder_settings(&mut self, cx: &mut Context<Self>) {
        self.save_folder_settings(cx);
        self.folder_settings_id = None;
        cx.notify();
    }

    fn save_folder_settings(&mut self, cx: &mut Context<Self>) {
        let Some(folder_id) = self.folder_settings_id.clone() else {
            return;
        };

        let name = self.folder_name_input.read(cx).value().to_string();
        let description = self.folder_description_input.read(cx).value().to_string();
        let headers = collect_kv_rows(&self.folder_header_rows, cx);
        let variables = self
            .folder_variable_rows
            .iter()
            .filter_map(|row| {
                let name = row.key.read(cx).value().to_string();
                let value = row.value.read(cx).value().to_string();
                (!name.is_empty() || !value.is_empty()).then(|| Variable {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    value,
                    enabled: row.enabled,
                })
            })
            .collect::<Vec<_>>();
        let mut auth = self.current_folder_auth(cx);
        auth.enabled = self.folder_auth_enabled;
        auth.auth_type = self.folder_auth_type.clone();
        self.sync_folder_auth_inputs_to_config(&mut auth, cx);
        Self::ensure_folder_auth_defaults(&mut auth);

        self.app_state.update(cx, |state, cx| {
            for collection in &mut state.workspace.collections {
                if let Some(folder) = find_folder_in_items_mut(&mut collection.items, &folder_id) {
                    folder.name = if name.trim().is_empty() {
                        folder.name.clone()
                    } else {
                        name.clone()
                    };
                    folder.description = description.clone();
                    folder.headers = headers.clone();
                    folder.variables = variables.clone();
                    folder.auth = auth.clone();
                    break;
                }
            }
            cx.emit(AppEvent::WorkspaceChanged);
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    fn add_folder_header_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let row = self.make_kv_row("", "", true, window, cx);
        self.folder_header_rows.push(row);
        cx.notify();
    }

    fn add_folder_variable_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let row = self.make_kv_row("", "", true, window, cx);
        self.folder_variable_rows.push(row);
        cx.notify();
    }

    pub fn folder_settings_open(&self) -> bool {
        self.folder_settings_id.is_some()
    }

    pub fn folder_settings_modal(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.render_folder_settings_modal(window, cx)
    }

    fn render_folder_settings_modal(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let title = self.folder_name_input.read(cx).value().to_string();

        // Size the modal to ~60% of the viewport, centered.
        let viewport = window.viewport_size();
        let modal_w = viewport.width * 0.6;
        let modal_h = viewport.height * 0.6;

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().background.opacity(0.72))
            // Backdrop: block clicks reaching the app behind, and close on click.
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.close_folder_settings(cx);
                }),
            )
            .on_mouse_down(MouseButton::Right, |_, _, _| {})
            .child(
                v_flex()
                    .w(modal_w)
                    .h(modal_h)
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .shadow_lg()
                    .overflow_hidden()
                    // Swallow clicks inside the panel so they don't close the modal.
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .on_mouse_down(MouseButton::Right, |_, _, _| {})
                    .child(
                        h_flex()
                            .h(px(58.))
                            .px_5()
                            .items_center()
                            .gap_3()
                            .child(
                                Icon::new(IconName::Folder).text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(cx.theme().foreground)
                                    .child(if title.trim().is_empty() {
                                        "Folder Settings".to_string()
                                    } else {
                                        title
                                    }),
                            )
                            .child(
                                Button::new("folder-settings-close")
                                    .icon(IconName::Close)
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.close_folder_settings(cx);
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .flex_1()
                            .min_h_0()
                            .items_start()
                            .child(self.render_folder_settings_sidebar(cx))
                            .child(self.render_folder_settings_content(window, cx)),
                    ),
            )
            .into_any_element()
    }

    fn render_folder_settings_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w(px(188.))
            .h_full()
            .px_4()
            .py_4()
            .gap_1()
            .border_r_1()
            .border_color(cx.theme().border)
            .child(self.render_folder_settings_tab("General", FolderSettingsTab::General, cx))
            .child(self.render_folder_settings_tab("Headers", FolderSettingsTab::Headers, cx))
            .child(self.render_folder_auth_dropdown(cx))
            .child(self.render_folder_settings_tab("Variables", FolderSettingsTab::Variables, cx))
            .into_any_element()
    }

    fn folder_auth_label(enabled: bool, auth_type: &AuthType) -> &'static str {
        if !enabled {
            return "Inherit from Parent";
        }
        match auth_type {
            AuthType::None => "No Auth",
            AuthType::ApiKey => "API Key",
            AuthType::AwsV4 => "AWS Signature",
            AuthType::Basic => "Basic Auth",
            AuthType::Bearer => "Bearer Token",
            AuthType::Jwt => "JWT Bearer",
            AuthType::OAuth2 => "OAuth 2.0",
        }
    }

    fn folder_auth_short_label(enabled: bool, auth_type: &AuthType) -> &'static str {
        if !enabled {
            return "Inherit";
        }
        match auth_type {
            AuthType::None => "No Auth",
            AuthType::ApiKey => "API Key",
            AuthType::AwsV4 => "AWS v4",
            AuthType::Basic => "Basic",
            AuthType::Bearer => "Bearer",
            AuthType::Jwt => "JWT",
            AuthType::OAuth2 => "OAuth 2",
        }
    }

    fn render_folder_auth_dropdown(&self, cx: &mut Context<Self>) -> AnyElement {
        let active = self.folder_settings_tab == FolderSettingsTab::Auth;
        let label = Self::folder_auth_short_label(self.folder_auth_enabled, &self.folder_auth_type);
        let cur_enabled = self.folder_auth_enabled;
        let cur_type = self.folder_auth_type.clone();
        let entity = cx.entity();

        let item = move |menu_label: &'static str, enabled: bool, auth_type: AuthType| {
            let selected = cur_enabled == enabled && cur_type == auth_type;
            let entity = entity.clone();
            PopupMenuItem::new(menu_label)
                .checked(selected)
                .on_click(move |_, window, cx| {
                    let auth_type = auth_type.clone();
                    entity.update(cx, |this, cx| {
                        this.set_folder_auth(enabled, auth_type, window, cx);
                    });
                })
        };

        div()
            .w_full()
            .overflow_hidden()
            .child(
                DropdownButton::new("folder-auth-dropdown")
            .button(
                Button::new("folder-auth-btn")
                    .label(label)
                    .ghost()
                    .small()
                    .when(active, |b| b.primary())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.folder_settings_tab = FolderSettingsTab::Auth;
                        cx.notify();
                    })),
            )
            .dropdown_menu(move |menu, _, _| {
                menu.item(item("API Key", true, AuthType::ApiKey))
                    .item(item("AWS Signature", true, AuthType::AwsV4))
                    .item(item("Basic Auth", true, AuthType::Basic))
                    .item(item("Bearer Token", true, AuthType::Bearer))
                    .item(item("JWT Bearer", true, AuthType::Jwt))
                    .item(item("OAuth 2.0", true, AuthType::OAuth2))
                    .separator()
                    .item(item("Inherit from Parent", false, AuthType::None))
                    .item(item("No Auth", true, AuthType::None))
            }),
            )
            .into_any_element()
    }

    fn render_folder_settings_tab(
        &self,
        label: &'static str,
        tab: FolderSettingsTab,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = self.folder_settings_tab == tab;
        div()
            .h(px(30.))
            .px_3()
            .rounded_md()
            .flex()
            .items_center()
            .cursor_pointer()
            .text_sm()
            .text_color(if selected {
                cx.theme().foreground
            } else {
                cx.theme().muted_foreground
            })
            .bg(if selected {
                cx.theme().secondary
            } else {
                cx.theme().background
            })
            .hover(|style| style.bg(cx.theme().secondary))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.folder_settings_tab = tab;
                    cx.notify();
                }),
            )
            .child(label)
            .into_any_element()
    }

    fn render_folder_settings_content(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let body = match self.folder_settings_tab {
            FolderSettingsTab::General => self.render_folder_general_settings(cx),
            FolderSettingsTab::Headers => self.render_folder_headers_settings(window, cx),
            FolderSettingsTab::Auth => self.render_folder_auth_settings(cx),
            FolderSettingsTab::Variables => self.render_folder_variables_settings(window, cx),
        };

        div()
            .id("folder-settings-content")
            .flex_1()
            .h_full()
            .min_w_0()
            .min_h_0()
            .p_4()
            .overflow_y_scroll()
            .child(body)
            .into_any_element()
    }

    fn render_folder_general_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .size_full()
            .gap_3()
            .child(label_text("Folder Name", cx))
            .child(Input::new(&self.folder_name_input).h(px(34.)))
            .child(Input::new(&self.folder_description_input).flex_1().w_full())
            .into_any_element()
    }

    fn render_folder_headers_settings(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(folder_kv_header(cx))
            .children(
                self.folder_header_rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| render_folder_kv_row(row, index, false, cx)),
            )
            .child(
                h_flex().justify_center().child(
                    Button::new("add-folder-header")
                        .icon(IconName::Plus)
                        .label("Add")
                        .small()
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.add_folder_header_row(window, cx);
                        })),
                ),
            )
            .into_any_element()
    }

    fn render_folder_variables_settings(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.folder_variable_rows.is_empty() {
            return div()
                .size_full()
                .border_1()
                .border_color(cx.theme().border)
                .rounded_lg()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    v_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_sm()
                                .italic()
                                .text_color(cx.theme().muted_foreground)
                                .child("Override variables for requests within this folder."),
                        )
                        .child(
                            Button::new("create-folder-env")
                                .label("Create Folder Environment")
                                .small()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.add_folder_variable_row(window, cx);
                                })),
                        ),
                )
                .into_any_element();
        }

        v_flex()
            .size_full()
            .gap_2()
            .child(folder_kv_header(cx))
            .children(
                self.folder_variable_rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| render_folder_kv_row(row, index, true, cx)),
            )
            .child(
                h_flex().justify_center().child(
                    Button::new("add-folder-variable")
                        .icon(IconName::Plus)
                        .label("Add")
                        .small()
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.add_folder_variable_row(window, cx);
                        })),
                ),
            )
            .into_any_element()
    }

    fn render_folder_auth_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let auth = self.current_folder_auth(cx);
        let enabled = self.folder_auth_enabled;
        let auth_type = self.folder_auth_type.clone();

        v_flex()
            .w_full()
            .gap_3()
            .child(label_text("Authentication", cx))
            .child(
                h_flex()
                    .h(px(34.))
                    .px_3()
                    .items_center()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().primary)
                    .bg(cx.theme().primary.opacity(0.1))
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(Self::folder_auth_label(enabled, &auth_type)),
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Checkbox::new("folder-auth-enabled")
                            .checked(enabled)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.folder_auth_enabled = *checked;
                                this.update_folder_auth(cx, |auth| {
                                    auth.enabled = *checked;
                                });
                                cx.notify();
                            })),
                    )
                    .child(div().text_sm().child("Enabled")),
            )
            .when(!enabled, |el| {
                el.child(
                    div()
                        .text_sm()
                        .italic()
                        .text_color(cx.theme().muted_foreground)
                        .child("Requests in this folder inherit authentication from the parent."),
                )
            })
            .when(enabled && matches!(auth_type, AuthType::None), |el| {
                el.child(
                    div()
                        .text_sm()
                        .italic()
                        .text_color(cx.theme().muted_foreground)
                        .child("Requests in this folder send no authentication."),
                )
            })
            .when(enabled && matches!(auth_type, AuthType::ApiKey), |el| {
                el.child(self.render_folder_api_key_auth(&auth, cx))
            })
            .when(enabled && matches!(auth_type, AuthType::AwsV4), |el| {
                el.child(self.render_folder_aws_auth(cx))
            })
            .when(enabled && matches!(auth_type, AuthType::Basic), |el| {
                el.child(self.render_folder_basic_auth(cx))
            })
            .when(enabled && matches!(auth_type, AuthType::Bearer), |el| {
                el.child(self.render_folder_bearer_auth(cx))
            })
            .when(enabled && matches!(auth_type, AuthType::Jwt), |el| {
                el.child(self.render_folder_jwt_auth(&auth, cx))
            })
            .when(enabled && matches!(auth_type, AuthType::OAuth2), |el| {
                el.child(self.render_folder_oauth_auth(&auth, cx))
            })
            .into_any_element()
    }

    fn render_folder_api_key_auth(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let api_key_in_header = auth.api_key_in_header;
        let behavior_label = if api_key_in_header {
            "Insert Header"
        } else {
            "Insert Query Param"
        };
        let entity = cx.entity();
        v_flex()
            .gap_3()
            .child(folder_auth_select_field(
                "Behavior",
                DropdownButton::new("folder-api-key-behavior")
                    .button(
                        Button::new("folder-api-key-behavior-btn")
                            .label(behavior_label)
                            .ghost()
                            .small(),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        let header_entity = entity.clone();
                        let query_entity = entity.clone();
                        menu.item(
                            PopupMenuItem::new("Insert Header")
                                .checked(api_key_in_header)
                                .on_click(move |_, _, cx| {
                                    header_entity.update(cx, |this, cx| {
                                        this.update_folder_auth(cx, |auth| {
                                            auth.api_key_in_header = true;
                                        });
                                    });
                                }),
                        )
                        .item(
                            PopupMenuItem::new("Insert Query Param")
                                .checked(!api_key_in_header)
                                .on_click(move |_, _, cx| {
                                    query_entity.update(cx, |this, cx| {
                                        this.update_folder_auth(cx, |auth| {
                                            auth.api_key_in_header = false;
                                        });
                                    });
                                }),
                        )
                    }),
                cx,
            ))
            .child(folder_auth_input_field(
                if api_key_in_header {
                    "Header Name*"
                } else {
                    "Query Param Name*"
                },
                &self.folder_auth_inputs.api_key_name,
                cx,
            ))
            .child(folder_auth_input_field(
                "API Key",
                &self.folder_auth_inputs.api_key_value,
                cx,
            ))
    }

    fn render_folder_aws_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(folder_auth_input_field(
                "Access Key ID*",
                &self.folder_auth_inputs.aws_access_key_id,
                cx,
            ))
            .child(folder_auth_input_field(
                "Secret Access Key*",
                &self.folder_auth_inputs.aws_secret_access_key,
                cx,
            ))
            .child(folder_auth_input_field(
                "Service Name*",
                &self.folder_auth_inputs.aws_service,
                cx,
            ))
            .child(folder_auth_input_field(
                "Region",
                &self.folder_auth_inputs.aws_region,
                cx,
            ))
            .child(folder_auth_input_field(
                "Session Token",
                &self.folder_auth_inputs.aws_session_token,
                cx,
            ))
    }

    fn render_folder_basic_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(folder_auth_input_field(
                "Username",
                &self.folder_auth_inputs.basic_username,
                cx,
            ))
            .child(folder_auth_input_field(
                "Password",
                &self.folder_auth_inputs.basic_password,
                cx,
            ))
    }

    fn render_folder_bearer_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(folder_auth_input_field(
                "Token",
                &self.folder_auth_inputs.bearer_token,
                cx,
            ))
            .child(folder_auth_input_field(
                "Prefix",
                &self.folder_auth_inputs.bearer_prefix,
                cx,
            ))
    }

    fn render_folder_jwt_auth(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let entity = cx.entity();
        let current_algorithm = auth.jwt_algorithm.clone();
        v_flex()
            .gap_3()
            .child(
                DropdownButton::new("folder-jwt-algorithm")
                    .button(
                        Button::new("folder-jwt-algorithm-btn")
                            .label(if auth.jwt_algorithm.is_empty() {
                                "HS256"
                            } else {
                                auth.jwt_algorithm.as_str()
                            })
                            .ghost()
                            .small(),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        ["HS256", "HS384", "HS512", "RS256", "RS384", "RS512"]
                            .into_iter()
                            .fold(menu, |menu, algorithm| {
                                let entity = entity.clone();
                                menu.item(
                                    PopupMenuItem::new(algorithm)
                                        .checked(current_algorithm == algorithm)
                                        .on_click(move |_, _, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.update_folder_auth(cx, |auth| {
                                                    auth.jwt_algorithm = algorithm.to_string();
                                                });
                                            });
                                        }),
                                )
                            })
                    }),
            )
            .child(folder_auth_input_field(
                "Secret or Private Key",
                &self.folder_auth_inputs.jwt_secret,
                cx,
            ))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Checkbox::new("folder-jwt-secret-base64")
                            .checked(auth.jwt_secret_base64)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.update_folder_auth(cx, |auth| {
                                    auth.jwt_secret_base64 = *checked;
                                });
                            })),
                    )
                    .child(div().text_sm().child("Secret is base64 encoded")),
            )
            .child(folder_auth_multiline_field(
                "Payload* (Json)",
                &self.folder_auth_inputs.jwt_payload,
                cx,
            ))
    }

    fn render_folder_oauth_auth(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let grant = folder_oauth_grant_label(&auth.oauth_grant_type);
        let is_authorization_code = grant == "Authorization Code";
        let is_implicit = grant == "Implicit";
        let is_resource_owner = grant == "Resource Owner Password Credential";
        let is_client_credentials = grant == "Client Credentials";

        v_flex()
            .gap_3()
            .child(self.folder_oauth_grant_dropdown(&auth.oauth_grant_type, cx))
            .when(is_authorization_code, |el| {
                el.child(folder_auth_input_field(
                    "Client ID",
                    &self.folder_auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Client Secret",
                    &self.folder_auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Authorization URL",
                    &self.folder_auth_inputs.oauth_authorization_url,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Access Token URL",
                    &self.folder_auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Redirect URI",
                    &self.folder_auth_inputs.oauth_redirect_uri,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "State",
                    &self.folder_auth_inputs.oauth_state,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Audience",
                    &self.folder_auth_inputs.oauth_audience,
                    cx,
                ))
                .child(folder_auth_select_field(
                    "Token for authorization",
                    self.folder_oauth_token_target_dropdown(auth, cx),
                    cx,
                ))
                .child(self.folder_oauth_pkce_checkbox(auth, cx))
            })
            .when(is_implicit, |el| {
                el.child(folder_auth_input_field(
                    "Client ID",
                    &self.folder_auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Authorization URL",
                    &self.folder_auth_inputs.oauth_authorization_url,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Redirect URI",
                    &self.folder_auth_inputs.oauth_redirect_uri,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "State",
                    &self.folder_auth_inputs.oauth_state,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Audience",
                    &self.folder_auth_inputs.oauth_audience,
                    cx,
                ))
                .child(folder_auth_select_field(
                    "Token for authorization",
                    self.folder_oauth_token_target_dropdown(auth, cx),
                    cx,
                ))
                .child(folder_auth_select_field(
                    "Response Type",
                    self.folder_oauth_response_type_dropdown(auth, cx),
                    cx,
                ))
            })
            .when(is_resource_owner, |el| {
                el.child(folder_auth_input_field(
                    "Client ID",
                    &self.folder_auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Client Secret",
                    &self.folder_auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Access Token URL",
                    &self.folder_auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Audience",
                    &self.folder_auth_inputs.oauth_audience,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Username",
                    &self.folder_auth_inputs.oauth_username,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Password",
                    &self.folder_auth_inputs.oauth_password,
                    cx,
                ))
            })
            .when(is_client_credentials, |el| {
                el.child(folder_auth_input_field(
                    "Client ID",
                    &self.folder_auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Client Secret",
                    &self.folder_auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Access Token URL",
                    &self.folder_auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Audience",
                    &self.folder_auth_inputs.oauth_audience,
                    cx,
                ))
            })
            .child(self.render_folder_oauth_advanced(auth, cx))
    }

    fn folder_oauth_grant_dropdown(
        &self,
        current: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let current = folder_oauth_grant_label(current);
        let entity = cx.entity();
        let item = move |grant: &'static str| {
            let entity = entity.clone();
            PopupMenuItem::new(grant)
                .checked(current == grant)
                .on_click(move |_, _, cx| {
                    entity.update(cx, |this, cx| {
                        this.update_folder_auth(cx, |auth| {
                            auth.oauth_grant_type = grant.to_string();
                        });
                    });
                })
        };
        DropdownButton::new("folder-oauth-grant")
            .button(
                Button::new("folder-oauth-grant-btn")
                    .label(current)
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                menu.item(item("Authorization Code"))
                    .item(item("Implicit"))
                    .item(item("Resource Owner Password Credential"))
                    .item(item("Client Credentials"))
            })
    }

    fn folder_oauth_token_target_dropdown(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let current = folder_default_if_empty(&auth.oauth_token_target, "access_token");
        let entity = cx.entity();
        DropdownButton::new("folder-oauth-token-target")
            .button(
                Button::new("folder-oauth-token-target-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["access_token", "id_token"]
                    .into_iter()
                    .fold(menu, |menu, token| {
                        let entity = entity.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(token)
                                .checked(current == token)
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.update_folder_auth(cx, |auth| {
                                            auth.oauth_token_target = token.to_string();
                                        });
                                    });
                                }),
                        )
                    })
            })
    }

    fn folder_oauth_response_type_dropdown(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let current = folder_default_if_empty(&auth.oauth_response_type, "Access Token");
        let entity = cx.entity();
        DropdownButton::new("folder-oauth-response-type")
            .button(
                Button::new("folder-oauth-response-type-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["Access Token", "ID Token", "Access Token + ID Token"]
                    .into_iter()
                    .fold(menu, |menu, response_type| {
                        let entity = entity.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(response_type)
                                .checked(current == response_type)
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.update_folder_auth(cx, |auth| {
                                            auth.oauth_response_type = response_type.to_string();
                                        });
                                    });
                                }),
                        )
                    })
            })
    }

    fn folder_oauth_send_credentials_dropdown(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let current = folder_default_if_empty(&auth.oauth_send_credentials, "In Request Body");
        let entity = cx.entity();
        DropdownButton::new("folder-oauth-send-credentials")
            .button(
                Button::new("folder-oauth-send-credentials-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["In Request Body", "As Basic Authentication"]
                    .into_iter()
                    .fold(menu, |menu, target| {
                        let entity = entity.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(target)
                                .checked(current == target)
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.update_folder_auth(cx, |auth| {
                                            auth.oauth_send_credentials = target.to_string();
                                        });
                                    });
                                }),
                        )
                    })
            })
    }

    fn folder_oauth_pkce_checkbox(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(
                Checkbox::new("folder-oauth-use-pkce")
                    .checked(auth.oauth_use_pkce)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.update_folder_auth(cx, |auth| {
                            auth.oauth_use_pkce = *checked;
                        });
                    })),
            )
            .child(div().text_sm().child("Use PKCE"))
    }

    fn render_folder_oauth_advanced(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_3()
            .p_3()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child(div().text_sm().child("Advanced"))
            .child(folder_auth_input_field(
                "Scope",
                &self.folder_auth_inputs.oauth_scope,
                cx,
            ))
            .child(folder_auth_input_field(
                "Header Name*",
                &self.folder_auth_inputs.oauth_header_name,
                cx,
            ))
            .child(folder_auth_input_field(
                "Header Prefix",
                &self.folder_auth_inputs.oauth_header_prefix,
                cx,
            ))
            .child(folder_auth_select_field(
                "Send Credentials",
                self.folder_oauth_send_credentials_dropdown(auth, cx),
                cx,
            ))
    }

    pub fn reveal_active_request(
        &mut self,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(req_id) = self.app_state.read(cx).active_request_id.clone() else {
            return;
        };

        let path = {
            let state = self.app_state.read(cx);
            let mut found_path = None;
            for col in &state.workspace.collections {
                let mut current_path = vec![col.id.clone()];
                if find_request_path(&col.items, &req_id, &mut current_path) {
                    found_path = Some(current_path);
                    break;
                }
            }
            found_path
        };

        if let Some(path) = path {
            for id in path {
                self.expanded.insert(id);
            }
        }

        self.selected_id = Some(req_id.clone());
        self.flat_dirty = true;

        if focus {
            let fh = self.focus_handle.clone();
            cx.defer_in(window, move |_, window, cx| {
                fh.focus(window, cx);
            });
        }

        cx.notify();

        let scroll = self.list_scroll.clone();
        cx.defer_in(window, move |this, _, cx| {
            if let Some(pos) = this
                .flat_items(cx)
                .iter()
                .position(|item| item.id == req_id)
            {
                scroll.scroll_to_item(pos, gpui::ScrollStrategy::Nearest);
            }
        });
    }

    fn render_rows(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let flat = self.flat_items(cx);
        let renaming_id = self.renaming_id.clone();
        let selected_id = self.selected_id.clone();
        let rename_input = self.rename_input.clone();
        let panel_entity = cx.entity();

        let mut rows: Vec<AnyElement> = Vec::new();
        for fi in flat
            .into_iter()
            .skip(range.start)
            .take(range.end.saturating_sub(range.start))
        {
            let id = fi.id.clone();
            let id2 = fi.id.clone();
            let id3 = fi.id.clone();
            let id4 = fi.id.clone();
            let id6 = fi.id.clone();
            let id7 = fi.id.clone();
            let id8 = fi.id.clone();
            let id9 = fi.id.clone();
            let display = fi.display.clone();
            let app_click = self.app_state.clone();
            let app_del = self.app_state.clone();
            let app_drop = self.app_state.clone();
            let app_copy = self.app_state.clone();
            let app_dup = self.app_state.clone();
            let app_move = self.app_state.clone();
            let app_new_http = self.app_state.clone();
            let app_new_folder = self.app_state.clone();
            let app_click_row = app_click.clone();
            let focus_handle = self.focus_handle.clone();
            let is_renaming = renaming_id.as_deref() == Some(&fi.id);
            let is_selected = selected_id.as_deref() == Some(&fi.id);
            let depth = fi.depth;
            let is_folder = fi.is_folder;
            let is_col_root = fi.is_collection_root;
            let is_expanded = fi.is_expanded;
            let method_str = fi.method_str.clone();
            let method_color = fi.method_color;
            let row_status = if is_folder {
                None
            } else {
                request_row_status(self.app_state.read(cx), &fi.id)
            };
            let drag_display = if !method_str.is_empty() {
                format!("{} {}", method_str, display)
            } else {
                display.clone()
            };

            let label_content: AnyElement = if is_renaming {
                h_flex()
                    .flex_1()
                    .flex_basis(px(0.))
                    .min_w_0()
                    .h(px(28.))
                    .w_full()
                    .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                        if ev.keystroke.key.as_str() == "escape" {
                            this.commit_rename(window, cx);
                        }
                    }))
                    .child(
                        Input::new(&rename_input)
                            .bordered(false)
                            .focus_bordered(false)
                            .flex_1()
                            .flex_basis(px(0.))
                            .min_w_0()
                            .h(px(28.))
                            .w_full(),
                    )
                    .into_any_element()
            } else {
                h_flex()
                    .flex_1()
                    .gap_1()
                    .items_center()
                    .overflow_hidden()
                    .when(is_folder, |el| {
                        el.child(
                            Icon::new(if is_expanded {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            })
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                        )
                    })
                    .when(!method_str.is_empty(), |el| {
                        el.child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(method_color)
                                .child(method_str.clone()),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(cx.theme().sidebar_foreground)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(display.clone()),
                    )
                    .into_any_element()
            };

            let target_for_drop = if is_col_root {
                format!("root:{}", id)
            } else if is_folder {
                format!("folder:{}", id)
            } else {
                String::new()
            };
            let bg = if is_selected {
                cx.theme().secondary
            } else {
                cx.theme().sidebar
            };

            let row = div()
                .id(SharedString::from(format!("row-{}", id)))
                // Occlude the empty-space background beneath so a right-click on a
                // row only opens the row's own context menu, not both.
                .occlude()
                .pl(px(8. + 16. * depth as f32))
                .pr_2()
                .py_0p5()
                .flex()
                .items_center()
                .gap_1()
                .w_full()
                .bg(bg)
                .hover(|s| s.bg(cx.theme().secondary))
                .on_click(move |_, window, cx| {
                    focus_handle.focus(window, cx);
                    if !is_folder {
                        app_click_row.update(cx, |state, cx| {
                            if state.select_request(&id2) {
                                cx.emit(AppEvent::RequestSelected);
                            }
                        });
                    }
                })
                .on_drag(
                    DragItem {
                        id: id3.clone(),
                        display: drag_display.clone(),
                    },
                    move |item, _offset, _window, cx| {
                        let label = item.display.clone();
                        cx.new(|_| DragPreview(label))
                    },
                )
                .child(label_content)
                .when_some(row_status, |row, status| {
                    row.child(render_request_row_status(status, cx))
                });

            let row = if is_folder {
                row.on_drop::<DragItem>(move |drag, _, cx| {
                    let dragged = drag.id.clone();
                    app_drop.update(cx, |state, cx| {
                        state.move_item_to(&dragged, &target_for_drop);
                        cx.emit(AppEvent::WorkspaceChanged);
                    });
                })
                .drag_over::<DragItem>(|style, _, _, cx| style.bg(cx.theme().primary.opacity(0.15)))
            } else {
                row
            };

            let row = if is_folder {
                let panel_entity = cx.entity();
                let toggle_id = fi.id.clone();
                row.on_click(move |_, _, cx| {
                    panel_entity.update(cx, |this, cx| {
                        this.selected_id = Some(toggle_id.clone());
                        if this.expanded.contains(&toggle_id) {
                            this.expanded.remove(&toggle_id);
                        } else {
                            this.expanded.insert(toggle_id.clone());
                        }
                        this.flat_dirty = true;
                        cx.notify();
                    });
                })
            } else {
                row
            };

            let row = if is_folder {
                let panel_for_rename = panel_entity.clone();
                let panel_for_settings = panel_entity.clone();
                let target_id = id6.clone();
                row.context_menu(move |menu, _, _| {
                    let settings_id = target_id.clone();
                    let target_for_http = target_id.clone();
                    let target_for_folder = target_id.clone();
                    let app_new_http = app_new_http.clone();
                    let app_new_folder = app_new_folder.clone();
                    let panel_for_settings = panel_for_settings.clone();
                    let panel_for_rename = panel_for_rename.clone();
                    let rename_id = target_id.clone();
                    let duplicate_id = target_id.clone();
                    let delete_id = target_id.clone();
                    let app_dup = app_dup.clone();
                    let app_del = app_del.clone();

                    menu.item(PopupMenuItem::new("Folder Settings").on_click(
                        move |_, window, cx| {
                            panel_for_settings.update(cx, |this, cx| {
                                this.open_folder_settings(settings_id.clone(), window, cx);
                            });
                        },
                    ))
                    .separator()
                    .item(
                        PopupMenuItem::new("New HTTP").on_click(move |_, window, cx| {
                            let target = Some(target_for_http.as_str());
                            app_new_http.update(cx, |state, cx| {
                                if let Some(new_id) = state.add_request_to_target(target) {
                                    state.select_request(&new_id);
                                    cx.emit(AppEvent::RequestSelected);
                                }
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                            window.dispatch_action(Box::new(FocusActiveRequest), cx);
                        }),
                    )
                    .item(PopupMenuItem::new("New Folder").on_click(move |_, _, cx| {
                        let target = Some(target_for_folder.as_str());
                        app_new_folder.update(cx, |state, cx| {
                            state.add_folder_to_target(target);
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
                    .separator()
                    .item(PopupMenuItem::new("Rename").on_click(move |_, window, cx| {
                        start_rename(&panel_for_rename, &rename_id, window, cx);
                    }))
                    .item(PopupMenuItem::new("Duplicate").on_click(move |_, _, cx| {
                        app_dup.update(cx, |state, cx| {
                            state.duplicate_item(&duplicate_id);
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
                    .item(PopupMenuItem::new("Move").disabled(true))
                    .separator()
                    .item(PopupMenuItem::new("Delete").on_click(move |_, _, cx| {
                        app_del.update(cx, |state, cx| {
                            state.delete_item(&delete_id);
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
                })
            } else {
                let panel_for_rename = panel_entity.clone();
                row.context_menu(move |menu, window, cx| {
                    let send_id = id4.clone();
                    let copy_id = id7.clone();
                    let rename_id = id8.clone();
                    let duplicate_id = id9.clone();
                    let delete_id = id6.clone();
                    let move_id = id.clone();
                    let app_click = app_click.clone();
                    let app_copy = app_copy.clone();
                    let app_dup = app_dup.clone();
                    let app_move = app_move.clone();
                    let app_del = app_del.clone();
                    let panel_for_rename = panel_for_rename.clone();
                    let move_targets = app_move.read(cx).get_folder_options();

                    menu.item(PopupMenuItem::new("Send").on_click(move |_, window, cx| {
                        app_click.update(cx, |state, cx| {
                            if state.select_request(&send_id) {
                                cx.emit(AppEvent::RequestSelected);
                            }
                        });
                        window.dispatch_action(Box::new(SendRequest), cx);
                    }))
                    .item(
                        PopupMenuItem::new("Copy as Curl").on_click(move |_, _, cx| {
                            if let Some(req) = app_copy.read(cx).find_request(&copy_id) {
                                cx.write_to_clipboard(ClipboardItem::new_string(format_curl(&req)));
                            }
                        }),
                    )
                    .item(PopupMenuItem::new("Rename").on_click(move |_, window, cx| {
                        start_rename(&panel_for_rename, &rename_id, window, cx);
                    }))
                    .item(PopupMenuItem::new("Duplicate").on_click(move |_, _, cx| {
                        app_dup.update(cx, |state, cx| {
                            if let Some(new_id) = state.duplicate_item(&duplicate_id) {
                                state.select_request(&new_id);
                                cx.emit(AppEvent::RequestSelected);
                            }
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
                    .submenu("Move", window, cx, move |mut submenu, _, _| {
                        for (target, label) in move_targets.clone() {
                            let app_move = app_move.clone();
                            let move_id = move_id.clone();
                            submenu = submenu.item(PopupMenuItem::new(label).on_click(
                                move |_, _, cx| {
                                    app_move.update(cx, |state, cx| {
                                        state.move_item_to(&move_id, &target);
                                        cx.emit(AppEvent::WorkspaceChanged);
                                    });
                                },
                            ));
                        }
                        submenu
                    })
                    .separator()
                    .item(PopupMenuItem::new("Delete").on_click(move |_, _, cx| {
                        app_del.update(cx, |state, cx| {
                            state.delete_item(&delete_id);
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
                })
            };

            rows.push(row.into_any_element());
        }

        rows
    }
}

impl Render for CollectionPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let row_count = self.flat_items(cx).len();

        let app_new_http = self.app_state.clone();
        let app_new_folder = self.app_state.clone();
        let selected_for_new = self.selected_id.clone();

        v_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "up" | "down" => {
                        let flat = this.flat_items(cx);
                        if flat.is_empty() {
                            return;
                        }
                        let current_pos = this
                            .selected_id
                            .as_ref()
                            .and_then(|sid| flat.iter().position(|f| f.id == *sid));
                        let new_pos = match event.keystroke.key.as_str() {
                            "up" => match current_pos {
                                Some(0) => flat.len() - 1,
                                Some(pos) => pos - 1,
                                None => flat.len() - 1,
                            },
                            _ => match current_pos {
                                Some(pos) => (pos + 1) % flat.len(),
                                None => 0,
                            },
                        };
                        let item = &flat[new_pos];
                        let item_id = item.id.clone();
                        let is_folder = item.is_folder;
                        this.selected_id = Some(item_id.clone());
                        if !is_folder {
                            this.app_state.update(cx, |state, cx| {
                                if state.select_request(&item_id) {
                                    cx.emit(AppEvent::RequestSelected);
                                }
                            });
                        }
                        this.list_scroll
                            .scroll_to_item(new_pos, gpui::ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    "space" => {
                        let sid = this.selected_id.clone();
                        if let Some(sid) = sid {
                            let is_folder = this
                                .flat_items(cx)
                                .iter()
                                .any(|f| f.id == sid && f.is_folder);
                            if is_folder {
                                if this.expanded.contains(&sid) {
                                    this.expanded.remove(&sid);
                                } else {
                                    this.expanded.insert(sid.clone());
                                }
                                this.flat_dirty = true;
                                cx.notify();
                            }
                        }
                    }
                    "right" => {
                        let sid = this.selected_id.clone();
                        if let Some(sid) = sid {
                            let is_folder = this
                                .flat_items(cx)
                                .iter()
                                .any(|f| f.id == sid && f.is_folder);
                            if is_folder {
                                this.expanded.insert(sid.clone());
                                this.flat_dirty = true;
                                cx.notify();
                            }
                        }
                    }
                    "left" => {
                        let sid = this.selected_id.clone();
                        if let Some(sid) = sid {
                            let is_folder = this
                                .flat_items(cx)
                                .iter()
                                .any(|f| f.id == sid && f.is_folder);
                            if is_folder {
                                this.expanded.remove(&sid);
                                this.flat_dirty = true;
                                cx.notify();
                            }
                        }
                    }
                    "escape" => {
                        if this.renaming_id.is_some() {
                            this.renaming_id = None;
                            let fh = this.focus_handle.clone();
                            cx.defer_in(window, move |_, window, cx| {
                                fh.focus(window, cx);
                            });
                            cx.notify();
                        }
                    }
                    _ => {}
                }
            }))
            .on_action(cx.listener(|this, _: &RenameSelected, window, cx| {
                if window.has_focused_input(cx) {
                    return;
                }
                if this.renaming_id.is_some() {
                    this.commit_rename(window, cx);
                    return;
                }
                if let Some(id) = this.selected_id.clone() {
                    let name = find_name_by_id(this.app_state.read(cx), &id).unwrap_or_default();
                    this.rename_input
                        .update(cx, |s, cx| s.set_value(name, window, cx));
                    this.renaming_id = Some(id);
                    // focus the rename input
                    let fh = this.rename_input.read(cx).focus_handle(cx);
                    fh.focus(window, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(
                |this, _: &crate::ui::actions::FocusActiveRequest, window, cx| {
                    this.reveal_active_request(true, window, cx);
                },
            ))
            // Header
            .child(
                h_flex()
                    .px_3()
                    .py_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .items_center()
                    .child(Input::new(&self.search_input).flex_1().h(px(28.))),
            )
            // List
            .child(
                div()
                    .id("collection-list-container")
                    .flex_1()
                    .relative()
                    // Background to catch right-clicks on empty space
                    .child(div().absolute().inset_0().context_menu(move |menu, _, _| {
                        let app_new_http = app_new_http.clone();
                        let app_new_folder = app_new_folder.clone();
                        let target_http = selected_for_new.clone();
                        let target_folder = selected_for_new.clone();

                        menu.item(
                            PopupMenuItem::new("New HTTP").on_click(move |_, window, cx| {
                                app_new_http.update(cx, |state, cx| {
                                    if let Some(new_id) =
                                        state.add_request_to_target(target_http.as_deref())
                                    {
                                        state.select_request(&new_id);
                                        cx.emit(AppEvent::RequestSelected);
                                    }
                                    cx.emit(AppEvent::WorkspaceChanged);
                                });
                                window.dispatch_action(Box::new(FocusActiveRequest), cx);
                            }),
                        )
                        .item(PopupMenuItem::new("New Folder").on_click(move |_, _, cx| {
                            app_new_folder.update(cx, |state, cx| {
                                state.add_folder_to_target(target_folder.as_deref());
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                        }))
                    }))
                    // The actual list over it
                    .child(
                        uniform_list(
                            "collection-list-scroll",
                            row_count,
                            cx.processor(move |this: &mut Self, range, _window, cx| {
                                this.render_rows(range, cx)
                            }),
                        )
                        .track_scroll(&self.list_scroll)
                        .size_full(),
                    ),
            )
    }
}

fn start_rename(panel: &Entity<CollectionPanel>, id: &str, window: &mut Window, cx: &mut App) {
    let id = id.to_string();
    panel.update(cx, |this, cx| {
        let name = find_name_by_id(this.app_state.read(cx), &id).unwrap_or_default();
        this.rename_input
            .update(cx, |s, cx| s.set_value(name, window, cx));
        this.renaming_id = Some(id);
        this.selected_id = this.renaming_id.clone();
        let fh = this.rename_input.read(cx).focus_handle(cx);
        fh.focus(window, cx);
        cx.notify();
    });
}

fn format_curl(req: &crate::models::ApiRequest) -> String {
    let mut parts = vec![
        "curl".to_string(),
        "-X".to_string(),
        shell_quote(req.method.as_str()),
        shell_quote(&req.url),
    ];

    for header in &req.headers {
        if header.enabled && !header.key.is_empty() {
            parts.push("-H".to_string());
            parts.push(shell_quote(&format!("{}: {}", header.key, header.value)));
        }
    }

    let body = if matches!(req.body.body_type, crate::models::BodyType::UrlEncoded)
        && !req.body.urlencoded.is_empty()
    {
        crate::http::build_urlencoded_body(&req.body.urlencoded)
    } else {
        req.body.content.clone()
    };

    if !body.is_empty() {
        parts.push("-d".to_string());
        parts.push(shell_quote(&body));
    }

    parts.join(" ")
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/:._?=&-%{}".contains(c))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn find_name_by_id(state: &AppState, id: &str) -> Option<String> {
    for col in &state.workspace.collections {
        if col.id == id {
            return Some(col.name.clone());
        }
        if let Some(n) = find_name_in_items(&col.items, id) {
            return Some(n);
        }
    }
    None
}

fn find_name_in_items(items: &[CollectionItem], id: &str) -> Option<String> {
    for item in items {
        match item {
            CollectionItem::Request(r) if r.id == id => return Some(r.name.clone()),
            CollectionItem::Folder(f) if f.id == id => return Some(f.name.clone()),
            CollectionItem::Folder(f) => {
                if let Some(n) = find_name_in_items(&f.items, id) {
                    return Some(n);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_request_path(
    items: &[CollectionItem],
    request_id: &str,
    current_path: &mut Vec<String>,
) -> bool {
    for item in items {
        match item {
            CollectionItem::Request(request) if request.id == request_id => return true,
            CollectionItem::Folder(folder) => {
                current_path.push(folder.id.clone());
                if find_request_path(&folder.items, request_id, current_path) {
                    return true;
                }
                current_path.pop();
            }
            _ => {}
        }
    }
    false
}

fn find_folder_in_items<'a>(items: &'a [CollectionItem], id: &str) -> Option<&'a Folder> {
    for item in items {
        match item {
            CollectionItem::Folder(folder) if folder.id == id => return Some(folder),
            CollectionItem::Folder(folder) => {
                if let Some(found) = find_folder_in_items(&folder.items, id) {
                    return Some(found);
                }
            }
            CollectionItem::Request(_) => {}
        }
    }
    None
}

fn find_folder_in_items_mut<'a>(
    items: &'a mut [CollectionItem],
    id: &str,
) -> Option<&'a mut Folder> {
    for item in items {
        if let CollectionItem::Folder(folder) = item {
            if folder.id == id {
                return Some(folder);
            }
            {
                if let Some(found) = find_folder_in_items_mut(&mut folder.items, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn collect_kv_rows(rows: &[KvRow], cx: &App) -> Vec<KeyValue> {
    rows.iter()
        .filter_map(|row| {
            let key = row.key.read(cx).value().to_string();
            let value = row.value.read(cx).value().to_string();
            (!key.is_empty() || !value.is_empty()).then(|| KeyValue {
                id: uuid::Uuid::new_v4().to_string(),
                key,
                value,
                enabled: row.enabled,
                description: String::new(),
            })
        })
        .collect()
}

fn label_text(label: &'static str, cx: &mut Context<CollectionPanel>) -> AnyElement {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(label)
        .into_any_element()
}

fn folder_default_if_empty(value: &str, default: &str) -> String {
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_string()
    }
}

fn folder_oauth_grant_label(value: &str) -> &'static str {
    match value {
        "Implicit" => "Implicit",
        "Resource Owner Password Credential" | "Password" => {
            "Resource Owner Password Credential"
        }
        "Client Credentials" => "Client Credentials",
        _ => "Authorization Code",
    }
}

fn folder_auth_select_field(
    label: &'static str,
    child: impl IntoElement,
    cx: &App,
) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(child)
}

fn folder_auth_input_field(
    label: &'static str,
    input: &Entity<InputState>,
    cx: &App,
) -> impl IntoElement {
    folder_auth_select_field(label, Input::new(input).w_full(), cx)
}

fn folder_auth_multiline_field(
    label: &'static str,
    input: &Entity<InputState>,
    cx: &App,
) -> impl IntoElement {
    folder_auth_select_field(label, Input::new(input).h(px(82.)).w_full(), cx)
}

fn folder_kv_header(cx: &mut Context<CollectionPanel>) -> AnyElement {
    h_flex()
        .gap_2()
        .items_center()
        .px_1()
        .child(div().w(px(18.)))
        .child(
            div()
                .flex_1()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Name"),
        )
        .child(
            div()
                .flex_1()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Value"),
        )
        .child(div().w(px(34.)))
        .into_any_element()
}

fn render_folder_kv_row(
    row: &KvRow,
    index: usize,
    variables: bool,
    cx: &mut Context<CollectionPanel>,
) -> AnyElement {
    let key = row.key.clone();
    let value = row.value.clone();
    let id = if variables {
        format!("remove-folder-var-{index}")
    } else {
        format!("remove-folder-header-{index}")
    };

    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .w(px(18.))
                .h(px(18.))
                .rounded_sm()
                .border_1()
                .border_color(cx.theme().primary)
                .bg(cx.theme().primary)
                .child(
                    Icon::new(IconName::Check)
                        .xsmall()
                        .text_color(cx.theme().background),
                ),
        )
        .child(div().flex_1().child(Input::new(&key).h(px(32.))))
        .child(div().flex_1().child(Input::new(&value).h(px(32.))))
        .child(
            Button::new(SharedString::from(id))
                .icon(IconName::Close)
                .ghost()
                .small()
                .on_click(cx.listener(move |this, _, _, cx| {
                    if variables {
                        if index < this.folder_variable_rows.len() {
                            this.folder_variable_rows.remove(index);
                        }
                    } else if index < this.folder_header_rows.len() {
                        this.folder_header_rows.remove(index);
                    }
                    cx.notify();
                })),
        )
        .into_any_element()
}

fn request_row_status(state: &AppState, request_id: &str) -> Option<RequestRowStatus> {
    let activity = state.request_activity.get(request_id);
    if activity
        .map(|activity| activity.is_loading())
        .unwrap_or(false)
    {
        return Some(RequestRowStatus::Loading);
    }

    if let Some(status) = activity.and_then(|activity| activity.last_status) {
        return Some(RequestRowStatus::Status(status));
    }

    if activity
        .and_then(|activity| activity.last_error.as_ref())
        .is_some()
    {
        return Some(RequestRowStatus::Error);
    }

    state
        .workspace
        .response_cache
        .get(request_id)
        .and_then(|history| history.last())
        .map(|response| RequestRowStatus::Status(response.status))
}

fn render_request_row_status(
    status: RequestRowStatus,
    cx: &mut Context<CollectionPanel>,
) -> AnyElement {
    match status {
        RequestRowStatus::Loading => Spinner::new()
            .xsmall()
            .color(cx.theme().primary)
            .into_any_element(),
        RequestRowStatus::Status(code) => {
            let color: Hsla = if (200..300).contains(&code) {
                rgb(0x00C853).into()
            } else if (300..400).contains(&code) {
                rgb(0xFF9800).into()
            } else {
                rgb(0xFF4081).into()
            };
            div()
                .flex_none()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(color)
                .child(code.to_string())
                .into_any_element()
        }
        RequestRowStatus::Error => div()
            .flex_none()
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .text_color(Hsla::from(rgb(0xFF4081)))
            .child("ERR")
            .into_any_element(),
    }
}

fn method_color(method: &str) -> Hsla {
    match method {
        "GET" => rgb(0x4CAF50).into(),
        "POST" => rgb(0x2196F3).into(),
        "PUT" => rgb(0xFF9800).into(),
        "PATCH" => rgb(0x9C27B0).into(),
        "DELETE" => rgb(0xF44336).into(),
        _ => rgb(0x607D8B).into(),
    }
}
