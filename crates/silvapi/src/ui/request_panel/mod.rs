use std::time::Duration;
use instant::Instant;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_kit::component::{
    ActiveTheme, IconName, IndexPath, Sizable as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{EditorState, Input, InputEvent, InputState},
    select::{Select, SelectEvent, SelectState},
    tab::{Tab, TabBar},
    v_flex,
};

use crate::state::{AppEvent, AppState};
use silvapi_core::models::{
    AuthType, BodyType, FormDataPart, HttpMethod, KeyValue,
};

const MAX_RESPONSE_HISTORY_PER_REQUEST: usize = 20;

pub(crate) const HTTP_METHODS: [&'static str; 7] = [
    "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS",
];

pub(crate) const BODY_TYPES: [&'static str; 6] = [
    "No Body", "JSON", "Multi-Part", "Url Encoded", "Other", "Binary File",
];

pub(crate) const AUTH_TYPES: [&'static str; 7] = [
    "No Auth", "Bearer Token", "Basic Auth", "API Key", "AWS Signature", "JWT Bearer", "OAuth 2.0",
];

pub(crate) const API_KEY_PLACEMENTS: [&'static str; 2] = [
    "Header", "Query Param",
];

pub(crate) const JWT_ALGORITHMS: [&'static str; 6] = [
    "HS256", "HS384", "HS512", "RS256", "RS384", "RS512",
];

pub(crate) const OAUTH_GRANTS: [&'static str; 4] = [
    "Authorization Code", "Implicit", "Resource Owner Password Credential", "Client Credentials",
];

pub(crate) const OAUTH_TOKEN_TARGETS: [&'static str; 2] = [
    "access_token", "id_token",
];

pub(crate) const OAUTH_RESPONSE_TYPES: [&'static str; 3] = [
    "Access Token", "ID Token", "Access Token + ID Token",
];

pub(crate) const OAUTH_SEND_CREDENTIALS: [&'static str; 2] = [
    "In Request Body", "As Basic Authentication",
];

pub(crate) const MAX_HISTORY_RESPONSE_BODY_BYTES: usize = 1024 * 1024; // 1 MB

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
    pub(crate) method_select: Entity<SelectState<Vec<&'static str>>>,
    pub(crate) body_type_select: Entity<SelectState<Vec<&'static str>>>,
    pub(crate) auth_type_select: Entity<SelectState<Vec<&'static str>>>,
    pub(crate) api_key_placement_select: Entity<SelectState<Vec<&'static str>>>,
    pub(crate) jwt_alg_select: Entity<SelectState<Vec<&'static str>>>,
    pub(crate) oauth_grant_select: Entity<SelectState<Vec<&'static str>>>,
    pub(crate) oauth_token_target_select: Entity<SelectState<Vec<&'static str>>>,
    pub(crate) oauth_response_type_select: Entity<SelectState<Vec<&'static str>>>,
    pub(crate) oauth_send_credentials_select: Entity<SelectState<Vec<&'static str>>>,
    pub(crate) body_editor: Entity<EditorState>,
    body_wrap: bool,
    auth_inputs: AuthInputs,
    url_variable_rows: Vec<UrlVariableRow>,
    path_param_keys: Vec<String>,
    // editable rows: (key_input, value_input)
    param_rows: Vec<(Entity<InputState>, Entity<InputState>, bool)>,
    header_rows: Vec<(Entity<InputState>, Entity<InputState>, bool)>,
    urlencoded_rows: Vec<(Entity<InputState>, Entity<InputState>, bool)>,
    multipart_rows: Vec<MultipartRow>,
    row_subs: Vec<Subscription>,
    url_variable_subs: Vec<Subscription>,
    _subs: Vec<Subscription>,
    focus_handle: gpui_kit::FocusHandle,
}

pub(crate) struct MultipartRow {
    pub(crate) id: String,
    pub(crate) name: Entity<InputState>,
    pub(crate) value: Entity<InputState>,
    pub(crate) kind_select: Entity<SelectState<Vec<&'static str>>>,
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

impl gpui_kit::Focusable for RequestPanel {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

mod auth;
mod body;
mod kv;
mod url_bar;

impl RequestPanel {
    fn input_line(value: impl AsRef<str>) -> String {
        value.as_ref().replace(['\r', '\n'], " ")
    }

