use std::collections::HashSet;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _,
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


const STREAM_PREVIEW_BYTES: usize = 96 * 1024;
const LARGE_RESPONSE_PREVIEW_BYTES: usize = 128 * 1024;
const FORMAT_RESPONSE_LIMIT_BYTES: usize = 1024 * 1024;
const SSE_ROW_PREVIEW_BYTES: usize = 512;
const SSE_MAX_EVENTS: usize = 2_000;
const SSE_EVENT_STORE_BYTES: usize = 256 * 1024;
const SSE_DETAIL_FORMAT_LIMIT_BYTES: usize = 256 * 1024;

fn single_line(value: impl AsRef<str>) -> String {
    value.as_ref().replace(['\r', '\n'], " ")
}

struct HeaderSection {
    title: &'static str,
    rows: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseTab {
    Response,
    Preview,
    Headers,
    Timeline,
}

impl ResponseTab {
    fn index(&self) -> usize {
        match self {
            ResponseTab::Response => 0,
            ResponseTab::Preview => 1,
            ResponseTab::Headers => 2,
            ResponseTab::Timeline => 3,
        }
    }

    fn from_index(i: usize) -> Self {
        match i {
            0 => ResponseTab::Response,
            1 => ResponseTab::Preview,
            2 => ResponseTab::Headers,
            _ => ResponseTab::Timeline,
        }
    }
}

pub struct ResponsePanel {
    app_state: Entity<AppState>,
    active_tab: ResponseTab,
    response_editor: Entity<InputState>,
    sse_detail_editor: Entity<InputState>,
    headers_editor: Entity<InputState>,
    response_text: String,
    soft_wrap: bool,
    response_editor_plain: bool,
    sse_detail_soft_wrap: bool,
    sse_detail_column_layout: bool,
    headers_source_view: bool,
    collapsed_header_sections: HashSet<&'static str>,
    _subs: Vec<Subscription>,
    selected_timeline_event: Option<usize>,
    selected_sse_event: Option<usize>,
    focus_handle: FocusHandle,
    sse_scroll: gpui::UniformListScrollHandle,
    timeline_scroll: gpui::UniformListScrollHandle,
    sse_detail_column_resize: Entity<ResizableState>,
    sse_detail_row_resize: Entity<ResizableState>,
}

impl Focusable for ResponsePanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ResponsePanel {
    pub fn new(app_state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let response_editor = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor("json")
                .searchable(true)
                .replaceable(false)
                .placeholder("Response will appear here... (Ctrl+F to search)")
                .soft_wrap(true)
        });
        let sse_detail_editor = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .searchable(true)
                .replaceable(false)
                .placeholder("Message detail")
                .soft_wrap(true)
        });
        let headers_editor = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor("http")
                .searchable(true)
                .replaceable(false)
                .placeholder("Headers")
                .soft_wrap(false)
        });

        let focus_handle = cx.focus_handle();

        let mut this = Self {
            app_state: app_state.clone(),
            active_tab: ResponseTab::Response,
            response_editor: response_editor.clone(),
            sse_detail_editor,
            headers_editor,
            response_text: String::new(),
            soft_wrap: true,
            response_editor_plain: false,
            sse_detail_soft_wrap: true,
            sse_detail_column_layout: false,
            headers_source_view: false,
            collapsed_header_sections: HashSet::new(),
            _subs: Vec::new(),
            selected_timeline_event: None,
            selected_sse_event: None,
            focus_handle,
            sse_scroll: gpui::UniformListScrollHandle::new(),
            timeline_scroll: gpui::UniformListScrollHandle::new(),
            sse_detail_column_resize: cx.new(|_| ResizableState::default()),
            sse_detail_row_resize: cx.new(|_| ResizableState::default()),
        };

        let app_sub = cx.subscribe_in(
            &app_state,
            window,
            move |this: &mut ResponsePanel, app_state, ev: &AppEvent, window, cx| {
                if matches!(ev, AppEvent::LoadingChanged) {
                    cx.notify();
                }

                if matches!(ev, AppEvent::ResponseReceived | AppEvent::RequestSelected) {
                    let (is_loading, body_len, preview_text) = {
                        let state = app_state.read(cx);
                        let body = state
                            .response
                            .as_ref()
                            .map(|r| r.body.as_str())
                            .unwrap_or("");
                        (
                            state.is_loading,
                            body.len(),
                            response_preview_text(body, state.is_loading),
                        )
                    };

                    if !is_loading {
                        if body_len <= FORMAT_RESPONSE_LIMIT_BYTES {
                            this.ensure_response_editor_mode(false, window, cx);
                            let text = app_state
                                .read(cx)
                                .response
                                .as_ref()
                                .map(|r| r.body.clone())
                                .unwrap_or_default();
                            let app_state_clone = app_state.clone();
                            cx.spawn(async move |_, cx| {
                                let formatted = cx
                                    .background_executor()
                                    .spawn(async move { try_format_json(&text) })
                                    .await;
                                let _ = cx.update(|cx| {
                                    app_state_clone.update(cx, |state, cx| {
                                        state.formatted_response = Some(formatted);
                                        cx.emit(AppEvent::ResponseFormatted);
                                    });
                                });
                            })
                            .detach();
                        } else {
                            this.ensure_response_editor_mode(true, window, cx);
                            this.response_text = preview_text.clone();
                            this.response_editor.update(cx, |state, cx| {
                                state.set_value(preview_text, window, cx);
                            });
                            cx.notify();
                        }
                    } else {
                        this.ensure_response_editor_mode(true, window, cx);
                        this.response_text = preview_text.clone();
                        this.response_editor.update(cx, |state, cx| {
                            state.set_value(preview_text, window, cx);
                        });
                        cx.notify();
                    }
                }

                if matches!(ev, AppEvent::ResponseFormatted) {
                    if let Some(formatted) = app_state.read(cx).formatted_response.clone() {
                        this.ensure_response_editor_mode(false, window, cx);
                        this.response_text = formatted.clone();
                        this.response_editor.update(cx, |state, cx| {
                            state.set_value(formatted, window, cx);
                        });
                        cx.notify();
                    }
                }

            },
        );
        let response_editor_sub = cx.subscribe_in(
            &response_editor,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Change) {
                    let editor = this.response_editor.clone();
                    let text = this.response_text.clone();
                    cx.defer_in(window, move |_, window, cx| {
                        editor.update(cx, |state, cx| state.set_value(text, window, cx));
                    });
                }
            },
        );
        let headers_editor = this.headers_editor.clone();
        let headers_editor_sub = cx.subscribe_in(
            &headers_editor,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Change) {
                    let editor = this.headers_editor.clone();
                    let source_text = this.current_headers_source_text(cx);
                    cx.defer_in(window, move |_, window, cx| {
                        editor.update(cx, |state, cx| state.set_value(source_text, window, cx));
                    });
                }
            },
        );
        let sse_detail_editor = this.sse_detail_editor.clone();
        let sse_detail_editor_sub = cx.subscribe_in(
            &sse_detail_editor,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Change) {
                    let editor = this.sse_detail_editor.clone();
                    let detail_text = this.current_sse_detail_text().unwrap_or_default();
                    cx.defer_in(window, move |_, window, cx| {
                        editor.update(cx, |state, cx| state.set_value(detail_text, window, cx));
                    });
                }
            },
        );
        this._subs = vec![
            app_sub,
            response_editor_sub,
            headers_editor_sub,
            sse_detail_editor_sub,
        ];
        this
    }

    fn current_headers_source_text(&self, cx: &mut Context<Self>) -> String {
        let state = self.app_state.read(cx);
        let resolved_url = state
            .active_request
            .as_ref()
            .map(|req| state.interpolate_variables(&req.url));

        format_headers_dump(
            state.active_request.as_ref(),
            state.response.as_ref(),
            resolved_url.as_deref(),
        )
    }

    fn current_sse_detail_text(&self) -> Option<String> {
        let selected_idx = self.selected_sse_event?;
        let mut idx = 0;
        for block in self.response_text.split("\n\n") {
            if block.is_empty() {
                continue;
            }
            if idx == selected_idx {
                let mut data = String::new();
                for line in block.split('\n') {
                    if let Some(rest) = line.strip_prefix("data: ") {
                        if !data.is_empty() {
                            data.push('\n');
                        }
                        data.push_str(rest);
                    }
                }
                return Some(format_sse_detail_text(&data));
            }
            idx += 1;
        }
        None
    }

    fn ensure_response_editor_mode(
        &mut self,
        plain: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.response_editor_plain == plain {
            return;
        }

        self.response_editor_plain = plain;
        let soft_wrap = self.soft_wrap && !plain;
        self.response_editor.update(cx, |state, cx| {
            let mut next = InputState::new(window, cx)
                .multi_line(true)
                .searchable(true)
                .replaceable(false)
                .placeholder("Response will appear here... (Ctrl+F to search)")
                .soft_wrap(soft_wrap);
            if !plain {
                next = next.code_editor("json");
            }
            *state = next;
        });
    }

    fn has_room_for_sse_side_detail(window: &Window) -> bool {
        let logical_width = window.bounds().size.width / window.scale_factor();
        logical_width >= px(720.)
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (
            is_loading,
            loading_elapsed_ms,
            loading_size,
            has_stream_body,
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
                                    if has_stream_body {
                                        "Receiving response..."
                                    } else {
                                        "Sending request..."
                                    },
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
                            rgb(0x4CAF50).into()
                        } else if is_redirect {
                            rgb(0xFF9800).into()
                        } else {
                            rgb(0xF44336).into()
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
                                            let label = format!(
                                                "{} -> {}",
                                                resp.status,
                                                format_duration(resp.time_ms)
                                            );
                                            menu = menu.item(PopupMenuItem::new(label).on_click(
                                                move |_, _, cx| {
                                                    app_state_clone4.update(cx, |s, cx| {
                                                        if let Some(id) = &s.active_request_id {
                                                            if let Some(history) =
                                                                s.workspace.response_cache.get(id)
                                                            {
                                                                if let Some(r) = history.get(idx) {
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
                                    let soft_wrap = this.soft_wrap && !this.response_editor_plain;
                                    let plain = this.response_editor_plain;
                                    this.response_editor.update(cx, |state, _cx| {
                                        let mut next = InputState::new(window, _cx)
                                            .multi_line(true)
                                            .searchable(true)
                                            .replaceable(false)
                                            .placeholder("Response will appear here... (Ctrl+F to search)")
                                            .soft_wrap(soft_wrap);
                                        if !plain {
                                            next = next.code_editor("json");
                                        }
                                        *state = next;
                                    });
                                    let text = this.response_text.clone();
                                    this.response_editor.update(cx, |state, cx| {
                                        state.set_value(text, window, cx);
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

    fn render_headers_tab(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (sections, source_text) = {
            let state = self.app_state.read(cx);
            let resolved_url = state
                .active_request
                .as_ref()
                .map(|req| state.interpolate_variables(&req.url));
            let sections = build_header_sections(
                state.active_request.as_ref(),
                state.response.as_ref(),
                resolved_url.as_deref(),
            );
            let source_text = if self.headers_source_view {
                format_headers_dump(
                    state.active_request.as_ref(),
                    state.response.as_ref(),
                    resolved_url.as_deref(),
                )
            } else {
                String::new()
            };
            (sections, source_text)
        };

        if self.headers_source_view {
            self.headers_editor.update(cx, |state, cx| {
                if state.value() != source_text {
                    state.set_value(source_text, window, cx);
                }
            });
        }

        let table = v_flex()
            .size_full()
            .overflow_y_scrollbar()
            .px_3()
            .py_2()
            .gap_3()
            .children(sections.into_iter().map(|section| {
                self.render_header_section(section.title, section.rows, cx)
                    .into_any_element()
            }));

        v_flex()
            .id("resp-headers-tab")
            .size_full()
            .child(
                h_flex()
                    .justify_end()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("headers-source-toggle")
                            .label(if self.headers_source_view {
                                "view parsed"
                            } else {
                                "view source"
                            })
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.headers_source_view = !this.headers_source_view;
                                cx.notify();
                            })),
                    ),
            )
            .child(if self.headers_source_view {
                div()
                    .id("resp-headers-editor")
                    .flex_1()
                    .p_1()
                    .child(Input::new(&self.headers_editor).h_full().w_full())
                    .into_any_element()
            } else {
                table.into_any_element()
            })
    }

    fn render_preview_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body = self
            .app_state
            .read(cx)
            .response
            .as_ref()
            .map(|r| r.body.clone())
            .unwrap_or_default();
        // TextView::html is a lightweight rich-text renderer, not a browser: it
        // has no CSS/JS engine, so strip script/style/head/comments (otherwise
        // their raw source renders as text) before handing it over. Also collapse
        // CR/LF to spaces — gpui's text shaper panics on a newline within a run.
        let body = sanitize_html_for_preview(&body);
        div()
            .id("preview-scroll")
            .size_full()
            .overflow_scroll()
            .p_4()
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .overflow_hidden()
                    .child(TextView::html("preview-html-view", body)),
            )
    }

    fn render_loading_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child("Receiving response")
                    .when_some(elapsed_ms, |el, elapsed_ms| {
                        el.child(dot_separator(cx))
                            .child(format_duration(elapsed_ms))
                    })
                    .child(dot_separator(cx))
                    .child(received),
            )
    }

    fn render_header_section(
        &self,
        title: &'static str,
        rows: Vec<(String, String)>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let collapsed = self.collapsed_header_sections.contains(title);

        v_flex()
            .w_full()
            .border_b_1()
            .border_color(cx.theme().border)
            .pb_2()
            .child(
                h_flex()
                    .h(px(26.))
                    .items_center()
                    .gap_1()
                    .cursor_pointer()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().foreground)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            if !this.collapsed_header_sections.remove(title) {
                                this.collapsed_header_sections.insert(title);
                            }
                            cx.notify();
                        }),
                    )
                    .child(
                        Icon::new(if collapsed {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        })
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                    )
                    .child(title),
            )
            .when(!collapsed, |el| {
                el.children(rows.into_iter().enumerate().map(|(idx, (key, value))| {
                    h_flex()
                        .w_full()
                        .items_start()
                        .gap_4()
                        .py_1()
                        .px_2()
                        .border_b_1()
                        .border_color(cx.theme().border.opacity(0.28))
                        .when(idx % 2 == 1, |row| row.bg(cx.theme().muted.opacity(0.12)))
                        .child(
                            div()
                                .w(px(190.))
                                .flex_shrink_0()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(single_line(key)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .whitespace_normal()
                                .child(single_line(value)),
                        )
                }))
            })
    }

    fn render_timeline_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let timeline = self
            .app_state
            .read(cx)
            .response
            .as_ref()
            .map(|r| r.timeline.clone())
            .unwrap_or_default();

        let selected_idx = self.selected_timeline_event;
        let events_len = timeline.len();

        let list = uniform_list(
            "timeline-scroll",
            events_len,
            cx.processor(move |_this: &mut Self, range, _window, cx| {
                let mut children = Vec::new();
                for idx in range {
                    if let Some(ev) = timeline.get(idx) {
                        let selected = selected_idx == Some(idx);

                        let (icon, color) = match ev.icon {
                            crate::models::TimelineIcon::Setting => {
                                (IconName::Settings, cx.theme().muted_foreground)
                            }
                            crate::models::TimelineIcon::Request => {
                                (IconName::ArrowUp, cx.theme().primary)
                            }
                            crate::models::TimelineIcon::Response => {
                                (IconName::ArrowDown, cx.theme().primary)
                            }
                            crate::models::TimelineIcon::Info => {
                                (IconName::Info, cx.theme().muted_foreground)
                            }
                        };

                        let bg_color = if selected {
                            cx.theme().primary.opacity(0.1)
                        } else {
                            gpui::transparent_black()
                        };
                        let border = if selected {
                            gpui::transparent_black()
                        } else {
                            cx.theme().border.opacity(0.5)
                        };

                        children.push(
                            h_flex()
                                .w_full()
                                .justify_between()
                                .gap_3()
                                .py_1()
                                .px_2()
                                .rounded_md()
                                .bg(bg_color)
                                .border_b_1()
                                .border_color(border)
                                .cursor_pointer()
                                .id(idx)
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        if this.selected_timeline_event == Some(idx) {
                                            this.selected_timeline_event = None;
                                        } else {
                                            this.selected_timeline_event = Some(idx);
                                        }
                                        cx.notify();
                                    }),
                                )
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .items_start()
                                        .child(Icon::new(icon).small().text_color(color))
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_family("monospace")
                                                .text_color(cx.theme().foreground)
                                                .child(single_line(&ev.name)),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_family("monospace")
                                        .text_color(cx.theme().muted_foreground)
                                        .child(ev.timestamp.clone()),
                                )
                                .into_any_element(),
                        );
                    }
                }
                children
            }),
        )
        .track_scroll(&self.timeline_scroll)
        .size_full();

        let mut container = v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if events_len == 0 {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "up" => {
                        let mut curr = this.selected_timeline_event.unwrap_or(events_len - 1);
                        if curr > 0 {
                            curr -= 1;
                        } else {
                            curr = events_len - 1;
                        }
                        this.selected_timeline_event = Some(curr);
                        this.timeline_scroll
                            .scroll_to_item(curr, ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    "down" => {
                        let mut curr = this.selected_timeline_event.unwrap_or(0);
                        curr = (curr + 1) % events_len;
                        this.selected_timeline_event = Some(curr);
                        this.timeline_scroll
                            .scroll_to_item(curr, ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(list);

        if let Some(idx) = selected_idx {
            if let Some(ev) = self
                .app_state
                .read(cx)
                .response
                .as_ref()
                .map(|r| r.timeline.clone())
                .unwrap_or_default()
                .get(idx)
            {
                if let Some(detail) = &ev.detail {
                    let pretty_detail = if let Ok(parsed) =
                        serde_json::from_str::<serde_json::Value>(detail)
                    {
                        serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| detail.to_string())
                    } else {
                        detail.to_string()
                    };
                    let md_content = if serde_json::from_str::<serde_json::Value>(detail).is_ok() {
                        format!("```json\n{}\n```", pretty_detail)
                    } else {
                        format!("```text\n{}\n```", pretty_detail)
                    };

                    let detail_view = v_flex()
                        .h(px(250.))
                        .w_full()
                        .bg(cx.theme().background)
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .p_3()
                        .child(
                            h_flex()
                                .justify_between()
                                .w_full()
                                .items_center()
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_lg()
                                        .child(single_line(&ev.name)),
                                )
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .items_center()
                                        .child(
                                            div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(ev.timestamp.clone()),
                                        )
                                        .child(
                                            Button::new("close-detail")
                                                .icon(IconName::Close)
                                                .tooltip("Close detail")
                                                .ghost()
                                                .xsmall()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.selected_timeline_event = None;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .mt_3()
                                .size_full()
                                .id("timeline-detail-scroll")
                                .overflow_y_scroll()
                                .child(
                                    TextView::markdown("timeline-md-view", md_content)
                                        .selectable(true),
                                ),
                        );

                    container = container.child(detail_view);
                }
            }
        }

        container
    }
    fn render_sse_tab(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text = self.response_text.clone();
        let mut events = Vec::new();
        for block in text.split("\n\n") {
            if events.len() >= SSE_MAX_EVENTS {
                break;
            }
            if block.is_empty() {
                continue;
            }
            let mut event_type = "message";
            let mut data = String::new();
            for line in block.split('\n') {
                if let Some(rest) = line.strip_prefix("event: ") {
                    event_type = rest;
                } else if let Some(rest) = line.strip_prefix("data: ") {
                    if !data.is_empty() {
                        append_sse_detail_text(&mut data, "\n");
                    }
                    append_sse_detail_text(&mut data, rest);
                }
            }
            if !data.is_empty() {
                events.push((event_type.to_string(), data));
            }
        }

        let selected_idx = self.selected_sse_event;
        let events_len = events.len();
        let selected_event_data =
            selected_idx.and_then(|idx| events.get(idx).map(|(_, data)| data.clone()));

        let list = uniform_list(
            "sse-scroll",
            events_len,
            cx.processor(move |_this: &mut Self, range, _window, cx| {
                let mut children = Vec::new();
                for idx in range {
                    if let Some((etype, data)) = events.get(idx) {
                        let selected = selected_idx == Some(idx);
                        let bg_color = if selected {
                            cx.theme().primary.opacity(0.1)
                        } else {
                            gpui::transparent_black()
                        };
                        let border = if selected {
                            gpui::transparent_black()
                        } else {
                            cx.theme().border.opacity(0.5)
                        };
                        let etype = etype.clone();
                        let data_preview = single_line_preview(data, SSE_ROW_PREVIEW_BYTES);

                        children.push(
                            h_flex()
                                .w_full()
                                .gap_3()
                                .py_1()
                                .px_2()
                                .rounded_md()
                                .bg(bg_color)
                                .border_b_1()
                                .border_color(border)
                                .cursor_pointer()
                                .id(idx)
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        if this.selected_sse_event == Some(idx) {
                                            this.selected_sse_event = None;
                                        } else {
                                            this.selected_sse_event = Some(idx);
                                        }
                                        cx.notify();
                                    }),
                                )
                                .child(
                                    div()
                                        .bg(cx.theme().muted)
                                        .px_2()
                                        .py_0p5()
                                        .rounded_md()
                                        .text_sm()
                                        .font_family("monospace")
                                        .child(idx.to_string()),
                                )
                                .child(
                                    div()
                                        .bg(cx.theme().muted)
                                        .px_2()
                                        .py_0p5()
                                        .rounded_md()
                                        .text_sm()
                                        .font_family("monospace")
                                        .child(single_line(etype)),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_family("monospace")
                                        .text_color(cx.theme().foreground)
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .child(single_line(data_preview)),
                                )
                                .into_any_element(),
                        );
                    }
                }
                children
            }),
        )
        .track_scroll(&self.sse_scroll)
        .size_full();

        let container = v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if events_len == 0 {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "up" => {
                        let mut curr = this.selected_sse_event.unwrap_or(events_len - 1);
                        if curr > 0 {
                            curr -= 1;
                        } else {
                            curr = events_len - 1;
                        }
                        this.selected_sse_event = Some(curr);
                        this.sse_scroll
                            .scroll_to_item(curr, ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    "down" => {
                        let mut curr = this.selected_sse_event.unwrap_or(0);
                        curr = (curr + 1) % events_len;
                        this.selected_sse_event = Some(curr);
                        this.sse_scroll
                            .scroll_to_item(curr, ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(list);

        if let Some(data) = selected_event_data {
            let allow_column_detail =
                self.sse_detail_column_layout && Self::has_room_for_sse_side_detail(window);
            let detail_text = format_sse_detail_text(&data);
            self.sse_detail_editor.update(cx, |state, cx| {
                if state.value() != detail_text {
                    *state = InputState::new(window, cx)
                        .multi_line(true)
                        .searchable(true)
                        .replaceable(false)
                        .placeholder("Message detail")
                        .soft_wrap(self.sse_detail_soft_wrap)
                        .code_editor("json");
                    state.set_value(detail_text, window, cx);
                }
            });
            let detail_view = v_flex()
                .size_full()
                .bg(cx.theme().background)
                .child(
                    h_flex()
                        .h(px(44.))
                        .px_3()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .justify_between()
                        .w_full()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_lg()
                                .child("Message Received"),
                        )
                        .child(
                            h_flex()
                                .flex_none()
                                .gap_2()
                                .items_center()
                                .child(
                                    Button::new("sse-detail-layout-toggle")
                                        .icon(if allow_column_detail {
                                            IconName::PanelBottomOpen
                                        } else {
                                            IconName::PanelRightOpen
                                        })
                                        .tooltip(if allow_column_detail {
                                            "Stack message below"
                                        } else {
                                            "Show message beside list"
                                        })
                                        .ghost()
                                        .xsmall()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            let wide_enough =
                                                Self::has_room_for_sse_side_detail(window);
                                            this.sse_detail_column_layout =
                                                wide_enough && !this.sse_detail_column_layout;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("sse-detail-wrap-toggle")
                                        .icon(if self.sse_detail_soft_wrap {
                                            IconName::ArrowLeft
                                        } else {
                                            IconName::ChevronsUpDown
                                        })
                                        .tooltip(if self.sse_detail_soft_wrap {
                                            "Disable wrap"
                                        } else {
                                            "Enable wrap"
                                        })
                                        .ghost()
                                        .xsmall()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.sse_detail_soft_wrap = !this.sse_detail_soft_wrap;
                                            let soft_wrap = this.sse_detail_soft_wrap;
                                            let text =
                                                this.sse_detail_editor.read(cx).value().to_string();
                                            this.sse_detail_editor.update(cx, |state, cx| {
                                                *state = InputState::new(window, cx)
                                                    .multi_line(true)
                                                    .searchable(true)
                                                    .replaceable(false)
                                                    .placeholder("Message detail")
                                                    .soft_wrap(soft_wrap)
                                                    .code_editor("json");
                                                state.set_value(text, window, cx);
                                            });
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("close-sse-detail")
                                        .icon(IconName::Close)
                                        .tooltip("Close message")
                                        .ghost()
                                        .xsmall()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.selected_sse_event = None;
                                            cx.notify();
                                        })),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .p_3()
                        .id("sse-detail-scroll")
                        .child(Input::new(&self.sse_detail_editor).h_full().w_full()),
                );

            if allow_column_detail {
                return h_flex()
                    .size_full()
                    .track_focus(&self.focus_handle)
                    .child(
                        h_resizable("sse-detail-column-split")
                            .with_state(&self.sse_detail_column_resize)
                            .child(
                                resizable_panel()
                                    .size(px(440.))
                                    .size_range(px(220.)..px(1400.))
                                    .child(container),
                            )
                            .child(
                                resizable_panel()
                                    .size(px(420.))
                                    .size_range(px(280.)..px(900.))
                                    .child(detail_view),
                            ),
                    )
                    .into_any_element();
            } else {
                return div()
                    .size_full()
                    .track_focus(&self.focus_handle)
                    .child(
                        v_resizable("sse-detail-row-split")
                            .with_state(&self.sse_detail_row_resize)
                            .child(
                                resizable_panel()
                                    .size(px(440.))
                                    .size_range(px(160.)..px(1200.))
                                    .child(container),
                            )
                            .child(
                                resizable_panel()
                                    .size(px(320.))
                                    .size_range(px(180.)..px(800.))
                                    .child(detail_view),
                            ),
                    )
                    .into_any_element();
            }
        }

        container.into_any_element()
    }

    pub fn has_focus(&self, window: &Window, cx: &App) -> bool {
        if self.focus_handle.contains_focused(window, cx) {
            return true;
        }
        if self
            .response_editor
            .read(cx)
            .focus_handle(cx)
            .contains_focused(window, cx)
        {
            return true;
        }
        false
    }
}

impl Render for ResponsePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_html = self
            .app_state
            .read(cx)
            .response
            .as_ref()
            .map_or(false, |r| {
                r.headers
                    .iter()
                    .any(|(k, v)| k.to_lowercase() == "content-type" && v.contains("text/html"))
            });

        let mut visible_tabs = vec![ResponseTab::Response];
        if is_html {
            visible_tabs.push(ResponseTab::Preview);
        }
        visible_tabs.push(ResponseTab::Headers);
        visible_tabs.push(ResponseTab::Timeline);

        if self.active_tab == ResponseTab::Preview && !is_html {
            self.active_tab = ResponseTab::Response;
        }
        let active_tab = self.active_tab;
        let selected_index = visible_tabs
            .iter()
            .position(|&t| t == active_tab)
            .unwrap_or(0);

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .track_focus(&self.focus_handle)
            .border_l_1()
            .border_color(cx.theme().border)
            .child(self.render_status_bar(cx))
            .child({
                let mut tab_bar = TabBar::new("resp-tabs")
                    .selected_index(selected_index)
                    .on_click(cx.listener({
                        let visible_tabs = visible_tabs.clone();
                        move |this, ix, _, cx| {
                            if let Some(&tab) = visible_tabs.get(*ix) {
                                this.active_tab = tab;
                                cx.notify();
                            }
                        }
                    }));
                for t in &visible_tabs {
                    let label = match t {
                        ResponseTab::Response => "Response",
                        ResponseTab::Preview => "Preview",
                        ResponseTab::Headers => "Headers",
                        ResponseTab::Timeline => "Timeline",
                    };
                    tab_bar = tab_bar.child(Tab::new().label(label));
                }
                tab_bar
            })
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when(active_tab == ResponseTab::Response, |el| {
                        if let Some(err) = &self.app_state.read(cx).error {
                            el.child(
                                div()
                                    .size_full()
                                    .p_4()
                                    .id("error-scroll")
                                    .overflow_y_scrollbar()
                                    .child(
                                        TextView::markdown(
                                            "error-view",
                                            format!("### Network Error\n\n```text\n{}\n```", err),
                                        )
                                        .selectable(true),
                                    ),
                            )
                        } else if {
                            let state = self.app_state.read(cx);
                            state.is_loading
                                && state
                                    .response
                                    .as_ref()
                                    .map_or(true, |resp| resp.body.is_empty())
                        } {
                            el.child(self.render_loading_body(cx))
                        } else {
                            let is_sse =
                                self.app_state
                                    .read(cx)
                                    .response
                                    .as_ref()
                                    .map_or(false, |r| {
                                        r.headers.iter().any(|(k, v)| {
                                            k.to_lowercase() == "content-type"
                                                && v.contains("text/event-stream")
                                        })
                                    });

                            if is_sse {
                                el.child(self.render_sse_tab(_window, cx))
                            } else {
                                el.child(
                                    div()
                                        .flex_1()
                                        .size_full()
                                        .p_1()
                                        .child(Input::new(&self.response_editor).h_full().w_full()),
                                )
                            }
                        }
                    })
                    .when(active_tab == ResponseTab::Preview, |el| {
                        el.child(self.render_preview_tab(cx))
                    })
                    .when(active_tab == ResponseTab::Headers, |el| {
                        el.child(self.render_headers_tab(_window, cx))
                    })
                    .when(active_tab == ResponseTab::Timeline, |el| {
                        el.child(self.render_timeline_tab(cx))
                    }),
            )
    }
}

