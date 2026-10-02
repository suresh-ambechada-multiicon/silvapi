#![allow(unused_imports)]
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_kit::component::dialog::{
    Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
};
use gpui_kit::component::{
    ActiveTheme, FocusTrapElement as _, Icon, IconName, Root, Sizable as _, StyledExt, Theme,
    ThemeRegistry, WindowExt as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::PopupMenuItem,
    resizable::{ResizableState, h_resizable, resizable_panel, v_resizable},
    tab::{Tab, TabBar},
    scroll::ScrollableElement as _,
    tooltip::Tooltip,
    v_flex, Size,
    list::{List, ListDelegate, ListEvent, ListItem, ListState},
    IndexPath,
};

use super::{ApiPickerEntry, AppView, ShortcutInput, ShortcutSpec};
use crate::state::{AppEvent, AppState};
use crate::ui::actions::{
    ApiPicker, CloseSettings, FocusActiveRequest, FocusCollectionPanel, FocusRequestPanel,
    FocusResponsePanel, FocusUrl, NextApi, OpenSettings, PrevApi, RenameSelected, SendRequest,
    ThemePicker, ToggleMaximize,
};
use crate::ui::collection_panel::CollectionPanel;
use crate::ui::request_panel::RequestPanel;
use crate::ui::response_panel::ResponsePanel;
use crate::ui::top_bar::TopBar;

const API_PICKER_ROW_HEIGHT: Pixels = px(40.);
const METHOD_BADGE_MIN_WIDTH: Pixels = px(48.);
const CONTEXT_PATH_MAX_WIDTH: Pixels = px(220.);
const URL_PREVIEW_MAX_WIDTH: Pixels = px(180.);

const THEME_PICKER_WIDTH: Pixels = px(300.);
const THEME_PICKER_MAX_HEIGHT: Pixels = px(520.);
const THEME_LIST_MAX_HEIGHT: Pixels = px(460.);

const TAB_PICKER_WIDTH: Pixels = px(680.);
const TAB_PICKER_MAX_HEIGHT: Pixels = px(468.);
const TAB_ROW_HEIGHT: Pixels = px(40.);

const FONT_PICKER_WIDTH: Pixels = px(420.);
const FONT_PICKER_MAX_HEIGHT: Pixels = px(560.);
const FONT_ROW_HEIGHT: Pixels = px(36.);

pub struct ApiPickerDelegate {
    app_view: Entity<AppView>,
    all_items: Vec<ApiPickerEntry>,
    filtered_items: Vec<ApiPickerEntry>,
    selected_index: Option<IndexPath>,
}

impl ListDelegate for ApiPickerDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.filtered_items.len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        let query = query.to_lowercase();
        if query.is_empty() {
            self.filtered_items = self.all_items.clone();
        } else {
            self.filtered_items = self
                .all_items
                .iter()
                .filter(|entry| search_matches(&entry.search_text, &query))
                .cloned()
                .collect();
        }
        self.selected_index = if self.filtered_items.is_empty() {
            None
        } else {
            Some(IndexPath::new(0))
        };
        cx.notify();
        Task::ready(())
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        if let Some(ix) = self.selected_index {
            if let Some(entry) = self.filtered_items.get(ix.row).cloned() {
                let app_view = self.app_view.clone();
                app_view.update(cx, |view, cx| {
                    view.api_picker_open = false;
                    view.select_tab(entry.id, window, cx);
                    let fh = view.focus_handle.clone();
                    cx.defer_in(window, move |_, window, cx| {
                        fh.focus(window, cx);
                    });
                });
                window.close_dialog(cx);
            }
        }
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let entry = self.filtered_items.get(ix.row)?;
        let is_selected = self.selected_index == Some(ix);

        let item = ListItem::new(ix)
            .selected(is_selected)
            .h(API_PICKER_ROW_HEIGHT)
            .px_3()
            .py_1()
            .child(
                h_flex()
                    .size_full()
                    .items_center()
                    .gap_2()
                    .overflow_hidden()
                    .child(
                        div()
                            .min_w(METHOD_BADGE_MIN_WIDTH)
                            .text_xs()
                            .font_semibold()
                            .text_color(method_color(&entry.method, cx))
                            .child(entry.method.clone()),
                    )
                    .child(
                        div()
                            .max_w(CONTEXT_PATH_MAX_WIDTH)
                            .min_w(px(80.))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(entry.context.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground.opacity(0.7))
                            .child("/"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_sm()
                            .text_color(if is_selected {
                                cx.theme().primary
                            } else {
                                cx.theme().foreground
                            })
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(entry.name.clone()),
                    )
                    .child(
                        div()
                            .max_w(URL_PREVIEW_MAX_WIDTH)
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(entry.url.clone()),
                    ),
            );

        Some(item)
    }
}

