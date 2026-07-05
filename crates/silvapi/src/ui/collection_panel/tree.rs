#![allow(unused_imports)]
use std::{collections::HashSet, ops::Range};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt as _, PopupMenuItem},
    spinner::Spinner,
    v_flex,
};

use silvapi_core::{models::{AuthConfig, AuthType, CollectionItem, Folder, KeyValue, Variable}};
use crate::{state::{AppEvent, AppState}};
use crate::ui::actions::{FocusActiveRequest, RenameSelected, SendRequest};

use super::*;

impl CollectionPanel {
    pub(super) fn commit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.renaming_id.take() {
            let name = self.rename_input.read(cx).value().to_string();
            if !name.is_empty() {
                self.app_state.update(cx, |state, cx| {
                    state.rename_item(&id, name);
                    cx.emit(AppEvent::WorkspaceChanged);
                });
            }

            let fh = self.focus_handle.clone();
            cx.defer_in(window, move |_, window, cx| {
                fh.focus(window, cx);
            });
        }
        cx.notify();
    }

    pub(super) fn flat_items(&mut self, cx: &App) -> Vec<FlatItem> {
        if self.flat_dirty {
            self.flat_cache = self.build_flat(cx);
            self.flat_dirty = false;
        }
        self.flat_cache.clone()
    }

    pub(super) fn build_flat(&self, cx: &App) -> Vec<FlatItem> {
        let state = self.app_state.read(cx);
        let mut result = Vec::new();
        let query = self.search_input.read(cx).value().to_lowercase();

        for col in &state.workspace.collections {
            if !query.is_empty() && !col.matches_query(&query) {
                continue;
            }

            let is_exp = self.expanded.contains(&col.id) || !query.is_empty();
            result.push(FlatItem {
                id: col.id.clone(),
                display: col.name.clone(),
                method_str: String::new(),
                method_color: cx.theme().muted_foreground,
                depth: 0,
                is_folder: true,
                is_collection_root: true,
                is_expanded: is_exp,
            });
            if is_exp {
                self.add_items(&col.items, 1, cx, &query, &mut result);
            }
        }
        result
    }

    pub(super) fn add_items(
        &self,
        items: &[CollectionItem],
        depth: usize,
        cx: &App,
        query: &str,
        result: &mut Vec<FlatItem>,
    ) {
        for item in items {
            if !query.is_empty() && !item.matches_query(query) {
                continue;
            }
            match item {
                CollectionItem::Folder(f) => {
                    let is_exp = self.expanded.contains(&f.id) || !query.is_empty();
                    result.push(FlatItem {
                        id: f.id.clone(),
                        display: f.name.clone(),
                        method_str: String::new(),
                        method_color: cx.theme().muted_foreground,
                        depth,
                        is_folder: true,
                        is_collection_root: false,
                        is_expanded: is_exp,
                    });
                    if is_exp {
                        self.add_items(&f.items, depth + 1, cx, query, result);
                    }
                }
                CollectionItem::Request(r) => {
                    let method = r.method.as_str().to_string();
                    let color = method_color(&method);
                    result.push(FlatItem {
                        id: r.id.clone(),
                        display: r.name.clone(),
                        method_str: method,
                        method_color: color,
                        depth,
                        is_folder: false,
                        is_collection_root: false,
                        is_expanded: false,
                    });
                }
            }
        }
    }

    pub fn reveal_active_request(
        &mut self,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(req_id) = self.app_state.read(cx).active_request_id.clone() else {
            return;
        };

        let path = {
            let state = self.app_state.read(cx);
            let mut found_path = None;
            for col in &state.workspace.collections {
                let mut current_path = vec![col.id.clone()];
                if find_request_path(&col.items, &req_id, &mut current_path) {
                    found_path = Some(current_path);
                    break;
                }
            }
            found_path
        };

        if let Some(path) = path {
            for id in path {
                self.expanded.insert(id);
            }
        }

        self.selected_id = Some(req_id.clone());
        self.flat_dirty = true;

        if focus {
            let fh = self.focus_handle.clone();
            cx.defer_in(window, move |_, window, cx| {
                fh.focus(window, cx);
            });
        }

        cx.notify();

        let scroll = self.list_scroll.clone();
        cx.defer_in(window, move |this, _, cx| {
            if let Some(pos) = this
                .flat_items(cx)
                .iter()
                .position(|item| item.id == req_id)
            {
                scroll.scroll_to_item(pos, gpui::ScrollStrategy::Nearest);
            }
        });
    }

    pub(super) fn render_rows(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let flat = self.flat_items(cx);
        let renaming_id = self.renaming_id.clone();
        let selected_id = self.selected_id.clone();
        let rename_input = self.rename_input.clone();
        let panel_entity = cx.entity();

        let mut rows: Vec<AnyElement> = Vec::new();
        for fi in flat
            .into_iter()
            .skip(range.start)
            .take(range.end.saturating_sub(range.start))
        {
            let id = fi.id.clone();
            let id2 = fi.id.clone();
            let id3 = fi.id.clone();
            let id4 = fi.id.clone();
            let id6 = fi.id.clone();
            let id7 = fi.id.clone();
            let id8 = fi.id.clone();
            let id9 = fi.id.clone();
            let display = fi.display.clone();
            let app_click = self.app_state.clone();
            let app_del = self.app_state.clone();
            let app_drop = self.app_state.clone();
            let app_copy = self.app_state.clone();
            let app_dup = self.app_state.clone();
            let app_move = self.app_state.clone();
            let app_new_http = self.app_state.clone();
            let app_new_folder = self.app_state.clone();
            let app_click_row = app_click.clone();
            let focus_handle = self.focus_handle.clone();
            let is_renaming = renaming_id.as_deref() == Some(&fi.id);
            let is_selected = selected_id.as_deref() == Some(&fi.id);
            let depth = fi.depth;
            let is_folder = fi.is_folder;
            let is_col_root = fi.is_collection_root;
            let is_expanded = fi.is_expanded;
            let method_str = fi.method_str.clone();
            let method_color = fi.method_color;
            let row_status = if is_folder {
                None
            } else {
                request_row_status(self.app_state.read(cx), &fi.id)
            };
            let drag_display = if !method_str.is_empty() {
                format!("{} {}", method_str, display)
            } else {
                display.clone()
            };

            let label_content: AnyElement = if is_renaming {
                h_flex()
                    .flex_1()
                    .flex_basis(px(0.))
                    .min_w_0()
                    .h(px(28.))
                    .w_full()
                    .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                        if ev.keystroke.key.as_str() == "escape" {
                            this.commit_rename(window, cx);
                        }
                    }))
                    .child(
                        Input::new(&rename_input)
                            .bordered(false)
                            .focus_bordered(false)
                            .flex_1()
                            .flex_basis(px(0.))
                            .min_w_0()
                            .h(px(28.))
                            .w_full(),
                    )
                    .into_any_element()
            } else {
                h_flex()
                    .flex_1()
                    .gap_1()
                    .items_center()
                    .overflow_hidden()
                    .when(is_folder, |el| {
                        el.child(
                            Icon::new(if is_expanded {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            })
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                        )
                    })
                    .when(!method_str.is_empty(), |el| {
                        el.child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(method_color)
                                .child(method_str.clone()),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(cx.theme().sidebar_foreground)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(display.clone()),
                    )
                    .into_any_element()
            };

            let target_for_drop = if is_col_root {
                format!("root:{}", id)
            } else if is_folder {
                format!("folder:{}", id)
            } else {
                String::new()
            };
            let bg = if is_selected {
                cx.theme().secondary
            } else {
                cx.theme().sidebar
            };

            let row = div()
                .id(SharedString::from(format!("row-{}", id)))
                // Block clicks (so a right-click on a row only opens the row's own
                // context menu, not the empty-space one beneath) — but allow scroll
                // wheel through, otherwise scrolling over rows can't scroll the list.
                .block_mouse_except_scroll()
                .pl(px(8. + 16. * depth as f32))
                .pr_2()
                .py_0p5()
                .flex()
                .items_center()
                .gap_1()
                .w_full()
                .bg(bg)
                .hover(|s| s.bg(cx.theme().secondary))
                .on_click(move |_, window, cx| {
                    focus_handle.focus(window, cx);
                    if !is_folder {
                        app_click_row.update(cx, |state, cx| {
                            if state.select_request(&id2) {
                                cx.emit(AppEvent::RequestSelected);
                            }
                        });
                    }
                })
                .on_drag(
                    DragItem {
                        id: id3.clone(),
                        display: drag_display.clone(),
                    },
                    move |item, _offset, _window, cx| {
                        let label = item.display.clone();
                        cx.new(|_| DragPreview(label))
                    },
                )
                .child(label_content)
                .when_some(row_status, |row, status| {
                    row.child(render_request_row_status(status, cx))
                });

            let row = if is_folder {
                row.on_drop::<DragItem>(move |drag, _, cx| {
                    let dragged = drag.id.clone();
                    app_drop.update(cx, |state, cx| {
                        state.move_item_to(&dragged, &target_for_drop);
                        cx.emit(AppEvent::WorkspaceChanged);
                    });
                })
                .drag_over::<DragItem>(|style, _, _, cx| style.bg(cx.theme().primary.opacity(0.15)))
            } else {
                row
            };

            let row = if is_folder {
                let panel_entity = cx.entity();
                let toggle_id = fi.id.clone();
                row.on_click(move |_, _, cx| {
                    panel_entity.update(cx, |this, cx| {
                        this.selected_id = Some(toggle_id.clone());
                        if this.expanded.contains(&toggle_id) {
                            this.expanded.remove(&toggle_id);
                        } else {
                            this.expanded.insert(toggle_id.clone());
                        }
                        this.flat_dirty = true;
                        cx.notify();
                    });
                })
            } else {
                row
            };

            let row = if is_folder {
                let panel_for_rename = panel_entity.clone();
                let panel_for_settings = panel_entity.clone();
                let target_id = id6.clone();
                row.context_menu(move |menu, _, _| {
                    let settings_id = target_id.clone();
                    let target_for_http = target_id.clone();
                    let target_for_folder = target_id.clone();
                    let app_new_http = app_new_http.clone();
                    let app_new_folder = app_new_folder.clone();
                    let panel_for_settings = panel_for_settings.clone();
                    let panel_for_rename = panel_for_rename.clone();
                    let rename_id = target_id.clone();
                    let duplicate_id = target_id.clone();
                    let delete_id = target_id.clone();
                    let app_dup = app_dup.clone();
                    let app_del = app_del.clone();

                    menu.item(PopupMenuItem::new("Folder Settings").on_click(
                        move |_, window, cx| {
                            panel_for_settings.update(cx, |this, cx| {
                                this.open_folder_settings(settings_id.clone(), window, cx);
                            });
                        },
                    ))
                    .separator()
                    .item(
                        PopupMenuItem::new("New HTTP").on_click(move |_, window, cx| {
                            let target = Some(target_for_http.as_str());
                            app_new_http.update(cx, |state, cx| {
                                if let Some(new_id) = state.add_request_to_target(target) {
                                    state.select_request(&new_id);
                                    cx.emit(AppEvent::RequestSelected);
                                }
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                            window.dispatch_action(Box::new(FocusActiveRequest), cx);
                        }),
                    )
                    .item(PopupMenuItem::new("New Folder").on_click(move |_, _, cx| {
                        let target = Some(target_for_folder.as_str());
                        app_new_folder.update(cx, |state, cx| {
                            state.add_folder_to_target(target);
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
                    .separator()
                    .item(PopupMenuItem::new("Rename").on_click(move |_, window, cx| {
                        start_rename(&panel_for_rename, &rename_id, window, cx);
                    }))
                    .item(PopupMenuItem::new("Duplicate").on_click(move |_, _, cx| {
                        app_dup.update(cx, |state, cx| {
                            state.duplicate_item(&duplicate_id);
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
                    .item(PopupMenuItem::new("Move").disabled(true))
                    .separator()
                    .item(PopupMenuItem::new("Delete").on_click(move |_, _, cx| {
                        app_del.update(cx, |state, cx| {
                            state.delete_item(&delete_id);
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
                })
            } else {
                let panel_for_rename = panel_entity.clone();
                row.context_menu(move |menu, window, cx| {
                    let send_id = id4.clone();
                    let copy_id = id7.clone();
                    let rename_id = id8.clone();
                    let duplicate_id = id9.clone();
                    let delete_id = id6.clone();
                    let move_id = id.clone();
                    let app_click = app_click.clone();
                    let app_copy = app_copy.clone();
                    let app_dup = app_dup.clone();
                    let app_move = app_move.clone();
                    let app_del = app_del.clone();
                    let panel_for_rename = panel_for_rename.clone();
                    let move_targets = app_move.read(cx).get_folder_options();

                    menu.item(PopupMenuItem::new("Send").on_click(move |_, window, cx| {
                        app_click.update(cx, |state, cx| {
                            if state.select_request(&send_id) {
                                cx.emit(AppEvent::RequestSelected);
                            }
                        });
                        window.dispatch_action(Box::new(SendRequest), cx);
                    }))
                    .item(
                        PopupMenuItem::new("Copy as Curl").on_click(move |_, _, cx| {
                            if let Some(req) = app_copy.read(cx).find_request(&copy_id) {
                                cx.write_to_clipboard(ClipboardItem::new_string(format_curl(&req)));
                            }
                        }),
                    )
                    .item(PopupMenuItem::new("Rename").on_click(move |_, window, cx| {
                        start_rename(&panel_for_rename, &rename_id, window, cx);
                    }))
                    .item(PopupMenuItem::new("Duplicate").on_click(move |_, _, cx| {
                        app_dup.update(cx, |state, cx| {
                            if let Some(new_id) = state.duplicate_item(&duplicate_id) {
                                state.select_request(&new_id);
                                cx.emit(AppEvent::RequestSelected);
                            }
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
                    .submenu("Move", window, cx, move |mut submenu, _, _| {
                        for (target, label) in move_targets.clone() {
                            let app_move = app_move.clone();
                            let move_id = move_id.clone();
                            submenu = submenu.item(PopupMenuItem::new(label).on_click(
                                move |_, _, cx| {
                                    app_move.update(cx, |state, cx| {
                                        state.move_item_to(&move_id, &target);
                                        cx.emit(AppEvent::WorkspaceChanged);
                                    });
                                },
                            ));
                        }
                        submenu
                    })
                    .separator()
                    .item(PopupMenuItem::new("Delete").on_click(move |_, _, cx| {
                        app_del.update(cx, |state, cx| {
                            state.delete_item(&delete_id);
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
                })
            };

            rows.push(row.into_any_element());
        }

        rows
    }
}