fn dot_separator(cx: &App) -> impl IntoElement {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child("•")
}

fn format_duration(ms: u64) -> String {
    if ms < 1_000 {
        format!("{} ms", ms)
    } else if ms < 60_000 {
        format!("{:.2} s", ms as f64 / 1_000.0)
    } else {
        let seconds = ms / 1_000;
        let minutes = seconds / 60;
        let rem_seconds = seconds % 60;
        format!("{}m {:02}s", minutes, rem_seconds)
    }
}

fn build_header_sections(
    req: Option<&crate::models::ApiRequest>,
    res: Option<&crate::models::HttpResponse>,
    resolved_url: Option<&str>,
) -> Vec<HeaderSection> {
    let mut sections = Vec::new();
    let mut general = Vec::new();

    if let Some(req) = req {
        general.push((
            "Request URL".to_string(),
            resolved_url
                .filter(|url| !url.is_empty())
                .unwrap_or(&req.url)
                .to_string(),
        ));
        if resolved_url.is_some_and(|url| url != req.url) {
            general.push(("Original URL".to_string(), req.url.clone()));
        }
        general.push((
            "Request Method".to_string(),
            req.method.as_str().to_string(),
        ));
    }

    if let Some(res) = res {
        let status = if res.status_text.is_empty() {
            res.status.to_string()
        } else {
            format!("{} {}", res.status, res.status_text)
        };
        general.push(("Status Code".to_string(), status));
        general.push(("Time".to_string(), format_duration(res.time_ms)));
        general.push(("Size".to_string(), res.formatted_size()));
    }
    general.push(("Version".to_string(), "HTTP/1.1".to_string()));
    sections.push(HeaderSection {
        title: "General",
        rows: general,
    });

    if let Some(res) = res
        && !res.headers.is_empty()
    {
        sections.push(HeaderSection {
            title: "Response Headers",
            rows: res.headers.clone(),
        });
    }

    if let Some(req) = req {
        let rows: Vec<(String, String)> = req
            .headers
            .iter()
            .filter(|header| header.enabled && !header.key.is_empty())
            .map(|header| (header.key.clone(), header.value.clone()))
            .collect();
        if !rows.is_empty() {
            sections.push(HeaderSection {
                title: "Request Headers",
                rows,
            });
        }
    }

    sections
}

