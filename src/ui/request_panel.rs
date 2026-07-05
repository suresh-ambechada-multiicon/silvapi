use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, Sizable as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::PopupMenuItem,
    tab::{Tab, TabBar},
    v_flex,
};

use crate::{
    models::{
        AuthConfig, AuthType, BodyType, FormDataPart, FormDataPartKind, HttpMethod, KeyValue,
    },
    state::{AppEvent, AppState},
};

const MAX_RESPONSE_HISTORY_PER_REQUEST: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestTab {
    Body,
    Params,
    Headers,
    Auth,
    Info,
}

impl RequestTab {
    fn index(&self) -> usize {
        match self {
            RequestTab::Body => 0,
            RequestTab::Params => 1,
            RequestTab::Headers => 2,
            RequestTab::Auth => 3,
            RequestTab::Info => 4,
        }
    }

    fn from_index(i: usize) -> Self {
        match i {
            0 => RequestTab::Body,
            1 => RequestTab::Params,
            2 => RequestTab::Headers,
            3 => RequestTab::Auth,
            _ => RequestTab::Info,
        }
    }
}

pub struct RequestPanel {
    app_state: Entity<AppState>,
    active_tab: RequestTab,
    url_input: Entity<InputState>,
    body_editor: Entity<InputState>,
    auth_inputs: AuthInputs,
    url_variable_rows: Vec<UrlVariableRow>,
    hovered_url_variable: Option<String>,
    hovered_url_path_param: Option<String>,
    path_param_keys: Vec<String>,
    // editable rows: (key_input, value_input)
    param_rows: Vec<(Entity<InputState>, Entity<InputState>, bool)>,
    header_rows: Vec<(Entity<InputState>, Entity<InputState>, bool)>,
    urlencoded_rows: Vec<(Entity<InputState>, Entity<InputState>, bool)>,
    multipart_rows: Vec<MultipartRow>,
    row_subs: Vec<Subscription>,
    url_variable_subs: Vec<Subscription>,
    _subs: Vec<Subscription>,
    focus_handle: gpui::FocusHandle,
}

struct MultipartRow {
    id: String,
    name: Entity<InputState>,
    value: Entity<InputState>,
}

struct UrlVariableRow {
    name: String,
    value: Entity<InputState>,
}

struct AuthInputs {
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

impl AuthInputs {
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

impl gpui::Focusable for RequestPanel {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl RequestPanel {
    fn input_line(value: impl AsRef<str>) -> String {
        value.as_ref().replace(['\r', '\n'], " ")
    }

    fn new_input(
        window: &mut Window,
        cx: &mut Context<Self>,
        placeholder: &'static str,
    ) -> Entity<InputState> {
        cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
    }

    fn new_multiline_input(
        window: &mut Window,
        cx: &mut Context<Self>,
        placeholder: &'static str,
        language: &'static str,
    ) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor(language)
                .placeholder(placeholder)
        })
    }

    fn make_auth_inputs(window: &mut Window, cx: &mut Context<Self>) -> AuthInputs {
        AuthInputs {
            bearer_token: Self::new_input(window, cx, "Token"),
            bearer_prefix: Self::new_input(window, cx, "Bearer"),
            basic_username: Self::new_input(window, cx, "Username"),
            basic_password: Self::new_input(window, cx, "Password"),
            api_key_name: Self::new_input(window, cx, "Header name"),
            api_key_value: Self::new_input(window, cx, "API key"),
            aws_access_key_id: Self::new_input(window, cx, "Access key ID"),
            aws_secret_access_key: Self::new_input(window, cx, "Secret access key"),
            aws_service: Self::new_input(window, cx, "sts"),
            aws_region: Self::new_input(window, cx, "us-east-1"),
            aws_session_token: Self::new_input(window, cx, "Session token"),
            jwt_secret: Self::new_input(window, cx, "Secret or private key"),
            jwt_payload: Self::new_multiline_input(window, cx, "Payload", "json"),
            oauth_client_id: Self::new_input(window, cx, "Client ID"),
            oauth_client_secret: Self::new_input(window, cx, "Client secret"),
            oauth_authorization_url: Self::new_input(window, cx, "Authorization URL"),
            oauth_access_token_url: Self::new_input(window, cx, "Access token URL"),
            oauth_redirect_uri: Self::new_input(window, cx, "Redirect URI"),
            oauth_state: Self::new_input(window, cx, "State"),
            oauth_audience: Self::new_input(window, cx, "Audience"),
            oauth_username: Self::new_input(window, cx, "Username"),
            oauth_password: Self::new_input(window, cx, "Password"),
            oauth_scope: Self::new_input(window, cx, "Scope"),
            oauth_header_name: Self::new_input(window, cx, "Authorization"),
            oauth_header_prefix: Self::new_input(window, cx, "Bearer"),
        }
    }

