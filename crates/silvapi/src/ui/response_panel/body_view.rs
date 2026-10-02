#![allow(unused_imports)]
use std::collections::HashSet;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::PopupMenuItem,
    resizable::{ResizableState, h_resizable, resizable_panel, v_resizable},
    scroll::ScrollableElement,
    spinner::Spinner,
    tab::{Tab, TabBar},
    text::TextView,
    v_flex,
};

use crate::state::{AppEvent, AppState};

use super::*;

pub(super) const INITIAL_BUFFER_CHUNK_BYTES: usize = 200 * 1024;

impl ResponsePanel {
    /// Set the response editor's text and force the wrap layout to recompute in
    /// the same frame. `set_value` alone only refreshes `wrap_width` on the next
    /// paint, so wrapped text otherwise appears stale until an interaction (e.g.
    /// moving the cursor) forces a relayout. The response editor keeps its
    /// measured width across updates, so forcing wrap here is safe.
    pub(super) fn set_response_text(
        &self,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if text.len() <= INITIAL_BUFFER_CHUNK_BYTES {
            self.response_editor.update(cx, |state, cx| {
                if state.value() != text {
                    state.set_value(text, window, cx);
                    state.set_highlighter("json", cx);
                }
            });
            return;
        }

        let chunk_end = INITIAL_BUFFER_CHUNK_BYTES.min(text.len());
        let initial_chunk = text[..chunk_end].to_string();
        self.response_editor.update(cx, |state, cx| {
            state.set_value(initial_chunk, window, cx);
            state.set_highlighter("json", cx);
        });

        let editor = self.response_editor.clone();
        let full_text = text;
        cx.spawn_in(window, async move |_, window| {
            let _ = window.update(|window, cx| {
                editor.update(cx, |state, cx| {
                    if state.value() != full_text {
                        state.set_value(full_text, window, cx);
                        state.set_highlighter("json", cx);
                    }
                });
            });
        })
        .detach();
    }

    pub(super) fn ensure_response_editor_mode(
        &mut self,
        plain: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.response_editor_plain == plain {
            return;
        }

        self.response_editor_plain = plain;
        self.response_editor.update(cx, |state, cx| {
            state.set_folding(!plain, window, cx);
        });
    }