fn format_headers_dump(
    req: Option<&crate::models::ApiRequest>,
    res: Option<&crate::models::HttpResponse>,
    resolved_url: Option<&str>,
) -> String {
    let mut out = String::new();

    out.push_str("General\n");
    if let Some(req) = req {
        push_header_line(
            &mut out,
            "Request URL",
            resolved_url
                .filter(|url| !url.is_empty())
                .unwrap_or(&req.url),
        );
        if resolved_url.is_some_and(|url| url != req.url) {
            push_header_line(&mut out, "Original URL", &req.url);
        }
        push_header_line(&mut out, "Request Method", req.method.as_str());
    }
    if let Some(res) = res {
        let status = if res.status_text.is_empty() {
            res.status.to_string()
        } else {
            format!("{} {}", res.status, res.status_text)
        };
        push_header_line(&mut out, "Status Code", &status);
        push_header_line(&mut out, "Time", &format_duration(res.time_ms));
        push_header_line(&mut out, "Size", &res.formatted_size());
    }
    push_header_line(&mut out, "Version", "HTTP/1.1");

    if let Some(req) = req
        && !req.headers.is_empty()
    {
        out.push_str("\nRequest Headers\n");
        for header in &req.headers {
            if header.enabled && !header.key.is_empty() {
                push_header_line(&mut out, &header.key, &header.value);
            }
        }
    }

    if let Some(res) = res
        && !res.headers.is_empty()
    {
        out.push_str("\nResponse Headers\n");
        for (key, value) in &res.headers {
            push_header_line(&mut out, key, value);
        }
    }

    out
}