impl AppView {
    pub(super) fn update_api_list(&mut self, cx: &mut Context<Self>) {
        self.api_list_dirty = true;
        cx.notify();
    }

    pub(super) fn rebuild_api_list(&mut self, cx: &App) {
        let query = self.api_search.read(cx).value().to_lowercase();
        self.rebuild_api_list_for_query(query, cx);
    }

    pub(super) fn ensure_api_list_for_query(&mut self, query: &str, cx: &App) {
        if self.api_list_dirty || self.api_list_query != query {
            self.rebuild_api_list_for_query(query.to_string(), cx);
        }
    }

    pub(super) fn rebuild_api_list_for_query(&mut self, query: String, cx: &App) {
        let mut out = Vec::new();
        let state = self.app_state.read(cx);
        for col in &state.workspace.collections {
            Self::collect_reqs(&col.items, &col.name, &mut out);
        }
        if let (Some(active_id), Some(active_req)) =
            (&state.active_request_id, &state.active_request)
        {
            if let Some(entry) = out.iter_mut().find(|entry| &entry.id == active_id) {
                entry.name = active_req.name.clone();
                entry.method = active_req.method.as_str().to_string();
                entry.url = active_req.url.clone();
                entry.full_path = format!("{} / {}", entry.context, entry.name);
                entry.search_text = api_entry_search_text(entry);
            }
        }

        if !query.is_empty() {
            out.retain(|entry| search_matches(&entry.search_text, &query));
        }

        self.api_list = out;
        self.api_list_query = query;
        self.api_list_dirty = false;
        self.api_cursor = 0;
        if !self.api_list.is_empty() {
            self.api_scroll.scroll_to_item(0, ScrollStrategy::Nearest);
        }
    }

    pub(super) fn collect_reqs(
        items: &[silvapi_core::models::CollectionItem],
        prefix: &str,
        out: &mut Vec<ApiPickerEntry>,
    ) {
        for item in items {
            match item {
                silvapi_core::models::CollectionItem::Folder(f) => {
                    let new_pref = format!("{} / {}", prefix, f.name);
                    Self::collect_reqs(&f.items, &new_pref, out);
                }
                silvapi_core::models::CollectionItem::Request(r) => {
                    let mut entry = ApiPickerEntry {
                        id: r.id.clone(),
                        context: prefix.to_string(),
                        name: r.name.clone(),
                        method: r.method.as_str().to_string(),
                        url: r.url.clone(),
                        full_path: format!("{} / {}", prefix, r.name),
                        search_text: String::new(),
                    };
                    entry.search_text = api_entry_search_text(&entry);
                    out.push(entry);
                }
            }
        }
    }

    pub(super) fn close_api_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.api_picker_open = false;
        window.close_dialog(cx);
        let fh = self.focus_handle.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    pub(super) fn select_api_from_picker(
        &mut self,
        req_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.api_picker_open = false;
        self.select_tab(req_id, window, cx);
    }

    pub(super) fn select_tab(
        &mut self,
        req_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ensure_tab_for_request(req_id.clone());
        if let Some(idx) = self.open_tabs.iter().position(|id| id == &req_id) {
            self.tab_bar_scroll.scroll_to_item(idx);
        }
        self.app_state.update(cx, |state, cx| {
            if state.select_request(&req_id) {
                cx.emit(AppEvent::RequestSelected);
            }
        });
        self.collection_panel.update(cx, |panel, cx| {
            panel.reveal_active_request(true, window, cx);
        });
        cx.notify();
    }

    pub(super) fn close_theme_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.theme_picker_open = false;
        let fh = self.focus_handle.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    pub(super) fn open_api_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.api_picker_open = true;
        let app_view = cx.entity().clone();

        let viewport = window.viewport_size();
        let picker_w = (viewport.width * 0.5).max(px(480.)).min(px(960.));
        let picker_max_h = (viewport.height * 0.7).max(px(320.)).min(px(720.));

