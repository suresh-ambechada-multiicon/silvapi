use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Root, Sizable as _, Theme, ThemeRegistry,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{PopupMenu, PopupMenuItem},
    resizable::{ResizableState, h_resizable, resizable_panel, v_resizable},
    v_flex,
};

use crate::state::{AppEvent, AppState};

use super::{
    actions::{
        ApiPicker, CloseSettings, NextApi, OpenSettings, PrevApi, RenameSelected, SendRequest,
        ThemePicker, ToggleMaximize,
    },
    collection_panel::CollectionPanel,
    request_panel::RequestPanel,
    response_panel::ResponsePanel,
    top_bar::TopBar,
};

pub struct AppView {
    focus_handle: FocusHandle,
    app_state: Entity<AppState>,
    top_bar: Entity<TopBar>,
    collection_panel: Entity<CollectionPanel>,
    request_panel: Entity<RequestPanel>,
    response_panel: Entity<ResponsePanel>,
    resize_state: Entity<ResizableState>,
    theme_picker_open: bool,
    theme_focus: FocusHandle,
    theme_cursor: usize,
    theme_list: Vec<String>,
    original_theme: String,
    theme_scroll: ScrollHandle,
    theme_search: Entity<InputState>,
    api_picker_open: bool,
    api_search: Entity<InputState>,
    api_list: Vec<(String, String)>,
    api_list_dirty: bool,
    api_cursor: usize,
    settings_open: bool,
    settings_focus: FocusHandle,
    api_scroll: ScrollHandle,
    save_task: Option<Task<()>>,
    _subs: Vec<Subscription>,
    maximized_panel: MaximizedPanel,
    response_layout: ResponseLayout,
    resize_state_v: Entity<ResizableState>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MaximizedPanel {
    None,
    Collection,
    Request,
    Response,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResponseLayout {
    Row,    // side-by-side (horizontal)
    Column, // stacked (vertical)
}

impl Focusable for AppView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let theme_focus = cx.focus_handle();

        let app_state = cx.new(|_| AppState::new());
        let top_bar = cx.new(|cx| TopBar::new(app_state.clone(), window, cx));
        let collection_panel = cx.new(|cx| CollectionPanel::new(app_state.clone(), window, cx));
        let request_panel = cx.new(|cx| RequestPanel::new(app_state.clone(), window, cx));
        let response_panel = cx.new(|cx| ResponsePanel::new(app_state.clone(), window, cx));
        let resize_state = cx.new(|_| ResizableState::default());
        let storage_sub = cx.subscribe_in(
            &app_state,
            window,
            |this, app_state, ev: &AppEvent, _, cx| {
                if matches!(ev, AppEvent::WorkspaceChanged | AppEvent::SaveNeeded) {
                    let app_state = app_state.clone();
                    this.save_task = Some(cx.spawn(async move |_, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(400))
                            .await;
                        let workspace = app_state.update(cx, |state, _| crate::models::Workspace {
                            id: state.workspace.id.clone(),
                            name: state.workspace.name.clone(),
                            collections: state.workspace.collections.clone(),
                            variables: state.workspace.variables.clone(),
                            response_cache: Default::default(),
                        });
                        let result = cx
                            .background_executor()
                            .spawn(async move { crate::storage::save_workspace(&workspace) })
                            .await;
                        if let Err(err) = result {
                            eprintln!("Failed to save workspace: {}", err);
                        }
                    }));
                }
                if matches!(ev, AppEvent::WorkspaceChanged)
                    || (this.api_picker_open && matches!(ev, AppEvent::SaveNeeded))
                {
                    this.api_list_dirty = true;
                    let selected = this
                        .api_list
                        .get(this.api_cursor)
                        .map(|(id, _)| id.clone())
                        .or_else(|| app_state.read(cx).active_request_id.clone());
                    this.rebuild_api_list(cx);
                    if let Some(selected) = selected {
                        if let Some(idx) = this.api_list.iter().position(|(id, _)| *id == selected)
                        {
                            this.api_cursor = idx;
                            this.api_scroll.scroll_to_item(idx);
                        }
                    }
                }
                if matches!(ev, AppEvent::ToggleLayout) {
                    this.response_layout = match this.response_layout {
                        ResponseLayout::Row => ResponseLayout::Column,
                        ResponseLayout::Column => ResponseLayout::Row,
                    };
                    cx.notify();
                }
            },
        );

        let theme_search = cx.new(|cx| InputState::new(window, cx).placeholder("Select Theme..."));

        let theme_search_sub = cx.subscribe_in(
            &theme_search,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change => this.update_theme_list(cx),
                InputEvent::PressEnter { .. } => {
                    this.commit_theme_selection(cx);
                    this.close_theme_picker(window, cx);
                    this.original_theme = String::new();
                }
                _ => {}
            },
        );

        let api_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search API..."));

        let api_search_sub = cx.subscribe_in(
            &api_search,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change => this.update_api_list(cx),
                InputEvent::PressEnter { .. } => {
                    this.close_api_picker(window, cx);
                    if let Some((id, _)) = this.api_list.get(this.api_cursor) {
                        let req_id = id.clone();
                        this.app_state.update(cx, |state, cx| {
                            if state.select_request(&req_id) {
                                cx.emit(AppEvent::RequestSelected);
                            }
                        });
                    }
                    cx.notify();
                }
                _ => {}
            },
        );

        cx.bind_keys([
            KeyBinding::new("ctrl-k", ThemePicker, None),
            KeyBinding::new("ctrl-p", ApiPicker, None),
            KeyBinding::new("ctrl-tab", NextApi, None),
            KeyBinding::new("shift-ctrl-tab", PrevApi, None),
            KeyBinding::new("ctrl-enter", SendRequest, None),
            KeyBinding::new("enter", RenameSelected, None),
            KeyBinding::new("shift-escape", ToggleMaximize, None),
            KeyBinding::new("ctrl-,", OpenSettings, None),
        ]);

        let fh = focus_handle.clone();
        cx.defer_in(window, move |_this, window, cx| {
            fh.focus(window, cx);
        });

        Self {
            focus_handle,
            app_state,
            top_bar,
            collection_panel,
            request_panel,
            response_panel,
            resize_state,
            theme_picker_open: false,
            theme_focus,
            theme_cursor: 0,
            theme_list: Vec::new(),
            original_theme: String::new(),
            theme_scroll: ScrollHandle::new(),
            theme_search,
            api_picker_open: false,
            api_search,
            api_list: Vec::new(),
            api_list_dirty: true,
            api_cursor: 0,
            settings_open: false,
            settings_focus: cx.focus_handle(),
            api_scroll: ScrollHandle::new(),
            save_task: None,
            _subs: vec![storage_sub, theme_search_sub, api_search_sub],
            maximized_panel: MaximizedPanel::None,
            response_layout: ResponseLayout::Row,
            resize_state_v: cx.new(|_| ResizableState::default()),
        }
    }

    fn update_api_list(&mut self, cx: &mut Context<Self>) {
        self.rebuild_api_list(cx);
        cx.notify();
    }

    fn rebuild_api_list(&mut self, cx: &App) {
        let query = self.api_search.read(cx).value().to_lowercase();

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
        self.api_list_dirty = false;
        self.api_cursor = 0;
        if !self.api_list.is_empty() {
            self.api_scroll.scroll_to_item(0);
        }
    }

    fn collect_reqs(
        items: &[crate::models::CollectionItem],
        prefix: &str,
        out: &mut Vec<(String, String)>,
    ) {
        for item in items {
            match item {
                crate::models::CollectionItem::Folder(f) => {
                    let new_pref = format!("{} / {}", prefix, f.name);
                    Self::collect_reqs(&f.items, &new_pref, out);
                }
                crate::models::CollectionItem::Request(r) => {
                    out.push((r.id.clone(), format!("{} / {}", prefix, r.name)));
                }
            }
        }
    }

    fn close_api_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.api_picker_open = false;
        let fh = self.focus_handle.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    fn close_theme_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.theme_picker_open = false;
        let fh = self.focus_handle.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    fn open_api_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.api_search
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.update_api_list(cx);

        if let Some(active_id) = &self.app_state.read(cx).active_request_id {
            if let Some(idx) = self.api_list.iter().position(|(id, _)| id == active_id) {
                self.api_cursor = idx;
                self.api_scroll.scroll_to_item(idx);
            }
        }

        self.api_picker_open = true;
        cx.notify();

        let fh = self.api_search.read(cx).focus_handle(cx);
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = true;
        let fh = self.settings_focus.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = false;
        let fh = self.focus_handle.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    fn update_theme_list(&mut self, cx: &mut Context<Self>) {
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

        println!(
            "update_theme_list: current={}, found pos={}, theme_list_len={}",
            current,
            self.theme_cursor,
            self.theme_list.len()
        );

        if !self.theme_list.is_empty() {
            self.theme_scroll.scroll_to_item(self.theme_cursor);
        }

        self.apply_theme_at_cursor(cx);
        cx.notify();
    }

    fn open_theme_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn apply_theme_at_cursor(&mut self, cx: &mut Context<Self>) {
        if let Some(name) = self.theme_list.get(self.theme_cursor) {
            let name = name.clone();
            if let Some(cfg) = ThemeRegistry::global(cx)
                .themes()
                .get(name.as_str())
                .cloned()
            {
                Theme::global_mut(cx).apply_config(&cfg);
                cx.refresh_windows();
            }
        }
    }

    fn restore_original_theme(&mut self, cx: &mut Context<Self>) {
        let name = self.original_theme.clone();
        if let Some(cfg) = ThemeRegistry::global(cx)
            .themes()
            .get(name.as_str())
            .cloned()
        {
            Theme::global_mut(cx).apply_config(&cfg);
            cx.refresh_windows();
        }
    }

    fn commit_theme_selection(&mut self, _cx: &mut Context<Self>) {
        if let Some(name) = self.theme_list.get(self.theme_cursor) {
            if let Err(err) = crate::storage::save_theme_name(name) {
                eprintln!("Failed to save theme: {}", err);
            }
        }
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        let theme_picker_open = self.theme_picker_open;

        // Pre-build theme list items for the overlay
        let mut theme_items: Vec<AnyElement> = Vec::new();
        if theme_picker_open {
            for (i, name) in self.theme_list.iter().enumerate() {
                let is_cursor = i == self.theme_cursor;
                let n = name.clone();
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
                let _ = n;
            }
        }

        div()
            .relative()
            .size_full()
            .bg(cx.theme().background)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &ThemePicker, window, cx| {
                if this.theme_picker_open {
                    this.restore_original_theme(cx);
                    this.close_theme_picker(window, cx);
                } else {
                    this.open_theme_picker(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                this.open_settings(window, cx);
            }))
            .on_action(cx.listener(|this, _: &CloseSettings, window, cx| {
                this.close_settings(window, cx);
            }))
            .on_action(cx.listener(|this, _: &NextApi, _window, cx| {
                let mut out = Vec::new();
                for col in &this.app_state.read(cx).workspace.collections {
                    Self::collect_reqs(&col.items, &col.name, &mut out);
                }
                if !out.is_empty() {
                    let current_id = this.app_state.read(cx).active_request_id.clone();
                    let current_idx = current_id.and_then(|id| out.iter().position(|(rid, _)| *rid == id)).unwrap_or(0);
                    let new_idx = (current_idx + 1) % out.len();
                    let target_id = out[new_idx].0.clone();
                    this.app_state.update(cx, |st, cx| {
                        if st.select_request(&target_id) {
                            cx.emit(AppEvent::RequestSelected);
                        }
                    });
                }
            }))
            .on_action(cx.listener(|this, _: &PrevApi, _window, cx| {
                let mut out = Vec::new();
                for col in &this.app_state.read(cx).workspace.collections {
                    Self::collect_reqs(&col.items, &col.name, &mut out);
                }
                if !out.is_empty() {
                    let current_id = this.app_state.read(cx).active_request_id.clone();
                    let current_idx = current_id.and_then(|id| out.iter().position(|(rid, _)| *rid == id)).unwrap_or(0);
                    let new_idx = if current_idx == 0 { out.len() - 1 } else { current_idx - 1 };
                    let target_id = out[new_idx].0.clone();
                    this.app_state.update(cx, |st, cx| {
                        if st.select_request(&target_id) {
                            cx.emit(AppEvent::RequestSelected);
                        }
                    });
                }
            }))
            .on_action(cx.listener(|this, _: &ApiPicker, window, cx| {
                if this.api_picker_open {
                    this.close_api_picker(window, cx);
                } else {
                    this.open_api_picker(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &SendRequest, window, cx| {
                this.request_panel.update(cx, |panel, cx| {
                    panel.send_request(window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleMaximize, window, cx| {
                if this.maximized_panel != MaximizedPanel::None {
                    this.maximized_panel = MaximizedPanel::None;
                } else {
                    let c_focus = this.collection_panel.read(cx).focus_handle(cx);
                    let req_focus = this.request_panel.read(cx).focus_handle(cx);
                    let res_focus = this.response_panel.read(cx).focus_handle(cx);
                    let cf = c_focus.contains_focused(window, cx);
                    let rf = req_focus.contains_focused(window, cx);
                    let resf = res_focus.contains_focused(window, cx);
                    let global_focused = window.focused(cx).is_some();
                    println!("ToggleMaximize: global_focused={} Collection={} Request={} Response={}", global_focused, cf, rf, resf);

                    if cf {
                        this.maximized_panel = MaximizedPanel::Collection;
                    } else if rf {
                        this.maximized_panel = MaximizedPanel::Request;
                    } else if resf {
                        this.maximized_panel = MaximizedPanel::Response;
                    } else {
                        // Check if any specific input in Request is focused
                        let mut req_input_focused = false;
                        this.request_panel.update(cx, |p, cx| {
                            req_input_focused = p.has_focus(window, cx);
                        });

                        let mut res_input_focused = false;
                        this.response_panel.update(cx, |p, cx| {
                            res_input_focused = p.has_focus(window, cx);
                        });

                        if req_input_focused {
                            this.maximized_panel = MaximizedPanel::Request;
                        } else if res_input_focused {
                            this.maximized_panel = MaximizedPanel::Response;
                        } else {
                            // default fallback
                            this.maximized_panel = MaximizedPanel::Response;
                        }
                    }
                }
                cx.notify();
            }))
            .child(
                v_flex().size_full().child(self.top_bar.clone()).child(
                    div().flex_1().overflow_hidden().child(
                        match self.maximized_panel {
                            MaximizedPanel::Collection => self.collection_panel.clone().into_any_element(),
                            MaximizedPanel::Request => self.request_panel.clone().into_any_element(),
                            MaximizedPanel::Response => self.response_panel.clone().into_any_element(),
                            MaximizedPanel::None => {
                                match self.response_layout {
                                    ResponseLayout::Row => {
                                        h_resizable("main-panels")
                                        .with_state(&self.resize_state)
                                        .child(
                                            resizable_panel()
                                                .size(px(260.))
                                                .size_range(px(180.)..px(400.))
                                                .child(self.collection_panel.clone()),
                                        )
                                        .child(
                                            resizable_panel()
                                                .size(px(400.))
                                                .size_range(px(300.)..px(800.))
                                                .child(self.request_panel.clone()),
                                        )
                                        .child(
                                            resizable_panel()
                                                .child(self.response_panel.clone()),
                                        )
                                        .into_any_element()
                                    }
                                    ResponseLayout::Column => {
                                        h_resizable("main-panels-col")
                                        .with_state(&self.resize_state)
                                        .child(
                                            resizable_panel()
                                                .size(px(260.))
                                                .size_range(px(180.)..px(400.))
                                                .child(self.collection_panel.clone()),
                                        )
                                        .child(
                                            resizable_panel()
                                                .child(
                                                    v_resizable("req-resp-col")
                                                        .with_state(&self.resize_state_v)
                                                        .child(
                                                            resizable_panel()
                                                                .size(px(350.))
                                                                .size_range(px(200.)..px(600.))
                                                                .child(self.request_panel.clone()),
                                                        )
                                                        .child(
                                                            resizable_panel()
                                                                .child(self.response_panel.clone()),
                                                        )
                                                ),
                                        )
                                        .into_any_element()
                                    }
                                }
                            }
                        }
                    ),
                ),
            )
            // ── Centered theme picker overlay ──────────────────────────────
            .when(theme_picker_open, |el| {
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .id("theme-picker-panel")
                                .w(px(300.))
                                .max_h(px(520.))
                                .bg(cx.theme().background)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_lg()
                                .shadow_lg()
                                .overflow_hidden()
                                .track_focus(&self.theme_focus)
                                .on_key_down(cx.listener(
                                    |this, event: &KeyDownEvent, window, cx| {
                                        let n = this.theme_list.len();
                                        if n == 0 {
                                            return;
                                        }
                                        match event.keystroke.key.as_str() {
                                            "up" => {
                                                if this.theme_cursor > 0 {
                                                    this.theme_cursor -= 1;
                                                } else {
                                                    this.theme_cursor = n - 1;
                                                }
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
                                            "enter" => {
                                                this.commit_theme_selection(cx);
                                                this.close_theme_picker(window, cx);
                                                this.original_theme = String::new();
                                            }
                                            "escape" => {
                                                this.restore_original_theme(cx);
                                                this.close_theme_picker(window, cx);
                                            }
                                            _ => {}
                                        }
                                    },
                                ))
                                .child(
                                    div()
                                        .px_3()
                                        .py_2()
                                        .border_b_1()
                                        .border_color(cx.theme().border)
                                        .child(Input::new(&self.theme_search)),
                                )
                                .child(
                                    div()
                                        .id("theme-list")
                                        .flex_1()
                                        .overflow_y_scroll()
                                        .track_scroll(&self.theme_scroll)
                                        .max_h(px(460.))
                                        .flex()
                                        .flex_col()
                                        .py_1()
                                        .children(theme_items),
                                ),
                        ),
                )
            })
            // ── Centered API picker overlay ──────────────────────────────
            .when(self.api_picker_open, |el| {
                if self.api_list_dirty {
                    self.rebuild_api_list(cx);
                }
                let visible_rows = self.api_list.len().max(1).min(10) as f32;
                let list_height = 8. + visible_rows * 48.;
                let panel_height = 62. + list_height;
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .id("api-picker-panel")
                                .w(px(500.))
                                .h(px(panel_height))
                                .max_h(px(520.))
                                .flex()
                                .flex_col()
                                .bg(cx.theme().background)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_lg()
                                .shadow_lg()
                                .overflow_hidden()
                                .on_key_down(cx.listener(
                                    |this, event: &KeyDownEvent, window, cx| {
                                        let n = this.api_list.len();
                                        if n == 0 {
                                            return;
                                        }
                                        match event.keystroke.key.as_str() {
                                            "up" => {
                                                if this.api_cursor > 0 {
                                                    this.api_cursor -= 1;
                                                } else {
                                                    this.api_cursor = n - 1;
                                                }
                                                this.api_scroll.scroll_to_item(this.api_cursor);
                                                cx.notify();
                                            }
                                            "down" => {
                                                this.api_cursor = (this.api_cursor + 1) % n;
                                                this.api_scroll.scroll_to_item(this.api_cursor);
                                                cx.notify();
                                            }
                                            "escape" => {
                                                this.close_api_picker(window, cx);
                                            }
                                            _ => {}
                                        }
                                    },
                                ))
                                .child(
                                    div()
                                        .px_3()
                                        .py_2()
                                        .border_b_1()
                                        .border_color(cx.theme().border)
                                        .child(Input::new(&self.api_search)),
                                )
                                .child(
                                    div()
                                        .id("api-list")
                                        .h(px(list_height))
                                        .max_h(px(458.))
                                        .min_h_0()
                                        .overflow_y_scroll()
                                        .track_scroll(&self.api_scroll)
                                        .py_1()
                                        .when(self.api_list.is_empty(), |el| {
                                            el.child(
                                                div()
                                                    .w_full()
                                                    .px_4()
                                                    .py_3()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("No APIs found"),
                                            )
                                        })
                                        .when(!self.api_list.is_empty(), |el| {
                                            el.children(self.api_list.iter().enumerate().map(
                                                |(idx, (id, name))| {
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
                                                    let req_id = id.clone();
                                                    div()
                                                        .id(SharedString::from(format!(
                                                            "api-picker-row-{}",
                                                            idx
                                                        )))
                                                        .w_full()
                                                        .px_4()
                                                        .py_2()
                                                        .bg(bg)
                                                        .cursor_pointer()
                                                        .on_mouse_down(
                                                            gpui::MouseButton::Left,
                                                            cx.listener(
                                                                move |this, _, window, cx| {
                                                                    this.close_api_picker(
                                                                        window, cx,
                                                                    );
                                                                    let req_id = req_id.clone();
                                                                    this.app_state.update(
                                                                        cx,
                                                                        |state, cx| {
                                                                            if state
                                                                                .select_request(
                                                                                    &req_id,
                                                                                )
                                                                            {
                                                                                cx.emit(AppEvent::RequestSelected);
                                                                            }
                                                                        },
                                                                    );
                                                                    cx.notify();
                                                                },
                                                            ),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .text_color(text_color)
                                                                .child(name.clone()),
                                                        )
                                                },
                                            ))
                                        }),
                                ),
                        ),
                )
            })
            .when(self.settings_open, |el| {
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .id("settings-panel")
                                .w(px(600.))
                                .h(px(500.))
                                .bg(cx.theme().background)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_lg()
                                .shadow_lg()
                                .overflow_hidden()
                                .track_focus(&self.settings_focus)
                                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                    if event.keystroke.key == "escape" {
                                        this.close_settings(window, cx);
                                    }
                                }))
                                .child(
                                    v_flex()
                                        .size_full()
                                        .child(
                                            h_flex()
                                                .px_4()
                                                .py_3()
                                                .border_b_1()
                                                .border_color(cx.theme().border)
                                                .justify_between()
                                                .child(div().font_weight(gpui::FontWeight::BOLD).child("Settings"))
                                                .child(
                                                    div()
                                                        .cursor_pointer()
                                                        .child("✕")
                                                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| {
                                                            this.close_settings(window, cx);
                                                        }))
                                                )
                                        )
                                        .child(
                                            div().id("settings-scroll").flex_1().overflow_y_scroll().p_4()
                                                .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).child("Keybindings"))
                                                .child(
                                                    v_flex().gap_2().mt_2()
                                                        .child(h_flex().justify_between().child("Open Theme Picker").child("Ctrl+K"))
                                                        .child(h_flex().justify_between().child("Open API Picker").child("Ctrl+P"))
                                                        .child(h_flex().justify_between().child("Open Settings").child("Ctrl+,"))
                                                        .child(h_flex().justify_between().child("Next API").child("Ctrl+Tab"))
                                                        .child(h_flex().justify_between().child("Previous API").child("Ctrl+Shift+Tab"))
                                                        .child(h_flex().justify_between().child("Send Request").child("Ctrl+Enter"))
                                                        .child(h_flex().justify_between().child("Rename Selected").child("Enter"))
                                                        .child(h_flex().justify_between().child("Toggle Maximize Panel").child("Shift+Escape"))
                                                        .child(h_flex().justify_between().child("Expand All Folders").child("Ctrl+Shift+="))
                                                        .child(h_flex().justify_between().child("Collapse All Folders").child("Ctrl+Shift+-"))
                                                )
                                                .child(div().mt_6().text_lg().font_weight(gpui::FontWeight::BOLD).child("Appearance"))
                                                .child(
                                                    v_flex().gap_4().mt_2()
                                                        .child(
                                                            h_flex().justify_between().child("Theme").child({
                                                                let mut themes = self.theme_list.clone();
                                                                if themes.is_empty() {
                                                                    // Fallback if not loaded
                                                                    themes = ThemeRegistry::global(cx)
                                                                        .themes()
                                                                        .keys()
                                                                        .map(|s| s.to_string())
                                                                        .collect();
                                                                    themes.sort();
                                                                }
                                                                let current = Theme::global(cx).theme_name().to_string();
                                                                DropdownButton::new("settings-theme-picker")
                                                                    .button(
                                                                        Button::new("theme-btn")
                                                                            .label(current.clone())
                                                                            .outline()
                                                                    )
                                                                    .dropdown_menu(move |mut menu, _, _| {
                                                                        menu = menu.scrollable(true);
                                                                        for name in themes.clone() {
                                                                            let is_current = name == current;
                                                                            let name_clone = name.clone();
                                                                            menu = menu.item(
                                                                                PopupMenuItem::new(name.clone())
                                                                                    .checked(is_current)
                                                                                    .on_click(move |_, _, cx: &mut App| {
                                                                                        if let Some(cfg) = ThemeRegistry::global(cx).themes().get(name_clone.as_str()).cloned() {
                                                                                            Theme::global_mut(cx).apply_config(&cfg);
                                                                                            cx.refresh_windows();
                                                                                            if let Err(err) = crate::storage::save_theme_name(&name_clone) {
                                                                                                eprintln!("Failed to save theme: {}", err);
                                                                                            }
                                                                                        }
                                                                                    })
                                                                            );
                                                                        }
                                                                        menu
                                                                    })
                                                            })
                                                        )
                                                )
                                        )
                                )
                        )
                )
            })
            .children(dialog_layer)
            .children(notification_layer)
    }
}