fn push_header_line(out: &mut String, key: &str, value: &str) {
    use std::fmt::Write as _;
    let _ = writeln!(out, "{}: {}", key, value);
}

fn response_preview_text(body: &str, is_loading: bool) -> String {
    let limit = if is_loading {
        STREAM_PREVIEW_BYTES
    } else {
        LARGE_RESPONSE_PREVIEW_BYTES
    };

    if body.len() <= limit {
        return body.to_string();
    }

    if is_loading {
        let tail = tail_on_char_boundary(body, limit);
        format!(
            "[Streaming response: showing last {} of {} bytes]\n\n{}",
            tail.len(),
            body.len(),
            tail
        )
    } else {
        let half = limit / 2;
        let head = head_on_char_boundary(body, half);
        let tail = tail_on_char_boundary(body, half);
        format!(
            "[Large response: showing first {} and last {} of {} bytes]\n\n{}\n\n... omitted {} bytes ...\n\n{}",
            head.len(),
            tail.len(),
            body.len(),
            head,
            body.len().saturating_sub(head.len() + tail.len()),
            tail
        )
    }
}

/// Prepare an HTML body for the lightweight `TextView::html` renderer:
/// remove elements it can't meaningfully render (whose raw source would
/// otherwise appear as text), strip comments, and collapse newlines (gpui's
/// text shaper panics on a newline inside a single text run).
fn sanitize_html_for_preview(html: &str) -> String {
    let mut out = html.to_string();
    for tag in ["script", "style", "head", "noscript", "svg", "template", "iframe"] {
        out = remove_html_blocks(&out, tag);
    }
    out = remove_between(&out, "<!--", "-->");
    out.replace(['\r', '\n'], " ")
}

