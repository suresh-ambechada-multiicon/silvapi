use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::PopupMenuItem,
    tab::{Tab, TabBar},
    v_flex,
};

use crate::{
    models::{AuthType, BodyType, HttpMethod, KeyValue},
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
    auth_input: Entity<InputState>,
    // editable rows: (key_input, value_input)
    param_rows: Vec<(Entity<InputState>, Entity<InputState>, bool)>,
    header_rows: Vec<(Entity<InputState>, Entity<InputState>, bool)>,
    row_subs: Vec<Subscription>,
    _subs: Vec<Subscription>,
    timer_task: Option<gpui::Task<()>>,
    response_task: Option<gpui::Task<()>>,
    focus_handle: gpui::FocusHandle,
}

impl gpui::Focusable for RequestPanel {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl RequestPanel {
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
        let auth_input = cx.new(|cx| InputState::new(window, cx).placeholder("Token value"));

        let focus_handle = cx.focus_handle();

        let request_sub = cx.subscribe_in(&app_state, window, {
            let url_input = url_input.clone();
            let body_editor = body_editor.clone();
            move |this, app_state, ev: &AppEvent, window, cx| {
                if matches!(ev, AppEvent::LoadingChanged | AppEvent::WorkspaceChanged) {
                    cx.notify();
                }

                if matches!(ev, AppEvent::RequestSelected) {
                    if let Some(req) = &app_state.read(cx).active_request {
                        let url = req.url.clone();
                        let body = req.body.content.clone();
                        let params = req.params.clone();
                        let headers = req.headers.clone();

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

                        this.header_rows = headers
                            .iter()
                            .map(|kv| this.make_kv_row(&kv.key, &kv.value, kv.enabled, window, cx))
                            .collect();

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

        let auth_sub = cx.subscribe_in(
            &auth_input,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.sync_active_request(window, cx);
                }
            },
        );

        Self {
            app_state,
            active_tab: RequestTab::Body,
            url_input,
            body_editor,
            auth_input,
            param_rows: vec![],
            header_rows: vec![],
            row_subs: vec![],
            _subs: vec![request_sub, url_sub, body_sub, auth_sub],
            timer_task: None,
            response_task: None,
            focus_handle,
        }
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
        k.update(cx, |s, cx| s.set_value(key.to_string(), window, cx));
        v.update(cx, |s, cx| s.set_value(value.to_string(), window, cx));

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
            || self
                .auth_input
                .read(cx)
                .focus_handle(cx)
                .contains_focused(window, cx)
        {
            return true;
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
        false
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
                self.header_rows = parsed
                    .headers
                    .iter()
                    .map(|kv| self.make_kv_row(&kv.key, &kv.value, kv.enabled, window, cx))
                    .collect();

                self.app_state.update(cx, |state, cx| {
                    let mut ar = parsed.clone();
                    if let Some(old) = &state.active_request {
                        ar.id = old.id.clone();
                    }
                    state.active_request = Some(ar);
                    state.save_active_request();
                    cx.emit(AppEvent::SaveNeeded);
                });
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
            req.url = url.clone();

            req.params = Self::collect_kv(&self.param_rows, cx);
            req.headers = Self::collect_kv(&self.header_rows, cx);
            req.body.content = self.body_editor.read(cx).value().to_string();

            let auth_val = self.auth_input.read(cx).value().to_string();
            match req.auth.auth_type {
                AuthType::Bearer => req.auth.bearer_token = auth_val,
                AuthType::Basic => req.auth.basic_username = auth_val,
                AuthType::ApiKey => req.auth.api_key_value = auth_val,
                AuthType::None => {}
            }

            state.save_active_request();
            cx.emit(AppEvent::SaveNeeded);
        });

        // Outside state update, update param rows
        let url = self.url_input.read(cx).value().to_string();
        let extracted = crate::path_params::extract_path_params(&url);

        let mut current_keys: Vec<String> = self
            .param_rows
            .iter()
            .map(|(k, _, _)| k.read(cx).value().to_string())
            .collect();
        let mut changed = false;
        for ext in extracted {
            if !current_keys.contains(&ext) {
                let row = self.make_kv_row(&ext, "", true, window, cx);
                self.param_rows.push(row);
                current_keys.push(ext);
                changed = true;
            }
        }

        if changed {
            self.app_state.update(cx, |state, cx| {
                if let Some(req) = &mut state.active_request {
                    req.params = Self::collect_kv(&self.param_rows, cx);
                    state.save_active_request();
                    cx.emit(AppEvent::SaveNeeded);
                }
            });
            cx.notify();
        }
    }

    fn render_url_bar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            // Input is a direct flex child with flex_1 so it fills remaining space
            .child(Input::new(&self.url_input).flex_1().min_w(px(0.)))
            .child(if is_loading {
                Button::new("cancel-btn")
                    .label("Cancel")
                    .danger()
                    .small()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.timer_task = None;
                        this.response_task = None;
                        this.app_state.update(cx, |state, cx| {
                            state.is_loading = false;
                            state.request_started_at = None;
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
                    .label("+ Add")
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
                    .label("+ Add")
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

    fn render_body_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body_type = self
            .app_state
            .read(cx)
            .active_request
            .as_ref()
            .map(|r| r.body.body_type.clone())
            .unwrap_or(BodyType::None);

        let types = [
            (BodyType::None, "none"),
            (BodyType::Json, "JSON"),
            (BodyType::FormData, "Form"),
            (BodyType::Raw, "Raw"),
        ];

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .px_3()
                    .py_1p5()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .children(types.iter().map(|(bt, label)| {
                        let selected = &body_type == bt;
                        let bt = bt.clone();
                        Button::new(format!("body-{}", label))
                            .label(*label)
                            .ghost()
                            .xsmall()
                            .when(selected, |b| b.primary())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.app_state.update(cx, |state, cx| {
                                    if let Some(req) = &mut state.active_request {
                                        req.body.body_type = bt.clone();
                                    }
                                    state.save_active_request();
                                    cx.emit(AppEvent::SaveNeeded);
                                });
                            }))
                    }))
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
                    .when(!matches!(body_type, BodyType::None), |el| {
                        el.child(Input::new(&self.body_editor).h_full())
                    })
                    .when(matches!(body_type, BodyType::None), |el| {
                        el.child(
                            div()
                                .flex()
                                .size_full()
                                .items_center()
                                .justify_center()
                                .text_color(cx.theme().muted_foreground)
                                .text_sm()
                                .child("No body"),
                        )
                    }),
            )
    }

