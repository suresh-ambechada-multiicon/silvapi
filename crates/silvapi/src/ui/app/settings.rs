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

    pub(super) fn save_font_settings(family: &str, size_input: &Entity<InputState>, cx: &mut Context<Self>) {
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

}