/// Remove every `<tag ...>...</tag>` block (case-insensitive) from `input`.
fn remove_html_blocks(input: &str, tag: &str) -> String {
    let lower = input.to_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut result = String::with_capacity(input.len());
    let mut pos = 0;
    while let Some(rel) = lower[pos..].find(&open) {
        let start = pos + rel;
        // Only treat as a real tag if followed by a delimiter (space, >, /, tab).
        let after = lower[start + open.len()..].chars().next();
        if !matches!(after, Some(' ') | Some('>') | Some('/') | Some('\t') | Some('\n') | Some('\r') | None) {
            result.push_str(&input[pos..start + open.len()]);
            pos = start + open.len();
            continue;
        }
        match lower[start..].find(&close) {
            Some(close_rel) => {
                let end = start + close_rel + close.len();
                result.push_str(&input[pos..start]);
                pos = end;
            }
            // Unclosed block: drop the rest.
            None => {
                result.push_str(&input[pos..start]);
                return result;
            }
        }
    }
    result.push_str(&input[pos..]);
    result
}

/// Remove every `start ... end` span (used for HTML comments).
fn remove_between(input: &str, start: &str, end: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut pos = 0;
    while let Some(rel) = input[pos..].find(start) {
        let s = pos + rel;
        match input[s..].find(end) {
            Some(e_rel) => {
                let e = s + e_rel + end.len();
                result.push_str(&input[pos..s]);
                pos = e;
            }
            None => {
                result.push_str(&input[pos..s]);
                return result;
            }
        }
    }
    result.push_str(&input[pos..]);
    result
}