    fn render_auth_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let auth_type = self
            .app_state
            .read(cx)
            .active_request
            .as_ref()
            .map(|r| r.auth.auth_type.clone())
            .unwrap_or(AuthType::None);

        let types = [
            (AuthType::None, "None"),
            (AuthType::Bearer, "Bearer"),
            (AuthType::Basic, "Basic"),
            (AuthType::ApiKey, "API Key"),
        ];

        v_flex()
            .size_full()
            .p_3()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().text_sm().child("Type:"))
                    .children(types.iter().map(|(at, label)| {
                        let selected = &auth_type == at;
                        let at = at.clone();
                        Button::new(format!("auth-{}", label))
                            .label(*label)
                            .ghost()
                            .xsmall()
                            .when(selected, |b| b.primary())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.app_state.update(cx, |state, cx| {
                                    if let Some(req) = &mut state.active_request {
                                        req.auth.auth_type = at.clone();
                                    }
                                    state.save_active_request();
                                    cx.emit(AppEvent::SaveNeeded);
                                });
                            }))
                    })),
            )
            .when(
                matches!(
                    auth_type,
                    AuthType::Bearer | AuthType::Basic | AuthType::ApiKey
                ),
                |el| {
                    el.child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(match &auth_type {
                                        AuthType::Bearer => "Bearer Token",
                                        AuthType::Basic => "Username:Password",
                                        AuthType::ApiKey => "API Key",
                                        _ => "Value",
                                    }),
                            )
                            .child(Input::new(&self.auth_input)),
                    )
                },
            )
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

    fn merge_path_params(url: &str, params: &mut Vec<KeyValue>) {
        for key in crate::path_params::extract_path_params(url) {
            if !params.iter().any(|param| param.key == key) {
                params.push(KeyValue::new(key, ""));
            }
        }
    }

    fn append_stream_batch(
        app_state: &Entity<AppState>,
        request_id: &Option<String>,
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
        req.body.content = self.body_editor.read(cx).value().to_string();
        let auth_val = self.auth_input.read(cx).value().to_string();
        match req.auth.auth_type {
            AuthType::Bearer => req.auth.bearer_token = auth_val,
            AuthType::Basic => req.auth.basic_username = auth_val,
            AuthType::ApiKey => req.auth.api_key_value = auth_val,
            AuthType::None => {}
        }

        let resolved_url = self.app_state.read(cx).interpolate_variables(&req.url);
        let response_request_id = self
            .app_state
            .read(cx)
            .active_request_id
            .clone()
            .or_else(|| Some(req.id.clone()));

        self.app_state.update(cx, |state, cx| {
            state.active_request = Some(req.clone());
            state.save_active_request();
            state.is_loading = true;
            state.request_started_at = Some(std::time::Instant::now());
            state.response = None;
            state.error = None;
            cx.emit(AppEvent::SaveNeeded);
            cx.emit(AppEvent::LoadingChanged);
        });

        let app_state_for_timer = self.app_state.clone();
        self.timer_task = Some(cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                let keep_running = app_state_for_timer.update(cx, |state, cx| {
                    let keep_running = state.is_loading;
                    if keep_running {
                        cx.emit(AppEvent::LoadingChanged);
                    }
                    keep_running
                });
                if !keep_running {
                    break;
                }
            }
        }));

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
        let chunk_request_id = response_request_id.clone();
        let done_request_id = response_request_id.clone();
        self.response_task = Some(cx.spawn(async move |_, mut cx| {
            use futures::StreamExt;
            if let Ok(initial_resp) = initial_rx.await {
                let _ = cx.update(|cx| {
                    app_state.update(cx, |state, cx| {
                        if state.is_active_response_target(&initial_request_id) {
                            state.response = Some(initial_resp);
                            cx.emit(AppEvent::ResponseReceived);
                        }
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
                    Self::append_stream_batch(&app_state, &chunk_request_id, body, chunks, &mut cx);
                    last_flush = Instant::now();
                }
            }
            Self::append_stream_batch(
                &app_state,
                &chunk_request_id,
                pending_body,
                pending_chunks,
                &mut cx,
            );

            if let Ok(result) = done_rx.await {
                let _ = cx.update(|cx| {
                    let mut response_to_store = None;
                    app_state.update(cx, |state, cx| {
                        state.is_loading = false;
                        match result {
                            Ok(resp) => {
                                if let Some(id) = &done_request_id {
                                    let history = state
                                        .workspace
                                        .response_cache
                                        .entry(id.clone())
                                        .or_default();
                                    history.push(resp.clone());
                                    let overflow = history
                                        .len()
                                        .saturating_sub(MAX_RESPONSE_HISTORY_PER_REQUEST);
                                    if overflow > 0 {
                                        history.drain(0..overflow);
                                    }
                                    response_to_store = Some((id.clone(), resp.clone()));
                                }
                                if state.is_active_response_target(&done_request_id) {
                                    state.response = Some(resp);
                                }
                                state.error = None;
                            }
                            Err(e) => {
                                if state.is_active_response_target(&done_request_id) {
                                    state.error = Some(e);
                                    state.response = None;
                                }
                            }
                        }
                        state.request_started_at = None;
                        cx.emit(AppEvent::ResponseReceived);
                    });
                    if let Some((id, resp)) = response_to_store {
                        cx.background_executor()
                            .spawn(async move {
                                if let Err(err) = crate::storage::append_response(&id, &resp) {
                                    eprintln!("Failed to save response history: {}", err);
                                }
                            })
                            .detach();
                    }
                });
            }
        }));
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
                        .child(Tab::new().label("Body"))
                        .child(Tab::new().label(params_label))
                        .child(Tab::new().label(headers_label))
                        .child(Tab::new().label("Auth"))
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
            .label("✕")
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
