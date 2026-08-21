use std::collections::HashSet;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, Sizable as _, Size,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    resizable::{ResizableState, resizable_panel, v_resizable},
    scroll::ScrollableElement,
    tab::{Tab, TabBar},
    table::{DataTable, TableState},
    text::TextView,
    v_flex,
};

use crate::state::{AppEvent, AppState};
use json_table::JsonTableDelegate;

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

#[derive(Clone)]
struct HeaderRequestInfo {
    method: String,
    url: String,
    resolved_url: Option<String>,
    headers: Vec<silvapi_core::models::KeyValue>,
    body_type: silvapi_core::models::BodyType,
    body: String,
    params: Vec<silvapi_core::models::KeyValue>,
    urlencoded: Vec<silvapi_core::models::KeyValue>,
    form_data: Vec<silvapi_core::models::FormDataPartSnapshot>,
    auth: Option<silvapi_core::models::AuthSnapshot>,
    content_type: Option<String>,
    body_bytes: usize,
    body_truncated: bool,
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024. * 1024.))
    }
}

/// Rows describing the request payload: content type, size, the body itself,
/// and the structured parts behind it for urlencoded / multipart bodies.
fn payload_rows(req: &HeaderRequestInfo) -> Vec<(String, String)> {
    let mut rows = vec![("Body Type".to_string(), format!("{:?}", req.body_type))];
    if let Some(content_type) = req.content_type.as_deref() {
        rows.push(("Content-Type".to_string(), content_type.to_string()));
    }
    if req.body_bytes > 0 {
        rows.push(("Body Size".to_string(), format_bytes(req.body_bytes as u64)));
    }

    for part in req.form_data.iter().filter(|part| part.enabled) {
        let kind = format!("{:?}", part.kind);
        let size = part
            .size_bytes
            .map(|size| format!(" · {}", format_bytes(size)))
            .unwrap_or_default();
        let content_type = if part.content_type.is_empty() {
            String::new()
        } else {
            format!(" · {}", part.content_type)
        };
        rows.push((
            format!("Part: {}", part.name),
            format!("{kind}{content_type}{size} · {}", part.value),
        ));
    }

    for pair in req.urlencoded.iter().filter(|pair| pair.enabled) {
        rows.push((format!("Field: {}", pair.key), pair.value.clone()));
    }

    if !req.body.trim().is_empty() {
        let label = if req.body_truncated {
            format!(
                "Body (first {} of {})",
                format_bytes(req.body.len() as u64),
                format_bytes(req.body_bytes as u64)
            )
        } else {
            "Body".to_string()
        };
        rows.push((label, req.body.clone()));
    }

    rows
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseTab {
    Response,
    Table,
    Preview,
    Headers,
    Timeline,
}

impl ResponseTab {
    fn index(&self) -> usize {
        match self {
            ResponseTab::Response => 0,
            ResponseTab::Table => 1,
            ResponseTab::Preview => 2,
            ResponseTab::Headers => 3,
            ResponseTab::Timeline => 4,
        }
    }

    fn from_index(i: usize) -> Self {
        match i {
            0 => ResponseTab::Response,
            1 => ResponseTab::Table,
            2 => ResponseTab::Preview,
            3 => ResponseTab::Headers,
            _ => ResponseTab::Timeline,
        }
    }
}

pub struct ResponsePanel {
    app_state: Entity<AppState>,
    active_tab: ResponseTab,
    response_editor: Entity<EditorState>,
    sse_detail_editor: Entity<EditorState>,
    headers_editor: Entity<EditorState>,
    table_search_input: Entity<InputState>,
    response_text: String,
    soft_wrap: bool,
    response_editor_plain: bool,
    sse_detail_soft_wrap: bool,
    sse_detail_column_layout: bool,
    headers_source_view: bool,
    headers_soft_wrap: bool,
    /// Rebuilt whenever the response body changes; `None` when the body is not
    /// tabular JSON, which also hides the Table tab.
    json_table: Option<Entity<TableState<JsonTableDelegate>>>,
    /// Keeps the panel re-rendering as the table's own selection/sort changes.
    json_table_sub: Option<Subscription>,
    /// Detail panel occupies the whole Table tab, hiding the grid.
    json_detail_maximized: bool,
    /// Explicit render mode for selected Markdown-looking cell values.
    json_detail_render_markdown: bool,
    /// Drag position of the grid/detail split.
    json_detail_resize: Entity<ResizableState>,
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
mod json_table;
mod preview;
mod timeline;

impl ResponsePanel {
    pub fn new(app_state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let response_editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("json")
                .folding(true)
                .searchable(true)
                .replaceable(false)
                .placeholder("Response will appear here... (Ctrl+F to search)")
                .soft_wrap(true)
        });
        let sse_detail_editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("json")
                .folding(true)
                .searchable(true)
                .replaceable(false)
                .placeholder("Message detail")
                .soft_wrap(true)
        });
        let headers_editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("http")
                .folding(true)
                .searchable(true)
                .replaceable(false)
                .placeholder("Headers")
                .soft_wrap(false)
        });
        let table_search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search table..."));

        let focus_handle = cx.focus_handle();

        let mut this = Self {
            app_state: app_state.clone(),
            active_tab: ResponseTab::Response,
            response_editor: response_editor.clone(),
            sse_detail_editor,
            headers_editor,
            table_search_input: table_search_input.clone(),
            response_text: String::new(),
            soft_wrap: true,
            response_editor_plain: false,
            sse_detail_soft_wrap: true,
            sse_detail_column_layout: false,
            headers_source_view: false,
            headers_soft_wrap: false,
            json_table: None,
            json_table_sub: None,
            json_detail_maximized: false,
            json_detail_render_markdown: false,
            json_detail_resize: cx.new(|_| ResizableState::default()),
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
                        this.rebuild_json_table(window, cx);
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
        let table_search_sub = cx.subscribe_in(
            &table_search_input,
            window,
            |this: &mut ResponsePanel, _, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) {
                    let query = this.table_query(cx);
                    if let Some(table) = this.json_table.clone() {
                        table.update(cx, |state, cx| {
                            if state.delegate_mut().set_query(&query) {
                                // Row indices shift under the filter, so the
                                // old selection no longer points at its value.
                                state.clear_selection(cx);
                                state.refresh(cx);
                            }
                        });
                    }
                    cx.notify();
                }
            },
        );
        this._subs = vec![
            app_sub,
            response_editor_sub,
            headers_editor_sub,
            sse_detail_editor_sub,
            table_search_sub,
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
        let has_json_table = self.json_table.is_some();

        let mut visible_tabs = vec![ResponseTab::Response];
        if has_json_table {
            visible_tabs.push(ResponseTab::Table);
        }
        if is_html {
            visible_tabs.push(ResponseTab::Preview);
        }
        visible_tabs.push(ResponseTab::Headers);
        visible_tabs.push(ResponseTab::Timeline);

        if self.active_tab == ResponseTab::Preview && !is_html {
            self.active_tab = ResponseTab::Response;
        }
        if self.active_tab == ResponseTab::Table && !has_json_table {
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
                        ResponseTab::Table => "Table",
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
                        // A failed history entry carries its own error, so
                        // selecting one shows that failure rather than the live
                        // request's (or nothing at all).
                        let state = self.app_state.read(cx);
                        let error = state.error.clone().or_else(|| {
                            state.response.as_ref().and_then(|resp| resp.error.clone())
                        });
                        if let Some(err) = &error {
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
                                        .child(Editor::new(&self.response_editor).h_full().w_full()),
                                )
                            }
                        }
                    })
                    .when(active_tab == ResponseTab::Preview, |el| {
                        el.child(self.render_preview_tab(cx))
                    })
                    .when(active_tab == ResponseTab::Table, |el| {
                        match self.render_json_table_tab(cx) {
                            Some(table) => el.child(table),
                            None => el,
                        }
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

impl ResponsePanel {
    fn current_json_value(&self, cx: &App) -> Option<serde_json::Value> {
        let body = self
            .app_state
            .read(cx)
            .response
            .as_ref()
            .map(|response| response.body.trim().to_string())?;
        if body.len() > FORMAT_RESPONSE_LIMIT_BYTES
            || !(body.starts_with('{') || body.starts_with('['))
        {
            return None;
        }
        serde_json::from_str(&body).ok()
    }

    /// Rebuild the Table tab's state from the current response body. Called on
    /// every response/selection change so the tab visibility and the table data
    /// stay in sync without re-parsing the body on each frame.
    fn rebuild_json_table(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let delegate = self
            .current_json_value(cx)
            .as_ref()
            .and_then(JsonTableDelegate::from_json);

        self.json_detail_maximized = false;
        self.json_detail_render_markdown = false;

        let Some(mut delegate) = delegate else {
            self.json_table = None;
            self.json_table_sub = None;
            return;
        };

        delegate.set_query(&self.table_query(cx));
        let table = cx.new(|cx| {
            TableState::new(delegate, window, cx)
                .cell_selectable(true)
                .row_header(false)
                .col_resizable(true)
                .col_movable(true)
                .sortable(true)
        });
        self.json_table_sub = Some(cx.observe(&table, |_, _, cx| cx.notify()));
        self.json_table = Some(table);
    }

    /// True when a cell is selected, so the detail panel has something to show.
    fn json_detail_open(&self, cx: &App) -> bool {
        self.json_table
            .as_ref()
            .is_some_and(|table| table.read(cx).selected_cell().is_some())
    }

    fn render_json_table_tab(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let table = self.json_table.clone()?;
        let grid = div().size_full().min_h(px(0.)).p_1().child(
            // Stripes off: the table pads its viewport with blank filler rows,
            // and striping makes those read as data.
            DataTable::new(&table)
                .stripe(false)
                .bordered(true)
                .with_size(Size::Small),
        );

        let body = match self.render_json_table_detail(&table, cx) {
            // Full screen: the detail takes the tab. The table's column widths
            // and selection live in the TableState entity, so they survive the
            // toggle even though the grid is not rendered.
            Some(detail) if self.json_detail_maximized => {
                div().size_full().child(detail).into_any_element()
            }
            Some(detail) => v_resizable("json-table-detail-split")
                .with_state(&self.json_detail_resize)
                .child(
                    resizable_panel()
                        .size(px(320.))
                        .size_range(px(120.)..px(1600.))
                        .child(grid),
                )
                .child(
                    resizable_panel()
                        .size(px(260.))
                        .size_range(px(80.)..px(1200.))
                        .child(detail),
                )
                .into_any_element(),
            None => grid.into_any_element(),
        };

        Some(
            v_flex()
                .size_full()
                .min_w(px(0.))
                .overflow_hidden()
                .child(self.render_json_table_toolbar(&table, cx))
                .child(div().flex_1().min_h(px(0.)).child(body))
                .into_any_element(),
        )
    }

    fn render_json_table_toolbar(
        &self,
        table: &Entity<TableState<JsonTableDelegate>>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let delegate = table.read(cx).delegate();
        let visible = delegate.visible_count();
        let total = delegate.total_count();
        let truncated = delegate.truncated_rows();
        let csv = delegate.visible_csv();

        let mut summary = if visible == total {
            format!("{total} rows")
        } else {
            format!("{visible} of {total} rows")
        };
        if truncated > 0 {
            summary.push_str(&format!(" · {truncated} not shown"));
        }

        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .w(px(260.))
                    .child(Input::new(&self.table_search_input)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(summary),
            )
            .child(div().flex_1())
            .child(
                Button::new("json-table-copy-csv")
                    .icon(IconName::File)
                    .tooltip("Copy visible rows as CSV")
                    .ghost()
                    .xsmall()
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(csv.clone()));
                    }),
            )
    }

    /// The detail panel below the table. `DataTable` rows are a uniform height,
    /// so this is where a long value is shown in full (and copied from) instead
    /// of a hover button inside the cell.
    fn render_json_table_detail(
        &self,
        table: &Entity<TableState<JsonTableDelegate>>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let state = table.read(cx);
        let (row_ix, col_ix) = state.selected_cell()?;
        let delegate = state.delegate();
        let value = delegate.cell(row_ix, col_ix)?.to_string();
        let column = delegate.column_name(col_ix).unwrap_or_default();
        let heading = match delegate.row_label(row_ix) {
            Some(label) if label != column.as_ref() => format!("{label} · {column}"),
            _ => column.to_string(),
        };
        let row_json = delegate.row_json(row_ix);
        let maximized = self.json_detail_maximized;
        let markdown_value = looks_like_markdown(&value);
        let render_markdown = markdown_value && self.json_detail_render_markdown;
        let table = table.clone();

        Some(
            v_flex()
                // Sized by the resizable split (or the whole tab when full
                // screen), so the panel takes all the height it is given.
                .size_full()
                .min_h(px(0.))
                .overflow_hidden()
                .border_t_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().muted.opacity(0.12))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap_1()
                        .px_2()
                        .py_1()
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .truncate()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(cx.theme().muted_foreground)
                                .child(heading),
                        )
                        .child(
                            Button::new("json-table-copy-value")
                                .icon(IconName::Copy)
                                .tooltip("Copy value")
                                .ghost()
                                .xsmall()
                                .on_click({
                                    let value = value.clone();
                                    move |_, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            value.clone(),
                                        ));
                                    }
                                }),
                        )
                        .when(markdown_value, |el| {
                            el.child(
                                Button::new("json-table-render-markdown")
                                    .icon(if render_markdown {
                                        IconName::SquareTerminal
                                    } else {
                                        IconName::File
                                    })
                                    .tooltip(if render_markdown {
                                        "Show source"
                                    } else {
                                        "Render Markdown"
                                    })
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.json_detail_render_markdown =
                                            !this.json_detail_render_markdown;
                                        cx.notify();
                                    })),
                            )
                        })
                        .children(row_json.map(|row_json| {
                            Button::new("json-table-copy-row")
                                .icon(IconName::Copy)
                                .tooltip("Copy row as JSON")
                                .ghost()
                                .xsmall()
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        row_json.clone(),
                                    ));
                                })
                        }))
                        .child(
                            Button::new("json-table-detail-maximize")
                                .icon(if maximized {
                                    IconName::WindowRestore
                                } else {
                                    IconName::Maximize
                                })
                                .tooltip(if maximized {
                                    "Restore detail"
                                } else {
                                    "Expand detail"
                                })
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.json_detail_maximized = !this.json_detail_maximized;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("json-table-close-detail")
                                .icon(IconName::Close)
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.json_detail_maximized = false;
                                    this.json_detail_render_markdown = false;
                                    table.update(cx, |state, cx| state.clear_selection(cx));
                                })),
                        ),
                )
                // Two levels on purpose. `overflow_y_scrollbar` copies its
                // element's sizing onto a wrapper that it also gives
                // `size_full`, so a `flex_1` scroll area resolves against the
                // panel's whole height and ignores the header row above it,
                // overflowing instead of scrolling. The outer div claims the
                // leftover space; the inner one takes a definite `size_full`
                // from it, which is what the scroll area needs to overflow
                // against.
                .child(
                    div()
                        .w_full()
                        .flex_1()
                        .min_h(px(0.))
                        .overflow_hidden()
                        .child(
                            div()
                                .id("json-table-detail-body")
                                .size_full()
                                .overflow_y_scrollbar()
                                .px_2()
                                .pb_2()
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .whitespace_normal()
                                .child(if render_markdown {
                                    TextView::markdown("json-table-detail-markdown", value.clone())
                                        .selectable(true)
                                        .into_any_element()
                                } else {
                                    div().child(value).into_any_element()
                                }),
                        ),
                )
                .into_any_element(),
        )
    }

    fn table_query(&self, cx: &App) -> String {
        self.table_search_input
            .read(cx)
            .value()
            .trim()
            .to_lowercase()
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
    req: Option<HeaderRequestInfo>,
    res: Option<&silvapi_core::models::HttpResponse>,
) -> Vec<HeaderSection> {
    let mut sections = Vec::new();
    let mut general = Vec::new();

    if let Some(req) = req.as_ref() {
        general.push((
            "Request URL".to_string(),
            req.resolved_url
                .as_deref()
                .filter(|url| !url.is_empty())
                .unwrap_or(&req.url)
                .to_string(),
        ));
        if req
            .resolved_url
            .as_deref()
            .is_some_and(|url| url != req.url)
        {
            general.push(("Original URL".to_string(), req.url.clone()));
        }
        general.push(("Request Method".to_string(), req.method.clone()));
    }

    if let Some(res) = res {
        if let Some(started_at) = res.started_at.as_deref() {
            general.push(("Sent At".to_string(), started_at.to_string()));
        }
        let status = if res.status_text.is_empty() {
            res.status.to_string()
        } else {
            format!("{} {}", res.status, res.status_text)
        };
        general.push(("Status Code".to_string(), status));
        general.push(("Time".to_string(), format_duration(res.time_ms)));
        general.push(("Size".to_string(), res.formatted_size()));
        if let Some(error) = res.error.as_deref() {
            general.push(("Error".to_string(), error.to_string()));
        }
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

    if let Some(req) = req.as_ref() {
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

        let params: Vec<(String, String)> = req
            .params
            .iter()
            .filter(|param| param.enabled && !param.key.is_empty())
            .map(|param| (param.key.clone(), param.value.clone()))
            .collect();
        if !params.is_empty() {
            sections.push(HeaderSection {
                title: "Query Params",
                rows: params,
            });
        }

        if let Some(auth) = req.auth.as_ref()
            && !auth.fields.is_empty()
        {
            let mut rows = vec![
                ("Type".to_string(), format!("{:?}", auth.auth_type)),
                ("Enabled".to_string(), auth.enabled.to_string()),
            ];
            rows.extend(auth.fields.iter().cloned());
            sections.push(HeaderSection {
                title: "Auth",
                rows,
            });
        }

        let payload = payload_rows(req);
        if payload.len() > 1 {
            sections.push(HeaderSection {
                title: "Request Payload",
                rows: payload,
            });
        }
    }

    sections
}

fn format_headers_dump(
    req: Option<HeaderRequestInfo>,
    res: Option<&silvapi_core::models::HttpResponse>,
) -> String {
    let mut out = String::new();

    out.push_str("General\n");
    if let Some(req) = req.as_ref() {
        push_header_line(
            &mut out,
            "Request URL",
            req.resolved_url
                .as_deref()
                .filter(|url| !url.is_empty())
                .unwrap_or(&req.url),
        );
        if req
            .resolved_url
            .as_deref()
            .is_some_and(|url| url != req.url)
        {
            push_header_line(&mut out, "Original URL", &req.url);
        }
        push_header_line(&mut out, "Request Method", &req.method);
    }
    if let Some(res) = res {
        if let Some(started_at) = res.started_at.as_deref() {
            push_header_line(&mut out, "Sent At", started_at);
        }
        let status = if res.status_text.is_empty() {
            res.status.to_string()
        } else {
            format!("{} {}", res.status, res.status_text)
        };
        push_header_line(&mut out, "Status Code", &status);
        push_header_line(&mut out, "Time", &format_duration(res.time_ms));
        push_header_line(&mut out, "Size", &res.formatted_size());
        if let Some(error) = res.error.as_deref() {
            push_header_line(&mut out, "Error", error);
        }
    }
    push_header_line(&mut out, "Version", "HTTP/1.1");

    if let Some(req) = req.as_ref() {
        let params: Vec<&silvapi_core::models::KeyValue> = req
            .params
            .iter()
            .filter(|param| param.enabled && !param.key.is_empty())
            .collect();
        if !params.is_empty() {
            out.push_str("\nQuery Params\n");
            for param in params {
                push_header_line(&mut out, &param.key, &param.value);
            }
        }

        if let Some(auth) = req.auth.as_ref()
            && !auth.fields.is_empty()
        {
            out.push_str("\nAuth\n");
            push_header_line(&mut out, "Type", &format!("{:?}", auth.auth_type));
            push_header_line(&mut out, "Enabled", &auth.enabled.to_string());
            for (key, value) in &auth.fields {
                push_header_line(&mut out, key, value);
            }
        }
    }

    if let Some(req) = req.as_ref()
        && !req.headers.is_empty()
    {
        out.push_str("\nRequest Headers\n");
        for header in &req.headers {
            if header.enabled && !header.key.is_empty() {
                push_header_line(&mut out, &header.key, &header.value);
            }
        }
    }

    if let Some(req) = req.as_ref() {
        let payload = payload_rows(req);
        if payload.len() > 1 {
            out.push_str("\nRequest Payload\n");
            for (key, value) in payload {
                push_header_line(&mut out, &key, &value);
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
    for tag in [
        "script", "style", "head", "noscript", "svg", "template", "iframe",
    ] {
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
        if !matches!(
            after,
            Some(' ') | Some('>') | Some('/') | Some('\t') | Some('\n') | Some('\r') | None
        ) {
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

fn looks_like_markdown(value: &str) -> bool {
    let text = value.trim();
    text.contains("```")
        || text.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with("# ")
                || line.starts_with("## ")
                || line.starts_with("- ")
                || line.starts_with("* ")
                || line.starts_with("> ")
                || line.starts_with("| ")
        })
        || text.contains("**")
        || text.contains("__")
        || text.contains("[") && text.contains("](")
}