fn head_on_char_boundary(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

fn tail_on_char_boundary(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut start = value.len().saturating_sub(max_bytes);
    while start < value.len() && !value.is_char_boundary(start) {
        start += 1;
    }
    &value[start..]
}

fn single_line_preview(value: &str, max_bytes: usize) -> String {
    let mut preview = head_on_char_boundary(value, max_bytes)
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    if value.len() > max_bytes {
        preview.push_str("...");
    }
    preview
}

fn append_sse_detail_text(out: &mut String, value: &str) {
    if out.len() >= SSE_EVENT_STORE_BYTES {
        return;
    }
    let remaining = SSE_EVENT_STORE_BYTES - out.len();
    if value.len() <= remaining {
        out.push_str(value);
        return;
    }
    out.push_str(head_on_char_boundary(value, remaining));
}

fn format_sse_detail_text(data: &str) -> String {
    if data.len() <= SSE_DETAIL_FORMAT_LIMIT_BYTES {
        return try_format_json(data);
    }
    data.to_string()
}

fn try_format_json(body: &str) -> String {
    let t = body.trim();
    if t.starts_with('{') || t.starts_with('[') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
            return serde_json::to_string_pretty(&v).unwrap_or_else(|_| body.to_string());
        }
    }
    body.to_string()
}