    fn subscribe_sync_input(
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(input, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        })
    }

    pub fn new(app_state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let url_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL or paste text..."));
        let body_editor = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor("json")
                .soft_wrap(false)
                .placeholder("Request body")
        });
        let auth_inputs = Self::make_auth_inputs(window, cx);
        let auth_subs = auth_inputs
            .all()
            .into_iter()
            .map(|input| Self::subscribe_sync_input(&input, window, cx))
            .collect::<Vec<_>>();

        let focus_handle = cx.focus_handle();

        let request_sub = cx.subscribe_in(&app_state, window, {
            let url_input = url_input.clone();
            let body_editor = body_editor.clone();
            move |this, app_state, ev: &AppEvent, window, cx| {
                if matches!(ev, AppEvent::LoadingChanged) {
                    cx.notify();
                }

                if matches!(ev, AppEvent::RequestSelected | AppEvent::WorkspaceChanged) {
                    if let Some(req) = app_state.read(cx).active_request.clone() {
                        let params = req.params.clone();
                        let url = Self::normalize_query_param_placeholders(&req.url, &params);
                        let url_changed = url != req.url;
                        let path_param_keys = Self::url_param_keys(&url);
                        let body = req.body.content.clone();
                        let headers = req.headers.clone();
                        let urlencoded = Self::urlencoded_fields_for_body(&req.body);
                        let form_data = req.body.form_data.clone();
                        let auth = req.auth.clone();

                        url_input.update(cx, |state, cx| {
                            state.set_value(url, window, cx);
                        });
                        body_editor.update(cx, |state, cx| {
                            state.set_value(body, window, cx);
                        });

                        // Rebuild param/header input rows
                        this.row_subs.clear();
                        this.param_rows = params
                            .iter()
                            .map(|kv| this.make_kv_row(&kv.key, &kv.value, kv.enabled, window, cx))
                            .collect();
                        this.path_param_keys = path_param_keys;
                        let params_changed = this.sync_path_param_rows(window, cx);

                        this.header_rows = headers
                            .iter()
                            .map(|kv| this.make_kv_row(&kv.key, &kv.value, kv.enabled, window, cx))
                            .collect();
                        this.urlencoded_rows = urlencoded
                            .iter()
                            .map(|kv| {
                                this.make_urlencoded_row(&kv.key, &kv.value, kv.enabled, window, cx)
                            })
                            .collect();
                        this.multipart_rows = form_data
                            .iter()
                            .map(|part| this.make_multipart_row(part, window, cx))
                            .collect();
                        this.load_auth_inputs(&auth, window, cx);
                        this.refresh_url_variable_rows(window, cx);
                        if params_changed || url_changed {
                            this.persist_url_and_param_rows(cx);
                        }

                        cx.notify();
                    }
                }
            }
        });

        let url_sub = cx.subscribe_in(
            &url_input,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.sync_active_request(window, cx);
                }
            },
        );

        let body_sub = cx.subscribe_in(
            &body_editor,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.sync_active_request(window, cx);
                }
            },
        );

        let mut this = Self {
            app_state,
            active_tab: RequestTab::Body,
            url_input,
            body_editor,
            auth_inputs,
            url_variable_rows: vec![],
            hovered_url_variable: None,
            hovered_url_path_param: None,
            path_param_keys: vec![],
            param_rows: vec![],
            header_rows: vec![],
            urlencoded_rows: vec![],
            multipart_rows: vec![],
            row_subs: vec![],
            url_variable_subs: vec![],
            _subs: {
                let mut subs = vec![request_sub, url_sub, body_sub];
                subs.extend(auth_subs);
                subs
            },
            focus_handle,
        };

        if let Some(req) = this.app_state.read(cx).active_request.clone() {
            let url = Self::normalize_query_param_placeholders(&req.url, &req.params);
            let url_changed = url != req.url;
            let path_param_keys = Self::url_param_keys(&url);
            this.url_input
                .update(cx, |state, cx| state.set_value(url, window, cx));
            this.body_editor.update(cx, |state, cx| {
                state.set_value(req.body.content.clone(), window, cx)
            });
            this.param_rows = req
                .params
                .iter()
                .map(|kv| this.make_kv_row(&kv.key, &kv.value, kv.enabled, window, cx))
                .collect();
            this.path_param_keys = path_param_keys;
            let params_changed = this.sync_path_param_rows(window, cx);
            this.header_rows = req
                .headers
                .iter()
                .map(|kv| this.make_kv_row(&kv.key, &kv.value, kv.enabled, window, cx))
                .collect();
            this.urlencoded_rows = Self::urlencoded_fields_for_body(&req.body)
                .iter()
                .map(|kv| this.make_urlencoded_row(&kv.key, &kv.value, kv.enabled, window, cx))
                .collect();
            this.multipart_rows = req
                .body
                .form_data
                .iter()
                .map(|part| this.make_multipart_row(part, window, cx))
                .collect();
            this.load_auth_inputs(&req.auth, window, cx);
            this.refresh_url_variable_rows(window, cx);
            if params_changed || url_changed {
                this.persist_url_and_param_rows(cx);
            }
        }

        this
    }

    fn make_kv_row(
        &mut self,
        key: &str,
        value: &str,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<InputState>, Entity<InputState>, bool) {
        let k = cx.new(|cx| InputState::new(window, cx).placeholder("Key"));
        let v = cx.new(|cx| InputState::new(window, cx).placeholder("Value"));
        k.update(cx, |s, cx| s.set_value(Self::input_line(key), window, cx));
        v.update(cx, |s, cx| s.set_value(Self::input_line(value), window, cx));

        let k_sub = cx.subscribe_in(&k, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        });
        let v_sub = cx.subscribe_in(&v, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        });
        self.row_subs.push(k_sub);
        self.row_subs.push(v_sub);
        (k, v, enabled)
    }

    fn make_urlencoded_row(
        &mut self,
        key: &str,
        value: &str,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<InputState>, Entity<InputState>, bool) {
        let k = cx.new(|cx| InputState::new(window, cx).placeholder("entry_name"));
        let v = cx.new(|cx| InputState::new(window, cx).placeholder("value"));
        k.update(cx, |s, cx| s.set_value(Self::input_line(key), window, cx));
        v.update(cx, |s, cx| s.set_value(Self::input_line(value), window, cx));

        let k_sub = cx.subscribe_in(&k, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        });
        let v_sub = cx.subscribe_in(&v, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        });
        self.row_subs.push(k_sub);
        self.row_subs.push(v_sub);
        (k, v, enabled)
    }

    fn make_url_variable_row(
        &mut self,
        name: &str,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> UrlVariableRow {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("value"));
        input.update(cx, |state, cx| {
            state.set_value(Self::input_line(value), window, cx);
        });

        let sub = cx.subscribe_in(&input, window, {
            let name = name.to_string();
            move |this, input, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) {
                    let value = input.read(cx).value().to_string();
                    this.update_global_variable(&name, value, cx);
                    cx.notify();
                }
            }
        });
        self.url_variable_subs.push(sub);

        UrlVariableRow {
            name: name.to_string(),
            value: input,
        }
    }

    fn refresh_url_variable_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = self.url_input.read(cx).value().to_string();
        let names = Self::extract_variable_names(&url);
        let current_names = self
            .url_variable_rows
            .iter()
            .map(|row| row.name.clone())
            .collect::<Vec<_>>();

        if names == current_names {
            for row in &self.url_variable_rows {
                if let Some(value) = self.variable_value(&row.name, cx) {
                    let value = Self::input_line(value);
                    if row.value.read(cx).value() != value {
                        row.value
                            .update(cx, |state, cx| state.set_value(value, window, cx));
                    }
                }
            }
            return;
        }

        self.url_variable_subs.clear();
        self.url_variable_rows = names
            .iter()
            .map(|name| {
                let value = self.variable_value(name, cx).unwrap_or_default();
                self.make_url_variable_row(name, &value, window, cx)
            })
            .collect();
    }

    fn extract_variable_names(text: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut rest = text;

        while let Some(start) = rest.find("{{") {
            let after_start = &rest[start + 2..];
            let Some(end) = after_start.find("}}") else {
                break;
            };
            let name = after_start[..end].trim();
            if Self::is_url_variable_name(name) && !names.iter().any(|existing| existing == name) {
                names.push(name.to_string());
            }
            rest = &after_start[end + 2..];
        }

        names
    }

    fn is_url_variable_name(name: &str) -> bool {
        !name.is_empty()
            && !name.starts_with('$')
            && name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
    }

    fn is_url_path_param_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
    }

    fn is_url_path_param_start(url: &str, colon_index: usize) -> bool {
        let next_is_name = url[colon_index + 1..]
            .chars()
            .next()
            .map(Self::is_url_path_param_char)
            .unwrap_or(false);

        if !next_is_name {
            return false;
        }

        colon_index == 0
            || matches!(
                url[..colon_index].chars().next_back(),
                Some('/') | Some('=')
            )
    }

    fn url_param_keys(url: &str) -> Vec<String> {
        let mut keys = crate::path_params::extract_path_params(url);
        for key in crate::path_params::extract_query_value_params(url) {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys
    }

    fn normalize_query_param_placeholders(url: &str, params: &[KeyValue]) -> String {
        let (without_fragment, fragment) = match url.split_once('#') {
            Some((base, fragment)) => (base, Some(fragment)),
            None => (url, None),
        };
        let Some((base, query)) = without_fragment.split_once('?') else {
            return url.to_string();
        };

        let mut changed = false;
        let pairs = query
            .split('&')
            .map(|pair| {
                let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
                let key = Self::decode_urlencoded_component(raw_key);
                if raw_value.is_empty()
                    && Self::can_use_query_placeholder(&key)
                    && params.iter().any(|param| param.key == key)
                {
                    changed = true;
                    format!("{raw_key}=:{key}")
                } else {
                    pair.to_string()
                }
            })
            .collect::<Vec<_>>();

        if !changed {
            return url.to_string();
        }

        let mut out = format!("{base}?{}", pairs.join("&"));
        if let Some(fragment) = fragment {
            out.push('#');
            out.push_str(fragment);
        }
        out
    }

    fn can_use_query_placeholder(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    }

    fn variable_value(&self, name: &str, cx: &App) -> Option<String> {
        let state = self.app_state.read(cx);
        state
            .workspace
            .variables
            .iter()
            .find(|var| var.name == name)
            .map(|var| var.value.clone())
            .or_else(|| {
                let request_id = state.active_request_id.as_deref()?;
                state
                    .workspace
                    .collections
                    .iter()
                    .find(|collection| collection_contains_request(collection, request_id))
                    .and_then(|collection| {
                        collection
                            .variables
                            .iter()
                            .find(|var| var.name == name)
                            .map(|var| var.value.clone())
                    })
            })
    }

    fn path_param_value(&self, name: &str, cx: &App) -> Option<String> {
        self.param_rows
            .iter()
            .find(|(key, _, enabled)| *enabled && key.read(cx).value().as_ref() == name)
            .map(|(_, value, _)| value.read(cx).value().to_string())
            .or_else(|| {
                self.app_state
                    .read(cx)
                    .active_request
                    .as_ref()
                    .and_then(|request| {
                        request
                            .params
                            .iter()
                            .find(|param| param.enabled && param.key == name)
                            .map(|param| param.value.clone())
                    })
            })
    }

    fn update_global_variable(&self, name: &str, value: String, cx: &mut Context<Self>) {
        self.app_state.update(cx, |state, cx| {
            if let Some(var) = state
                .workspace
                .variables
                .iter_mut()
                .find(|var| var.name == name)
            {
                var.value = value;
                var.enabled = true;
            } else {
                state
                    .workspace
                    .variables
                    .push(crate::models::Variable::new(name, value));
            }
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    fn make_multipart_row(
        &mut self,
        part: &FormDataPart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> MultipartRow {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("name"));
        let value_placeholder = match part.kind {
            FormDataPartKind::Text => "value",
            FormDataPartKind::File => "file path",
        };
        let value = cx.new(|cx| InputState::new(window, cx).placeholder(value_placeholder));
        name.update(cx, |s, cx| {
            s.set_value(Self::input_line(&part.name), window, cx)
        });
        value.update(cx, |s, cx| {
            s.set_value(Self::input_line(&part.value), window, cx)
        });

        let name_sub = cx.subscribe_in(&name, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        });
        let value_sub = cx.subscribe_in(&value, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        });
        self.row_subs.push(name_sub);
        self.row_subs.push(value_sub);

        MultipartRow {
            id: part.id.clone(),
            name,
            value,
        }
    }

    fn load_auth_inputs(&self, auth: &AuthConfig, window: &mut Window, cx: &mut Context<Self>) {
        self.auth_inputs.bearer_token.update(cx, |s, cx| {
            s.set_value(auth.bearer_token.clone(), window, cx)
        });
        self.auth_inputs.bearer_prefix.update(cx, |s, cx| {
            s.set_value(default_if_empty(&auth.bearer_prefix, "Bearer"), window, cx)
        });
        self.auth_inputs.basic_username.update(cx, |s, cx| {
            s.set_value(auth.basic_username.clone(), window, cx)
        });
        self.auth_inputs.basic_password.update(cx, |s, cx| {
            s.set_value(auth.basic_password.clone(), window, cx)
        });
        self.auth_inputs.api_key_name.update(cx, |s, cx| {
            s.set_value(auth.api_key_name.clone(), window, cx)
        });
        self.auth_inputs.api_key_value.update(cx, |s, cx| {
            s.set_value(auth.api_key_value.clone(), window, cx)
        });
        self.auth_inputs.aws_access_key_id.update(cx, |s, cx| {
            s.set_value(auth.aws_access_key_id.clone(), window, cx)
        });
        self.auth_inputs.aws_secret_access_key.update(cx, |s, cx| {
            s.set_value(auth.aws_secret_access_key.clone(), window, cx)
        });
        self.auth_inputs.aws_service.update(cx, |s, cx| {
            s.set_value(default_if_empty(&auth.aws_service, "sts"), window, cx)
        });
        self.auth_inputs.aws_region.update(cx, |s, cx| {
            s.set_value(default_if_empty(&auth.aws_region, "us-east-1"), window, cx)
        });
        self.auth_inputs.aws_session_token.update(cx, |s, cx| {
            s.set_value(auth.aws_session_token.clone(), window, cx)
        });
        self.auth_inputs
            .jwt_secret
            .update(cx, |s, cx| s.set_value(auth.jwt_secret.clone(), window, cx));
        self.auth_inputs.jwt_payload.update(cx, |s, cx| {
            s.set_value(
                default_if_empty(&auth.jwt_payload, "{\n  \"foo\": \"bar\"\n}"),
                window,
                cx,
            )
        });
        self.auth_inputs.oauth_client_id.update(cx, |s, cx| {
            s.set_value(auth.oauth_client_id.clone(), window, cx)
        });
        self.auth_inputs.oauth_client_secret.update(cx, |s, cx| {
            s.set_value(auth.oauth_client_secret.clone(), window, cx)
        });
        self.auth_inputs
            .oauth_authorization_url
            .update(cx, |s, cx| {
                s.set_value(
                    default_if_empty(
                        &auth.oauth_authorization_url,
                        "https://github.com/login/oauth/authorize",
                    ),
                    window,
                    cx,
                )
            });
        self.auth_inputs.oauth_access_token_url.update(cx, |s, cx| {
            s.set_value(
                default_if_empty(
                    &auth.oauth_access_token_url,
                    "https://github.com/login/oauth/access_token",
                ),
                window,
                cx,
            )
        });
        self.auth_inputs.oauth_redirect_uri.update(cx, |s, cx| {
            s.set_value(auth.oauth_redirect_uri.clone(), window, cx)
        });
        self.auth_inputs.oauth_state.update(cx, |s, cx| {
            s.set_value(auth.oauth_state.clone(), window, cx)
        });
        self.auth_inputs.oauth_audience.update(cx, |s, cx| {
            s.set_value(auth.oauth_audience.clone(), window, cx)
        });
        self.auth_inputs.oauth_username.update(cx, |s, cx| {
            s.set_value(auth.oauth_username.clone(), window, cx)
        });
        self.auth_inputs.oauth_password.update(cx, |s, cx| {
            s.set_value(auth.oauth_password.clone(), window, cx)
        });
        self.auth_inputs.oauth_scope.update(cx, |s, cx| {
            s.set_value(auth.oauth_scope.clone(), window, cx)
        });
        self.auth_inputs.oauth_header_name.update(cx, |s, cx| {
            s.set_value(
                default_if_empty(&auth.oauth_header_name, "Authorization"),
                window,
                cx,
            )
        });
        self.auth_inputs.oauth_header_prefix.update(cx, |s, cx| {
            s.set_value(
                default_if_empty(&auth.oauth_header_prefix, "Bearer"),
                window,
                cx,
            )
        });
    }

    fn sync_auth_inputs_to_config(&self, auth: &mut AuthConfig, cx: &App) {
        auth.bearer_token = self.auth_inputs.bearer_token.read(cx).value().to_string();
        auth.bearer_prefix = self.auth_inputs.bearer_prefix.read(cx).value().to_string();
        auth.basic_username = self.auth_inputs.basic_username.read(cx).value().to_string();
        auth.basic_password = self.auth_inputs.basic_password.read(cx).value().to_string();
        auth.api_key_name = self.auth_inputs.api_key_name.read(cx).value().to_string();
        auth.api_key_value = self.auth_inputs.api_key_value.read(cx).value().to_string();
        auth.aws_access_key_id = self
            .auth_inputs
            .aws_access_key_id
            .read(cx)
            .value()
            .to_string();
        auth.aws_secret_access_key = self
            .auth_inputs
            .aws_secret_access_key
            .read(cx)
            .value()
            .to_string();
        auth.aws_service = self.auth_inputs.aws_service.read(cx).value().to_string();
        auth.aws_region = self.auth_inputs.aws_region.read(cx).value().to_string();
        auth.aws_session_token = self
            .auth_inputs
            .aws_session_token
            .read(cx)
            .value()
            .to_string();
        auth.jwt_secret = self.auth_inputs.jwt_secret.read(cx).value().to_string();
        auth.jwt_payload = self.auth_inputs.jwt_payload.read(cx).value().to_string();
        auth.oauth_client_id = self
            .auth_inputs
            .oauth_client_id
            .read(cx)
            .value()
            .to_string();
        auth.oauth_client_secret = self
            .auth_inputs
            .oauth_client_secret
            .read(cx)
            .value()
            .to_string();
        auth.oauth_authorization_url = self
            .auth_inputs
            .oauth_authorization_url
            .read(cx)
            .value()
            .to_string();
        auth.oauth_access_token_url = self
            .auth_inputs
            .oauth_access_token_url
            .read(cx)
            .value()
            .to_string();
        auth.oauth_redirect_uri = self
            .auth_inputs
            .oauth_redirect_uri
            .read(cx)
            .value()
            .to_string();
        auth.oauth_state = self.auth_inputs.oauth_state.read(cx).value().to_string();
        auth.oauth_audience = self.auth_inputs.oauth_audience.read(cx).value().to_string();
        auth.oauth_username = self.auth_inputs.oauth_username.read(cx).value().to_string();
        auth.oauth_password = self.auth_inputs.oauth_password.read(cx).value().to_string();
        auth.oauth_scope = self.auth_inputs.oauth_scope.read(cx).value().to_string();
        auth.oauth_header_name = self
            .auth_inputs
            .oauth_header_name
            .read(cx)
            .value()
            .to_string();
        auth.oauth_header_prefix = self
            .auth_inputs
            .oauth_header_prefix
            .read(cx)
            .value()
            .to_string();
    }

    pub fn has_focus(&self, window: &Window, cx: &App) -> bool {
        if self.focus_handle.contains_focused(window, cx) {
            return true;
        }
        if self
            .url_input
            .read(cx)
            .focus_handle(cx)
            .contains_focused(window, cx)
            || self
                .body_editor
                .read(cx)
                .focus_handle(cx)
                .contains_focused(window, cx)
        {
            return true;
        }
        for input in self.auth_inputs.all() {
            if input.read(cx).focus_handle(cx).contains_focused(window, cx) {
                return true;
            }
        }
        for row in &self.url_variable_rows {
            if row
                .value
                .read(cx)
                .focus_handle(cx)
                .contains_focused(window, cx)
            {
                return true;
            }
        }
        for (k, v, _) in &self.param_rows {
            if k.read(cx).focus_handle(cx).contains_focused(window, cx)
                || v.read(cx).focus_handle(cx).contains_focused(window, cx)
            {
                return true;
            }
        }
        for (k, v, _) in &self.header_rows {
            if k.read(cx).focus_handle(cx).contains_focused(window, cx)
                || v.read(cx).focus_handle(cx).contains_focused(window, cx)
            {
                return true;
            }
        }
        for (k, v, _) in &self.urlencoded_rows {
            if k.read(cx).focus_handle(cx).contains_focused(window, cx)
                || v.read(cx).focus_handle(cx).contains_focused(window, cx)
            {
                return true;
            }
        }
        for row in &self.multipart_rows {
            if row
                .name
                .read(cx)
                .focus_handle(cx)
                .contains_focused(window, cx)
                || row
                    .value
                    .read(cx)
                    .focus_handle(cx)
                    .contains_focused(window, cx)
            {
                return true;
            }
        }
        false
    }

    pub fn focus_url(&self, window: &mut Window, cx: &mut Context<Self>) {
        let fh = self.url_input.read(cx).focus_handle(cx);
        fh.focus(window, cx);
        cx.notify();
    }

    pub(crate) fn sync_active_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url_raw = self.url_input.read(cx).value().to_string();
        if url_raw.trim_start().starts_with("curl ") || url_raw.trim_start().starts_with("curl\t") {
            if let Ok(mut parsed) = crate::import::parse_curl(&url_raw) {
                parsed.url = crate::path_params::normalize_path_params(&parsed.url);
                Self::merge_path_params(&parsed.url, &mut parsed.params);
                self.url_input
                    .update(cx, |s, cx| s.set_value(parsed.url.clone(), window, cx));
                self.body_editor.update(cx, |s, cx| {
                    s.set_value(parsed.body.content.clone(), window, cx)
                });

                self.row_subs.clear();
                self.param_rows = parsed
                    .params
                    .iter()
                    .map(|kv| self.make_kv_row(&kv.key, &kv.value, kv.enabled, window, cx))
                    .collect();
                self.path_param_keys = Self::url_param_keys(&parsed.url);
                self.header_rows = parsed
                    .headers
                    .iter()
                    .map(|kv| self.make_kv_row(&kv.key, &kv.value, kv.enabled, window, cx))
                    .collect();
                self.urlencoded_rows = Self::urlencoded_fields_for_body(&parsed.body)
                    .iter()
                    .map(|kv| self.make_urlencoded_row(&kv.key, &kv.value, kv.enabled, window, cx))
                    .collect();
                self.multipart_rows.clear();

                self.app_state.update(cx, |state, cx| {
                    let mut ar = parsed.clone();
                    ar.body.urlencoded = Self::urlencoded_fields_for_body(&ar.body);
                    if matches!(ar.body.body_type, BodyType::UrlEncoded)
                        && !ar.body.urlencoded.is_empty()
                    {
                        ar.body.content = crate::http::build_urlencoded_body(&ar.body.urlencoded);
                    }
                    if let Some(old) = &state.active_request {
                        ar.id = old.id.clone();
                    }
                    state.active_request = Some(ar);
                    state.save_active_request();
                    cx.emit(AppEvent::SaveNeeded);
                });
                self.refresh_url_variable_rows(window, cx);
                cx.notify();
                return;
            }
        }

        self.app_state.update(cx, |state, cx| {
            let Some(req) = &mut state.active_request else {
                return;
            };

            let mut url = self.url_input.read(cx).value().to_string();
            let normalized_url = crate::path_params::normalize_path_params(&url);
            if normalized_url != url {
                url = normalized_url;
                self.url_input.update(cx, |state, cx| {
                    state.set_value(url.clone(), window, cx);
                });
            }

            req.params = Self::collect_kv(&self.param_rows, cx);
            let placeholder_url = Self::normalize_query_param_placeholders(&url, &req.params);
            if placeholder_url != url {
                url = placeholder_url;
                self.url_input.update(cx, |state, cx| {
                    state.set_value(url.clone(), window, cx);
                });
            }
            req.url = url.clone();
            req.headers = Self::collect_kv(&self.header_rows, cx);
            req.body.urlencoded = Self::collect_kv(&self.urlencoded_rows, cx);
            if matches!(req.body.body_type, BodyType::UrlEncoded) {
                req.body.content = crate::http::build_urlencoded_body(&req.body.urlencoded);
            } else {
                req.body.content = self.body_editor.read(cx).value().to_string();
            }
            req.body.form_data = Self::collect_multipart(&self.multipart_rows, &req.body, cx);

            self.sync_auth_inputs_to_config(&mut req.auth, cx);

            state.save_active_request();
            cx.emit(AppEvent::SaveNeeded);
        });

        // Outside state update, update path parameter rows from the normalized URL.
        let changed = self.sync_path_param_rows(window, cx);
        if changed {
            self.persist_param_rows(cx);
            cx.notify();
        }
        self.refresh_url_variable_rows(window, cx);
    }

    fn sync_path_param_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let url = self.url_input.read(cx).value().to_string();
        let extracted = Self::url_param_keys(&url);
        let previous_path_params = self.path_param_keys.clone();

        let mut changed = false;
        let mut retained_rows = Vec::with_capacity(self.param_rows.len());
        let mut retained_path_params = Vec::new();
        for row in self.param_rows.drain(..) {
            let key = row.0.read(cx).value().to_string();
            let value = row.1.read(cx).value().to_string();
            if Self::is_stale_path_param_row(&key, &value, &extracted, &previous_path_params) {
                changed = true;
                continue;
            }
            if extracted.contains(&key) {
                if retained_path_params.contains(&key) {
                    changed = true;
                    continue;
                }
                retained_path_params.push(key.clone());
            }
            retained_rows.push(row);
        }
        self.param_rows = retained_rows;

        let mut current_keys: Vec<String> = self
            .param_rows
            .iter()
            .map(|(k, _, _)| k.read(cx).value().to_string())
            .collect();
        for ext in &extracted {
            if !current_keys.contains(&ext) {
                let row = self.make_kv_row(&ext, "", true, window, cx);
                self.param_rows.push(row);
                current_keys.push(ext.clone());
                changed = true;
            }
        }
        self.path_param_keys = extracted;

        changed
    }

    fn is_stale_path_param_row(
        key: &str,
        value: &str,
        extracted: &[String],
        previous_path_params: &[String],
    ) -> bool {
        let key = key.trim();
        if key.is_empty() || extracted.iter().any(|param| param == key) {
            return false;
        }

        if previous_path_params.iter().any(|param| param == key) {
            return true;
        }

        value.trim().is_empty()
            && extracted
                .iter()
                .any(|param| !key.is_empty() && key.len() < param.len() && param.starts_with(key))
    }

    fn persist_param_rows(&self, cx: &mut Context<Self>) {
        self.app_state.update(cx, |state, cx| {
            if let Some(req) = &mut state.active_request {
                req.params = Self::collect_kv(&self.param_rows, cx);
                state.save_active_request();
                cx.emit(AppEvent::SaveNeeded);
            }
        });
    }

    fn persist_url_and_param_rows(&self, cx: &mut Context<Self>) {
        let url = self.url_input.read(cx).value().to_string();
        self.app_state.update(cx, |state, cx| {
            if let Some(req) = &mut state.active_request {
                req.url = url;
                req.params = Self::collect_kv(&self.param_rows, cx);
                state.save_active_request();
                cx.emit(AppEvent::SaveNeeded);
            }
        });
    }

    fn render_url_bar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let method = self
            .app_state
            .read(cx)
            .active_request
            .as_ref()
            .map(|r| r.method.clone())
            .unwrap_or(HttpMethod::GET);

        let is_loading = self.app_state.read(cx).is_loading;
        let app_state_for_menu = self.app_state.clone();
        let method_label = method.as_str().to_string();

        h_flex()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .px_3()
            .py_2()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                DropdownButton::new("method-dropdown")
                    .button(
                        Button::new("method-btn")
                            .label(method_label)
                            .ghost()
                            .small(),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        let as1 = app_state_for_menu.clone();
                        let as2 = app_state_for_menu.clone();
                        let as3 = app_state_for_menu.clone();
                        let as4 = app_state_for_menu.clone();
                        let as5 = app_state_for_menu.clone();
                        menu.item(PopupMenuItem::new("GET").on_click(move |_, _, cx| {
                            as1.update(cx, |s, cx| {
                                if let Some(r) = &mut s.active_request {
                                    r.method = HttpMethod::GET;
                                }
                                s.save_active_request();
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                        }))
                        .item(PopupMenuItem::new("POST").on_click(move |_, _, cx| {
                            as2.update(cx, |s, cx| {
                                if let Some(r) = &mut s.active_request {
                                    r.method = HttpMethod::POST;
                                }
                                s.save_active_request();
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                        }))
                        .item(PopupMenuItem::new("PUT").on_click(move |_, _, cx| {
                            as3.update(cx, |s, cx| {
                                if let Some(r) = &mut s.active_request {
                                    r.method = HttpMethod::PUT;
                                }
                                s.save_active_request();
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                        }))
                        .item(PopupMenuItem::new("PATCH").on_click(move |_, _, cx| {
                            as4.update(cx, |s, cx| {
                                if let Some(r) = &mut s.active_request {
                                    r.method = HttpMethod::PATCH;
                                }
                                s.save_active_request();
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                        }))
                        .item(PopupMenuItem::new("DELETE").on_click(move |_, _, cx| {
                            as5.update(cx, |s, cx| {
                                if let Some(r) = &mut s.active_request {
                                    r.method = HttpMethod::DELETE;
                                }
                                s.save_active_request();
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                        }))
                    }),
            )
            .child(self.render_url_field(window, cx))
            .child(if is_loading {
                Button::new("cancel-btn")
                    .label("Cancel")
                    .danger()
                    .small()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.app_state.update(cx, |state, cx| {
                            state.cancel_active_request();
                            cx.emit(AppEvent::LoadingChanged);
                        });
                    }))
                    .into_any_element()
            } else {
                Button::new("send-btn")
                    .label("Send")
                    .primary()
                    .small()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.send_request(window, cx);
                    }))
                    .into_any_element()
            })
    }

    fn render_url_field(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let url = self.url_input.read(cx).value().to_string();
        let has_decorated_segments = !Self::extract_variable_names(&url).is_empty()
            || !Self::url_param_keys(&url).is_empty();
        let is_focused = self
            .url_input
            .read(cx)
            .focus_handle(cx)
            .contains_focused(window, cx);

        if !has_decorated_segments {
            return Input::new(&self.url_input)
                .flex_1()
                .min_w(px(0.))
                .into_any_element();
        }

        div()
            .id("decorated-url-input")
            .relative()
            .flex_1()
            .min_w(px(0.))
            .h(px(32.))
            .child(Input::new(&self.url_input).w_full())
            .when(!is_focused, |el| {
                el.child(
                    div()
                        .absolute()
                        .inset_0()
                        .px_2()
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded_md()
                        .bg(cx.theme().background)
                        .overflow_hidden()
                        .cursor(CursorStyle::IBeam)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                this.url_input.read(cx).focus_handle(cx).focus(window, cx);
                                cx.notify();
                            }),
                        )
                        .child(
                            h_flex()
                                .h_full()
                                .items_center()
                                .gap_1()
                                .overflow_hidden()
                                .children(self.render_url_segments(&url, cx)),
                        ),
                )
            })
            .into_any_element()
    }

    fn render_url_segments(&self, url: &str, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut segments = Vec::new();
        let mut text_start = 0;
        let mut i = 0;
        let mut chip_ix = 0;

        while i < url.len() {
            let rest = &url[i..];

            if rest.starts_with("{{") {
                if let Some(end) = rest[2..].find("}}") {
                    let raw_end = i + 2 + end + 2;
                    let name = rest[2..2 + end].trim();

                    if Self::is_url_variable_name(name) {
                        if text_start < i {
                            segments.push(Self::render_url_text_segment(&url[text_start..i]));
                        }
                        segments.push(self.render_inline_url_variable(name, chip_ix, cx));
                        chip_ix += 1;
                        i = raw_end;
                        text_start = i;
                        continue;
                    }

                    i = raw_end;
                    continue;
                }
            }

            if rest.starts_with(':') && Self::is_url_path_param_start(url, i) {
                let name_start = i + 1;
                let mut name_end = name_start;
                while name_end < url.len() {
                    let Some(ch) = url[name_end..].chars().next() else {
                        break;
                    };
                    if !Self::is_url_path_param_char(ch) {
                        break;
                    }
                    name_end += ch.len_utf8();
                }

                if name_end > name_start {
                    if text_start < i {
                        segments.push(Self::render_url_text_segment(&url[text_start..i]));
                    }
                    segments.push(self.render_inline_url_path_param(
                        &url[name_start..name_end],
                        chip_ix,
                        cx,
                    ));
                    chip_ix += 1;
                    i = name_end;
                    text_start = i;
                    continue;
                }
            }

            let Some(ch) = rest.chars().next() else {
                break;
            };
            i += ch.len_utf8();
        }

        if text_start < url.len() {
            segments.push(Self::render_url_text_segment(&url[text_start..]));
        }

        segments
    }

    fn render_url_text_segment(text: &str) -> AnyElement {
        div()
            .flex_shrink_0()
            .text_sm()
            .font_family("monospace")
            .whitespace_nowrap()
            .child(Self::input_line(text))
            .into_any_element()
    }

    fn render_url_chip_tooltip(tooltip: String, cx: &mut Context<Self>) -> impl IntoElement {
        deferred(
            anchored()
                .anchor(Anchor::TopLeft)
                .snap_to_window_with_margin(px(8.))
                .offset(point(px(0.), px(22.)))
                .child(
                    div()
                        .occlude()
                        .max_w(px(360.))
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .border_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().tokens.popover)
                        .text_color(cx.theme().popover_foreground)
                        .shadow_lg()
                        .text_xs()
                        .font_family("monospace")
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(tooltip),
                ),
        )
        .with_priority(2)
    }

    fn render_inline_url_variable(
        &self,
        name: &str,
        index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let variable_name = name.to_string();
        let hover_name = variable_name.clone();
        let is_hovered = self.hovered_url_variable.as_deref() == Some(name);
        let tooltip = if is_hovered {
            let value = self
                .variable_value(name, cx)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "No value".to_string());
            Some(format!("{name} = {value}"))
        } else {
            None
        };

        div()
            .id(SharedString::from(format!(
                "url-variable-chip-{index}-{variable_name}"
            )))
            .relative()
            .flex_shrink_0()
            .px_1p5()
            .py_0p5()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().primary.opacity(0.45))
            .bg(cx.theme().primary.opacity(0.14))
            .text_xs()
            .text_color(cx.theme().primary)
            .font_family("monospace")
            .whitespace_nowrap()
            .on_hover(cx.listener(move |this, hovered, _, cx| {
                if *hovered {
                    this.hovered_url_variable = Some(hover_name.clone());
                    this.hovered_url_path_param = None;
                } else if this.hovered_url_variable.as_deref() == Some(hover_name.as_str()) {
                    this.hovered_url_variable = None;
                }
                cx.notify();
            }))
            .when_some(tooltip, |el, tooltip| {
                el.child(Self::render_url_chip_tooltip(tooltip, cx))
            })
            .child(variable_name)
            .into_any_element()
    }

    fn render_inline_url_path_param(
        &self,
        name: &str,
        index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let param_name = name.to_string();
        let hover_name = param_name.clone();
        let is_hovered = self.hovered_url_path_param.as_deref() == Some(name);
        let tooltip = if is_hovered {
            let value = self
                .path_param_value(name, cx)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "No value".to_string());
            Some(format!(":{name} = {value}"))
        } else {
            None
        };

        div()
            .id(SharedString::from(format!(
                "url-path-param-chip-{index}-{param_name}"
            )))
            .relative()
            .flex_shrink_0()
            .px_1p5()
            .py_0p5()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().muted_foreground.opacity(0.35))
            .bg(cx.theme().muted.opacity(0.24))
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .font_family("monospace")
            .whitespace_nowrap()
            .on_hover(cx.listener(move |this, hovered, _, cx| {
                if *hovered {
                    this.hovered_url_path_param = Some(hover_name.clone());
                    this.hovered_url_variable = None;
                } else if this.hovered_url_path_param.as_deref() == Some(hover_name.as_str()) {
                    this.hovered_url_path_param = None;
                }
                cx.notify();
            }))
            .when_some(tooltip, |el, tooltip| {
                el.child(Self::render_url_chip_tooltip(tooltip, cx))
            })
            .child(format!(":{param_name}"))
            .into_any_element()
    }

    fn render_params_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, (key, val, enabled)) in self.param_rows.iter().enumerate() {
            rows.push(kv_input_row(i, key, val, *enabled, false, cx).into_any_element());
        }
        div()
            .id("params-scroll")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .child(kv_header(cx))
            .children(rows)
            .child(
                Button::new("add-param")
                    .icon(IconName::Plus)
                    .label("Add")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|this, _, window, cx| {
                        let row = this.make_kv_row("", "", true, window, cx);
                        this.param_rows.push(row);
                        this.sync_active_request(window, cx);
                        cx.notify();
                    })),
            )
    }

    fn render_headers_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, (key, val, enabled)) in self.header_rows.iter().enumerate() {
            rows.push(kv_input_row(i, key, val, *enabled, true, cx).into_any_element());
        }
        div()
            .id("headers-scroll")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .child(kv_header(cx))
            .children(rows)
            .child(
                Button::new("add-header")
                    .icon(IconName::Plus)
                    .label("Add")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|this, _, window, cx| {
                        let row = this.make_kv_row("", "", true, window, cx);
                        this.header_rows.push(row);
                        this.sync_active_request(window, cx);
                        cx.notify();
                    })),
            )
    }

    fn body_type_label(body_type: &BodyType) -> &'static str {
        match body_type {
            BodyType::FormData => "Multi-Part",
            BodyType::UrlEncoded => "Url Encoded",
            BodyType::Json => "JSON",
            BodyType::Raw => "Other",
            BodyType::BinaryFile => "Binary File",
            BodyType::None => "No Body",
        }
    }

    fn is_text_body_type(body_type: &BodyType) -> bool {
        matches!(body_type, BodyType::Json | BodyType::Raw)
    }

    fn empty_body_label(body_type: &BodyType) -> &'static str {
        match body_type {
            BodyType::FormData => "No multipart fields",
            BodyType::BinaryFile => "No file selected",
            _ => "No body",
        }
    }

    fn body_menu_item(
        app_state: Entity<AppState>,
        current: &BodyType,
        target: BodyType,
        label: &'static str,
    ) -> PopupMenuItem {
        let selected = current == &target;
        PopupMenuItem::new(label)
            .checked(selected)
            .on_click(move |_, _, cx| {
                app_state.update(cx, |state, cx| {
                    if let Some(req) = &mut state.active_request {
                        req.body.body_type = target.clone();
                        if matches!(target, BodyType::UrlEncoded) && req.body.urlencoded.is_empty()
                        {
                            req.body.urlencoded = Self::parse_urlencoded_content(&req.body.content);
                            if req.body.urlencoded.is_empty() {
                                req.body.urlencoded.push(KeyValue::empty());
                            }
                        }
                        if matches!(target, BodyType::FormData) && req.body.form_data.is_empty() {
                            req.body.form_data.push(FormDataPart::empty());
                        }
                    }
                    state.save_active_request();
                    cx.emit(AppEvent::WorkspaceChanged);
                    cx.emit(AppEvent::SaveNeeded);
                });
            })
    }

    fn render_body_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body_type = self
            .app_state
            .read(cx)
            .active_request
            .as_ref()
            .map(|r| r.body.body_type.clone())
            .unwrap_or(BodyType::None);
        let body_label = Self::body_type_label(&body_type);
        let menu_body_type = body_type.clone();
        let app_state_for_menu = self.app_state.clone();

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .px_3()
                    .py_1p5()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        DropdownButton::new("body-type-dropdown")
                            .button(
                                Button::new("body-type-btn")
                                    .label(body_label)
                                    .ghost()
                                    .xsmall(),
                            )
                            .dropdown_menu(move |menu, _, _| {
                                let item = |target, label| {
                                    Self::body_menu_item(
                                        app_state_for_menu.clone(),
                                        &menu_body_type,
                                        target,
                                        label,
                                    )
                                };
                                menu.item(PopupMenuItem::label("Form Data"))
                                    .item(item(BodyType::UrlEncoded, "Url Encoded"))
                                    .item(item(BodyType::FormData, "Multi-Part"))
                                    .separator()
                                    .item(PopupMenuItem::label("Text Content"))
                                    .item(item(BodyType::Json, "JSON"))
                                    .item(item(BodyType::Raw, "Other"))
                                    .separator()
                                    .item(PopupMenuItem::label("Other"))
                                    .item(item(BodyType::BinaryFile, "Binary File"))
                                    .item(item(BodyType::None, "No Body"))
                            }),
                    )
                    .when(matches!(body_type, BodyType::Json), |el| {
                        el.child(
                            div().flex_1().flex().justify_end().child(
                                Button::new("format-json-btn")
                                    .label("Format JSON")
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        let text = this.body_editor.read(cx).value().to_string();
                                        if let Ok(parsed) =
                                            serde_json::from_str::<serde_json::Value>(&text)
                                        {
                                            if let Ok(pretty) =
                                                serde_json::to_string_pretty(&parsed)
                                            {
                                                this.body_editor.update(cx, |editor, cx| {
                                                    editor.set_value(pretty, window, cx);
                                                });
                                                this.sync_active_request(window, cx);
                                                cx.notify();
                                            }
                                        }
                                    })),
                            ),
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .p_1()
                    .when(matches!(body_type, BodyType::FormData), |el| {
                        el.child(self.render_multipart_form(cx))
                    })
                    .when(matches!(body_type, BodyType::UrlEncoded), |el| {
                        el.child(self.render_urlencoded_form(cx))
                    })
                    .when(Self::is_text_body_type(&body_type), |el| {
                        el.child(Input::new(&self.body_editor).h_full())
                    })
                    .when(
                        !Self::is_text_body_type(&body_type)
                            && !matches!(body_type, BodyType::UrlEncoded)
                            && !matches!(body_type, BodyType::FormData),
                        |el| {
                            el.child(
                                div()
                                    .flex()
                                    .size_full()
                                    .items_center()
                                    .justify_center()
                                    .text_color(cx.theme().muted_foreground)
                                    .text_sm()
                                    .child(Self::empty_body_label(&body_type)),
                            )
                        },
                    ),
            )
    }

    fn render_urlencoded_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, (key, val, enabled)) in self.urlencoded_rows.iter().enumerate() {
            rows.push(
                self.render_urlencoded_row(i, key, val, *enabled, cx)
                    .into_any_element(),
            );
        }

        div()
            .id("urlencoded-scroll")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .child(urlencoded_header(cx))
            .children(rows)
            .child(
                Button::new("add-urlencoded")
                    .icon(IconName::Plus)
                    .label("Add")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|this, _, window, cx| {
                        let row = this.make_urlencoded_row("", "", true, window, cx);
                        this.urlencoded_rows.push(row);
                        this.sync_active_request(window, cx);
                        cx.notify();
                    })),
            )
    }

    fn render_urlencoded_row(
        &self,
        i: usize,
        key: &Entity<InputState>,
        val: &Entity<InputState>,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .py_0p5()
            .child(
                Checkbox::new(format!("chk-urlencoded-{}", i))
                    .checked(enabled)
                    .on_click(cx.listener(move |this, enabled: &bool, window, cx| {
                        if let Some(row) = this.urlencoded_rows.get_mut(i) {
                            row.2 = *enabled;
                            this.sync_active_request(window, cx);
                            cx.notify();
                        }
                    })),
            )
            .child(div().flex_1().child(Input::new(key)))
            .child(div().flex_1().child(Input::new(val)))
            .child(
                Button::new(format!("del-urlencoded-row-{}", i))
                    .icon(IconName::Close)
                    .tooltip("Remove row")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if i < this.urlencoded_rows.len() {
                            this.urlencoded_rows.remove(i);
                        }
                        this.sync_active_request(window, cx);
                        cx.notify();
                    })),
            )
    }

    fn render_multipart_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let parts = self
            .app_state
            .read(cx)
            .active_request
            .as_ref()
            .map(|req| req.body.form_data.clone())
            .unwrap_or_default();

        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, row) in self.multipart_rows.iter().enumerate() {
            let part = parts
                .iter()
                .find(|part| part.id == row.id)
                .cloned()
                .unwrap_or_else(|| FormDataPart {
                    id: row.id.clone(),
                    name: row.name.read(cx).value().to_string(),
                    value: row.value.read(cx).value().to_string(),
                    enabled: true,
                    kind: FormDataPartKind::Text,
                    content_type: String::new(),
                });
            rows.push(
                self.render_multipart_row(i, row, &part, cx)
                    .into_any_element(),
            );
        }

        div()
            .id("multipart-scroll")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .child(multipart_header(cx))
            .children(rows)
            .child(
                Button::new("add-multipart")
                    .icon(IconName::Plus)
                    .label("Add")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|this, _, window, cx| {
                        let part = FormDataPart::empty();
                        this.app_state.update(cx, |state, cx| {
                            if let Some(req) = &mut state.active_request {
                                req.body.body_type = BodyType::FormData;
                                req.body.form_data.push(part.clone());
                            }
                            state.save_active_request();
                            cx.emit(AppEvent::SaveNeeded);
                        });
                        let row = this.make_multipart_row(&part, window, cx);
                        this.multipart_rows.push(row);
                        cx.notify();
                    })),
            )
    }

    fn render_multipart_row(
        &self,
        i: usize,
        row: &MultipartRow,
        part: &FormDataPart,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let row_id = row.id.clone();
        let enabled = part.enabled;
        let kind = part.kind.clone();
        let kind_label = match kind {
            FormDataPartKind::Text => "Text",
            FormDataPartKind::File => "File",
        };
        let is_text = matches!(kind, FormDataPartKind::Text);
        let is_file = matches!(kind, FormDataPartKind::File);
        let app_state_for_menu = self.app_state.clone();
        let menu_row_id = row.id.clone();
        let content_type = part.content_type.clone();
        let picker_app_state = self.app_state.clone();
        let picker_row_id = row.id.clone();
        let picker_value = row.value.clone();
        let delete_row_id = row.id.clone();

        h_flex()
            .gap_2()
            .items_center()
            .py_0p5()
            .child(
                Checkbox::new(format!("chk-multipart-{}", row_id))
                    .checked(enabled)
                    .on_click(cx.listener(move |this, enabled: &bool, _, cx| {
                        Self::update_multipart_part(&this.app_state, &row_id, cx, |part| {
                            part.enabled = *enabled;
                        });
                    })),
            )
            .child(div().flex_1().child(Input::new(&row.name)))
            .child(
                h_flex()
                    .flex_1()
                    .gap_1()
                    .child(div().flex_1().child(Input::new(&row.value)))
                    .when(is_file, |el| {
                        el.child(
                            Button::new(format!("multipart-file-picker-{}", i))
                                .icon(IconName::FolderOpen)
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    let rx = cx.prompt_for_paths(PathPromptOptions {
                                        files: true,
                                        directories: false,
                                        multiple: false,
                                        prompt: Some("Select multipart file".into()),
                                    });
                                    let app_state = picker_app_state.clone();
                                    let row_id = picker_row_id.clone();
                                    let value_input = picker_value.clone();
                                    cx.spawn_in(window, async move |_, window| {
                                        let path = rx.await.ok()?.ok()??.iter().next()?.clone();
                                        let path = path.to_string_lossy().to_string();
                                        window
                                            .update(|window, cx| {
                                                value_input.update(cx, |state, cx| {
                                                    state.set_value(path.clone(), window, cx);
                                                });
                                                app_state.update(cx, |state, cx| {
                                                    if let Some(req) = &mut state.active_request {
                                                        if let Some(part) =
                                                            req.body.form_data.iter_mut().find(
                                                                |part| part.id == row_id.as_str(),
                                                            )
                                                        {
                                                            part.kind = FormDataPartKind::File;
                                                            part.value = path.clone();
                                                        }
                                                    }
                                                    state.save_active_request();
                                                    cx.emit(AppEvent::SaveNeeded);
                                                });
                                            })
                                            .ok();

                                        Some(())
                                    })
                                    .detach();
                                })),
                        )
                    }),
            )
            .when(!content_type.is_empty(), |el| {
                el.child(
                    div()
                        .max_w(px(150.))
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(content_type.replace(['\r', '\n'], " ")),
                )
            })
            .child(
                DropdownButton::new(format!("multipart-kind-{}", i))
                    .button(
                        Button::new(format!("multipart-kind-btn-{}", i))
                            .label(kind_label)
                            .ghost()
                            .xsmall(),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        let text_state = app_state_for_menu.clone();
                        let text_id = menu_row_id.clone();
                        let file_state = app_state_for_menu.clone();
                        let file_id = menu_row_id.clone();
                        let set_content_type_state = app_state_for_menu.clone();
                        let set_content_type_id = menu_row_id.clone();
                        let unset_content_type_state = app_state_for_menu.clone();
                        let unset_content_type_id = menu_row_id.clone();
                        let unset_file_state = app_state_for_menu.clone();
                        let unset_file_id = menu_row_id.clone();

                        menu.item(PopupMenuItem::new("Text").checked(is_text).on_click(
                            move |_, _, cx| {
                                Self::update_multipart_part(&text_state, &text_id, cx, |part| {
                                    part.kind = FormDataPartKind::Text;
                                });
                            },
                        ))
                        .item(PopupMenuItem::new("File").checked(is_file).on_click(
                            move |_, _, cx| {
                                Self::update_multipart_part(&file_state, &file_id, cx, |part| {
                                    part.kind = FormDataPartKind::File;
                                });
                            },
                        ))
                        .separator()
                        .item(
                            PopupMenuItem::new("Set Content-Type").on_click(move |_, _, cx| {
                                Self::update_multipart_part(
                                    &set_content_type_state,
                                    &set_content_type_id,
                                    cx,
                                    |part| {
                                        part.content_type = match part.kind {
                                            FormDataPartKind::Text => "text/plain".to_string(),
                                            FormDataPartKind::File => {
                                                "application/octet-stream".to_string()
                                            }
                                        };
                                    },
                                );
                            }),
                        )
                        .item(
                            PopupMenuItem::new("Unset Content-Type").on_click(move |_, _, cx| {
                                Self::update_multipart_part(
                                    &unset_content_type_state,
                                    &unset_content_type_id,
                                    cx,
                                    |part| {
                                        part.content_type.clear();
                                    },
                                );
                            }),
                        )
                        .item(PopupMenuItem::new("Unset File").on_click(move |_, _, cx| {
                            Self::update_multipart_part(
                                &unset_file_state,
                                &unset_file_id,
                                cx,
                                |part| {
                                    if matches!(part.kind, FormDataPartKind::File) {
                                        part.value.clear();
                                    }
                                },
                            );
                        }))
                    }),
            )
            .child(
                Button::new(format!("del-multipart-row-{}", i))
                    .icon(IconName::Close)
                    .tooltip("Remove row")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        Self::delete_multipart_part(&this.app_state, &delete_row_id, cx);
                    })),
            )
    }

    fn update_multipart_part<F>(app_state: &Entity<AppState>, id: &str, cx: &mut App, f: F)
    where
        F: FnOnce(&mut FormDataPart),
    {
        app_state.update(cx, |state, cx| {
            if let Some(req) = &mut state.active_request {
                if let Some(part) = req.body.form_data.iter_mut().find(|part| part.id == id) {
                    f(part);
                    if part.kind == FormDataPartKind::File && part.content_type == "text/plain" {
                        part.content_type.clear();
                    }
                }
            }
            state.save_active_request();
            cx.emit(AppEvent::WorkspaceChanged);
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    fn delete_multipart_part(app_state: &Entity<AppState>, id: &str, cx: &mut App) {
        app_state.update(cx, |state, cx| {
            if let Some(req) = &mut state.active_request {
                req.body.form_data.retain(|part| part.id != id);
            }
            state.save_active_request();
            cx.emit(AppEvent::WorkspaceChanged);
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    fn render_auth_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let auth = self
            .app_state
            .read(cx)
            .active_request
            .as_ref()
            .map(|r| r.auth.clone())
            .unwrap_or_default();
        let auth_type = auth.auth_type.clone();

        div()
            .id("auth-scroll")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_3()
            .gap_3()
            .child(Self::auth_type_dropdown(
                self.app_state.clone(),
                &auth.auth_type,
            ))
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Checkbox::new("auth-enabled")
                            .checked(auth.enabled)
                            .on_click(cx.listener(|this, enabled: &bool, _, cx| {
                                Self::update_auth_config(&this.app_state, cx, |auth| {
                                    auth.enabled = *enabled;
                                });
                            })),
                    )
                    .child(div().text_sm().child("Enabled")),
            )
            .when(matches!(auth_type, AuthType::ApiKey), |el| {
                el.child(self.render_api_key_auth(&auth, cx))
            })
            .when(matches!(auth_type, AuthType::AwsV4), |el| {
                el.child(self.render_aws_auth(cx))
            })
            .when(matches!(auth_type, AuthType::Basic), |el| {
                el.child(self.render_basic_auth(cx))
            })
            .when(matches!(auth_type, AuthType::Bearer), |el| {
                el.child(self.render_bearer_auth(cx))
            })
            .when(matches!(auth_type, AuthType::Jwt), |el| {
                el.child(self.render_jwt_auth(&auth, cx))
            })
            .when(matches!(auth_type, AuthType::OAuth2), |el| {
                el.child(self.render_oauth_auth(&auth, cx))
            })
            .when(matches!(auth_type, AuthType::None), |el| {
                el.child(
                    div()
                        .flex()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("No Auth"),
                )
            })
    }

    fn auth_type_label(auth_type: &AuthType) -> &'static str {
        match auth_type {
            AuthType::ApiKey => "API Key",
            AuthType::AwsV4 => "AWS v4",
            AuthType::Basic => "Basic",
            AuthType::Bearer => "Bearer",
            AuthType::Jwt => "JWT",
            AuthType::OAuth2 => "OAuth 2",
            AuthType::None => "No Auth",
        }
    }

    fn auth_type_dropdown(app_state: Entity<AppState>, current: &AuthType) -> impl IntoElement {
        let current = current.clone();
        let current_label = Self::auth_type_label(&current);
        let current_for_items = current.clone();
        let item = move |target: AuthType, label: &'static str| {
            let app_state = app_state.clone();
            let selected = current_for_items == target;
            PopupMenuItem::new(label)
                .checked(selected)
                .on_click(move |_, _, cx| {
                    app_state.update(cx, |state, cx| {
                        if let Some(req) = &mut state.active_request {
                            req.auth.auth_type = target.clone();
                            Self::ensure_auth_defaults(&mut req.auth);
                        }
                        state.save_active_request();
                        cx.emit(AppEvent::WorkspaceChanged);
                        cx.emit(AppEvent::SaveNeeded);
                    });
                })
        };

        DropdownButton::new("auth-type-dropdown")
            .button(
                Button::new("auth-type-btn")
                    .label(current_label)
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                menu.item(item(AuthType::ApiKey, "API Key"))
                    .item(item(AuthType::AwsV4, "AWS Signature"))
                    .item(item(AuthType::Basic, "Basic Auth"))
                    .item(item(AuthType::Bearer, "Bearer Token"))
                    .item(item(AuthType::Jwt, "JWT Bearer"))
                    .item(item(AuthType::OAuth2, "OAuth 2.0"))
                    .separator()
                    .item(item(AuthType::None, "Inherit from Parent"))
                    .item(item(AuthType::None, "No Auth"))
            })
    }

    fn render_api_key_auth(&self, auth: &AuthConfig, cx: &mut Context<Self>) -> impl IntoElement {
        let api_key_in_header = auth.api_key_in_header;
        let behavior_label = if auth.api_key_in_header {
            "Insert Header"
        } else {
            "Insert Query Param"
        };
        v_flex()
            .gap_3()
            .child(Self::auth_select_field(
                "Behavior",
                DropdownButton::new("api-key-behavior")
                    .button(
                        Button::new("api-key-behavior-btn")
                            .label(behavior_label)
                            .ghost()
                            .small(),
                    )
                    .dropdown_menu({
                        let app_state = self.app_state.clone();
                        move |menu, _, _| {
                            let header_state = app_state.clone();
                            let query_state = app_state.clone();
                            menu.item(
                                PopupMenuItem::new("Insert Header")
                                    .checked(api_key_in_header)
                                    .on_click(move |_, _, cx| {
                                        Self::update_auth_config(&header_state, cx, |auth| {
                                            auth.api_key_in_header = true;
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("Insert Query Param")
                                    .checked(!api_key_in_header)
                                    .on_click(move |_, _, cx| {
                                        Self::update_auth_config(&query_state, cx, |auth| {
                                            auth.api_key_in_header = false;
                                        });
                                    }),
                            )
                        }
                    }),
                cx,
            ))
            .child(Self::auth_input_field(
                if auth.api_key_in_header {
                    "Header Name*"
                } else {
                    "Query Param Name*"
                },
                &self.auth_inputs.api_key_name,
                cx,
            ))
            .child(Self::auth_input_field(
                "API Key",
                &self.auth_inputs.api_key_value,
                cx,
            ))
    }

    fn render_aws_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(Self::auth_input_field(
                "Access Key ID*",
                &self.auth_inputs.aws_access_key_id,
                cx,
            ))
            .child(Self::auth_input_field(
                "Secret Access Key*",
                &self.auth_inputs.aws_secret_access_key,
                cx,
            ))
            .child(Self::auth_input_field(
                "Service Name*",
                &self.auth_inputs.aws_service,
                cx,
            ))
            .child(Self::auth_input_field(
                "Region",
                &self.auth_inputs.aws_region,
                cx,
            ))
            .child(Self::auth_input_field(
                "Session Token",
                &self.auth_inputs.aws_session_token,
                cx,
            ))
    }

    fn render_basic_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(Self::auth_input_field(
                "Username",
                &self.auth_inputs.basic_username,
                cx,
            ))
            .child(Self::auth_input_field(
                "Password",
                &self.auth_inputs.basic_password,
                cx,
            ))
    }

    fn render_bearer_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(Self::auth_input_field(
                "Token",
                &self.auth_inputs.bearer_token,
                cx,
            ))
            .child(Self::auth_input_field(
                "Prefix",
                &self.auth_inputs.bearer_prefix,
                cx,
            ))
    }

    fn render_jwt_auth(&self, auth: &AuthConfig, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                DropdownButton::new("jwt-algorithm")
                    .button(
                        Button::new("jwt-algorithm-btn")
                            .label(if auth.jwt_algorithm.is_empty() {
                                "HS256"
                            } else {
                                auth.jwt_algorithm.as_str()
                            })
                            .ghost()
                            .small(),
                    )
                    .dropdown_menu({
                        let app_state = self.app_state.clone();
                        let current = auth.jwt_algorithm.clone();
                        move |menu, _, _| {
                            ["HS256", "HS384", "HS512", "RS256", "RS384", "RS512"]
                                .into_iter()
                                .fold(menu, |menu, algorithm| {
                                    let app_state = app_state.clone();
                                    menu.item(
                                        PopupMenuItem::new(algorithm)
                                            .checked(current == algorithm)
                                            .on_click(move |_, _, cx| {
                                                Self::update_auth_config(&app_state, cx, |auth| {
                                                    auth.jwt_algorithm = algorithm.to_string();
                                                });
                                            }),
                                    )
                                })
                        }
                    }),
            )
            .child(Self::auth_input_field(
                "Secret or Private Key",
                &self.auth_inputs.jwt_secret,
                cx,
            ))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Checkbox::new("jwt-secret-base64")
                            .checked(auth.jwt_secret_base64)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                Self::update_auth_config(&this.app_state, cx, |auth| {
                                    auth.jwt_secret_base64 = *checked;
                                });
                            })),
                    )
                    .child(div().text_sm().child("Secret is base64 encoded")),
            )
            .child(Self::auth_multiline_field(
                "Payload* (Json)",
                &self.auth_inputs.jwt_payload,
                cx,
            ))
    }

    fn render_oauth_auth(&self, auth: &AuthConfig, cx: &mut Context<Self>) -> impl IntoElement {
        let grant = Self::oauth_grant_label(&auth.oauth_grant_type);
        let is_authorization_code = grant == "Authorization Code";
        let is_implicit = grant == "Implicit";
        let is_resource_owner = grant == "Resource Owner Password Credential";
        let is_client_credentials = grant == "Client Credentials";

        v_flex()
            .gap_3()
            .child(Self::oauth_grant_dropdown(
                self.app_state.clone(),
                &auth.oauth_grant_type,
            ))
            .when(is_authorization_code, |el| {
                el.child(Self::auth_input_field(
                    "Client ID",
                    &self.auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Client Secret",
                    &self.auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Authorization URL",
                    &self.auth_inputs.oauth_authorization_url,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Access Token URL",
                    &self.auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Redirect URI",
                    &self.auth_inputs.oauth_redirect_uri,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "State",
                    &self.auth_inputs.oauth_state,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Audience",
                    &self.auth_inputs.oauth_audience,
                    cx,
                ))
                .child(Self::auth_select_field(
                    "Token for authorization",
                    Self::oauth_token_target_dropdown(self.app_state.clone(), auth),
                    cx,
                ))
                .child(Self::oauth_pkce_checkbox(auth, cx))
            })
            .when(is_implicit, |el| {
                el.child(Self::auth_input_field(
                    "Client ID",
                    &self.auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Authorization URL",
                    &self.auth_inputs.oauth_authorization_url,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Redirect URI",
                    &self.auth_inputs.oauth_redirect_uri,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "State",
                    &self.auth_inputs.oauth_state,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Audience",
                    &self.auth_inputs.oauth_audience,
                    cx,
                ))
                .child(Self::auth_select_field(
                    "Token for authorization",
                    Self::oauth_token_target_dropdown(self.app_state.clone(), auth),
                    cx,
                ))
                .child(Self::auth_select_field(
                    "Response Type",
                    Self::oauth_response_type_dropdown(self.app_state.clone(), auth),
                    cx,
                ))
            })
            .when(is_resource_owner, |el| {
                el.child(Self::auth_input_field(
                    "Client ID",
                    &self.auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Client Secret",
                    &self.auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Access Token URL",
                    &self.auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Audience",
                    &self.auth_inputs.oauth_audience,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Username",
                    &self.auth_inputs.oauth_username,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Password",
                    &self.auth_inputs.oauth_password,
                    cx,
                ))
            })
            .when(is_client_credentials, |el| {
                el.child(Self::auth_input_field(
                    "Client ID",
                    &self.auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Client Secret",
                    &self.auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Access Token URL",
                    &self.auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Audience",
                    &self.auth_inputs.oauth_audience,
                    cx,
                ))
            })
            .child(self.render_oauth_advanced(auth, cx))
    }

    fn oauth_grant_label(value: &str) -> &'static str {
        match value {
            "Implicit" => "Implicit",
            "Resource Owner Password Credential" | "Password" => {
                "Resource Owner Password Credential"
            }
            "Client Credentials" => "Client Credentials",
            _ => "Authorization Code",
        }
    }

    fn oauth_grant_dropdown(app_state: Entity<AppState>, current: &str) -> impl IntoElement {
        let current = Self::oauth_grant_label(current);
        let app_state_for_items = app_state.clone();
        let item = move |grant: &'static str| {
            let app_state = app_state_for_items.clone();
            PopupMenuItem::new(grant)
                .checked(current == grant)
                .on_click(move |_, _, cx| {
                    Self::update_auth_config(&app_state, cx, |auth| {
                        auth.oauth_grant_type = grant.to_string();
                    });
                })
        };

        DropdownButton::new("oauth-grant")
            .button(
                Button::new("oauth-grant-btn")
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

    fn oauth_token_target_dropdown(
        app_state: Entity<AppState>,
        auth: &AuthConfig,
    ) -> impl IntoElement {
        let current = default_if_empty(&auth.oauth_token_target, "access_token");
        DropdownButton::new("oauth-token-target")
            .button(
                Button::new("oauth-token-target-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["access_token", "id_token"]
                    .into_iter()
                    .fold(menu, |menu, token| {
                        let app_state = app_state.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(token)
                                .checked(current == token)
                                .on_click(move |_, _, cx| {
                                    Self::update_auth_config(&app_state, cx, |auth| {
                                        auth.oauth_token_target = token.to_string();
                                    });
                                }),
                        )
                    })
            })
    }

    fn oauth_response_type_dropdown(
        app_state: Entity<AppState>,
        auth: &AuthConfig,
    ) -> impl IntoElement {
        let current = default_if_empty(&auth.oauth_response_type, "Access Token");
        DropdownButton::new("oauth-response-type")
            .button(
                Button::new("oauth-response-type-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["Access Token", "ID Token", "Access Token + ID Token"]
                    .into_iter()
                    .fold(menu, |menu, response_type| {
                        let app_state = app_state.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(response_type)
                                .checked(current == response_type)
                                .on_click(move |_, _, cx| {
                                    Self::update_auth_config(&app_state, cx, |auth| {
                                        auth.oauth_response_type = response_type.to_string();
                                    });
                                }),
                        )
                    })
            })
    }

    fn oauth_send_credentials_dropdown(
        app_state: Entity<AppState>,
        auth: &AuthConfig,
    ) -> impl IntoElement {
        let current = default_if_empty(&auth.oauth_send_credentials, "In Request Body");
        DropdownButton::new("oauth-send-credentials")
            .button(
                Button::new("oauth-send-credentials-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["In Request Body", "As Basic Authentication"]
                    .into_iter()
                    .fold(menu, |menu, target| {
                        let app_state = app_state.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(target)
                                .checked(current == target)
                                .on_click(move |_, _, cx| {
                                    Self::update_auth_config(&app_state, cx, |auth| {
                                        auth.oauth_send_credentials = target.to_string();
                                    });
                                }),
                        )
                    })
            })
    }

    fn oauth_pkce_checkbox(auth: &AuthConfig, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(
                Checkbox::new("oauth-use-pkce")
                    .checked(auth.oauth_use_pkce)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        Self::update_auth_config(&this.app_state, cx, |auth| {
                            auth.oauth_use_pkce = *checked;
                        });
                    })),
            )
            .child(div().text_sm().child("Use PKCE"))
    }

    fn render_oauth_advanced(&self, auth: &AuthConfig, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .p_3()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child(div().text_sm().child("Advanced"))
            .child(Self::auth_input_field(
                "Scope",
                &self.auth_inputs.oauth_scope,
                cx,
            ))
            .child(Self::auth_input_field(
                "Header Name*",
                &self.auth_inputs.oauth_header_name,
                cx,
            ))
            .child(Self::auth_input_field(
                "Header Prefix",
                &self.auth_inputs.oauth_header_prefix,
                cx,
            ))
            .child(Self::auth_select_field(
                "Send Credentials",
                Self::oauth_send_credentials_dropdown(self.app_state.clone(), auth),
                cx,
            ))
    }

    fn auth_select_field(
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

    fn auth_input_field(
        label: &'static str,
        input: &Entity<InputState>,
        cx: &App,
    ) -> impl IntoElement {
        Self::auth_select_field(label, Input::new(input).w_full(), cx)
    }

    fn auth_multiline_field(
        label: &'static str,
        input: &Entity<InputState>,
        cx: &App,
    ) -> impl IntoElement {
        Self::auth_select_field(label, Input::new(input).h(px(82.)).w_full(), cx)
    }

    fn update_auth_config<F>(app_state: &Entity<AppState>, cx: &mut App, f: F)
    where
        F: FnOnce(&mut AuthConfig),
    {
        app_state.update(cx, |state, cx| {
            if let Some(req) = &mut state.active_request {
                f(&mut req.auth);
                Self::ensure_auth_defaults(&mut req.auth);
            }
            state.save_active_request();
            cx.emit(AppEvent::WorkspaceChanged);
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    fn ensure_auth_defaults(auth: &mut AuthConfig) {
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
            auth.oauth_access_token_url = "https://github.com/login/oauth/access_token".to_string();
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

    fn collect_kv(
        rows: &[(Entity<InputState>, Entity<InputState>, bool)],
        cx: &App,
    ) -> Vec<KeyValue> {
        rows.iter()
            .map(|(k, v, enabled)| KeyValue {
                id: uuid::Uuid::new_v4().to_string(),
                key: k.read(cx).value().to_string(),
                value: v.read(cx).value().to_string(),
                enabled: *enabled,
                description: String::new(),
            })
            .filter(|kv| !kv.key.is_empty())
            .collect()
    }

    fn urlencoded_fields_for_body(body: &crate::models::RequestBody) -> Vec<KeyValue> {
        if !body.urlencoded.is_empty() {
            body.urlencoded.clone()
        } else if matches!(body.body_type, BodyType::UrlEncoded) {
            Self::parse_urlencoded_content(&body.content)
        } else {
            Vec::new()
        }
    }

    fn parse_urlencoded_content(content: &str) -> Vec<KeyValue> {
        if content.contains(['\r', '\n']) {
            return Vec::new();
        }

        content
            .split('&')
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                let (key, value) = entry.split_once('=').unwrap_or((entry, ""));
                KeyValue::new(
                    Self::input_line(Self::decode_urlencoded_component(key)),
                    Self::input_line(Self::decode_urlencoded_component(value)),
                )
            })
            .filter(|kv| !kv.key.is_empty() || !kv.value.is_empty())
            .collect()
    }

    fn decode_urlencoded_component(value: &str) -> String {
        let bytes = value.as_bytes();
        let mut decoded = Vec::with_capacity(bytes.len());
        let mut i = 0;

        while i < bytes.len() {
            match bytes[i] {
                b'+' => {
                    decoded.push(b' ');
                    i += 1;
                }
                b'%' if i + 2 < bytes.len() => {
                    if let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2]))
                    {
                        decoded.push((hi << 4) | lo);
                        i += 3;
                    } else {
                        decoded.push(bytes[i]);
                        i += 1;
                    }
                }
                byte => {
                    decoded.push(byte);
                    i += 1;
                }
            }
        }

        String::from_utf8_lossy(&decoded).to_string()
    }

    fn collect_multipart(
        rows: &[MultipartRow],
        body: &crate::models::RequestBody,
        cx: &App,
    ) -> Vec<FormDataPart> {
        rows.iter()
            .map(|row| {
                let existing = body.form_data.iter().find(|part| part.id == row.id);
                FormDataPart {
                    id: row.id.clone(),
                    name: row.name.read(cx).value().to_string(),
                    value: row.value.read(cx).value().to_string(),
                    enabled: existing.map(|part| part.enabled).unwrap_or(true),
                    kind: existing
                        .map(|part| part.kind.clone())
                        .unwrap_or(FormDataPartKind::Text),
                    content_type: existing
                        .map(|part| part.content_type.clone())
                        .unwrap_or_default(),
                }
            })
            .filter(|part| !part.name.is_empty() || !part.value.is_empty())
            .collect()
    }

    fn merge_path_params(url: &str, params: &mut Vec<KeyValue>) {
        for key in Self::url_param_keys(url) {
            if !params.iter().any(|param| param.key == key) {
                params.push(KeyValue::new(key, ""));
            }
        }
    }

    fn append_stream_batch(
        app_state: &Entity<AppState>,
        request_id: &Option<String>,
        run_id: u64,
        body: String,
        chunk_count: usize,
        cx: &mut AsyncApp,
    ) {
        if body.is_empty() || chunk_count == 0 {
            return;
        }

        let byte_count = body.len();
        let request_id = request_id.clone();
        let _ = cx.update(|cx| {
            app_state.update(cx, |state, cx| {
                if let Some(id) = &request_id {
                    if !state.is_request_run_current(id, run_id) {
                        return;
                    }
                }
                if !state.is_active_response_target(&request_id) {
                    return;
                }
                if let Some(resp) = &mut state.response {
                    resp.body.push_str(&body);
                    resp.size_bytes = resp.body.len();
                    resp.timeline.push(crate::models::TimelineEvent {
                        name: format!("{} chunks received ({} B)", chunk_count, byte_count),
                        timestamp: chrono::Local::now().format("%H:%M:%S.%3f").to_string(),
                        icon: crate::models::TimelineIcon::Info,
                        detail: None,
                    });
                    cx.emit(AppEvent::ResponseReceived);
                }
            });
        });
    }

    pub(crate) fn send_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut url_raw = self.url_input.read(cx).value().to_string();
        let normalized_url = crate::path_params::normalize_path_params(&url_raw);
        if normalized_url != url_raw {
            url_raw = normalized_url;
            self.url_input.update(cx, |state, cx| {
                state.set_value(url_raw.clone(), window, cx);
            });
        }
        if url_raw.is_empty() {
            return;
        }

        let mut req: crate::models::ApiRequest;
        req = self
            .app_state
            .read(cx)
            .active_request
            .clone()
            .unwrap_or_else(crate::models::ApiRequest::default);
        req.url = url_raw;

        req.params = Self::collect_kv(&self.param_rows, cx);
        Self::merge_path_params(&req.url, &mut req.params);
        req.headers = Self::collect_kv(&self.header_rows, cx);
        req.body.urlencoded = Self::collect_kv(&self.urlencoded_rows, cx);
        if matches!(req.body.body_type, BodyType::UrlEncoded) {
            req.body.content = crate::http::build_urlencoded_body(&req.body.urlencoded);
        } else {
            req.body.content = self.body_editor.read(cx).value().to_string();
        }
        req.body.form_data = Self::collect_multipart(&self.multipart_rows, &req.body, cx);
        self.sync_auth_inputs_to_config(&mut req.auth, cx);

        let resolved_url = self.app_state.read(cx).interpolate_variables(&req.url);
        let response_request_id = self
            .app_state
            .read(cx)
            .active_request_id
            .clone()
            .unwrap_or_else(|| req.id.clone());

        let run_id = self.app_state.update(cx, |state, cx| {
            state.active_request = Some(req.clone());
            state.save_active_request();
            let run_id = state.request_started(&response_request_id);
            state.response = None;
            cx.emit(AppEvent::SaveNeeded);
            cx.emit(AppEvent::LoadingChanged);
            run_id
        });

        let app_state_for_timer = self.app_state.clone();
        let timer_request_id = response_request_id.clone();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                let keep_running = app_state_for_timer.update(cx, |state, cx| {
                    let keep_running = state.request_is_loading(&timer_request_id);
                    state.sync_active_activity_fields();
                    if keep_running {
                        cx.emit(AppEvent::LoadingChanged);
                    }
                    keep_running
                });
                if !keep_running {
                    break;
                }
            }
        })
        .detach();

        let (initial_tx, initial_rx) =
            futures::channel::oneshot::channel::<crate::models::HttpResponse>();
        let (chunk_tx, mut chunk_rx) = futures::channel::mpsc::unbounded::<String>();
        let (done_tx, done_rx) =
            futures::channel::oneshot::channel::<Result<crate::models::HttpResponse, String>>();

        std::thread::spawn(move || {
            let client = crate::http::HttpClient::new();
            let mut initial_tx = Some(initial_tx);
            let chunk_tx_clone = chunk_tx.clone();

            let result = client.execute(
                &req,
                &resolved_url,
                |initial_resp| {
                    if let Some(tx) = initial_tx.take() {
                        let _ = tx.send(initial_resp);
                    }
                },
                |chunk_bytes| {
                    let s = String::from_utf8_lossy(chunk_bytes).to_string();
                    let _ = chunk_tx_clone.unbounded_send(s);
                },
            );
            drop(chunk_tx);
            let _ = done_tx.send(result);
        });

        let app_state = self.app_state.clone();
        let initial_request_id = response_request_id.clone();
        let chunk_request_id = Some(response_request_id.clone());
        let done_request_id = response_request_id.clone();
        cx.spawn(async move |_, mut cx| {
            use futures::StreamExt;
            if let Ok(initial_resp) = initial_rx.await {
                let _ = cx.update(|cx| {
                    app_state.update(cx, |state, cx| {
                        let run_is_current =
                            state.is_request_run_current(&initial_request_id, run_id);
                        if run_is_current {
                            state.request_initial_response(
                                &initial_request_id,
                                run_id,
                                &initial_resp,
                            );
                        }
                        if run_is_current
                            && state.is_active_response_target(&Some(initial_request_id.clone()))
                        {
                            state.response = Some(initial_resp);
                            cx.emit(AppEvent::ResponseReceived);
                        }
                        cx.emit(AppEvent::LoadingChanged);
                    });
                });
            }

            let mut pending_body = String::new();
            let mut pending_chunks = 0usize;
            let mut last_flush = Instant::now();
            while let Some(chunk) = chunk_rx.next().await {
                pending_body.push_str(&chunk);
                pending_chunks += 1;

                if pending_body.len() >= 32 * 1024
                    || pending_chunks >= 32
                    || last_flush.elapsed() >= Duration::from_millis(80)
                {
                    let body = std::mem::take(&mut pending_body);
                    let chunks = std::mem::take(&mut pending_chunks);
                    Self::append_stream_batch(
                        &app_state,
                        &chunk_request_id,
                        run_id,
                        body,
                        chunks,
                        &mut cx,
                    );
                    last_flush = Instant::now();
                }
            }
            Self::append_stream_batch(
                &app_state,
                &chunk_request_id,
                run_id,
                pending_body,
                pending_chunks,
                &mut cx,
            );

            if let Ok(result) = done_rx.await {
                let _ = cx.update(|cx| {
                    let mut response_to_store = None;
                    app_state.update(cx, |state, cx| {
                        let run_is_current = state.is_request_run_current(&done_request_id, run_id);
                        state.request_finished(&done_request_id, run_id, &result);
                        if run_is_current {
                            match result {
                                Ok(resp) => {
                                    let history = state
                                        .workspace
                                        .response_cache
                                        .entry(done_request_id.clone())
                                        .or_default();
                                    history.push(resp.clone());
                                    let overflow = history
                                        .len()
                                        .saturating_sub(MAX_RESPONSE_HISTORY_PER_REQUEST);
                                    if overflow > 0 {
                                        history.drain(0..overflow);
                                    }
                                    response_to_store =
                                        Some((done_request_id.clone(), resp.clone()));
                                    if state
                                        .is_active_response_target(&Some(done_request_id.clone()))
                                    {
                                        state.response = Some(resp);
                                    }
                                }
                                Err(e) => {
                                    if state
                                        .is_active_response_target(&Some(done_request_id.clone()))
                                    {
                                        state.error = Some(e);
                                        state.response = None;
                                    }
                                }
                            }
                        }
                        state.sync_active_activity_fields();
                        cx.emit(AppEvent::LoadingChanged);
                        cx.emit(AppEvent::ResponseReceived);
                    });
                    if let Some((id, resp)) = response_to_store {
                        cx.background_executor()
                            .spawn(async move {
                                let _ = crate::storage::append_response(&id, &resp);
                            })
                            .detach();
                    }
                });
            }
        })
        .detach();
    }
}