    pub(super) fn render_loading_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let elapsed_ms = self.app_state.read(cx).loading_elapsed_ms();
        let received = self
            .app_state
            .read(cx)
            .response
            .as_ref()
            .map(|response| response.formatted_size())
            .unwrap_or_else(|| "0 B".to_string());

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .text_color(cx.theme().muted_foreground)
            .child(Spinner::new().color(cx.theme().primary))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .text_sm()
                    .child("Loading response")
                    .when_some(elapsed_ms, |el, elapsed_ms| {
                        el.child(dot_separator(cx))
                            .child(format_duration(elapsed_ms))
                    })
                    .child(dot_separator(cx))
                    .child(received),
            )
    }

    pub(super) fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (
            is_loading,
            loading_elapsed_ms,
            loading_size,
            _has_stream_body,
            response_summary,
            has_response,
            has_error,
        ) = {
            let state = self.app_state.read(cx);
            let response = state.response.as_ref();
            (
                state.is_loading,
                state.loading_elapsed_ms(),
                response
                    .map(|resp| resp.formatted_size())
                    .unwrap_or_else(|| "0 B".to_string()),
                response.map_or(false, |resp| !resp.body.is_empty()),
                (!state.is_loading).then(|| response).flatten().map(|resp| {
                    (
                        resp.status,
                        resp.status_text.clone(),
                        resp.time_ms,
                        resp.formatted_size(),
                        resp.is_success(),
                        resp.is_redirect(),
                    )
                }),
                response.is_some(),
                state.error.is_some(),
            )
        };
        let has_nothing = !is_loading && !has_response && !has_error;

        h_flex()
            .px_3()
            .py_2()
            .items_center()
            .min_h(px(40.))
            .border_b_1()
            .border_color(cx.theme().border)
            .justify_between()
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .when(is_loading, |el| {
                        el.child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(div().text_sm().text_color(cx.theme().muted_foreground).child(
                                    "Loading response...",
                                ))
                                .when_some(loading_elapsed_ms, |el, elapsed_ms| {
                                    el.child(dot_separator(cx)).child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().foreground)
                                            .child(format_duration(elapsed_ms)),
                                    )
                                })
                                .child(dot_separator(cx))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().foreground)
                                        .child(loading_size),
                                )
                                .child(
                                    Button::new("cancel-resp")
                                        .label("Cancel")
                                        .danger()
                                        .xsmall()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.app_state.update(cx, |state, cx| {
                                                state.cancel_active_request();
                                                cx.emit(AppEvent::LoadingChanged);
                                            });
                                        })),
                                ),
                        )
                    })
                    .when_some(response_summary, |el, resp| {
                        let (status, status_text, time_ms, size, is_success, is_redirect) = resp;
                        let status_color: Hsla = if is_success {
                            cx.theme().success
                        } else if is_redirect {
                            cx.theme().warning
                        } else {
                            cx.theme().danger
                        };

                        el.child(
                            h_flex()
                                .gap_3()
                                .items_center()
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .font_weight(FontWeight::BOLD)
                                                .text_sm()
                                                .text_color(status_color)
                                                .child(status.to_string()),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(single_line(status_text)),
                                        ),
                                )
                                .child(dot_separator(cx))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().foreground)
                                        .child(format_duration(time_ms)),
                                )
                                .child(dot_separator(cx))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().foreground)
                                        .child(size),
                                )
                        )
                    })
                    .when(has_nothing, |el| {
                        el.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .whitespace_normal()
                                .text_center()
                                .child("Send a request to see the response"),
                        )
                    })
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .when(has_response, |el| {
                        el.child({
                            let app_state_clone = self.app_state.clone();
                            DropdownButton::new("history-dropdown")
                                .button(Button::new("history-btn").label("History").ghost())
                                .dropdown_menu(move |mut menu, _, cx| {
                                    let state = app_state_clone.read(cx);
                                    let id = if let Some(id) = &state.active_request_id {
                                        id.clone()
                                    } else {
                                        return menu;
                                    };
                                    let history_len = state
                                        .workspace
                                        .response_cache
                                        .get(&id)
                                        .map(|history| history.len())
                                        .unwrap_or(0);
                                    let app_state_clone2 = app_state_clone.clone();

                                    menu = menu.item(
                                        PopupMenuItem::new(format!("Delete {} Responses", history_len))
                                            .on_click(move |_, _, cx| {
                                                let mut request_id = None;
                                                app_state_clone2.update(cx, |s, cx| {
                                                    if let Some(id) = &s.active_request_id {
                                                        request_id = Some(id.clone());
                                                        s.workspace.response_cache.remove(id);
                                                        s.response = None;
                                                    }
                                                    cx.emit(AppEvent::ResponseReceived);
                                                });
                                                if let Some(id) = request_id {
                                                    cx.background_executor()
                                                        .spawn(async move {
                                                            let _ = crate::storage::delete_response_history(&id);
                                                        })
                                                        .detach();
                                                }
                                            }),
                                    );

                                    menu = menu.separator();

                                    let app_state_clone3 = app_state_clone.clone();
                                    if let Some(history) = state.workspace.response_cache.get(&id) {
                                        for (idx, resp) in history.iter().enumerate().rev() {
                                            let app_state_clone4 = app_state_clone3.clone();
                                            let status = if resp.is_failure() {
                                                "ERR".to_string()
                                            } else {
                                                resp.status.to_string()
                                            };
                                            let mut label = format!(
                                                "{}  {}  {}  {}",
                                                resp.started_at.as_deref().unwrap_or("—"),
                                                resp.request
                                                    .as_ref()
                                                    .map(|req| req.method.as_str())
                                                    .unwrap_or(""),
                                                status,
                                                format_duration(resp.time_ms),
                                            );
                                            if !resp.is_failure() {
                                                label.push_str(&format!("  {}", resp.formatted_size()));
                                            }
                                            menu = menu.item(PopupMenuItem::new(label).on_click(
                                                move |_, _, cx| {
                                                    app_state_clone4.update(cx, |s, cx| {
                                                        if let Some(id) = &s.active_request_id {
                                                            if let Some(history) =
                                                                s.workspace.response_cache.get(id)
                                                            {
                                                                if let Some(r) = history.get(idx) {
                                                                    // Clear the live error so the
                                                                    // selected entry's own outcome
                                                                    // is what gets rendered.
                                                                    s.error = None;
                                                                    s.response = Some(r.clone());
                                                                    cx.emit(AppEvent::ResponseReceived);
                                                                }
                                                            }
                                                        }
                                                    });
                                                },
                                            ));
                                        }
                                    }
                                    menu
                                })
                        })
                        .child(
                            Button::new("response-download")
                                .icon(IconName::File)
                                .tooltip("Download response body")
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if let Some(response) = this.app_state.read(cx).response.clone()
                                    {
                                        this.download_response_body(response, window, cx);
                                    }
                                })),
                        )
                        .child(
                            Button::new("response-wrap-toggle")
                                .icon(if self.soft_wrap {
                                    IconName::ArrowLeft
                                } else {
                                    IconName::ChevronsUpDown
                                })
                                .tooltip(if self.soft_wrap {
                                    "Disable wrap"
                                } else {
                                    "Enable wrap"
                                })
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.soft_wrap = !this.soft_wrap;
                                    let wrap = this.soft_wrap;
                                    this.response_editor.update(cx, |editor, cx| {
                                        editor.set_soft_wrap(wrap, window, cx);
                                    });
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        Button::new("response-layout-toggle")
                            .icon(IconName::PanelRightOpen)
                            .tooltip("Toggle response layout")
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.app_state.update(cx, |_, cx| {
                                    cx.emit(AppEvent::ToggleLayout);
                                });
                            })),
                    )
            )
    }

    pub(super) fn download_response_body(
        &self,
        response: silvapi_core::models::HttpResponse,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let filename = response_download_filename(&response);
        let bytes = response.raw_body_bytes();
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Select download folder".into()),
        });

        cx.spawn_in(window, async move |_, window| {
            let dir = rx.await.ok()?.ok()??.iter().next()?.clone();
            let path = dir.join(filename);
            let result = std::fs::write(&path, bytes);
            window
                .update(|window, cx| match result {
                    Ok(_) => {
                        window.push_notification(format!("Saved response: {}", path.display()), cx)
                    }
                    Err(err) => {
                        window.push_notification(format!("Failed to save response: {}", err), cx)
                    }
                })
                .ok();
            Some(())
        })
        .detach();
    }
}

