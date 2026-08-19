#![allow(unused_imports)]
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, Sizable as _, Theme, ThemeRegistry,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::PopupMenuItem,
    resizable::{ResizableState, h_resizable, resizable_panel, v_resizable},
    tooltip::Tooltip,
    v_flex,
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

    pub(super) fn render_api_picker_rows(
        &mut self,
        range: std::ops::Range<usize>,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut rows = Vec::new();
        for idx in range {
            let Some(entry) = self.api_list.get(idx).cloned() else {
                continue;
            };
            let is_selected = idx == self.api_cursor;
            let bg = if is_selected {
                cx.theme().primary.opacity(0.1)
            } else {
                gpui::transparent_black()
            };
            let text_color = if is_selected {
                cx.theme().primary
            } else {
                cx.theme().foreground
            };

            rows.push(
                div()
                    .id(SharedString::from(format!("api-picker-row-{}", idx)))
                    .w_full()
                    .h(px(40.))
                    .px_3()
                    .py_1()
                    .bg(bg)
                    .cursor_pointer()
                    .tooltip({
                        let tooltip = entry.full_path.clone();
                        move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx)
                    })
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            this.api_cursor = idx;
                            this.select_api_from_picker(entry.id.clone(), window, cx);
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
                                    .max_w(px(230.))
                                    .min_w(px(110.))
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
                                    .text_color(text_color)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(entry.name.clone()),
                            ),
                    )
                    .into_any_element(),
            );
        }
        rows
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
        self.api_search
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.ensure_api_list_for_query("", cx);

        if let Some(active_id) = &self.app_state.read(cx).active_request_id {
            if let Some(idx) = self
                .api_list
                .iter()
                .position(|entry| &entry.id == active_id)
            {
                self.api_cursor = idx;
                self.api_scroll.scroll_to_item(idx, ScrollStrategy::Nearest);
            }
        }

        self.api_picker_open = true;
        cx.notify();

        let fh = self.api_search.read(cx).focus_handle(cx);
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
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

        h_flex()
            .id("request-tabs")
            .w_full()
            .h(px(34.))
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .overflow_x_scroll()
            .children(tabs.into_iter().enumerate().filter_map(|(idx, id)| {
                let entry = self.api_list.iter().find(|entry| entry.id == id).cloned()?;
                let is_active = active_id.as_deref() == Some(id.as_str());
                let bg = if is_active {
                    cx.theme().muted.opacity(0.14)
                } else {
                    gpui::transparent_black()
                };
                let border = if is_active {
                    cx.theme().primary
                } else {
                    gpui::transparent_black()
                };
                Some(
                    h_flex()
                        .id(SharedString::from(format!("request-tab-{}", idx)))
                        .h_full()
                        .min_w(px(120.))
                        .max_w(px(220.))
                        .px_2()
                        .gap_1()
                        .items_center()
                        .border_b_2()
                        .border_color(border)
                        .bg(bg)
                        .cursor_pointer()
                        .tooltip({
                            let tooltip = entry.full_path.clone();
                            move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx)
                        })
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener({
                                let id = id.clone();
                                move |this, _, window, cx| {
                                    this.select_tab(id.clone(), window, cx);
                                }
                            }),
                        )
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
                                .text_color(if is_active {
                                    cx.theme().foreground
                                } else {
                                    cx.theme().muted_foreground
                                })
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(entry.name.clone()),
                        )
                        .child(
                            Button::new(SharedString::from(format!("close-tab-{}", idx)))
                                .icon(IconName::Close)
                                .ghost()
                                .xsmall()
                                .tooltip("Close tab")
                                .on_click(cx.listener({
                                    let id = id.clone();
                                    move |this, _, window, cx| {
                                        if let Some(pos) =
                                            this.open_tabs.iter().position(|tab| tab == &id)
                                        {
                                            this.open_tabs.remove(pos);
                                            if this.app_state.read(cx).active_request_id.as_deref()
                                                == Some(id.as_str())
                                            {
                                                if let Some(next_id) = this
                                                    .open_tabs
                                                    .get(pos)
                                                    .or_else(|| this.open_tabs.last())
                                                    .cloned()
                                                {
                                                    this.select_tab(next_id, window, cx);
                                                }
                                            }
                                            cx.notify();
                                        }
                                    }
                                })),
                        )
                        .into_any_element(),
                )
            }))
            .into_any_element()
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
                gpui::transparent_black()
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
                        gpui::MouseButton::Left,
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
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect()
}

fn method_color(method: &str, cx: &App) -> Hsla {
    match method {
        "GET" => cx.theme().success,
        "POST" => cx.theme().primary,
        "PUT" | "PATCH" => cx.theme().warning,
        "DELETE" => cx.theme().danger,
        _ => cx.theme().muted_foreground,
    }
}
