#![allow(unused_imports)]
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_kit::component::{
    ActiveTheme, FocusTrapElement as _, Icon, IconName, Root, Sizable as _, Theme, ThemeRegistry,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::PopupMenuItem,
    resizable::{ResizableState, h_resizable, resizable_panel, v_resizable},
    select::Select,
    v_flex,
};

use super::{AppView, ShortcutInput, ShortcutSpec};
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
    pub(super) fn system_font_families(cx: &App) -> Vec<String> {
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

    pub(super) fn configured_font_family(cx: &mut Context<Self>) -> SharedString {
        crate::storage::load_setting("ui.font_family")
            .ok()
            .flatten()
            .filter(|font| !font.trim().is_empty())
            .map(SharedString::from)
            .unwrap_or_else(|| cx.theme().font_family.clone())
    }

    pub(super) fn configured_font_size(cx: &mut Context<Self>) -> Pixels {
        crate::storage::load_setting("ui.font_size")
            .ok()
            .flatten()
            .and_then(|size| size.parse::<f32>().ok())
            .filter(|size| (10.0..=24.0).contains(size))
            .map(px)
            .unwrap_or_else(|| cx.theme().font_size)
    }

    pub(super) fn apply_user_font_settings(cx: &mut Context<Self>) {
        let font_family = Self::configured_font_family(cx);
        let font_size = Self::configured_font_size(cx);
        let theme = Theme::global_mut(cx);
        theme.font_family = font_family;
        theme.font_size = font_size;
        cx.refresh_windows();
    }

    pub(super) fn apply_user_font_settings_app(cx: &mut App) {
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

    pub(super) fn save_font_settings(
        family: &str,
        size_input: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) {
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

    pub(super) fn update_font_list(&mut self, cx: &mut Context<Self>) {
        self.rebuild_font_list(cx);
        cx.notify();
    }

    pub(super) fn rebuild_font_list(&mut self, cx: &App) {
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

    pub(super) fn open_font_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn close_font_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.font_picker_open = false;
        let fh = self.settings_focus.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    pub(super) fn select_font(&mut self, font: String, cx: &mut Context<Self>) {
        self.selected_font_family = font.clone();
        let _ = crate::storage::save_setting("ui.font_family", &font);
        Self::save_font_settings(&font, &self.font_size_input, cx);
        cx.notify();
    }

    pub(super) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = true;
        let fh = self.settings_focus.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    pub(super) fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.recording_shortcut.is_some() {
            self.cancel_recording_shortcut(cx);
        }
        self.settings_open = false;
        let fh = self.focus_handle.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
        cx.notify();
    }

    /// Begin recording a new keybinding for `id`. Clears all global key
    /// bindings so the combo the user presses next is captured by the field
    /// instead of triggering its action, and focuses the capture element.
    pub(super) fn start_recording_shortcut(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.recording_shortcut = Some(id.to_string());
        self.shortcut_message = None;
        // NOTE: do NOT clear key bindings here. `cx.clear_key_bindings()` wipes
        // *all* bindings globally — including gpui-component's text-input editing
        // actions (backspace, arrows, delete) — leaving every input broken.
        // Instead, `AppView`'s own action handlers are gated on `recording_shortcut`
        // so a pressed combo doesn't trigger its action while recording.
        cx.notify();
        // Focus after this render, once the capture element (which only gets
        // `track_focus` while recording) actually exists — otherwise the focus
        // has nothing to attach to and the first click appears to do nothing.
        let fh = self.shortcut_capture_focus.clone();
        cx.defer_in(window, move |_, window, cx| {
            fh.focus(window, cx);
        });
    }

    pub(super) fn finish_recording_shortcut(&mut self, cx: &mut Context<Self>) {
        self.recording_shortcut = None;
        cx.notify();
    }

    pub(super) fn cancel_recording_shortcut(&mut self, cx: &mut Context<Self>) {
        self.finish_recording_shortcut(cx);
    }

    /// Renders the centered settings dialog. Only shown when `settings_open && !font_picker_open`.
    pub(super) fn render_settings_modal(
        &mut self,
        settings_w: Pixels,
        settings_h: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .absolute().top_0().left_0().size_full()
            .flex().items_center().justify_center()
            .bg(cx.theme().background.opacity(0.72))
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.close_settings(window, cx);
            }))
            .child(
                div()
                    .id("settings-panel")
                    .w(settings_w).h(settings_h)
                    .bg(cx.theme().background)
                    .border_1().border_color(cx.theme().border)
                    .rounded_lg().shadow_lg().overflow_hidden()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .track_focus(&self.settings_focus)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape" { this.close_settings(window, cx); }
                    }))
                    .child(
                        v_flex().size_full()
                            .child(self.render_settings_header(window, cx))
                            .child(self.render_settings_body(window, cx)),
                    )
                    .focus_trap("settings-panel-trap", &self.settings_focus),
            )
    }

    fn render_settings_header(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .px_4().py_3()
            .border_b_1().border_color(cx.theme().border)
            .justify_between()
            .child(div().font_weight(gpui_kit::FontWeight::BOLD).child("Settings"))
            .child(
                Button::new("settings-close")
                    .icon(IconName::Close)
                    .tooltip("Close settings")
                    .ghost().xsmall()
                    .on_click(cx.listener(|this, _, window, cx| { this.close_settings(window, cx); })),
            )
    }

    fn render_settings_body(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().id("settings-scroll").flex_1().overflow_y_scroll().p_4()
            .child(div().text_lg().font_weight(gpui_kit::FontWeight::BOLD).child("Keybindings"))
            .child(div().text_xs().text_color(cx.theme().muted_foreground)
                .child("Click a shortcut, then press the keys (Enter is allowed). Esc cancels."))
            .child(self.render_shortcut_list(window, cx))
            .when_some(self.shortcut_message.clone(), |el, message| {
                el.child(div().mt_2().text_xs().text_color(cx.theme().danger).child(message))
            })
            .child(div().mt_6().text_lg().font_weight(gpui_kit::FontWeight::BOLD).child("Appearance"))
            .child(self.render_appearance_settings(window, cx))
    }

    fn render_shortcut_list(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Collect all data needed to build rows upfront to avoid self borrow conflicts.
        struct ShortcutRow {
            spec: super::ShortcutSpec,
            recording: bool,
            current: String,
            label_text: String,
            capture_input: gpui_kit::Entity<gpui_kit::component::input::InputState>,
            capture_spec: super::ShortcutSpec,
            reset_input: gpui_kit::Entity<gpui_kit::component::input::InputState>,
            reset_spec: super::ShortcutSpec,
        }
        let capture_focus = self.shortcut_capture_focus.clone();
        let rows_data: Vec<ShortcutRow> = self.shortcut_inputs.iter().map(|s| {
            let spec = s.spec.clone();
            let recording = self.recording_shortcut.as_deref() == Some(spec.id);
            let current = s.input.read(cx).value().to_string();
            let label_text = if recording { "Press keys…".to_string() }
                else if current.trim().is_empty() { "Unassigned".to_string() }
                else { current.clone() };
            ShortcutRow {
                spec: spec.clone(), recording, current, label_text,
                capture_input: s.input.clone(), capture_spec: spec.clone(),
                reset_input: s.input.clone(), reset_spec: spec,
            }
        }).collect();

        let mut rows: Vec<AnyElement> = Vec::new();
        for row in rows_data {
            let spec = row.spec;
            let recording = row.recording;
            let current = row.current;
            let label_text = row.label_text;
            let capture_input = row.capture_input;
            let capture_spec  = row.capture_spec;
            let reset_input   = row.reset_input;
            let reset_spec    = row.reset_spec;
            let start_id      = spec.id;

            // Key box element (inline to avoid second &mut self borrow)
            let key_box = {
                let lt = label_text.clone();
                let cur = current.clone();
                div()
                    .id(SharedString::from(format!("kb-{}", start_id)))
                    .w(px(180.)).h(px(30.)).px_2()
                    .flex().items_center()
                    .rounded_md()
                    .border_1().border_color(if recording { cx.theme().primary } else { cx.theme().border })
                    .bg(if recording { cx.theme().primary.opacity(0.1) } else { cx.theme().background })
                    .cursor_pointer().text_sm()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(if recording { cx.theme().primary }
                        else if cur.trim().is_empty() { cx.theme().muted_foreground }
                        else { cx.theme().foreground })
                    .when(recording, |el| {
                        el.track_focus(&capture_focus)
                            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                                let ks = &event.keystroke;
                                let key = ks.key.as_str();
                                cx.stop_propagation();
                                if key == "escape" { this.cancel_recording_shortcut(cx); return; }
                                if key.is_empty() || matches!(key, "control"|"shift"|"alt"|"platform"|"cmd"|"super"|"fn"|"function") { return; }
                                let mods = ks.modifiers;
                                let has_real_modifier = mods.control || mods.alt || mods.platform;
                                let is_single_char = key.chars().count() == 1;
                                if is_single_char && !has_real_modifier {
                                    this.shortcut_message = Some("Use Ctrl or Alt — Shift alone isn't enough for a shortcut.".to_string());
                                    cx.notify();
                                    return;
                                }
                                let combo = ks.unparse();
                                let taken_by = this.shortcut_inputs.iter()
                                    .find(|s| s.spec.id != capture_spec.id && s.input.read(cx).value().to_string() == combo)
                                    .map(|s| s.spec.label);
                                if let Some(label) = taken_by {
                                    this.shortcut_message = Some(format!("That shortcut is already used by \"{}\". Pick a different key.", label));
                                    this.finish_recording_shortcut(cx);
                                    return;
                                }
                                this.shortcut_message = None;
                                let old_key = capture_input.read(cx).value().to_string();
                                Self::shadow_shortcut_key(&old_key, &combo, cx);
                                capture_input.update(cx, |state, cx| { state.set_value(combo.clone(), window, cx); });
                                Self::save_shortcut(&capture_spec, &capture_input, cx);
                                this.finish_recording_shortcut(cx);
                            }))
                    })
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                        this.start_recording_shortcut(start_id, window, cx);
                    }))
                    .child(lt)
            };

            rows.push(
                h_flex().justify_between().items_center().gap_4()
                    .child(div().flex_1().text_sm().child(spec.label))
                    .child(
                        h_flex().gap_1().items_center()
                            .child(key_box)
                            .child(
                                Button::new(SharedString::from(format!("kb-reset-{}", spec.id)))
                                    .icon(IconName::Undo2)
                                    .tooltip("Reset to default")
                                    .ghost().xsmall()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        let default_key = reset_spec.default_key.to_string();
                                        let old_key = reset_input.read(cx).value().to_string();
                                        Self::shadow_shortcut_key(&old_key, &default_key, cx);
                                        reset_input.update(cx, |state, cx| { state.set_value(default_key, window, cx); });
                                        Self::save_shortcut(&reset_spec, &reset_input, cx);
                                        if this.recording_shortcut.as_deref() == Some(reset_spec.id) {
                                            this.finish_recording_shortcut(cx);
                                        } else { cx.notify(); }
                                    })),
                            ),
                    )
                    .into_any_element()
            );
        }
        v_flex().gap_2().mt_2().children(rows)
    }


    fn render_appearance_settings(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_4().mt_2()
            .child(
                h_flex().justify_between().items_center()
                    .child("Theme")
                    .child(div().w(px(240.)).child(Select::new(&self.theme_select).small())),
            )
            .child(
                h_flex().justify_between().items_center().gap_4()
                    .child("UI Font")
                    .child(
                        Button::new("ui-font-btn")
                            .label(self.selected_font_family.clone())
                            .outline()
                            .on_click(cx.listener(|this, _, window, cx| { this.open_font_picker(window, cx); })),
                    ),
            )
            .child(
                h_flex().justify_between().items_center().gap_4()
                    .child("UI Font Size")
                    .child(
                        h_flex().items_center().gap_3()
                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child("10"))
                            .child(div().w(px(160.)).child(gpui_kit::component::slider::Slider::new(&self.font_size_slider)))
                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child("24"))
                            .child(Input::new(&self.font_size_input).w(px(64.)).h(px(30.))),
                    ),
            )
    }
}
