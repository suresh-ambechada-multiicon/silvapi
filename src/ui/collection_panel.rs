use std::{collections::HashSet, ops::Range};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, WindowExt as _, h_flex,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem},
    v_flex,
};

use crate::{
    models::CollectionItem,
    state::{AppEvent, AppState},
};

use super::actions::{RenameSelected, SendRequest};

// ── Drag payload ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct DragItem {
    id: String,
    display: String,
}

struct DragPreview(String);
impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .text_sm()
            .text_color(cx.theme().foreground)
            .child(self.0.clone())
    }
}

// ── Flat list item ───────────────────────────────────────────────────────────

#[derive(Clone)]
struct FlatItem {
    id: String,
    display: String,
    method_str: String,
    method_color: Hsla,
    depth: usize,
    is_folder: bool,
    is_collection_root: bool,
    is_expanded: bool,
}

// ── Panel ────────────────────────────────────────────────────────────────────

pub struct CollectionPanel {
    focus_handle: FocusHandle,
    app_state: Entity<AppState>,
    expanded: HashSet<String>,
    selected_id: Option<String>,
    renaming_id: Option<String>,
    rename_input: Entity<InputState>,
    search_input: Entity<InputState>,
    flat_cache: Vec<FlatItem>,
    flat_dirty: bool,
    list_scroll: gpui::UniformListScrollHandle,

    _subs: Vec<Subscription>,
}