    fn request_with_resolved_variables(
        state: &AppState,
        mut req: silvapi_core::models::ApiRequest,
    ) -> silvapi_core::models::ApiRequest {
        req.url = state.interpolate_variables(&req.url);
        for param in &mut req.params {
            param.key = state.interpolate_variables(&param.key);
            param.value = state.interpolate_variables(&param.value);
        }
        for header in &mut req.headers {
            header.key = state.interpolate_variables(&header.key);
            header.value = state.interpolate_variables(&header.value);
        }
        req.body.content = state.interpolate_variables(&req.body.content);
        for field in &mut req.body.urlencoded {
            field.key = state.interpolate_variables(&field.key);
            field.value = state.interpolate_variables(&field.value);
        }
        for part in &mut req.body.form_data {
            part.name = state.interpolate_variables(&part.name);
            part.value = state.interpolate_variables(&part.value);
            part.content_type = state.interpolate_variables(&part.content_type);
        }
        let auth = &mut req.auth;
        auth.bearer_token = state.interpolate_variables(&auth.bearer_token);
        auth.bearer_prefix = state.interpolate_variables(&auth.bearer_prefix);
        auth.basic_username = state.interpolate_variables(&auth.basic_username);
        auth.basic_password = state.interpolate_variables(&auth.basic_password);
        auth.api_key_name = state.interpolate_variables(&auth.api_key_name);
        auth.api_key_value = state.interpolate_variables(&auth.api_key_value);
        auth.aws_access_key_id = state.interpolate_variables(&auth.aws_access_key_id);
        auth.aws_secret_access_key = state.interpolate_variables(&auth.aws_secret_access_key);
        auth.aws_service = state.interpolate_variables(&auth.aws_service);
        auth.aws_region = state.interpolate_variables(&auth.aws_region);
        auth.aws_session_token = state.interpolate_variables(&auth.aws_session_token);
        auth.jwt_secret = state.interpolate_variables(&auth.jwt_secret);
        auth.jwt_payload = state.interpolate_variables(&auth.jwt_payload);
        auth.oauth_client_id = state.interpolate_variables(&auth.oauth_client_id);
        auth.oauth_client_secret = state.interpolate_variables(&auth.oauth_client_secret);
        auth.oauth_authorization_url = state.interpolate_variables(&auth.oauth_authorization_url);
        auth.oauth_access_token_url = state.interpolate_variables(&auth.oauth_access_token_url);
        auth.oauth_redirect_uri = state.interpolate_variables(&auth.oauth_redirect_uri);
        auth.oauth_state = state.interpolate_variables(&auth.oauth_state);
        auth.oauth_audience = state.interpolate_variables(&auth.oauth_audience);
        auth.oauth_username = state.interpolate_variables(&auth.oauth_username);
        auth.oauth_password = state.interpolate_variables(&auth.oauth_password);
        auth.oauth_scope = state.interpolate_variables(&auth.oauth_scope);
        auth.oauth_header_name = state.interpolate_variables(&auth.oauth_header_name);
        auth.oauth_header_prefix = state.interpolate_variables(&auth.oauth_header_prefix);
        req
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
        _language: &'static str,
    ) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
        })
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

    pub(crate) fn method_to_index(method: &HttpMethod) -> usize {
        match method {
            HttpMethod::GET => 0,
            HttpMethod::POST => 1,
            HttpMethod::PUT => 2,
            HttpMethod::PATCH => 3,
            HttpMethod::DELETE => 4,
            HttpMethod::HEAD => 5,
            HttpMethod::OPTIONS => 6,
        }
    }

    pub(crate) fn index_to_method(idx: usize) -> HttpMethod {
        match idx {
            0 => HttpMethod::GET,
            1 => HttpMethod::POST,
            2 => HttpMethod::PUT,
            3 => HttpMethod::PATCH,
            4 => HttpMethod::DELETE,
            5 => HttpMethod::HEAD,
            6 => HttpMethod::OPTIONS,
            _ => HttpMethod::GET,
        }
    }

    pub(crate) fn body_type_to_index(b: &BodyType) -> usize {
        match b {
            BodyType::None => 0,
            BodyType::Json => 1,
            BodyType::FormData => 2,
            BodyType::UrlEncoded => 3,
            BodyType::Raw => 4,
            BodyType::BinaryFile => 5,
        }
    }