fn response_download_filename(response: &silvapi_core::models::HttpResponse) -> String {
    if let Some(filename) = response
        .headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("content-disposition"))
        .and_then(|(_, value)| content_disposition_filename(value))
    {
        return sanitize_filename(&filename);
    }

    if let Some(name) = response
        .request
        .as_ref()
        .and_then(|request| filename_from_url(&request.resolved_url))
        .filter(|name| !name.is_empty())
    {
        return sanitize_filename(&name);
    }

    let extension = response_content_extension(response);
    format!("response.{extension}")
}

fn content_disposition_filename(value: &str) -> Option<String> {
    value.split(';').find_map(|part| {
        let part = part.trim();
        let raw = part
            .strip_prefix("filename*=")
            .or_else(|| part.strip_prefix("filename="))?;
        let raw = raw.trim_matches('"');
        Some(raw.rsplit("''").next().unwrap_or(raw).to_string())
    })
}

fn filename_from_url(url: &str) -> Option<String> {
    let without_query = url.split(['?', '#']).next().unwrap_or(url);
    without_query
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
}

fn response_content_extension(response: &silvapi_core::models::HttpResponse) -> &'static str {
    let content_type = response
        .headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.to_ascii_lowercase())
        .unwrap_or_default();

    if content_type.contains("application/json") {
        "json"
    } else if content_type.contains("application/pdf") {
        "pdf"
    } else if content_type.contains("image/png") {
        "png"
    } else if content_type.contains("image/jpeg") || content_type.contains("image/jpg") {
        "jpg"
    } else if content_type.contains("image/gif") {
        "gif"
    } else if content_type.contains("image/webp") {
        "webp"
    } else if content_type.contains("video/mp4") {
        "mp4"
    } else if content_type.contains("video/webm") {
        "webm"
    } else if content_type.contains("audio/mpeg") {
        "mp3"
    } else if content_type.starts_with("text/") {
        "txt"
    } else {
        "bin"
    }
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            _ => ch,
        })
        .collect();
    let cleaned = cleaned.trim_matches(['.', ' ']).trim();
    if cleaned.is_empty() {
        "response.bin".to_string()
    } else {
        cleaned.to_string()
    }
}