impl Render for RequestPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_request = self.app_state.read(cx).active_request.is_some();
        let active_tab = self.active_tab;

        let param_count = self
            .param_rows
            .iter()
            .filter(|(k, _, _)| !k.read(cx).value().is_empty())
            .count();
        let header_count = self
            .header_rows
            .iter()
            .filter(|(k, _, _)| !k.read(cx).value().is_empty())
            .count();
        let urlencoded_count = self
            .urlencoded_rows
            .iter()
            .filter(|(k, v, _)| !k.read(cx).value().is_empty() || !v.read(cx).value().is_empty())
            .count();
        let (body_type, multipart_count, auth_type) = self
            .app_state
            .read(cx)
            .active_request
            .as_ref()
            .map(|req| {
                (
                    req.body.body_type.clone(),
                    req.body
                        .form_data
                        .iter()
                        .filter(|part| !part.name.is_empty() || !part.value.is_empty())
                        .count(),
                    req.auth.auth_type.clone(),
                )
            })
            .unwrap_or((BodyType::None, 0, AuthType::None));

        let params_label = if param_count > 0 {
            format!("Params ({})", param_count)
        } else {
            "Params".to_string()
        };
        let headers_label = if header_count > 0 {
            format!("Headers ({})", header_count)
        } else {
            "Headers".to_string()
        };
        let body_count = match &body_type {
            BodyType::FormData => multipart_count,
            BodyType::UrlEncoded => urlencoded_count,
            _ => 0,
        };
        let body_label = if body_count > 0 {
            format!("{} ({})", Self::body_type_label(&body_type), body_count)
        } else {
            Self::body_type_label(&body_type).to_string()
        };
        let auth_label = if matches!(auth_type, AuthType::None) {
            "Auth".to_string()
        } else {
            Self::auth_type_label(&auth_type).to_string()
        };

        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().background)
            .border_l_1()
            .border_color(cx.theme().border)
            // URL bar always visible — paste cURL here or select a request
            .child(self.render_url_bar(window, cx))
            .when(!has_request, |el| {
                el.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .overflow_hidden()
                        .text_color(cx.theme().muted_foreground)
                        .text_sm()
                        .px_4()
                        .text_center()
                        .child("Paste cURL or select a request"),
                )
            })
            .when(has_request, |el| {
                el.child(
                    TabBar::new("req-tabs")
                        .selected_index(active_tab.index())
                        .on_click(cx.listener(|this, ix, _, cx| {
                            this.active_tab = RequestTab::from_index(*ix);
                            cx.notify();
                        }))
                        .child(Tab::new().label(body_label))
                        .child(Tab::new().label(params_label))
                        .child(Tab::new().label(headers_label))
                        .child(Tab::new().label(auth_label))
                        .child(Tab::new().label("Info")),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .when(active_tab == RequestTab::Params, |el| {
                            el.child(self.render_params_tab(cx))
                        })
                        .when(active_tab == RequestTab::Headers, |el| {
                            el.child(self.render_headers_tab(cx))
                        })
                        .when(active_tab == RequestTab::Body, |el| {
                            el.child(self.render_body_tab(cx))
                        })
                        .when(active_tab == RequestTab::Auth, |el| {
                            el.child(self.render_auth_tab(cx))
                        })
                        .when(active_tab == RequestTab::Info, |el| {
                            el.child(
                                div()
                                    .p_3()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Request info"),
                            )
                        }),
                )
            })
    }
}

