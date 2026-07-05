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
    v_flex,
};

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
use super::{AppView, ShortcutInput, ShortcutSpec};

impl AppView {
    pub(super) fn update_api_list(&mut self, cx: &mut Context<Self>) {
        let query = self.api_search.read(cx).value().to_lowercase();
        self.ensure_api_list_for_query(&query, cx);
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
            if let Some((_, name)) = out.iter_mut().find(|(id, _)| id == active_id) {
                if let Some((prefix, _)) = name.rsplit_once(" / ") {
                    *name = format!("{} / {}", prefix, active_req.name);
                } else {
                    *name = active_req.name.clone();
                }
            }
        }

        if !query.is_empty() {
            out.retain(|(_, name)| name.to_lowercase().contains(&query));
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
            let Some((id, name)) = self.api_list.get(idx).cloned() else {
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
                    .h(px(48.))
                    .px_4()
                    .py_2()
                    .bg(bg)
                    .cursor_pointer()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            this.api_cursor = idx;
                            this.select_api_from_picker(id.clone(), window, cx);
                        }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(text_color)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(name),
                    )
                    .into_any_element(),
            );
        }
        rows
    }

    pub(super) fn collect_reqs(
        items: &[silvapi_core::models::CollectionItem],
        prefix: &str,
        out: &mut Vec<(String, String)>,
    ) {
        for item in items {
            match item {
                silvapi_core::models::CollectionItem::Folder(f) => {
                    let new_pref = format!("{} / {}", prefix, f.name);
                    Self::collect_reqs(&f.items, &new_pref, out);
                }
                silvapi_core::models::CollectionItem::Request(r) => {
                    out.push((r.id.clone(), format!("{} / {}", prefix, r.name)));
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
            if let Some(idx) = self.api_list.iter().position(|(id, _)| id == active_id) {
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

    pub(super) fn select_adjacent_api(&mut self, step: isize, cx: &mut Context<Self>) {
        self.ensure_api_list_for_query("", cx);
        let len = self.api_list.len();
        if len == 0 {
            return;
        }

        let current_id = self.app_state.read(cx).active_request_id.clone();
        let current_idx = current_id
            .as_ref()
            .and_then(|id| {
                if self
                    .api_list
                    .get(self.api_cursor)
                    .map(|(rid, _)| rid == id)
                    .unwrap_or(false)
                {
                    Some(self.api_cursor)
                } else {
                    self.api_list.iter().position(|(rid, _)| rid == id)
                }
            })
            .unwrap_or(0);

        let target_idx = if step.is_negative() {
            (current_idx + len - 1) % len
        } else {
            (current_idx + 1) % len
        };
        let target_id = self.api_list[target_idx].0.clone();
        self.api_cursor = target_idx;

        self.app_state.update(cx, |state, cx| {
            if state.select_request(&target_id) {
                cx.emit(AppEvent::RequestSelected);
            }
        });
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