    pub(crate) fn index_to_body_type(idx: usize) -> BodyType {
        match idx {
            0 => BodyType::None,
            1 => BodyType::Json,
            2 => BodyType::FormData,
            3 => BodyType::UrlEncoded,
            4 => BodyType::Raw,
            5 => BodyType::BinaryFile,
            _ => BodyType::None,
        }
    }

    pub(crate) fn auth_type_to_index(a: &AuthType) -> usize {
        match a {
            AuthType::None => 0,
            AuthType::Bearer => 1,
            AuthType::Basic => 2,
            AuthType::ApiKey => 3,
            AuthType::AwsV4 => 4,
            AuthType::Jwt => 5,
            AuthType::OAuth2 => 6,
        }
    }

    pub(crate) fn index_to_auth_type(idx: usize) -> AuthType {
        match idx {
            0 => AuthType::None,
            1 => AuthType::Bearer,
            2 => AuthType::Basic,
            3 => AuthType::ApiKey,
            4 => AuthType::AwsV4,
            5 => AuthType::Jwt,
            6 => AuthType::OAuth2,
            _ => AuthType::None,
        }
    }

    pub(crate) fn sync_selects_from_request(
        &self,
        req: &silvapi_core::models::ApiRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let method_idx = Self::method_to_index(&req.method);
        self.method_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(method_idx)), window, cx);
        });

        let body_idx = Self::body_type_to_index(&req.body.body_type);
        self.body_type_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(body_idx)), window, cx);
        });

        let auth_idx = Self::auth_type_to_index(&req.auth.auth_type);
        self.auth_type_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(auth_idx)), window, cx);
        });

        let api_key_idx = if req.auth.api_key_in_header { 0 } else { 1 };
        self.api_key_placement_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(api_key_idx)), window, cx);
        });

        let jwt_alg_idx = JWT_ALGORITHMS
            .iter()
            .position(|&a| a.eq_ignore_ascii_case(&req.auth.jwt_algorithm))
            .unwrap_or(0);
        self.jwt_alg_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(jwt_alg_idx)), window, cx);
        });

        let oauth_grant_idx = OAUTH_GRANTS
            .iter()
            .position(|&g| g.eq_ignore_ascii_case(&req.auth.oauth_grant_type))
            .unwrap_or(0);
        self.oauth_grant_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(oauth_grant_idx)), window, cx);
        });

        let oauth_target_idx = OAUTH_TOKEN_TARGETS
            .iter()
            .position(|&t| t.eq_ignore_ascii_case(&req.auth.oauth_token_target))
            .unwrap_or(0);
        self.oauth_token_target_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(oauth_target_idx)), window, cx);
        });

        let oauth_resp_idx = OAUTH_RESPONSE_TYPES
            .iter()
            .position(|&r| r.eq_ignore_ascii_case(&req.auth.oauth_response_type))
            .unwrap_or(0);
        self.oauth_response_type_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(oauth_resp_idx)), window, cx);
        });

        let oauth_send_idx = OAUTH_SEND_CREDENTIALS
            .iter()
            .position(|&c| c.eq_ignore_ascii_case(&req.auth.oauth_send_credentials))
            .unwrap_or(0);
        self.oauth_send_credentials_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(oauth_send_idx)), window, cx);
        });
    }

    pub fn new(app_state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let url_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL or paste text..."));
        let body_editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("json")
                .line_number(true)
                .folding(true)
                .placeholder("Request body")
        });
        let method_select = cx.new(|cx| {
            SelectState::new(
                HTTP_METHODS.to_vec(),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
            .searchable(false)
        });
        let body_type_select = cx.new(|cx| {
            SelectState::new(
                BODY_TYPES.to_vec(),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
            .searchable(false)
        });
        let auth_type_select = cx.new(|cx| {
            SelectState::new(
                AUTH_TYPES.to_vec(),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
            .searchable(false)
        });
        let api_key_placement_select = cx.new(|cx| {
            SelectState::new(
                API_KEY_PLACEMENTS.to_vec(),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
            .searchable(false)
        });
        let jwt_alg_select = cx.new(|cx| {
            SelectState::new(
                JWT_ALGORITHMS.to_vec(),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
            .searchable(false)
        });
        let oauth_grant_select = cx.new(|cx| {
            SelectState::new(
                OAUTH_GRANTS.to_vec(),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
            .searchable(false)
        });
        let oauth_token_target_select = cx.new(|cx| {
            SelectState::new(
                OAUTH_TOKEN_TARGETS.to_vec(),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
            .searchable(false)
        });
        let oauth_response_type_select = cx.new(|cx| {
            SelectState::new(
                OAUTH_RESPONSE_TYPES.to_vec(),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
            .searchable(false)
        });
        let oauth_send_credentials_select = cx.new(|cx| {
            SelectState::new(
                OAUTH_SEND_CREDENTIALS.to_vec(),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
            .searchable(false)
        });

        let method_sub = cx.subscribe_in(
            &method_select,
            window,
            |this, _, ev: &SelectEvent<Vec<&'static str>>, _window, cx| {
                if let SelectEvent::Confirm(Some(val)) = ev {
                    if let Some(idx) = HTTP_METHODS.iter().position(|m| m == val) {
                        let method = Self::index_to_method(idx);
                        this.app_state.update(cx, |s, cx| {
                            if let Some(r) = &mut s.active_request {
                                r.method = method;
                            }
                            s.save_active_request();
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }
                }
            },
        );

        let body_type_sub = cx.subscribe_in(
            &body_type_select,
            window,
            |this, _, ev: &SelectEvent<Vec<&'static str>>, _window, cx| {
                if let SelectEvent::Confirm(Some(val)) = ev {
                    if let Some(idx) = BODY_TYPES.iter().position(|b| b == val) {
                        let target = Self::index_to_body_type(idx);
                        let lang = if matches!(target, BodyType::Json) {
                            "json"
                        } else {
                            "text"
                        };
                        this.body_editor.update(cx, |state, cx| {
                            state.set_highlighter(lang, cx);
                        });
                        this.app_state.update(cx, |state, cx| {
                            if let Some(req) = &mut state.active_request {
                                req.body.body_type = target.clone();
                                if matches!(target, BodyType::UrlEncoded)
                                    && req.body.urlencoded.is_empty()
                                {
                                    req.body.urlencoded =
                                        Self::parse_urlencoded_content(&req.body.content);
                                    if req.body.urlencoded.is_empty() {
                                        req.body.urlencoded.push(KeyValue::empty());
                                    }
                                }
                                if matches!(target, BodyType::FormData)
                                    && req.body.form_data.is_empty()
                                {
                                    req.body.form_data.push(FormDataPart::empty());
                                }
                            }
                            state.save_active_request();
                            cx.emit(AppEvent::WorkspaceChanged);
                            cx.emit(AppEvent::SaveNeeded);
                        });
                    }
                }
            },
        );

        let auth_type_sub = cx.subscribe_in(
            &auth_type_select,
            window,
            |this, _, ev: &SelectEvent<Vec<&'static str>>, _window, cx| {
                if let SelectEvent::Confirm(Some(val)) = ev {
                    if let Some(idx) = AUTH_TYPES.iter().position(|a| a == val) {
                        let target = Self::index_to_auth_type(idx);
                        this.app_state.update(cx, |state, cx| {
                            if let Some(req) = &mut state.active_request {
                                req.auth.auth_type = target;
                                Self::ensure_auth_defaults(&mut req.auth);
                            }
                            state.save_active_request();
                            cx.emit(AppEvent::WorkspaceChanged);
                            cx.emit(AppEvent::SaveNeeded);
                        });
                    }
                }
            },
        );

        let api_key_sub = cx.subscribe_in(
            &api_key_placement_select,
            window,
            |this, _, ev: &SelectEvent<Vec<&'static str>>, _window, cx| {
                if let SelectEvent::Confirm(Some(val)) = ev {
                    let in_header = *val == "Header";
                    Self::update_auth_config(&this.app_state, cx, |auth| {
                        auth.api_key_in_header = in_header;
                    });
                }
            },
        );

        let jwt_alg_sub = cx.subscribe_in(
            &jwt_alg_select,
            window,
            |this, _, ev: &SelectEvent<Vec<&'static str>>, _window, cx| {
                if let SelectEvent::Confirm(Some(val)) = ev {
                    let alg = val.to_string();
                    Self::update_auth_config(&this.app_state, cx, |auth| {
                        auth.jwt_algorithm = alg;
                    });
                }
            },
        );

        let oauth_grant_sub = cx.subscribe_in(
            &oauth_grant_select,
            window,
            |this, _, ev: &SelectEvent<Vec<&'static str>>, _window, cx| {
                if let SelectEvent::Confirm(Some(val)) = ev {
                    let grant = val.to_string();
                    Self::update_auth_config(&this.app_state, cx, |auth| {
                        auth.oauth_grant_type = grant;
                    });
                }
            },
        );

        let oauth_token_target_sub = cx.subscribe_in(
            &oauth_token_target_select,
            window,
            |this, _, ev: &SelectEvent<Vec<&'static str>>, _window, cx| {
                if let SelectEvent::Confirm(Some(val)) = ev {
                    let target = val.to_string();
                    Self::update_auth_config(&this.app_state, cx, |auth| {
                        auth.oauth_token_target = target;
                    });
                }
            },
        );

        let oauth_response_type_sub = cx.subscribe_in(
            &oauth_response_type_select,
            window,
            |this, _, ev: &SelectEvent<Vec<&'static str>>, _window, cx| {
                if let SelectEvent::Confirm(Some(val)) = ev {
                    let resp = val.to_string();
                    Self::update_auth_config(&this.app_state, cx, |auth| {
                        auth.oauth_response_type = resp;
                    });
                }
            },
        );

        let oauth_send_credentials_sub = cx.subscribe_in(
            &oauth_send_credentials_select,
            window,
            |this, _, ev: &SelectEvent<Vec<&'static str>>, _window, cx| {
                if let SelectEvent::Confirm(Some(val)) = ev {
                    let send = val.to_string();
                    Self::update_auth_config(&this.app_state, cx, |auth| {
                        auth.oauth_send_credentials = send;
                    });
                }
            },
        );

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

                        let body_lang = if matches!(req.body.body_type, BodyType::Json) {
                            "json"
                        } else {
                            "text"
                        };
                        url_input.update(cx, |state, cx| {
                            state.set_value(url, window, cx);
                        });
                        body_editor.update(cx, |state, cx| {
                            state.set_highlighter(body_lang, cx);
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
                        this.sync_selects_from_request(&req, window, cx);
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
            method_select,
            body_type_select,
            auth_type_select,
            api_key_placement_select,
            jwt_alg_select,
            oauth_grant_select,
            oauth_token_target_select,
            oauth_response_type_select,
            oauth_send_credentials_select,
            body_editor,
            body_wrap: false,
            auth_inputs,
            url_variable_rows: vec![],
            path_param_keys: vec![],
            param_rows: vec![],
            header_rows: vec![],
            urlencoded_rows: vec![],
            multipart_rows: vec![],
            row_subs: vec![],
            url_variable_subs: vec![],
            _subs: {
                let mut subs = vec![
                    request_sub,
                    url_sub,
                    body_sub,
                    method_sub,
                    body_type_sub,
                    auth_type_sub,
                    api_key_sub,
                    jwt_alg_sub,
                    oauth_grant_sub,
                    oauth_token_target_sub,
                    oauth_response_type_sub,
                    oauth_send_credentials_sub,
                ];
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
            this.sync_selects_from_request(&req, window, cx);
            if params_changed || url_changed {
                this.persist_url_and_param_rows(cx);
            }
        }

        this
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
            if let Ok(mut parsed) = silvapi_core::import::parse_curl(&url_raw) {
                parsed.url = silvapi_core::path_params::normalize_path_params(&parsed.url);
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
            let normalized_url = silvapi_core::path_params::normalize_path_params(&url);
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

    pub(crate) fn send_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut url_raw = self.url_input.read(cx).value().to_string();
        let normalized_url = silvapi_core::path_params::normalize_path_params(&url_raw);
        if normalized_url != url_raw {
            url_raw = normalized_url;
            self.url_input.update(cx, |state, cx| {
                state.set_value(url_raw.clone(), window, cx);
            });
        }
        if url_raw.is_empty() {
            return;
        }

        let mut req: silvapi_core::models::ApiRequest;
        req = self
            .app_state
            .read(cx)
            .active_request
            .clone()
            .unwrap_or_else(silvapi_core::models::ApiRequest::default);
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

        let send_req = {
            let state = self.app_state.read(cx);
            Self::request_with_resolved_variables(&state, req.clone())
        };
        let resolved_url = send_req.url.clone();
        // Fallback snapshot for a request that dies before `execute` assembles
        // the wire form (bad URL, unreadable body file). The real one comes back
        // over `prepared_rx` and supersedes this.
        let fallback_snapshot =
            silvapi_core::models::RequestSnapshot::from_request(&send_req, resolved_url.clone());
        let started_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let run_started = Instant::now();
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
            futures::channel::oneshot::channel::<silvapi_core::models::HttpResponse>();
        let (prepared_tx, prepared_rx) =
            futures::channel::oneshot::channel::<silvapi_core::models::RequestSnapshot>();
        let (chunk_tx, mut chunk_rx) = futures::channel::mpsc::unbounded::<String>();
        let (done_tx, done_rx) = futures::channel::oneshot::channel::<
            Result<silvapi_core::models::HttpResponse, String>,
        >();

        let join = crate::runtime::spawn(async move {
            let client = crate::http::HttpClient::new();
            let mut initial_tx = Some(initial_tx);
            let mut prepared_tx = Some(prepared_tx);
            let chunk_tx_clone = chunk_tx.clone();

            let result = client
                .execute(
                    &send_req,
                    &resolved_url,
                    |snapshot| {
                        if let Some(tx) = prepared_tx.take() {
                            let _ = tx.send(snapshot);
                        }
                    },
                    |initial_resp| {
                        if let Some(tx) = initial_tx.take() {
                            let _ = tx.send(initial_resp);
                        }
                    },
                    |chunk_bytes| {
                        let s = String::from_utf8_lossy(chunk_bytes).to_string();
                        let _ = chunk_tx_clone.unbounded_send(s);
                    },
                )
                .await;
            drop(chunk_tx);
            let _ = done_tx.send(result);
        });

        // Register the abort handle so "Cancel" can stop the request mid-flight.
        self.app_state.update(cx, |state, _| {
            state.register_run_abort(run_id, join.abort_handle());
        });

        let app_state = self.app_state.clone();
        let initial_request_id = response_request_id.clone();
        let chunk_request_id = Some(response_request_id.clone());
        let done_request_id = response_request_id.clone();
        cx.spawn(async move |_, mut cx| {
            use futures::StreamExt;
            // Resolves before the request is sent. Cancelled (=> `None`) only when
            // the request died during assembly, in which case the definition-only
            // fallback is the best record we have.
            let request_snapshot = prepared_rx.await.ok().unwrap_or(fallback_snapshot);

            if let Ok(mut initial_resp) = initial_rx.await {
                initial_resp.started_at = Some(started_at.clone());
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

                if (pending_body.len() >= 64 * 1024
                    && last_flush.elapsed() >= Duration::from_millis(50))
                    || last_flush.elapsed() >= Duration::from_millis(100)
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
                            // Failures are recorded too, so a timeout or TLS
                            // error stays inspectable in history alongside the
                            // request that caused it.
                            let (mut resp, error) = match result {
                                Ok(resp) => (resp, None),
                                Err(e) => (
                                    silvapi_core::models::HttpResponse::failed(
                                        e.clone(),
                                        run_started.elapsed().as_millis() as u64,
                                        None,
                                        None,
                                    ),
                                    Some(e),
                                ),
                            };
                            resp.request = Some(request_snapshot.clone());
                            resp.started_at = Some(started_at.clone());

                            if resp.body.len() <= MAX_HISTORY_RESPONSE_BODY_BYTES {
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
                                response_to_store = Some((done_request_id.clone(), resp.clone()));
                            }

                            if state.is_active_response_target(&Some(done_request_id.clone())) {
                                match error {
                                    Some(e) => {
                                        state.error = Some(e);
                                        state.response = None;
                                    }
                                    None => state.response = Some(resp),
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
                        .underline()
                        .small()
                        .px_3()
                        .border_b_1()
                        .border_color(cx.theme().border)
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

fn collection_contains_request(
    collection: &silvapi_core::models::Collection,
    request_id: &str,
) -> bool {
    items_contain_request(&collection.items, request_id)
}

fn items_contain_request(items: &[silvapi_core::models::CollectionItem], request_id: &str) -> bool {
    items.iter().any(|item| match item {
        silvapi_core::models::CollectionItem::Request(request) => request.id == request_id,
        silvapi_core::models::CollectionItem::Folder(folder) => {
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
mod tests;