        let mut all_items = Vec::new();
        let state = self.app_state.read(cx);
        for col in &state.workspace.collections {
            Self::collect_reqs(&col.items, &col.name, &mut all_items);
        }
        if let (Some(active_id), Some(active_req)) =
            (&state.active_request_id, &state.active_request)
        {
            if let Some(entry) = all_items.iter_mut().find(|entry| &entry.id == active_id) {
                entry.name = active_req.name.clone();
                entry.method = active_req.method.as_str().to_string();
                entry.url = active_req.url.clone();
                entry.full_path = format!("{} / {}", entry.context, entry.name);
                entry.search_text = api_entry_search_text(entry);
            }
        }
        let filtered_items = all_items.clone();
        let selected_index = if filtered_items.is_empty() {
            None
        } else {
            Some(IndexPath::new(0))
        };

        let delegate = ApiPickerDelegate {
            app_view: app_view.clone(),
            all_items,
            filtered_items,
            selected_index,
        };

        let list_state = cx.new(|cx| ListState::new(delegate, window, cx).searchable(true));

        let list_focus = list_state.read(cx).focus_handle(cx);
        cx.defer_in(window, move |_, window, cx| {
            list_focus.focus(window, cx);
        });

        cx.subscribe_in(&list_state, window, move |_this, _state, ev: &ListEvent, window, cx| {
            if matches!(ev, ListEvent::Cancel) {
                window.close_dialog(cx);
            }
        }).detach();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let view = app_view.clone();
            let state = list_state.clone();
            dialog
                .w(picker_w)
                .max_h(picker_max_h)
                .close_button(false)
                .overlay(true)
                .overlay_closable(true)
                .keyboard(true)
                .on_close(move |_, _window, cx| {
                    view.update(cx, |this, _cx| {
                        this.api_picker_open = false;
                    });
                })
                .child(
                    div()
                        .w_full()
                        .h(picker_max_h)
                        .child(List::new(&state)),
                )
        });
    }

    pub(super) fn ensure_tab_for_request(&mut self, req_id: String) {
        if !self.open_tabs.iter().any(|id| id == &req_id) {
            self.open_tabs.push(req_id);
        }
    }

    pub(super) fn active_tab_index(&self, cx: &App) -> Option<usize> {
        let active_id = self.app_state.read(cx).active_request_id.clone()?;
        self.open_tabs.iter().position(|id| id == &active_id)
    }

    pub(super) fn select_adjacent_tab(&mut self, step: isize, cx: &mut Context<Self>) {
        if self.open_tabs.is_empty() {
            if let Some(id) = self.app_state.read(cx).active_request_id.clone() {
                self.ensure_tab_for_request(id);
            }
        }
        let len = self.open_tabs.len();
        if len == 0 {
            return;
        }
        let current = self.active_tab_index(cx).unwrap_or(0);
        let next = if step.is_negative() {
            (current + len - 1) % len
        } else {
            (current + 1) % len
        };
        let req_id = self.open_tabs[next].clone();
        self.app_state.update(cx, |state, cx| {
            if state.select_request(&req_id) {
                cx.emit(AppEvent::RequestSelected);
            }
        });
        cx.notify();
    }

    pub(super) fn select_tab_number(
        &mut self,
        idx: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(req_id) = self.open_tabs.get(idx).cloned() {
            self.select_tab(req_id, window, cx);
        }
    }

    pub(super) fn close_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(idx) = self.active_tab_index(cx) else {
            return;
        };
        self.open_tabs.remove(idx);
        if let Some(next_id) = self
            .open_tabs
            .get(idx)
            .or_else(|| self.open_tabs.last())
            .cloned()
        {
            self.select_tab(next_id, window, cx);
        } else {
            self.app_state.update(cx, |state, _| {
                state.active_request = None;
                state.active_request_id = None;
                state.response = None;
                state.formatted_response = None;
                state.error = None;
            });
            cx.notify();
        }
    }

    pub(super) fn update_tab_list(&mut self, cx: &mut Context<Self>) {
        let query = self.tab_search.read(cx).value().to_lowercase();
        self.rebuild_tab_list_for_query(query, cx);
        cx.notify();
    }

    pub(super) fn rebuild_tab_list_for_query(&mut self, query: String, cx: &App) {
        self.ensure_api_list_for_query("", cx);
        let mut entries = Vec::new();
        for id in &self.open_tabs {
            if let Some(entry) = self.api_list.iter().find(|entry| &entry.id == id).cloned() {
                entries.push(entry);
            }
        }
        if !query.is_empty() {
            entries.retain(|entry| search_matches(&entry.search_text, &query));
        }
        self.tab_list = entries;
        self.tab_list_query = query;
        self.tab_cursor = 0;
        if !self.tab_list.is_empty() {
            self.tab_scroll.scroll_to_item(0, ScrollStrategy::Nearest);
        }
    }

    pub(super) fn open_tab_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tab_search
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.rebuild_tab_list_for_query(String::new(), cx);
        if let Some(active_id) = &self.app_state.read(cx).active_request_id {
            if let Some(idx) = self
                .tab_list
                .iter()
                .position(|entry| &entry.id == active_id)
            {
                self.tab_cursor = idx;
                self.tab_scroll.scroll_to_item(idx, ScrollStrategy::Nearest);
            }
        }
        self.tab_picker_open = true;
        cx.notify();
        let fh = self.tab_search.read(cx).focus_handle(cx);
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
    }

    pub(super) fn close_tab_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tab_picker_open = false;
        let fh = self.focus_handle.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    pub(super) fn render_tab_strip(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.open_tabs.is_empty() {
            return div().into_any_element();
        }

        self.ensure_api_list_for_query("", cx);
        let active_id = self.app_state.read(cx).active_request_id.clone();
        let tabs = self.open_tabs.clone();
        let view = cx.entity().clone();

        let selected_index = tabs
            .iter()
            .position(|id| active_id.as_deref() == Some(id.as_str()))
            .unwrap_or(0);

        let tab_items: Vec<Tab> = tabs
            .iter()
            .enumerate()
            .map(|(idx, id)| {
                let (name, method) = if let Some(entry) = self.api_list.iter().find(|entry| &entry.id == id) {
                    (entry.name.clone(), entry.method.clone())
                } else if let Some(req) = self.app_state.read(cx).find_request(id) {
                    (req.name.clone(), req.method.as_str().to_string())
                } else {
                    ("Request".to_string(), "GET".to_string())
                };
                let id_for_close = id.clone();
                let method_col = method_color(&method, cx);
                let view_for_close = view.clone();

                Tab::new()
                    .prefix(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(method_col)
                            .child(method),
                    )
                    .label(name)
                    .suffix(
                        Button::new(SharedString::from(format!("close-tab-{}", idx)))
                            .icon(IconName::Close)
                            .ghost()
                            .xsmall()
                            .on_click(move |_event, window, cx| {
                                view_for_close.update(cx, |this, cx| {
                                    this.close_tab_by_id(&id_for_close, window, cx);
                                });
                            }),
                    )
            })
            .collect();

        TabBar::new("request-tabs")
            .underline()
            .small()
            .menu(true)
            .track_scroll(&self.tab_bar_scroll)
            .selected_index(selected_index)
            .children(tab_items)
            .on_click(move |idx, window, cx| {
                view.update(cx, |this, cx| {
                    if let Some(id) = this.open_tabs.get(*idx).cloned() {
                        this.select_tab(id, window, cx);
                    }
                });
            })
            .into_any_element()
    }

    pub(super) fn close_tab_by_id(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(pos) = self.open_tabs.iter().position(|tab| tab == id) {
            self.open_tabs.remove(pos);
            if self.app_state.read(cx).active_request_id.as_deref() == Some(id) {
                if let Some(next_id) = self.open_tabs.get(pos).or_else(|| self.open_tabs.last()).cloned() {
                    self.select_tab(next_id, window, cx);
                }
            }
            cx.notify();
        }
    }

    pub(super) fn render_tab_picker_rows(
        &mut self,
        range: std::ops::Range<usize>,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut rows = Vec::new();
        for idx in range {
            let Some(entry) = self.tab_list.get(idx).cloned() else {
                continue;
            };
            let is_selected = idx == self.tab_cursor;
            let bg = if is_selected {
                cx.theme().primary.opacity(0.1)
            } else {
                gpui_kit::transparent_black()
            };
            rows.push(
                div()
                    .id(SharedString::from(format!("tab-picker-row-{}", idx)))
                    .w_full()
                    .h(px(40.))
                    .px_3()
                    .bg(bg)
                    .cursor_pointer()
                    .tooltip({
                        let tooltip = entry.full_path.clone();
                        move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx)
                    })
                    .on_mouse_down(
                        gpui_kit::MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            this.tab_cursor = idx;
                            this.select_tab(entry.id.clone(), window, cx);
                            this.close_tab_picker(window, cx);
                        }),
                    )
                    .child(
                        h_flex()
                            .size_full()
                            .items_center()
                            .gap_2()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(method_color(&entry.method, cx))
                                    .child(entry.method.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .text_sm()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(entry.full_path.clone()),
                            )
                            .child(
                                div()
                                    .max_w(px(180.))
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(entry.url.clone()),
                            ),
                    )
                    .into_any_element(),
            );
        }
        rows
    }

    pub(super) fn update_theme_list(&mut self, cx: &mut Context<Self>) {
        let query = self.theme_search.read(cx).value().to_lowercase();
        let mut sorted: Vec<String> = ThemeRegistry::global(cx)
            .themes()
            .keys()
            .map(|s| s.to_string())
            .filter(|s| query.is_empty() || s.to_lowercase().contains(&query))
            .collect();
        sorted.sort();
        self.theme_list = sorted;

        let current = Theme::global(cx).theme_name().to_string();
        self.theme_cursor = self
            .theme_list
            .iter()
            .position(|n| n == &current)
            .unwrap_or(0);

        if !self.theme_list.is_empty() {
            self.theme_scroll.scroll_to_item(self.theme_cursor);
        }

        self.apply_theme_at_cursor(cx);
        cx.notify();
    }

    pub(super) fn open_theme_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.original_theme = Theme::global(cx).theme_name().to_string();

        self.theme_search
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.update_theme_list(cx);

        self.theme_picker_open = true;
        cx.notify();

        let cursor = self.theme_cursor;
        let scroll = self.theme_scroll.clone();
        cx.defer_in(window, move |_, _, _| {
            scroll.scroll_to_item(cursor);
        });

        let fh = self.theme_search.read(cx).focus_handle(cx);
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
    }

    pub(super) fn apply_theme_at_cursor(&mut self, cx: &mut Context<Self>) {
        if let Some(name) = self.theme_list.get(self.theme_cursor) {
            let name = name.clone();
            if let Some(cfg) = ThemeRegistry::global(cx)
                .themes()
                .get(name.as_str())
                .cloned()
            {
                Theme::global_mut(cx).apply_config(&cfg);
                Self::apply_user_font_settings(cx);
                cx.refresh_windows();
            }
        }
    }

    pub(super) fn restore_original_theme(&mut self, cx: &mut Context<Self>) {
        let name = self.original_theme.clone();
        if let Some(cfg) = ThemeRegistry::global(cx)
            .themes()
            .get(name.as_str())
            .cloned()
        {
            Theme::global_mut(cx).apply_config(&cfg);
            Self::apply_user_font_settings(cx);
            cx.refresh_windows();
        }
    }

    pub(super) fn commit_theme_selection(&mut self, _cx: &mut Context<Self>) {
        if let Some(name) = self.theme_list.get(self.theme_cursor) {
            let _ = crate::storage::save_theme_name(name);
        }
    }

    pub(super) fn render_theme_picker_overlay(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut theme_items: Vec<AnyElement> = Vec::new();
        for (i, name) in self.theme_list.iter().enumerate() {
            let is_cursor = i == self.theme_cursor;
            let idx = i;
            theme_items.push(
                Button::new(SharedString::from(format!("tp-{}", i)))
                    .label(name.clone())
                    .ghost()
                    .small()
                    .when(is_cursor, |b| b.primary())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.theme_cursor = idx;
                        this.apply_theme_at_cursor(cx);
                        this.commit_theme_selection(cx);
                        this.close_theme_picker(window, cx);
                        this.original_theme = String::new();
                    }))
                    .into_any_element(),
            );
        }

        div()
            .absolute().top_0().left_0().size_full()
            .flex().items_center().justify_center()
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.close_theme_picker(window, cx);
            }))
            .child(
                div()
                    .id("theme-picker-panel")
                    .w(THEME_PICKER_WIDTH).max_h(THEME_PICKER_MAX_HEIGHT)
                    .bg(cx.theme().background)
                    .border_1().border_color(cx.theme().border)
                    .rounded_lg().shadow_lg().overflow_hidden().occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .track_focus(&self.theme_focus)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        let n = this.theme_list.len();
                        if n == 0 { return; }
                        match event.keystroke.key.as_str() {
                            "up" => {
                                this.theme_cursor = if this.theme_cursor > 0 { this.theme_cursor - 1 } else { n - 1 };
                                this.apply_theme_at_cursor(cx);
                                this.theme_scroll.scroll_to_item(this.theme_cursor);
                                cx.notify();
                            }
                            "down" => {
                                this.theme_cursor = (this.theme_cursor + 1) % n;
                                this.apply_theme_at_cursor(cx);
                                this.theme_scroll.scroll_to_item(this.theme_cursor);
                                cx.notify();
                            }
                            "enter" => { this.commit_theme_selection(cx); this.close_theme_picker(window, cx); this.original_theme = String::new(); }
                            "escape" => { this.restore_original_theme(cx); this.close_theme_picker(window, cx); }
                            _ => {}
                        }
                    }))
                    .child(div().px_3().py_2().border_b_1().border_color(cx.theme().border).child(Input::new(&self.theme_search)))
                    .child(
                        div().id("theme-list").flex_1().overflow_y_scroll()
                            .track_scroll(&self.theme_scroll).max_h(THEME_LIST_MAX_HEIGHT)
                            .flex().flex_col().py_1()
                            .children(theme_items),
                    )
                    .focus_trap("theme-picker-trap", &self.theme_focus),
            )
    }

    pub(super) fn render_tab_picker_overlay(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let visible_rows = self.tab_list.len().max(1).min(10) as f32;
        let list_height = px(6.) + TAB_ROW_HEIGHT * visible_rows;
        let panel_height = px(62.) + list_height;

        div()
            .absolute().top_0().left_0().size_full()
            .flex().items_center().justify_center()
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.close_tab_picker(window, cx);
            }))
            .child(
                div()
                    .id("tab-picker-panel")
                    .w(TAB_PICKER_WIDTH).h(panel_height).max_h(TAB_PICKER_MAX_HEIGHT)
                    .flex().flex_col()
                    .bg(cx.theme().background)
                    .border_1().border_color(cx.theme().border)
                    .rounded_lg().shadow_lg().overflow_hidden().occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        let n = this.tab_list.len();
                        match event.keystroke.key.as_str() {
                            "up" if n > 0 => {
                                this.tab_cursor = if this.tab_cursor > 0 { this.tab_cursor - 1 } else { n - 1 };
                                this.tab_scroll.scroll_to_item(this.tab_cursor, ScrollStrategy::Nearest);
                                cx.notify();
                            }
                            "down" if n > 0 => {
                                this.tab_cursor = (this.tab_cursor + 1) % n;
                                this.tab_scroll.scroll_to_item(this.tab_cursor, ScrollStrategy::Nearest);
                                cx.notify();
                            }
                            "escape" => { this.close_tab_picker(window, cx); }
                            _ => {}
                        }
                    }))
                    .child(div().px_3().py_1().border_b_1().border_color(cx.theme().border).child(Input::new(&self.tab_search)))
                    .child(
                        div().id("tab-list").h(list_height).max_h(px(406.)).min_h_0()
                            .when(self.tab_list.is_empty(), |el| {
                                el.child(div().w_full().px_4().py_3().text_sm().text_color(cx.theme().muted_foreground).child("No open tabs"))
                            })
                            .when(!self.tab_list.is_empty(), |el| {
                                el.child(
                                    uniform_list("tab-picker-list", self.tab_list.len(),
                                        cx.processor(|this: &mut AppView, range: std::ops::Range<usize>, _window, cx| {
                                            this.render_tab_picker_rows(range, cx)
                                        }),
                                    ).track_scroll(&self.tab_scroll).size_full(),
                                )
                            }),
                    )
                    .focus_trap("tab-picker-trap", &self.tab_focus),
            )
    }

    pub(super) fn render_font_picker_overlay(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let visible_rows = self.font_list.len().max(1).min(12) as f32;
        let list_height = px(8.) + FONT_ROW_HEIGHT * visible_rows;
        let panel_height = px(62.) + list_height;
        let font_count = self.font_list.len();

        div()
            .absolute().top_0().left_0().size_full()
            .flex().items_center().justify_center()
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.close_font_picker(window, cx);
            }))
            .child(
                div()
                    .id("font-picker-panel")
                    .w(FONT_PICKER_WIDTH).h(panel_height).max_h(FONT_PICKER_MAX_HEIGHT)
                    .flex().flex_col()
                    .bg(cx.theme().background)
                    .border_1().border_color(cx.theme().border)
                    .rounded_lg().shadow_lg().overflow_hidden().occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        let n = this.font_list.len();
                        match event.keystroke.key.as_str() {
                            "up" if n > 0 => {
                                this.font_cursor = if this.font_cursor > 0 { this.font_cursor - 1 } else { n - 1 };
                                this.font_scroll.scroll_to_item(this.font_cursor, ScrollStrategy::Nearest);
                                cx.notify();
                            }
                            "down" if n > 0 => {
                                this.font_cursor = (this.font_cursor + 1) % n;
                                this.font_scroll.scroll_to_item(this.font_cursor, ScrollStrategy::Nearest);
                                cx.notify();
                            }
                            "enter" if n > 0 => {
                                if let Some(font) = this.font_list.get(this.font_cursor).cloned() {
                                    this.select_font(font, cx);
                                }
                                this.close_font_picker(window, cx);
                            }
                            "escape" => { this.close_font_picker(window, cx); }
                            _ => {}
                        }
                    }))
                    .child(div().px_3().py_2().border_b_1().border_color(cx.theme().border).child(Input::new(&self.font_search)))
                    .child(
                        div().id("font-list").h(list_height).max_h(px(498.)).min_h_0()
                            .when(font_count == 0, |el| {
                                el.child(div().w_full().px_4().py_3().text_sm().text_color(cx.theme().muted_foreground).child("No fonts found"))
                            })
                            .when(font_count > 0, |el| {
                                el.child(
                                    uniform_list("font-picker-list", font_count,
                                        cx.processor(|this: &mut AppView, range: std::ops::Range<usize>, _window, cx| {
                                            let mut rows: Vec<AnyElement> = Vec::new();
                                            for idx in range {
                                                if idx >= this.font_list.len() { continue; }
                                                let font = this.font_list[idx].clone();
                                                let is_cursor   = idx == this.font_cursor;
                                                let is_selected = font == this.selected_font_family;
                                                let bg = if is_cursor { cx.theme().primary.opacity(0.1) } else { gpui_kit::transparent_black() };
                                                let text_color = if is_selected { cx.theme().primary } else { cx.theme().foreground };
                                                let font_click  = font.clone();
                                                let font_style  = font.clone();
                                                rows.push(
                                                    h_flex()
                                                        .id(SharedString::from(format!("font-picker-row-{}", idx)))
                                                        .w_full().h(FONT_ROW_HEIGHT).px_3().gap_2().items_center()
                                                        .bg(bg).cursor_pointer()
                                                        .on_mouse_down(gpui_kit::MouseButton::Left, cx.listener(move |this, _, window, cx| {
                                                            this.font_cursor = idx;
                                                            this.select_font(font_click.clone(), cx);
                                                            this.close_font_picker(window, cx);
                                                        }))
                                                        .child(div().w(px(18.)).child(if is_selected {
                                                             Icon::new(IconName::Check).xsmall().text_color(cx.theme().primary).into_any_element()
                                                        } else {
                                                            div().into_any_element()
                                                        }))
                                                        .child(div().flex_1().overflow_hidden().text_ellipsis().text_sm().text_color(text_color).font_family(font_style).child(font))
                                                        .into_any_element(),
                                                );
                                            }
                                            rows
                                        }),
                                    ).track_scroll(&self.font_scroll).size_full(),
                                )
                            }),
                    )
                    .focus_trap("font-picker-trap", &self.font_focus),
            )
    }
}

fn api_entry_search_text(entry: &ApiPickerEntry) -> String {
    let raw = format!(
        "{} {} {} {} {}",
        entry.context, entry.name, entry.method, entry.url, entry.full_path
    );
    format!("{} {}", raw.to_lowercase(), normalize_search_text(&raw))
}

fn search_matches(text: &str, query: &str) -> bool {
    normalize_search_text(query)
        .split_whitespace()
        .all(|needle| text.contains(needle))
}

fn normalize_search_text(text: &str) -> String {
    text.chars()
        .map(|ch| if ch.is_alphanumeric() { ch.to_ascii_lowercase() } else { ' ' })
        .collect()
}

fn method_color(method: &str, cx: &App) -> Hsla {
    match method {
        "GET"          => cx.theme().success,
        "POST"         => cx.theme().primary,
        "PUT" | "PATCH"=> cx.theme().warning,
        "DELETE"       => cx.theme().danger,
        _              => cx.theme().muted_foreground,
    }
}