impl Focusable for CollectionPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl CollectionPanel {
    pub fn new(app_state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let rename_input = cx.new(|cx| InputState::new(window, cx).placeholder("Rename..."));
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));

        // Expand all collections by default
        let mut expanded = HashSet::new();
        for col in &app_state.read(cx).workspace.collections {
            expanded.insert(col.id.clone());
        }

        let ws_sub = cx.subscribe_in(
            &app_state,
            window,
            |this, app_state, ev: &AppEvent, _, cx| {
                match ev {
                    AppEvent::WorkspaceChanged => {
                        // Auto-expand new collections
                        for col in &app_state.read(cx).workspace.collections {
                            this.expanded.insert(col.id.clone());
                        }
                        this.flat_dirty = true;
                        cx.notify();
                    }
                    AppEvent::RequestSelected => {
                        this.selected_id = app_state.read(cx).active_request_id.clone();
                        cx.notify();
                    }
                    _ => {}
                }
            },
        );

        let rename_sub = cx.subscribe_in(
            &rename_input,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { .. } => this.commit_rename(window, cx),
                InputEvent::Blur => this.commit_rename(window, cx),
                _ => {}
            },
        );

        let search_sub = cx.subscribe_in(
            &search_input,
            window,
            |this, _, ev: &InputEvent, _, cx| match ev {
                InputEvent::Change => {
                    this.flat_dirty = true;
                    cx.notify();
                }
                _ => {}
            },
        );

        Self {
            focus_handle,
            app_state,
            expanded,
            selected_id: None,
            renaming_id: None,
            rename_input,
            search_input,
            flat_cache: Vec::new(),
            flat_dirty: true,
            list_scroll: gpui::UniformListScrollHandle::new(),
            _subs: vec![ws_sub, rename_sub, search_sub],
        }
    }

    fn commit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn flat_items(&mut self, cx: &App) -> Vec<FlatItem> {
        if self.flat_dirty {
            self.flat_cache = self.build_flat(cx);
            self.flat_dirty = false;
        }
        self.flat_cache.clone()
    }

    fn build_flat(&self, cx: &App) -> Vec<FlatItem> {
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

    fn add_items(
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

    fn render_rows(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
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
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(if is_expanded { "▾" } else { "▸" }),
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
                .child(label_content);

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
                let target_id = id6.clone();
                row.context_menu(move |menu, _, _| {
                    let target_for_http = target_id.clone();
                    let target_for_folder = target_id.clone();
                    let app_new_http = app_new_http.clone();
                    let app_new_folder = app_new_folder.clone();
                    let panel_for_rename = panel_for_rename.clone();
                    let rename_id = target_id.clone();
                    let duplicate_id = target_id.clone();
                    let delete_id = target_id.clone();
                    let app_dup = app_dup.clone();
                    let app_del = app_del.clone();

                    menu.item(PopupMenuItem::new("New HTTP").on_click(move |_, _, cx| {
                        let target = Some(target_for_http.as_str());
                        app_new_http.update(cx, |state, cx| {
                            if let Some(new_id) = state.add_request_to_target(target) {
                                state.select_request(&new_id);
                                cx.emit(AppEvent::RequestSelected);
                            }
                            cx.emit(AppEvent::WorkspaceChanged);
                        });
                    }))
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

impl Render for CollectionPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let row_count = self.flat_items(cx).len();

        let app_new_http = self.app_state.clone();
        let app_new_folder = self.app_state.clone();
        let selected_for_new = self.selected_id.clone();

        v_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "up" | "down" => {
                        let flat = this.flat_items(cx);
                        if flat.is_empty() {
                            return;
                        }
                        let current_pos = this
                            .selected_id
                            .as_ref()
                            .and_then(|sid| flat.iter().position(|f| f.id == *sid));
                        let new_pos = match event.keystroke.key.as_str() {
                            "up" => match current_pos {
                                Some(0) => flat.len() - 1,
                                Some(pos) => pos - 1,
                                None => flat.len() - 1,
                            },
                            _ => match current_pos {
                                Some(pos) => (pos + 1) % flat.len(),
                                None => 0,
                            },
                        };
                        let item = &flat[new_pos];
                        let item_id = item.id.clone();
                        let is_folder = item.is_folder;
                        this.selected_id = Some(item_id.clone());
                        if !is_folder {
                            this.app_state.update(cx, |state, cx| {
                                if state.select_request(&item_id) {
                                    cx.emit(AppEvent::RequestSelected);
                                }
                            });
                        }
                        this.list_scroll
                            .scroll_to_item(new_pos, gpui::ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    "space" => {
                        let sid = this.selected_id.clone();
                        if let Some(sid) = sid {
                            let is_folder = this
                                .flat_items(cx)
                                .iter()
                                .any(|f| f.id == sid && f.is_folder);
                            if is_folder {
                                if this.expanded.contains(&sid) {
                                    this.expanded.remove(&sid);
                                } else {
                                    this.expanded.insert(sid.clone());
                                }
                                this.flat_dirty = true;
                                cx.notify();
                            }
                        }
                    }
                    "right" => {
                        let sid = this.selected_id.clone();
                        if let Some(sid) = sid {
                            let is_folder = this
                                .flat_items(cx)
                                .iter()
                                .any(|f| f.id == sid && f.is_folder);
                            if is_folder {
                                this.expanded.insert(sid.clone());
                                this.flat_dirty = true;
                                cx.notify();
                            }
                        }
                    }
                    "left" => {
                        let sid = this.selected_id.clone();
                        if let Some(sid) = sid {
                            let is_folder = this
                                .flat_items(cx)
                                .iter()
                                .any(|f| f.id == sid && f.is_folder);
                            if is_folder {
                                this.expanded.remove(&sid);
                                this.flat_dirty = true;
                                cx.notify();
                            }
                        }
                    }
                    "escape" => {
                        if this.renaming_id.is_some() {
                            this.renaming_id = None;
                            let fh = this.focus_handle.clone();
                            cx.defer_in(window, move |_, window, cx| {
                                fh.focus(window, cx);
                            });
                            cx.notify();
                        }
                    }
                    _ => {}
                }
            }))
            .on_action(cx.listener(|this, _: &RenameSelected, window, cx| {
                if window.has_focused_input(cx) {
                    return;
                }
                if this.renaming_id.is_some() {
                    this.commit_rename(window, cx);
                    return;
                }
                if let Some(id) = this.selected_id.clone() {
                    let name = find_name_by_id(this.app_state.read(cx), &id).unwrap_or_default();
                    this.rename_input
                        .update(cx, |s, cx| s.set_value(name, window, cx));
                    this.renaming_id = Some(id);
                    // focus the rename input
                    let fh = this.rename_input.read(cx).focus_handle(cx);
                    fh.focus(window, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(
                |this, _: &crate::ui::actions::FocusActiveRequest, window, cx| {
                    if let Some(req_id) = this.app_state.read(cx).active_request_id.clone() {
                        // Expand parent folders of this request
                        let state = this.app_state.read(cx);
                        let mut path: Vec<String> = Vec::new();
                        fn find_path(
                            items: &[CollectionItem],
                            target: &str,
                            current_path: &mut Vec<String>,
                        ) -> bool {
                            for item in items {
                                if item.id() == target {
                                    return true;
                                }
                                if let CollectionItem::Folder(f) = item {
                                    current_path.push(f.id.clone());
                                    if find_path(&f.items, target, current_path) {
                                        return true;
                                    }
                                    current_path.pop();
                                }
                            }
                            false
                        }
                        for col in &state.workspace.collections {
                            let mut current_path = vec![col.id.clone()];
                            if find_path(&col.items, &req_id, &mut current_path) {
                                for id in current_path {
                                    this.expanded.insert(id);
                                }
                                break;
                            }
                        }
                        this.selected_id = Some(req_id);
                        this.flat_dirty = true;

                        let fh = this.focus_handle.clone();
                        cx.defer_in(window, move |_, window, cx| {
                            fh.focus(window, cx);
                        });
                        cx.notify();

                        // We must wait for flat_items to be regenerated to find the new position
                        let scroll = this.list_scroll.clone();
                        let target = this.selected_id.clone();
                        cx.defer_in(window, move |this, _, cx| {
                            if let Some(target) = target {
                                if let Some(pos) =
                                    this.flat_items(cx).iter().position(|f| f.id == target)
                                {
                                    scroll.scroll_to_item(pos, gpui::ScrollStrategy::Nearest);
                                }
                            }
                        });
                    }
                },
            ))
            // Header
            .child(
                h_flex()
                    .px_3()
                    .py_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .items_center()
                    .child(Input::new(&self.search_input).flex_1().h(px(28.))),
            )
            // List
            .child(
                div()
                    .id("collection-list-container")
                    .flex_1()
                    .relative()
                    // Background to catch right-clicks on empty space
                    .child(div().absolute().inset_0().context_menu(move |menu, _, _| {
                        let app_new_http = app_new_http.clone();
                        let app_new_folder = app_new_folder.clone();
                        let target_http = selected_for_new.clone();
                        let target_folder = selected_for_new.clone();

                        menu.item(PopupMenuItem::new("New HTTP").on_click(move |_, _, cx| {
                            app_new_http.update(cx, |state, cx| {
                                if let Some(new_id) =
                                    state.add_request_to_target(target_http.as_deref())
                                {
                                    state.select_request(&new_id);
                                    cx.emit(AppEvent::RequestSelected);
                                }
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                        }))
                        .item(PopupMenuItem::new("New Folder").on_click(move |_, _, cx| {
                            app_new_folder.update(cx, |state, cx| {
                                state.add_folder_to_target(target_folder.as_deref());
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                        }))
                    }))
                    // The actual list over it
                    .child(
                        uniform_list(
                            "collection-list-scroll",
                            row_count,
                            cx.processor(move |this: &mut Self, range, _window, cx| {
                                this.render_rows(range, cx)
                            }),
                        )
                        .track_scroll(&self.list_scroll)
                        .size_full(),
                    ),
            )
    }
}

fn start_rename(panel: &Entity<CollectionPanel>, id: &str, window: &mut Window, cx: &mut App) {
    let id = id.to_string();
    panel.update(cx, |this, cx| {
        let name = find_name_by_id(this.app_state.read(cx), &id).unwrap_or_default();
        this.rename_input
            .update(cx, |s, cx| s.set_value(name, window, cx));
        this.renaming_id = Some(id);
        this.selected_id = this.renaming_id.clone();
        let fh = this.rename_input.read(cx).focus_handle(cx);
        fh.focus(window, cx);
        cx.notify();
    });
}

fn format_curl(req: &crate::models::ApiRequest) -> String {
    let mut parts = vec![
        "curl".to_string(),
        "-X".to_string(),
        shell_quote(req.method.as_str()),
        shell_quote(&req.url),
    ];

    for header in &req.headers {
        if header.enabled && !header.key.is_empty() {
            parts.push("-H".to_string());
            parts.push(shell_quote(&format!("{}: {}", header.key, header.value)));
        }
    }

    if !req.body.content.is_empty() {
        parts.push("-d".to_string());
        parts.push(shell_quote(&req.body.content));
    }

    parts.join(" ")
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/:._?=&-%{}".contains(c))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn find_name_by_id(state: &AppState, id: &str) -> Option<String> {
    for col in &state.workspace.collections {
        if col.id == id {
            return Some(col.name.clone());
        }
        if let Some(n) = find_name_in_items(&col.items, id) {
            return Some(n);
        }
    }
    None
}

fn find_name_in_items(items: &[CollectionItem], id: &str) -> Option<String> {
    for item in items {
        match item {
            CollectionItem::Request(r) if r.id == id => return Some(r.name.clone()),
            CollectionItem::Folder(f) if f.id == id => return Some(f.name.clone()),
            CollectionItem::Folder(f) => {
                if let Some(n) = find_name_in_items(&f.items, id) {
                    return Some(n);
                }
            }
            _ => {}
        }
    }
    None
}

fn method_color(method: &str) -> Hsla {
    match method {
        "GET" => rgb(0x4CAF50).into(),
        "POST" => rgb(0x2196F3).into(),
        "PUT" => rgb(0xFF9800).into(),
        "PATCH" => rgb(0x9C27B0).into(),
        "DELETE" => rgb(0xF44336).into(),
        _ => rgb(0x607D8B).into(),
    }
}