fn kv_header(cx: &App) -> impl IntoElement {
    h_flex()
        .gap_2()
        .pb_1()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .flex_1()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Key"),
        )
        .child(
            div()
                .flex_1()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Value"),
        )
        .child(div().w(px(28.)).child(""))
}

fn multipart_header(cx: &App) -> impl IntoElement {
    h_flex()
        .gap_2()
        .pb_1()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(div().w(px(20.)).child(""))
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
                .child("Value / File"),
        )
        .child(div().w(px(84.)).text_xs().child("Type"))
        .child(div().w(px(28.)).child(""))
}

fn urlencoded_header(cx: &App) -> impl IntoElement {
    h_flex()
        .gap_2()
        .pb_1()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(div().w(px(20.)).child(""))
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
        .child(div().w(px(28.)).child(""))
}

fn kv_input_row(
    i: usize,
    key: &Entity<InputState>,
    val: &Entity<InputState>,
    enabled: bool,
    is_headers: bool,
    cx: &mut Context<RequestPanel>,
) -> impl IntoElement {
    h_flex()
        .gap_2()
        .items_center()
        .py_0p5()
        .child(
            Checkbox::new(format!("chk-{}-{}", if is_headers { "h" } else { "p" }, i))
                .checked(enabled)
                .on_click(cx.listener(move |this, enabled: &bool, window, cx| {
                    let rows = if is_headers {
                        &mut this.header_rows
                    } else {
                        &mut this.param_rows
                    };
                    if let Some(row) = rows.get_mut(i) {
                        row.2 = *enabled;
                        this.sync_active_request(window, cx);
                        cx.notify();
                    }
                })),
        )
        .child(div().flex_1().child(Input::new(key)))
        .child(div().flex_1().child(Input::new(val)))
        .child(
            Button::new(format!(
                "del-row-{}-{}",
                if is_headers { "h" } else { "p" },
                i
            ))
            .icon(IconName::Close)
            .tooltip("Remove row")
            .ghost()
            .xsmall()
            .on_click(cx.listener(move |this, _, window, cx| {
                let rows = if is_headers {
                    &mut this.header_rows
                } else {
                    &mut this.param_rows
                };
                if i < rows.len() {
                    rows.remove(i);
                }
                this.sync_active_request(window, cx);
                cx.notify();
            })),
        )
}

