#![allow(unused_imports)]
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

use silvapi_core::{models::{AuthConfig, AuthType, BodyType, FormDataPart, FormDataPartKind, HttpMethod, KeyValue,}};
use crate::{state::{AppEvent, AppState}};

use super::*;

impl RequestPanel {
    pub(super) fn make_url_variable_row(
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

    pub(super) fn refresh_url_variable_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn extract_variable_names(text: &str) -> Vec<String> {
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

    pub(super) fn is_url_variable_name(name: &str) -> bool {
        !name.is_empty()
            && !name.starts_with('$')
            && name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
    }

    pub(super) fn is_url_path_param_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
    }

    pub(super) fn is_url_path_param_start(url: &str, colon_index: usize) -> bool {
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

    pub(super) fn url_param_keys(url: &str) -> Vec<String> {
        let mut keys = silvapi_core::path_params::extract_path_params(url);
        for key in silvapi_core::path_params::extract_query_value_params(url) {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys
    }

    pub(super) fn normalize_query_param_placeholders(url: &str, params: &[KeyValue]) -> String {
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

    pub(super) fn can_use_query_placeholder(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    }

    pub(super) fn variable_value(&self, name: &str, cx: &App) -> Option<String> {
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

    pub(super) fn path_param_value(&self, name: &str, cx: &App) -> Option<String> {
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

    pub(super) fn update_global_variable(&self, name: &str, value: String, cx: &mut Context<Self>) {
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
                    .push(silvapi_core::models::Variable::new(name, value));
            }
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    pub(super) fn render_url_bar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(super) fn render_url_field(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
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

    pub(super) fn render_url_segments(&self, url: &str, cx: &mut Context<Self>) -> Vec<AnyElement> {
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

    pub(super) fn render_url_text_segment(text: &str) -> AnyElement {
        div()
            .flex_shrink_0()
            .text_sm()
            .font_family("monospace")
            .whitespace_nowrap()
            .child(Self::input_line(text))
            .into_any_element()
    }

    pub(super) fn render_url_chip_tooltip(tooltip: String, cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(super) fn render_inline_url_variable(
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

    pub(super) fn render_inline_url_path_param(
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

    pub(super) fn sync_path_param_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
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

    pub(super) fn is_stale_path_param_row(
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

    pub(super) fn persist_param_rows(&self, cx: &mut Context<Self>) {
        self.app_state.update(cx, |state, cx| {
            if let Some(req) = &mut state.active_request {
                req.params = Self::collect_kv(&self.param_rows, cx);
                state.save_active_request();
                cx.emit(AppEvent::SaveNeeded);
            }
        });
    }

    pub(super) fn persist_url_and_param_rows(&self, cx: &mut Context<Self>) {
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

    pub(super) fn merge_path_params(url: &str, params: &mut Vec<KeyValue>) {
        for key in Self::url_param_keys(url) {
            if !params.iter().any(|param| param.key == key) {
                params.push(KeyValue::new(key, ""));
            }
        }
    }

}
