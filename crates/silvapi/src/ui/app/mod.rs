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
        ApiPicker, CloseSettings, FocusCollectionPanel, FocusRequestPanel,
        FocusResponsePanel, FocusUrl, NextApi, OpenSettings, PrevApi, SendRequest,
        ShortcutNoOp, ThemePicker, ToggleMaximize,
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
    font_search: Entity<InputState>,
    font_list: Vec<String>,
    font_cursor: usize,
    font_scroll: gpui::UniformListScrollHandle,
    font_size_input: Entity<InputState>,
    font_size_slider: Entity<gpui_component::slider::SliderState>,
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
                        let workspace = app_state.update(cx, |state, _| silvapi_core::models::Workspace {
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
        let initial_font_size = crate::storage::load_setting("ui.font_size")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<f32>().ok())
            .filter(|s| (10.0..=24.0).contains(s))
            .unwrap_or(16.0);
        let font_size_slider = cx.new(|_| {
            gpui_component::slider::SliderState::new()
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
            move |this, _, ev: &gpui_component::slider::SliderEvent, window, cx| {
                match ev {
                    // While dragging, only reflect the number — applying the
                    // font size live would rescale the whole UI (and the slider
                    // itself) on every frame, making it jump under the cursor.
                    gpui_component::slider::SliderEvent::Change(value) => {
                        let size = value.start().round() as i32;
                        font_size_for_slider.update(cx, |state, cx| {
                            state.set_value(size.to_string(), window, cx);
                        });
                    }
                    // Apply (and persist) once the drag ends.
                    gpui_component::slider::SliderEvent::Release(_) => {
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
            api_picker_open: false,
            api_search,
            api_list: Vec::new(),
            api_list_query: String::new(),
            api_list_dirty: true,
            api_cursor: 0,
            settings_open: false,
            settings_focus: cx.focus_handle(),
            shortcut_inputs,
            recording_shortcut: None,
            shortcut_capture_focus: cx.focus_handle(),
            shortcut_message: None,
            available_fonts,
            selected_font_family,
            font_picker_open: false,
            font_search,
            font_list: Vec::new(),
            font_cursor: 0,
            font_scroll: gpui::UniformListScrollHandle::new(),
            font_size_input,
            font_size_slider,
            api_scroll: gpui::UniformListScrollHandle::new(),
            save_task: None,
            _subs: {
                let mut subs = vec![
                    storage_sub,
                    theme_search_sub,
                    api_search_sub,
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
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        // Settings dialog size: large, but capped and viewport-relative so it
        // fits on small windows.
        let viewport = window.viewport_size();
        let clamp_px = |value: Pixels, lo: f32, hi: f32| {
            if value < px(lo) {
                px(lo)
            } else if value > px(hi) {
                px(hi)
            } else {
                value
            }
        };
        let settings_w = clamp_px(viewport.width * 0.7, 480.0, 860.0);
        let settings_h = clamp_px(viewport.height * 0.8, 420.0, 720.0);

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
            // Swallows keys whose old binding was reassigned (see shadow_shortcut_key).
            .on_action(cx.listener(|_this, _: &ShortcutNoOp, _window, _cx| {}))
            .on_action(cx.listener(|this, _: &ThemePicker, window, cx| {
                if this.recording_shortcut.is_some() { return; }
                if this.theme_picker_open {
                    this.restore_original_theme(cx);
                    this.close_theme_picker(window, cx);
                } else {
                    this.open_theme_picker(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.open_settings(window, cx);
            }))
            .on_action(cx.listener(|this, _: &CloseSettings, window, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.close_settings(window, cx);
            }))
            .on_action(cx.listener(|this, _: &NextApi, _window, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.select_adjacent_api(1, cx);
            }))
            .on_action(cx.listener(|this, _: &PrevApi, _window, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.select_adjacent_api(-1, cx);
            }))
            .on_action(cx.listener(|this, _: &ApiPicker, window, cx| {
                if this.recording_shortcut.is_some() { return; }
                if this.api_picker_open {
                    this.close_api_picker(window, cx);
                } else {
                    this.open_api_picker(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &FocusCollectionPanel, window, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.maximized_panel = MaximizedPanel::None;
                let fh = this.collection_panel.read(cx).focus_handle(cx);
                cx.defer_in(window, move |_, window, cx| {
                    fh.focus(window, cx);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusRequestPanel, window, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.maximized_panel = MaximizedPanel::None;
                let fh = this.request_panel.read(cx).focus_handle(cx);
                cx.defer_in(window, move |_, window, cx| {
                    fh.focus(window, cx);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusResponsePanel, window, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.maximized_panel = MaximizedPanel::None;
                let fh = this.response_panel.read(cx).focus_handle(cx);
                cx.defer_in(window, move |_, window, cx| {
                    fh.focus(window, cx);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusUrl, window, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.maximized_panel = MaximizedPanel::None;
                this.request_panel.update(cx, |panel, cx| {
                    panel.focus_url(window, cx);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SendRequest, window, cx| {
                if this.recording_shortcut.is_some() { return; }
                this.request_panel.update(cx, |panel, cx| {
                    panel.send_request(window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleMaximize, window, cx| {
                if this.recording_shortcut.is_some() { return; }
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
                        .occlude()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                this.close_theme_picker(window, cx);
                            }),
                        )
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
                                .occlude()
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
                        .occlude()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                this.close_api_picker(window, cx);
                            }),
                        )
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
                                .occlude()
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
                        .occlude()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                this.close_font_picker(window, cx);
                            }),
                        )
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
                                .occlude()
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
                        .bg(cx.theme().background.opacity(0.72))
                        // Backdrop: block clicks reaching the app behind and
                        // close the dialog on an outside click.
                        .occlude()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                this.close_settings(window, cx);
                            }),
                        )
                        .child(
                            div()
                                .id("settings-panel")
                                .w(settings_w)
                                .h(settings_h)
                                .bg(cx.theme().background)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_lg()
                                .shadow_lg()
                                .overflow_hidden()
                                // Swallow clicks inside the panel so they don't
                                // fall through to the backdrop and close it.
                                .occlude()
                                .on_mouse_down(MouseButton::Left, |_, _, _| {})
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
                                                    div()
                                                        .text_xs()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child("Click a shortcut, then press the keys (Enter is allowed). Esc cancels."),
                                                )
                                                .child(
                                                    v_flex().gap_2().mt_2()
                                                        .children(self.shortcut_inputs.iter().map(|shortcut| {
                                                            let spec = shortcut.spec.clone();
                                                            let recording =
                                                                self.recording_shortcut.as_deref() == Some(spec.id);
                                                            let current = shortcut.input.read(cx).value().to_string();
                                                            let label_text = if recording {
                                                                "Press keys…".to_string()
                                                            } else if current.trim().is_empty() {
                                                                "Unassigned".to_string()
                                                            } else {
                                                                current.clone()
                                                            };
                                                            let capture_input = shortcut.input.clone();
                                                            let capture_spec = spec.clone();
                                                            let reset_input = shortcut.input.clone();
                                                            let reset_spec = spec.clone();
                                                            let start_id = spec.id;
                                                            h_flex()
                                                                .justify_between()
                                                                .items_center()
                                                                .gap_4()
                                                                .child(
                                                                    div()
                                                                        .flex_1()
                                                                        .text_sm()
                                                                        .child(spec.label),
                                                                )
                                                                .child(
                                                                    h_flex()
                                                                        .gap_1()
                                                                        .items_center()
                                                                        .child(
                                                                            div()
                                                                                .id(SharedString::from(format!("kb-{}", spec.id)))
                                                                                .w(px(180.))
                                                                                .h(px(30.))
                                                                                .px_2()
                                                                                .flex()
                                                                                .items_center()
                                                                                .rounded_md()
                                                                                .border_1()
                                                                                .border_color(if recording {
                                                                                    cx.theme().primary
                                                                                } else {
                                                                                    cx.theme().border
                                                                                })
                                                                                .bg(if recording {
                                                                                    cx.theme().primary.opacity(0.1)
                                                                                } else {
                                                                                    cx.theme().background
                                                                                })
                                                                                .cursor_pointer()
                                                                                .text_sm()
                                                                                .font_family("monospace")
                                                                                .text_color(if recording {
                                                                                    cx.theme().primary
                                                                                } else if current.trim().is_empty() {
                                                                                    cx.theme().muted_foreground
                                                                                } else {
                                                                                    cx.theme().foreground
                                                                                })
                                                                                .when(recording, |el| {
                                                                                    el.track_focus(&self.shortcut_capture_focus)
                                                                                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                                                                                            let ks = &event.keystroke;
                                                                                            let key = ks.key.as_str();
                                                                                            cx.stop_propagation();
                                                                                            if key == "escape" {
                                                                                                this.cancel_recording_shortcut(cx);
                                                                                                return;
                                                                                            }
                                                                                            if key.is_empty()
                                                                                                || matches!(key, "control" | "shift" | "alt" | "platform" | "cmd" | "super" | "fn" | "function")
                                                                                            {
                                                                                                return;
                                                                                            }
                                                                                            // Character keys need Ctrl/Alt/Cmd — Shift alone
                                                                                            // just makes an uppercase character, so it doesn't
                                                                                            // count. Named keys (enter, tab, f-keys, arrows, …)
                                                                                            // are allowed without a modifier.
                                                                                            let mods = ks.modifiers;
                                                                                            let has_real_modifier =
                                                                                                mods.control || mods.alt || mods.platform;
                                                                                            let is_single_char = key.chars().count() == 1;
                                                                                            if is_single_char && !has_real_modifier {
                                                                                                this.shortcut_message = Some(
                                                                                                    "Use Ctrl or Alt — Shift alone isn't enough for a shortcut.".to_string(),
                                                                                                );
                                                                                                cx.notify();
                                                                                                return;
                                                                                            }
                                                                                            let combo = ks.unparse();
                                                                                            // Enforce uniqueness: reject a combo that is
                                                                                            // already bound to a different action so one
                                                                                            // keystroke never maps to two actions. The existing
                                                                                            // binding is left untouched.
                                                                                            let taken_by = this
                                                                                                .shortcut_inputs
                                                                                                .iter()
                                                                                                .find(|s| {
                                                                                                    s.spec.id != capture_spec.id
                                                                                                        && s.input.read(cx).value().to_string() == combo
                                                                                                })
                                                                                                .map(|s| s.spec.label);
                                                                                            if let Some(label) = taken_by {
                                                                                                this.shortcut_message = Some(format!(
                                                                                                    "That shortcut is already used by \"{}\". Pick a different key.",
                                                                                                    label
                                                                                                ));
                                                                                                this.finish_recording_shortcut(cx);
                                                                                                return;
                                                                                            }
                                                                                            this.shortcut_message = None;
                                                                                            // Disable the previous key so this action
                                                                                            // doesn't keep responding to both.
                                                                                            let old_key = capture_input.read(cx).value().to_string();
                                                                                            Self::shadow_shortcut_key(&old_key, &combo, cx);
                                                                                            capture_input.update(cx, |state, cx| {
                                                                                                state.set_value(combo.clone(), window, cx);
                                                                                            });
                                                                                            Self::save_shortcut(&capture_spec, &capture_input, cx);
                                                                                            this.finish_recording_shortcut(cx);
                                                                                        }))
                                                                                })
                                                                                .on_mouse_down(
                                                                                    MouseButton::Left,
                                                                                    cx.listener(move |this, _, window, cx| {
                                                                                        this.start_recording_shortcut(start_id, window, cx);
                                                                                    }),
                                                                                )
                                                                                .child(label_text),
                                                                        )
                                                                        .child(
                                                                            Button::new(SharedString::from(format!("kb-reset-{}", spec.id)))
                                                                                .icon(IconName::Undo2)
                                                                                .tooltip("Reset to default")
                                                                                .ghost()
                                                                                .xsmall()
                                                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                                                    let default_key = reset_spec.default_key.to_string();
                                                                                    // Disable the current key before reverting to default.
                                                                                    let old_key = reset_input.read(cx).value().to_string();
                                                                                    Self::shadow_shortcut_key(&old_key, &default_key, cx);
                                                                                    reset_input.update(cx, |state, cx| {
                                                                                        state.set_value(default_key, window, cx);
                                                                                    });
                                                                                    Self::save_shortcut(&reset_spec, &reset_input, cx);
                                                                                    if this.recording_shortcut.as_deref() == Some(reset_spec.id) {
                                                                                        this.finish_recording_shortcut(cx);
                                                                                    } else {
                                                                                        cx.notify();
                                                                                    }
                                                                                })),
                                                                        ),
                                                                )
                                                }))
                                                )
                                                .when_some(self.shortcut_message.clone(), |el, message| {
                                                    el.child(
                                                        div()
                                                            .mt_2()
                                                            .text_xs()
                                                            .text_color(cx.theme().danger)
                                                            .child(message),
                                                    )
                                                })
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
                                                                    h_flex()
                                                                        .items_center()
                                                                        .gap_3()
                                                                        .child(
                                                                            div()
                                                                                .text_xs()
                                                                                .text_color(cx.theme().muted_foreground)
                                                                                .child("10"),
                                                                        )
                                                                        .child(
                                                                            div().w(px(160.)).child(
                                                                                gpui_component::slider::Slider::new(
                                                                                    &self.font_size_slider,
                                                                                ),
                                                                            ),
                                                                        )
                                                                        .child(
                                                                            div()
                                                                                .text_xs()
                                                                                .text_color(cx.theme().muted_foreground)
                                                                                .child("24"),
                                                                        )
                                                                        .child(
                                                                            Input::new(&self.font_size_input)
                                                                                .w(px(64.))
                                                                                .h(px(30.)),
                                                                        ),
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

