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

use super::{
    actions::{
        ApiPicker, CloseSettings, FocusActiveRequest, FocusCollectionPanel, FocusRequestPanel,
        FocusResponsePanel, FocusUrl, NextApi, OpenSettings, PrevApi, RenameSelected, SendRequest,
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
    row_resize_state: Entity<ResizableState>,
    column_resize_state: Entity<ResizableState>,
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
    api_list_query: String,
    api_list_dirty: bool,
    api_cursor: usize,
    settings_open: bool,
    settings_focus: FocusHandle,
    shortcut_inputs: Vec<ShortcutInput>,
    available_fonts: Vec<String>,
    selected_font_family: String,
    font_picker_open: bool,
    font_search: Entity<InputState>,
    font_list: Vec<String>,
    font_cursor: usize,
    font_scroll: gpui::UniformListScrollHandle,
    font_size_input: Entity<InputState>,
    api_scroll: gpui::UniformListScrollHandle,
    save_task: Option<Task<()>>,
    _subs: Vec<Subscription>,
    maximized_panel: MaximizedPanel,
    response_layout: ResponseLayout,
    resize_state_v: Entity<ResizableState>,
}

#[derive(Clone)]
struct ShortcutSpec {
    id: &'static str,
    label: &'static str,
    default_key: &'static str,
}

struct ShortcutInput {
    spec: ShortcutSpec,
    input: Entity<InputState>,
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
    fn shortcut_specs() -> Vec<ShortcutSpec> {
        vec![
            ShortcutSpec {
                id: "theme_picker",
                label: "Open Theme Picker",
                default_key: "ctrl-k",
            },
            ShortcutSpec {
                id: "api_picker",
                label: "Open API Picker",
                default_key: "ctrl-p",
            },
            ShortcutSpec {
                id: "next_api",
                label: "Next API",
                default_key: "ctrl-tab",
            },
            ShortcutSpec {
                id: "prev_api",
                label: "Previous API",
                default_key: "shift-ctrl-tab",
            },
            ShortcutSpec {
                id: "send_request",
                label: "Send Request",
                default_key: "ctrl-enter",
            },
            ShortcutSpec {
                id: "rename_selected",
                label: "Rename Selected",
                default_key: "enter",
            },
            ShortcutSpec {
                id: "toggle_maximize",
                label: "Toggle Maximize Panel",
                default_key: "shift-escape",
            },
            ShortcutSpec {
                id: "open_settings",
                label: "Open Settings",
                default_key: "ctrl-,",
            },
            ShortcutSpec {
                id: "focus_active_request",
                label: "Reveal Active Request",
                default_key: "ctrl-shift-f",
            },
            ShortcutSpec {
                id: "focus_url",
                label: "Focus URL",
                default_key: "ctrl-l",
            },
            ShortcutSpec {
                id: "focus_collection_panel",
                label: "Focus Request List",
                default_key: "alt-1",
            },
            ShortcutSpec {
                id: "focus_request_panel",
                label: "Focus Request Editor",
                default_key: "alt-2",
            },
            ShortcutSpec {
                id: "focus_response_panel",
                label: "Focus Response Panel",
                default_key: "alt-3",
            },
        ]
    }

    fn system_font_families(cx: &App) -> Vec<String> {
        // Enumerate via gpui's own text system rather than font-kit: these are
        // exactly the family names `.font_family()` can resolve. On Linux the
        // two agree (fontconfig), but on Windows gpui's DirectWrite backend has
        // its own name set — font-kit names it can't match silently fall back,
        // so a picked font would appear to do nothing.
        let mut fonts = cx.text_system().all_font_names();
        fonts.retain(|font| !font.trim().is_empty() && font != ".SystemUIFont");
        fonts.sort_by_key(|font| font.to_lowercase());
        fonts.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

        let mut out = vec![".SystemUIFont".to_string()];
        for preferred in ["Inter", "Arial", "DejaVu Sans", "Noto Sans", "Segoe UI"] {
            if fonts.iter().any(|font| font == preferred) {
                out.push(preferred.to_string());
            }
        }
        for font in fonts {
            if !out
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&font))
            {
                out.push(font);
            }
        }
        out
    }

    fn shortcut_key(spec: &ShortcutSpec) -> String {
        let key = format!("shortcut.{}", spec.id);
        crate::storage::load_setting(&key)
            .ok()
            .flatten()
            .filter(|saved| Keystroke::parse(saved).is_ok())
            .unwrap_or_else(|| spec.default_key.to_string())
    }

    fn bind_shortcut(id: &str, key: &str, cx: &mut Context<Self>) {
        if Keystroke::parse(key).is_err() {
            return;
        }
        match id {
            "theme_picker" => cx.bind_keys([KeyBinding::new(key, ThemePicker, None)]),
            "api_picker" => cx.bind_keys([KeyBinding::new(key, ApiPicker, None)]),
            "next_api" => cx.bind_keys([KeyBinding::new(key, NextApi, None)]),
            "prev_api" => cx.bind_keys([KeyBinding::new(key, PrevApi, None)]),
            "send_request" => cx.bind_keys([KeyBinding::new(key, SendRequest, None)]),
            "rename_selected" => cx.bind_keys([KeyBinding::new(key, RenameSelected, None)]),
            "toggle_maximize" => cx.bind_keys([KeyBinding::new(key, ToggleMaximize, None)]),
            "open_settings" => cx.bind_keys([KeyBinding::new(key, OpenSettings, None)]),
            "focus_active_request" => {
                cx.bind_keys([KeyBinding::new(key, FocusActiveRequest, None)])
            }
            "focus_url" => cx.bind_keys([KeyBinding::new(key, FocusUrl, None)]),
            "focus_collection_panel" => {
                cx.bind_keys([KeyBinding::new(key, FocusCollectionPanel, None)])
            }
            "focus_request_panel" => cx.bind_keys([KeyBinding::new(key, FocusRequestPanel, None)]),
            "focus_response_panel" => {
                cx.bind_keys([KeyBinding::new(key, FocusResponsePanel, None)])
            }
            _ => {}
        }
    }

    fn bind_configured_shortcuts(cx: &mut Context<Self>) {
        for spec in Self::shortcut_specs() {
            let key = Self::shortcut_key(&spec);
            Self::bind_shortcut(spec.id, &key, cx);
        }
    }

    fn save_shortcut(spec: &ShortcutSpec, input: &Entity<InputState>, cx: &mut Context<Self>) {
        let value = input.read(cx).value().trim().to_lowercase();
        if value.is_empty() || Keystroke::parse(&value).is_err() {
            return;
        }
        let key = format!("shortcut.{}", spec.id);
        if crate::storage::save_setting(&key, &value).is_err() {
            return;
        }
        Self::bind_shortcut(spec.id, &value, cx);
    }

    fn configured_font_family(cx: &mut Context<Self>) -> SharedString {
        crate::storage::load_setting("ui.font_family")
            .ok()
            .flatten()
            .filter(|font| !font.trim().is_empty())
            .map(SharedString::from)
            .unwrap_or_else(|| cx.theme().font_family.clone())
    }

    fn configured_font_size(cx: &mut Context<Self>) -> Pixels {
        crate::storage::load_setting("ui.font_size")
            .ok()
            .flatten()
            .and_then(|size| size.parse::<f32>().ok())
            .filter(|size| (10.0..=24.0).contains(size))
            .map(px)
            .unwrap_or_else(|| cx.theme().font_size)
    }

    fn apply_user_font_settings(cx: &mut Context<Self>) {
        let font_family = Self::configured_font_family(cx);
        let font_size = Self::configured_font_size(cx);
        let theme = Theme::global_mut(cx);
        theme.font_family = font_family;
        theme.font_size = font_size;
        cx.refresh_windows();
    }

    fn apply_user_font_settings_app(cx: &mut App) {
        if let Some(font_family) = crate::storage::load_setting("ui.font_family")
            .ok()
            .flatten()
            .filter(|font| !font.trim().is_empty())
        {
            Theme::global_mut(cx).font_family = SharedString::from(font_family);
        }
        if let Some(font_size) = crate::storage::load_setting("ui.font_size")
            .ok()
            .flatten()
            .and_then(|size| size.parse::<f32>().ok())
            .filter(|size| (10.0..=24.0).contains(size))
        {
            Theme::global_mut(cx).font_size = px(font_size);
        }
        cx.refresh_windows();
    }

    fn save_font_settings(family: &str, size_input: &Entity<InputState>, cx: &mut Context<Self>) {
        let family = family.trim().to_string();
        if !family.is_empty() {
            let _ = crate::storage::save_setting("ui.font_family", &family);
        }

        let size_text = size_input.read(cx).value().trim().to_string();
        if let Ok(size) = size_text.parse::<f32>() {
            if (10.0..=24.0).contains(&size) {
                let _ = crate::storage::save_setting("ui.font_size", &size.to_string());
            }
        }

        Self::apply_user_font_settings(cx);
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let theme_focus = cx.focus_handle();

        let app_state = cx.new(|_| AppState::new());
        let top_bar = cx.new(|cx| TopBar::new(app_state.clone(), window, cx));
        let collection_panel = cx.new(|cx| CollectionPanel::new(app_state.clone(), window, cx));
        let request_panel = cx.new(|cx| RequestPanel::new(app_state.clone(), window, cx));
        let response_panel = cx.new(|cx| ResponsePanel::new(app_state.clone(), window, cx));
        let row_resize_state = cx.new(|_| ResizableState::default());
        let column_resize_state = cx.new(|_| ResizableState::default());
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
                        let _ = result;
                    }));
                }
                if matches!(ev, AppEvent::WorkspaceChanged) {
                    this.api_list_dirty = true;
                    if this.api_picker_open {
                        let selected = this
                            .api_list
                            .get(this.api_cursor)
                            .map(|(id, _)| id.clone())
                            .or_else(|| app_state.read(cx).active_request_id.clone());
                        this.rebuild_api_list(cx);
                        if let Some(selected) = selected {
                            if let Some(idx) =
                                this.api_list.iter().position(|(id, _)| *id == selected)
                            {
                                this.api_cursor = idx;
                                this.api_scroll.scroll_to_item(idx, ScrollStrategy::Nearest);
                            }
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
                    if let Some((id, _)) = this.api_list.get(this.api_cursor) {
                        this.select_api_from_picker(id.clone(), window, cx);
                    }
                }
                _ => {}
            },
        );

        let font_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search font..."));
        let font_search_sub = cx.subscribe_in(
            &font_search,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change => this.update_font_list(cx),
                InputEvent::PressEnter { .. } => {
                    if let Some(font) = this.font_list.get(this.font_cursor).cloned() {
                        this.select_font(font, cx);
                    }
                    this.close_font_picker(window, cx);
                }
                _ => {}
            },
        );

        Self::apply_user_font_settings(cx);
        let available_fonts = Self::system_font_families(cx);
        let selected_font_family = crate::storage::load_setting("ui.font_family")
            .ok()
            .flatten()
            .filter(|font| available_fonts.iter().any(|available| available == font))
            .unwrap_or_else(|| cx.theme().font_family.to_string());
        let font_size_input = cx.new(|cx| InputState::new(window, cx).placeholder("16"));
        font_size_input.update(cx, |state, cx| {
            let value = crate::storage::load_setting("ui.font_size")
                .ok()
                .flatten()
                .unwrap_or_else(|| "16".to_string());
            state.set_value(value, window, cx);
        });
        let font_size_for_sub = font_size_input.clone();
        let font_size_sub = cx.subscribe_in(
            &font_size_input,
            window,
            move |this, _, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    let family = this.selected_font_family.clone();
                    Self::save_font_settings(&family, &font_size_for_sub, cx);
                }
            },
        );

        let mut shortcut_inputs = Vec::new();
        let mut shortcut_subs = Vec::new();
        for spec in Self::shortcut_specs() {
            let value = Self::shortcut_key(&spec);
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(spec.default_key));
            input.update(cx, |state, cx| state.set_value(value, window, cx));
            let spec_for_sub = spec.clone();
            let input_for_sub = input.clone();
            shortcut_subs.push(cx.subscribe_in(
                &input,
                window,
                move |_, _, ev: &InputEvent, _, cx| match ev {
                    InputEvent::PressEnter { .. } | InputEvent::Blur => {
                        Self::save_shortcut(&spec_for_sub, &input_for_sub, cx);
                    }
                    _ => {}
                },
            ));
            shortcut_inputs.push(ShortcutInput { spec, input });
        }

        Self::bind_configured_shortcuts(cx);

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
            row_resize_state,
            column_resize_state,
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
            api_list_query: String::new(),
            api_list_dirty: true,
            api_cursor: 0,
            settings_open: false,
            settings_focus: cx.focus_handle(),
            shortcut_inputs,
            available_fonts,
            selected_font_family,
            font_picker_open: false,
            font_search,
            font_list: Vec::new(),
            font_cursor: 0,
            font_scroll: gpui::UniformListScrollHandle::new(),
            font_size_input,
            api_scroll: gpui::UniformListScrollHandle::new(),
            save_task: None,
            _subs: {
                let mut subs = vec![
                    storage_sub,
                    theme_search_sub,
                    api_search_sub,
                    font_search_sub,
                    font_size_sub,
                ];
                subs.extend(shortcut_subs);
                subs
            },
            maximized_panel: MaximizedPanel::None,
            response_layout: ResponseLayout::Row,
            resize_state_v: cx.new(|_| ResizableState::default()),
        }
    }

    fn update_api_list(&mut self, cx: &mut Context<Self>) {
        let query = self.api_search.read(cx).value().to_lowercase();
        self.ensure_api_list_for_query(&query, cx);
        cx.notify();
    }

    fn update_font_list(&mut self, cx: &mut Context<Self>) {
        self.rebuild_font_list(cx);
        cx.notify();
    }

    fn rebuild_font_list(&mut self, cx: &App) {
        let query = self.font_search.read(cx).value().to_lowercase();
        self.font_list = self
            .available_fonts
            .iter()
            .filter(|font| query.is_empty() || font.to_lowercase().contains(&query))
            .cloned()
            .collect();

        self.font_cursor = self
            .font_list
            .iter()
            .position(|font| font == &self.selected_font_family)
            .unwrap_or(0);
        if !self.font_list.is_empty() {
            self.font_scroll
                .scroll_to_item(self.font_cursor, ScrollStrategy::Nearest);
        }
    }

    fn rebuild_api_list(&mut self, cx: &App) {
        let query = self.api_search.read(cx).value().to_lowercase();
        self.rebuild_api_list_for_query(query, cx);
    }

    fn ensure_api_list_for_query(&mut self, query: &str, cx: &App) {
        if self.api_list_dirty || self.api_list_query != query {
            self.rebuild_api_list_for_query(query.to_string(), cx);
        }
    }

    fn rebuild_api_list_for_query(&mut self, query: String, cx: &App) {
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

    fn render_api_picker_rows(
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

    fn select_api_from_picker(
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

    fn close_theme_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.theme_picker_open = false;
        let fh = self.focus_handle.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    fn open_font_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.font_search
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.rebuild_font_list(cx);
        self.font_picker_open = true;
        cx.notify();

        let fh = self.font_search.read(cx).focus_handle(cx);
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
    }

    fn close_font_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.font_picker_open = false;
        let fh = self.settings_focus.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    fn select_font(&mut self, font: String, cx: &mut Context<Self>) {
        self.selected_font_family = font.clone();
        let _ = crate::storage::save_setting("ui.font_family", &font);
        Self::save_font_settings(&font, &self.font_size_input, cx);
        cx.notify();
    }

    fn open_api_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn select_adjacent_api(&mut self, step: isize, cx: &mut Context<Self>) {
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
                Self::apply_user_font_settings(cx);
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
            Self::apply_user_font_settings(cx);
            cx.refresh_windows();
        }
    }

    fn commit_theme_selection(&mut self, _cx: &mut Context<Self>) {
        if let Some(name) = self.theme_list.get(self.theme_cursor) {
            let _ = crate::storage::save_theme_name(name);
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
                this.select_adjacent_api(1, cx);
            }))
            .on_action(cx.listener(|this, _: &PrevApi, _window, cx| {
                this.select_adjacent_api(-1, cx);
            }))
            .on_action(cx.listener(|this, _: &ApiPicker, window, cx| {
                if this.api_picker_open {
                    this.close_api_picker(window, cx);
                } else {
                    this.open_api_picker(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &FocusCollectionPanel, window, cx| {
                this.maximized_panel = MaximizedPanel::None;
                let fh = this.collection_panel.read(cx).focus_handle(cx);
                cx.defer_in(window, move |_, window, cx| {
                    fh.focus(window, cx);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusRequestPanel, window, cx| {
                this.maximized_panel = MaximizedPanel::None;
                let fh = this.request_panel.read(cx).focus_handle(cx);
                cx.defer_in(window, move |_, window, cx| {
                    fh.focus(window, cx);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusResponsePanel, window, cx| {
                this.maximized_panel = MaximizedPanel::None;
                let fh = this.response_panel.read(cx).focus_handle(cx);
                cx.defer_in(window, move |_, window, cx| {
                    fh.focus(window, cx);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusUrl, window, cx| {
                this.maximized_panel = MaximizedPanel::None;
                this.request_panel.update(cx, |panel, cx| {
                    panel.focus_url(window, cx);
                });
                cx.notify();
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
                                        .with_state(&self.row_resize_state)
                                        .child(
                                            resizable_panel()
                                                .size(px(260.))
                                                .size_range(px(180.)..px(400.))
                                                .child(self.collection_panel.clone()),
                                        )
                                        .child(
                                            resizable_panel()
                                                .size(px(520.))
                                                .size_range(px(340.)..px(900.))
                                                .child(self.request_panel.clone()),
                                        )
                                        .child(
                                            resizable_panel()
                                                .size(px(640.))
                                                .size_range(px(360.)..px(1600.))
                                                .child(self.response_panel.clone()),
                                        )
                                        .into_any_element()
                                    }
                                    ResponseLayout::Column => {
                                        h_resizable("main-panels-col")
                                        .with_state(&self.column_resize_state)
                                        .child(
                                            resizable_panel()
                                                .size(px(260.))
                                                .size_range(px(180.)..px(400.))
                                                .child(self.collection_panel.clone()),
                                        )
                                        .child(
                                            resizable_panel()
                                                .size(px(1000.))
                                                .size_range(px(620.)..px(2200.))
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
                                                this.api_scroll.scroll_to_item(
                                                    this.api_cursor,
                                                    ScrollStrategy::Nearest,
                                                );
                                                cx.notify();
                                            }
                                            "down" => {
                                                this.api_cursor = (this.api_cursor + 1) % n;
                                                this.api_scroll.scroll_to_item(
                                                    this.api_cursor,
                                                    ScrollStrategy::Nearest,
                                                );
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
                                            el.child(
                                                uniform_list(
                                                    "api-picker-list",
                                                    self.api_list.len(),
                                                    cx.processor(
                                                        |this: &mut AppView,
                                                         range: std::ops::Range<usize>,
                                                         _window,
                                                         cx| {
                                                            this.render_api_picker_rows(range, cx)
                                                        },
                                                    ),
                                                )
                                                .track_scroll(&self.api_scroll)
                                                .size_full(),
                                            )
                                        }),
                                ),
                        ),
                )
            })
            .when(self.font_picker_open, |el| {
                let visible_rows = self.font_list.len().max(1).min(12) as f32;
                let list_height = 8. + visible_rows * 36.;
                let panel_height = 62. + list_height;
                let font_count = self.font_list.len();
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
                                .id("font-picker-panel")
                                .w(px(420.))
                                .h(px(panel_height))
                                .max_h(px(560.))
                                .flex()
                                .flex_col()
                                .bg(cx.theme().background)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_lg()
                                .shadow_lg()
                                .overflow_hidden()
                                .on_key_down(cx.listener(
                                    move |this, event: &KeyDownEvent, window, cx| {
                                        let n = this.font_list.len();
                                        match event.keystroke.key.as_str() {
                                            "up" if n > 0 => {
                                                if this.font_cursor > 0 {
                                                    this.font_cursor -= 1;
                                                } else {
                                                    this.font_cursor = n - 1;
                                                }
                                                this.font_scroll
                                                    .scroll_to_item(this.font_cursor, ScrollStrategy::Nearest);
                                                cx.notify();
                                            }
                                            "down" if n > 0 => {
                                                this.font_cursor = (this.font_cursor + 1) % n;
                                                this.font_scroll
                                                    .scroll_to_item(this.font_cursor, ScrollStrategy::Nearest);
                                                cx.notify();
                                            }
                                            "enter" if n > 0 => {
                                                if let Some(font) =
                                                    this.font_list.get(this.font_cursor).cloned()
                                                {
                                                    this.select_font(font, cx);
                                                }
                                                this.close_font_picker(window, cx);
                                            }
                                            "escape" => {
                                                this.close_font_picker(window, cx);
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
                                        .child(Input::new(&self.font_search)),
                                )
                                .child(
                                    div()
                                        .id("font-list")
                                        .h(px(list_height))
                                        .max_h(px(498.))
                                        .min_h_0()
                                        .when(font_count == 0, |el| {
                                            el.child(
                                                div()
                                                    .w_full()
                                                    .px_4()
                                                    .py_3()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("No fonts found"),
                                            )
                                        })
                                        .when(font_count > 0, |el| {
                                            el.child(
                                                uniform_list(
                                                    "font-picker-list",
                                                    font_count,
                                                    cx.processor(
                                                        |this: &mut AppView,
                                                         range: std::ops::Range<usize>,
                                                         _window,
                                                         cx| {
                                                            let mut rows: Vec<AnyElement> = Vec::new();
                                                            for idx in range {
                                                                if idx >= this.font_list.len() {
                                                                    continue;
                                                                }
                                                                let font = this.font_list[idx].clone();
                                                                let is_cursor =
                                                                    idx == this.font_cursor;
                                                                let is_selected =
                                                                    font == this.selected_font_family;
                                                                let bg = if is_cursor {
                                                                    cx.theme().primary.opacity(0.1)
                                                                } else {
                                                                    gpui::transparent_black()
                                                                };
                                                                let text_color = if is_selected {
                                                                    cx.theme().primary
                                                                } else {
                                                                    cx.theme().foreground
                                                                };
                                                                let font_for_click = font.clone();
                                                                let font_for_style = font.clone();
                                                                rows.push(
                                                                    h_flex()
                                                                        .id(SharedString::from(
                                                                            format!(
                                                                                "font-picker-row-{}",
                                                                                idx
                                                                            ),
                                                                        ))
                                                                        .w_full()
                                                                        .h(px(36.))
                                                                        .px_3()
                                                                        .gap_2()
                                                                        .items_center()
                                                                        .bg(bg)
                                                                        .cursor_pointer()
                                                                        .on_mouse_down(
                                                                            gpui::MouseButton::Left,
                                                                            cx.listener(
                                                                                move |this, _, window, cx| {
                                                                                    this.font_cursor = idx;
                                                                                    this.select_font(font_for_click.clone(), cx);
                                                                                    this.close_font_picker(window, cx);
                                                                                },
                                                                            ),
                                                                        )
                                                                        .child(
                                                                            div()
                                                                                .w(px(18.))
                                                                                .child(if is_selected {
                                                                                    Icon::new(IconName::Check)
                                                                                        .xsmall()
                                                                                        .text_color(cx.theme().primary)
                                                                                        .into_any_element()
                                                                                } else {
                                                                                    div().into_any_element()
                                                                                }),
                                                                        )
                                                                        .child(
                                                                            div()
                                                                                .flex_1()
                                                                                .overflow_hidden()
                                                                                .text_ellipsis()
                                                                                .text_sm()
                                                                                .text_color(text_color)
                                                                                .font_family(font_for_style)
                                                                                .child(font),
                                                                        )
                                                                        .into_any_element(),
                                                                );
                                                            }
                                                            rows
                                                        },
                                                    ),
                                                )
                                                .track_scroll(&self.font_scroll)
                                                .size_full(),
                                            )
                                        }),
                                ),
                        ),
                )
            })
            .when(self.settings_open && !self.font_picker_open, |el| {
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
                                                    Button::new("settings-close")
                                                        .icon(IconName::Close)
                                                        .tooltip("Close settings")
                                                        .ghost()
                                                        .xsmall()
                                                        .on_click(cx.listener(|this, _, window, cx| {
                                                            this.close_settings(window, cx);
                                                        }))
                                                )
                                        )
                                        .child(
                                            div().id("settings-scroll").flex_1().overflow_y_scroll().p_4()
                                                .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).child("Keybindings"))
                                                .child(
                                                    v_flex().gap_2().mt_2()
                                                        .children(self.shortcut_inputs.iter().map(|shortcut| {
                                                            h_flex()
                                                                .justify_between()
                                                                .items_center()
                                                                .gap_4()
                                                                .child(
                                                                    div()
                                                                        .flex_1()
                                                                        .text_sm()
                                                                        .child(shortcut.spec.label),
                                                                )
                                                                .child(
                                                                    Input::new(&shortcut.input)
                                                                        .w(px(180.))
                                                                        .h(px(30.)),
                                                                )
                                                        }))
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
                                                                                            AppView::apply_user_font_settings_app(cx);
                                                                                            cx.refresh_windows();
                                                                                            let _ = crate::storage::save_theme_name(&name_clone);
                                                                                        }
                                                                                    })
                                                                            );
                                                                        }
                                                                        menu
                                                                    })
                                                            })
                                                        )
                                                        .child(
                                                            h_flex()
                                                                .justify_between()
                                                                .items_center()
                                                                .gap_4()
                                                                .child("UI Font")
                                                                .child(
                                                                    Button::new("ui-font-btn")
                                                                        .label(self.selected_font_family.clone())
                                                                        .outline()
                                                                        .on_click(cx.listener(|this, _, window, cx| {
                                                                            this.open_font_picker(window, cx);
                                                                        })),
                                                                )
                                                        )
                                                        .child(
                                                            h_flex()
                                                                .justify_between()
                                                                .items_center()
                                                                .gap_4()
                                                                .child("UI Font Size")
                                                                .child(
                                                                    Input::new(&self.font_size_input)
                                                                        .w(px(90.))
                                                                        .h(px(30.)),
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
            })
            // ── Folder settings modal (full-app centered overlay) ──────────
            .when(self.collection_panel.read(cx).folder_settings_open(), |el| {
                let modal = self
                    .collection_panel
                    .update(cx, |panel, cx| panel.folder_settings_modal(window, cx));
                el.child(modal)
            })
            .children(dialog_layer)
            .children(notification_layer)
    }
}
