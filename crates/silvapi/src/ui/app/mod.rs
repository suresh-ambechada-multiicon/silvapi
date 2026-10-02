use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_kit::component::{
    ActiveTheme, IndexPath, Theme, ThemeRegistry,
    input::{InputEvent, InputState},
    resizable::{ResizableState, h_resizable, resizable_panel, v_resizable},
    select::{SearchableVec, SelectEvent, SelectState},
    v_flex,
};

use crate::state::{AppEvent, AppState};

use super::{
    actions::{
        ApiPicker, CloseSettings, CloseTab, FocusCollectionPanel, FocusRequestPanel,
        FocusResponsePanel, FocusUrl, NextApi, OpenSettings, PrevApi, SendRequest, ShortcutNoOp,
        SwitchLastTab, SwitchTab1, SwitchTab2, SwitchTab3, SwitchTab4, SwitchTab5, SwitchTab6,
        SwitchTab7, SwitchTab8, SwitchTab9, TabPicker, ThemePicker, ToggleMaximize,
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
    pub(super) theme_select: Entity<SelectState<SearchableVec<SharedString>>>,
    api_picker_open: bool,
    _api_focus: FocusHandle,
    api_search: Entity<InputState>,
    api_list: Vec<ApiPickerEntry>,
    api_list_query: String,
    api_list_dirty: bool,
    api_cursor: usize,
    open_tabs: Vec<String>,
    tab_bar_scroll: ScrollHandle,
    tab_picker_open: bool,
    tab_focus: FocusHandle,
    tab_search: Entity<InputState>,
    tab_list: Vec<ApiPickerEntry>,
    tab_list_query: String,
    tab_cursor: usize,
    tab_scroll: gpui_kit::UniformListScrollHandle,
    settings_open: bool,
    settings_focus: FocusHandle,
    shortcut_inputs: Vec<ShortcutInput>,
    /// The id of the shortcut currently being recorded (button clicked, waiting
    /// for a keypress), or None. While recording, global key bindings are
    /// cleared so the pressed combo doesn't trigger its action.
    recording_shortcut: Option<String>,
    shortcut_capture_focus: FocusHandle,
    /// Message shown when the last shortcut capture was rejected (duplicate, or
    /// a bare key with no modifier). None when there's nothing to report.
    shortcut_message: Option<String>,
    available_fonts: Vec<String>,
    selected_font_family: String,
    font_picker_open: bool,
    font_focus: FocusHandle,
    font_search: Entity<InputState>,
    font_list: Vec<String>,
    font_cursor: usize,
    font_scroll: gpui_kit::UniformListScrollHandle,
    font_size_input: Entity<InputState>,
    font_size_slider: Entity<gpui_kit::component::slider::SliderState>,
    api_scroll: gpui_kit::UniformListScrollHandle,
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

#[derive(Clone)]
pub(super) struct ApiPickerEntry {
    pub id: String,
    pub context: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub full_path: String,
    pub search_text: String,
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

mod pickers;
mod settings;
mod shortcuts;

impl AppView {
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
                        let workspace =
                            app_state.update(cx, |state, _| silvapi_core::models::Workspace {
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
                            .map(|entry| entry.id.clone())
                            .or_else(|| app_state.read(cx).active_request_id.clone());
                        this.rebuild_api_list(cx);
                        if let Some(selected) = selected {
                            if let Some(idx) =
                                this.api_list.iter().position(|entry| entry.id == selected)
                            {
                                this.api_cursor = idx;
                                this.api_scroll.scroll_to_item(idx, ScrollStrategy::Nearest);
                            }
                        }
                    }
                }
                if matches!(ev, AppEvent::RequestSelected) {
                    if let Some(id) = app_state.read(cx).active_request_id.clone() {
                        this.ensure_tab_for_request(id.clone());
                        if let Some(idx) = this.open_tabs.iter().position(|t| t == &id) {
                            this.tab_bar_scroll.scroll_to_item(idx);
                        }
                    }
                    cx.notify();
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

        let theme_items: Vec<SharedString> = {
            let mut names: Vec<String> = ThemeRegistry::global(cx)
                .themes()
                .keys()
                .map(|s| s.to_string())
                .collect();
            names.sort();
            names.into_iter().map(SharedString::from).collect()
        };
        let current_theme_name = Theme::global(cx).theme_name().to_string();
        let current_theme_idx = theme_items
            .iter()
            .position(|t| t.as_ref() == current_theme_name)
            .unwrap_or(0);
        let theme_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(theme_items),
                Some(IndexPath::new(current_theme_idx)),
                window,
                cx,
            )
            .searchable(true)
        });

        let theme_select_sub = cx.subscribe_in(
            &theme_select,
            window,
            |_this, _select, ev: &SelectEvent<SearchableVec<SharedString>>, _window, cx| {
                if let SelectEvent::Confirm(Some(val)) = ev {
                    let name = val.to_string();
                    if let Some(cfg) = ThemeRegistry::global(cx).themes().get(name.as_str()).cloned() {
                        Theme::global_mut(cx).apply_config(&cfg);
                        AppView::apply_user_font_settings_app(cx);
                        cx.refresh_windows();
                        let _ = crate::storage::save_theme_name(&name);
                    }
                }
            },
        );

        let api_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search API..."));

        let api_search_sub = cx.subscribe_in(
            &api_search,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change => this.update_api_list(cx),
                InputEvent::PressEnter { .. } => {
                    if let Some(entry) = this.api_list.get(this.api_cursor) {
                        this.select_api_from_picker(entry.id.clone(), window, cx);
                    }
                }
                _ => {}
            },
        );

        let tab_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search tabs..."));
        let tab_search_sub = cx.subscribe_in(
            &tab_search,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change => this.update_tab_list(cx),
                InputEvent::PressEnter { .. } => {
                    if let Some(entry) = this.tab_list.get(this.tab_cursor) {
                        this.select_tab(entry.id.clone(), window, cx);
                        this.close_tab_picker(window, cx);
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
        let initial_font_size = crate::storage::load_setting("ui.font_size")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<f32>().ok())
            .filter(|s| (10.0..=24.0).contains(s))
            .unwrap_or(16.0);
        let font_size_slider = cx.new(|_| {
            gpui_kit::component::slider::SliderState::new()
                .min(10.0)
                .max(24.0)
                .step(1.0)
                .default_value(initial_font_size)
        });

        let font_size_for_sub = font_size_input.clone();
        let slider_for_input_sub = font_size_slider.clone();
        let font_size_sub = cx.subscribe_in(
            &font_size_input,
            window,
            move |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    let family = this.selected_font_family.clone();
                    Self::save_font_settings(&family, &font_size_for_sub, cx);
                    // Keep the slider in sync with a manually-typed size.
                    if let Ok(size) = font_size_for_sub.read(cx).value().trim().parse::<f32>() {
                        let size = size.clamp(10.0, 24.0);
                        slider_for_input_sub.update(cx, |s, cx| {
                            s.set_value(size, window, cx);
                        });
                    }
                }
            },
        );

        let font_size_for_slider = font_size_input.clone();
        let font_size_slider_sub = cx.subscribe_in(
            &font_size_slider,
            window,
            move |this, _, ev: &gpui_kit::component::slider::SliderEvent, window, cx| {
                match ev {
                    // While dragging, only reflect the number — applying the
                    // font size live would rescale the whole UI (and the slider
                    // itself) on every frame, making it jump under the cursor.
                    gpui_kit::component::slider::SliderEvent::Change(value) => {
                        let size = value.start().round() as i32;
                        font_size_for_slider.update(cx, |state, cx| {
                            state.set_value(size.to_string(), window, cx);
                        });
                    }
                    // Apply (and persist) once the drag ends.
                    gpui_kit::component::slider::SliderEvent::Release(_) => {
                        let family = this.selected_font_family.clone();
                        Self::save_font_settings(&family, &font_size_for_slider, cx);
                    }
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
            theme_select,
            api_picker_open: false,
            _api_focus: cx.focus_handle(),
            api_search,
            api_list: Vec::new(),
            api_list_query: String::new(),
            api_list_dirty: true,
            api_cursor: 0,
            open_tabs: Vec::new(),
            tab_bar_scroll: ScrollHandle::new(),
            tab_picker_open: false,
            tab_focus: cx.focus_handle(),
            tab_search,
            tab_list: Vec::new(),
            tab_list_query: String::new(),
            tab_cursor: 0,
            tab_scroll: gpui_kit::UniformListScrollHandle::new(),
            settings_open: false,
            settings_focus: cx.focus_handle(),
            shortcut_inputs,
            recording_shortcut: None,
            shortcut_capture_focus: cx.focus_handle(),
            shortcut_message: None,
            available_fonts,
            selected_font_family,
            font_picker_open: false,
            font_focus: cx.focus_handle(),
            font_search,
            font_list: Vec::new(),
            font_cursor: 0,
            font_scroll: gpui_kit::UniformListScrollHandle::new(),
            font_size_input,
            font_size_slider,
            api_scroll: gpui_kit::UniformListScrollHandle::new(),
            save_task: None,
            _subs: {
                let mut subs = vec![
                    storage_sub,
                    theme_search_sub,
                    theme_select_sub,
                    api_search_sub,
                    tab_search_sub,
                    font_search_sub,
                    font_size_sub,
                    font_size_slider_sub,
                ];
                subs.extend(shortcut_subs);
                subs
            },
            maximized_panel: MaximizedPanel::None,
            response_layout: ResponseLayout::Row,
            resize_state_v: cx.new(|_| ResizableState::default()),
        }
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Clamp settings dialog to 70% of viewport, within sensible absolute bounds.
        let viewport = window.viewport_size();
        let clamp = |v: Pixels, lo: f32, hi: f32| v.max(px(lo)).min(px(hi));
        let settings_w = clamp(viewport.width * 0.7, 480.0, 860.0);
        let settings_h = clamp(viewport.height * 0.8, 420.0, 720.0);

        div()
            .relative().size_full()
            .bg(cx.theme().background)
            .track_focus(&self.focus_handle)
            // Swallows keys whose old binding was reassigned (see shadow_shortcut_key).
            .on_action(cx.listener(|_this, _: &ShortcutNoOp, _window, _cx| {}))
            .on_action(cx.listener(|this, _: &ThemePicker, window, cx| {
                if this.recording_shortcut.is_some() { return; }
                if this.theme_picker_open { this.restore_original_theme(cx); this.close_theme_picker(window, cx); }
                else { this.open_theme_picker(window, cx); }
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                if this.recording_shortcut.is_none() { this.open_settings(window, cx); }
            }))
            .on_action(cx.listener(|this, _: &CloseSettings, window, cx| {
                if this.recording_shortcut.is_none() { this.close_settings(window, cx); }
            }))
            .on_action(cx.listener(|this, _: &NextApi, _w, cx| { if this.recording_shortcut.is_none() { this.select_adjacent_tab(1, cx); } }))
            .on_action(cx.listener(|this, _: &PrevApi, _w, cx| { if this.recording_shortcut.is_none() { this.select_adjacent_tab(-1, cx); } }))
            .on_action(cx.listener(|this, _: &CloseTab, w, cx| { if this.recording_shortcut.is_none() { this.close_active_tab(w, cx); } }))
            .on_action(cx.listener(|this, _: &SwitchTab1, w, cx| { if this.recording_shortcut.is_none() { this.select_tab_number(0, w, cx); } }))
            .on_action(cx.listener(|this, _: &SwitchTab2, w, cx| { if this.recording_shortcut.is_none() { this.select_tab_number(1, w, cx); } }))
            .on_action(cx.listener(|this, _: &SwitchTab3, w, cx| { if this.recording_shortcut.is_none() { this.select_tab_number(2, w, cx); } }))
            .on_action(cx.listener(|this, _: &SwitchTab4, w, cx| { if this.recording_shortcut.is_none() { this.select_tab_number(3, w, cx); } }))
            .on_action(cx.listener(|this, _: &SwitchTab5, w, cx| { if this.recording_shortcut.is_none() { this.select_tab_number(4, w, cx); } }))
            .on_action(cx.listener(|this, _: &SwitchTab6, w, cx| { if this.recording_shortcut.is_none() { this.select_tab_number(5, w, cx); } }))
            .on_action(cx.listener(|this, _: &SwitchTab7, w, cx| { if this.recording_shortcut.is_none() { this.select_tab_number(6, w, cx); } }))
            .on_action(cx.listener(|this, _: &SwitchTab8, w, cx| { if this.recording_shortcut.is_none() { this.select_tab_number(7, w, cx); } }))
            .on_action(cx.listener(|this, _: &SwitchTab9, w, cx| { if this.recording_shortcut.is_none() { this.select_tab_number(8, w, cx); } }))
            .on_action(cx.listener(|this, _: &SwitchLastTab, w, cx| {
                if this.recording_shortcut.is_none() && !this.open_tabs.is_empty() {
                    this.select_tab_number(this.open_tabs.len() - 1, w, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &ApiPicker, w, cx| {
                if this.recording_shortcut.is_some() { return; }
                if this.api_picker_open { this.close_api_picker(w, cx); } else { this.open_api_picker(w, cx); }
            }))
            .on_action(cx.listener(|this, _: &TabPicker, w, cx| {
                if this.recording_shortcut.is_some() { return; }
                if this.tab_picker_open { this.close_tab_picker(w, cx); } else { this.open_tab_picker(w, cx); }
            }))
            .on_action(cx.listener(|this, _: &FocusCollectionPanel, w, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.maximized_panel = MaximizedPanel::None;
                let fh = this.collection_panel.read(cx).focus_handle(cx);
                cx.defer_in(w, move |_, w, cx| { fh.focus(w, cx); });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusRequestPanel, w, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.maximized_panel = MaximizedPanel::None;
                let fh = this.request_panel.read(cx).focus_handle(cx);
                cx.defer_in(w, move |_, w, cx| { fh.focus(w, cx); });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusResponsePanel, w, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.maximized_panel = MaximizedPanel::None;
                let fh = this.response_panel.read(cx).focus_handle(cx);
                cx.defer_in(w, move |_, w, cx| { fh.focus(w, cx); });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusUrl, w, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.maximized_panel = MaximizedPanel::None;
                this.request_panel.update(cx, |p, cx| { p.focus_url(w, cx); });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SendRequest, w, cx| {
                if this.recording_shortcut.is_none() {
                    this.request_panel.update(cx, |p, cx| { p.send_request(w, cx); });
                }
            }))
            .on_action(cx.listener(|this, _: &ToggleMaximize, w, cx| {
                if this.recording_shortcut.is_none() { this.toggle_maximize(w, cx); }
            }))
            .child(self.render_main_layout(window, cx))
            .when(self.theme_picker_open, |el| el.child(self.render_theme_picker_overlay(window, cx)))
            .when(self.tab_picker_open,   |el| el.child(self.render_tab_picker_overlay(window, cx)))
            .when(self.font_picker_open,  |el| el.child(self.render_font_picker_overlay(window, cx)))
            .when(self.settings_open && !self.font_picker_open, |el| {
                el.child(self.render_settings_modal(settings_w, settings_h, window, cx))
            })
            .when(self.collection_panel.read(cx).folder_settings_open(), |el| {
                let modal = self.collection_panel.update(cx, |p, cx| p.folder_settings_modal(window, cx));
                el.child(modal)
            })
    }
}

impl AppView {
    fn toggle_maximize(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.maximized_panel != MaximizedPanel::None {
            self.maximized_panel = MaximizedPanel::None;
        } else {
            let cf  = self.collection_panel.read(cx).focus_handle(cx);
            let rf  = self.request_panel.read(cx).focus_handle(cx);
            let rsf = self.response_panel.read(cx).focus_handle(cx);
            self.maximized_panel = if cf.contains_focused(window, cx) {
                MaximizedPanel::Collection
            } else if rf.contains_focused(window, cx) {
                MaximizedPanel::Request
            } else if rsf.contains_focused(window, cx) {
                MaximizedPanel::Response
            } else {
                let mut req_focused = false;
                let mut res_focused = false;
                self.request_panel.update(cx, |p, cx| { req_focused = p.has_focus(window, cx); });
                self.response_panel.update(cx, |p, cx| { res_focused = p.has_focus(window, cx); });
                if req_focused { MaximizedPanel::Request }
                else if res_focused { MaximizedPanel::Response }
                else { MaximizedPanel::Response }
            };
        }
        cx.notify();
    }

    fn render_main_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panels: AnyElement = match self.maximized_panel {
            MaximizedPanel::Collection => self.collection_panel.clone().into_any_element(),
            MaximizedPanel::Request    => self.request_panel.clone().into_any_element(),
            MaximizedPanel::Response   => self.response_panel.clone().into_any_element(),
            MaximizedPanel::None => match self.response_layout {
                ResponseLayout::Row => h_resizable("main-panels")
                    .with_state(&self.row_resize_state)
                    .child(resizable_panel().size(px(260.)).size_range(px(180.)..px(400.)).child(self.collection_panel.clone()))
                    .child(resizable_panel().size(px(520.)).size_range(px(340.)..px(900.)).child(self.request_panel.clone()))
                    .child(resizable_panel().size(px(640.)).size_range(px(360.)..px(1600.)).child(self.response_panel.clone()))
                    .into_any_element(),
                ResponseLayout::Column => h_resizable("main-panels-col")
                    .with_state(&self.column_resize_state)
                    .child(resizable_panel().size(px(260.)).size_range(px(180.)..px(400.)).child(self.collection_panel.clone()))
                    .child(
                        resizable_panel().size(px(1000.)).size_range(px(620.)..px(2200.)).child(
                            v_resizable("req-resp-col")
                                .with_state(&self.resize_state_v)
                                .child(resizable_panel().size(px(350.)).size_range(px(200.)..px(600.)).child(self.request_panel.clone()))
                                .child(resizable_panel().child(self.response_panel.clone())),
                        ),
                    )
                    .into_any_element(),
            },
        };
        v_flex()
            .size_full()
            .child(self.top_bar.clone())
            .child(self.render_tab_strip(window, cx))
            .child(div().flex_1().overflow_hidden().child(panels))
    }
}
