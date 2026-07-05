use std::collections::HashSet;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Sizable as _,
    button::ButtonVariants as _,
    input::{Input, InputEvent, InputState},
    resizable::ResizableState,
    scroll::ScrollableElement,
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
    headers_soft_wrap: bool,
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


mod body_view;
mod headers;
mod preview;
mod timeline;

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
                .code_editor("json")
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
            headers_soft_wrap: false,
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
                            this.set_response_text(preview_text, window, cx);
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
                        this.set_response_text(formatted, window, cx);
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
    req: Option<&silvapi_core::models::ApiRequest>,
    res: Option<&silvapi_core::models::HttpResponse>,
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
    req: Option<&silvapi_core::models::ApiRequest>,
    res: Option<&silvapi_core::models::HttpResponse>,
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