fn collection_contains_request(collection: &crate::models::Collection, request_id: &str) -> bool {
    items_contain_request(&collection.items, request_id)
}

fn items_contain_request(items: &[crate::models::CollectionItem], request_id: &str) -> bool {
    items.iter().any(|item| match item {
        crate::models::CollectionItem::Request(request) => request.id == request_id,
        crate::models::CollectionItem::Folder(folder) => {
            items_contain_request(&folder.items, request_id)
        }
    })
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn default_if_empty(value: &str, default: &str) -> String {
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::RequestPanel;
    use crate::models::KeyValue;

    #[test]
    fn stale_path_param_prefix_rows_are_pruned() {
        let extracted = vec!["paramssksjska".to_string()];
        let previous = Vec::new();

        assert!(RequestPanel::is_stale_path_param_row(
            "params", "", &extracted, &previous
        ));
        assert!(!RequestPanel::is_stale_path_param_row(
            "paramssksjska",
            "",
            &extracted,
            &previous
        ));
        assert!(!RequestPanel::is_stale_path_param_row(
            "limit", "", &extracted, &previous
        ));
        assert!(!RequestPanel::is_stale_path_param_row(
            "params", "manual", &extracted, &previous
        ));
    }

    #[test]
    fn blank_query_params_with_rows_become_placeholders() {
        let params = vec![KeyValue::new("limit", "10"), KeyValue::new("offset", "")];

        assert_eq!(
            RequestPanel::normalize_query_param_placeholders(
                "{{baseUrl}}/api/v2/berry/?limit=&offset=&q=",
                &params
            ),
            "{{baseUrl}}/api/v2/berry/?limit=:limit&offset=:offset&q="
        );
    }
}
