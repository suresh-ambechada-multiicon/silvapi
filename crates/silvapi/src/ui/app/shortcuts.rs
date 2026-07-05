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
    pub(super) fn shortcut_specs() -> Vec<ShortcutSpec> {
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

    pub(super) fn shortcut_key(spec: &ShortcutSpec) -> String {
        let key = format!("shortcut.{}", spec.id);
        crate::storage::load_setting(&key)
            .ok()
            .flatten()
            .filter(|saved| Keystroke::parse(saved).is_ok())
            .unwrap_or_else(|| spec.default_key.to_string())
    }

    pub(super) fn bind_shortcut(id: &str, key: &str, cx: &mut Context<Self>) {
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

    pub(super) fn bind_configured_shortcuts(cx: &mut Context<Self>) {
        for spec in Self::shortcut_specs() {
            let key = Self::shortcut_key(&spec);
            Self::bind_shortcut(spec.id, &key, cx);
        }
    }

    pub(super) fn save_shortcut(spec: &ShortcutSpec, input: &Entity<InputState>, cx: &mut Context<Self>) {
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

    /// Disable a previously-bound key. gpui has no targeted unbind, but the
    /// last-registered binding for a key wins, so binding the old key to a
    /// no-op action (which has a global handler) shadows its former action —
    /// preventing an action from keeping multiple keys after reassignment.
    pub(super) fn shadow_shortcut_key(old_key: &str, new_key: &str, cx: &mut Context<Self>) {
        let old = old_key.trim().to_lowercase();
        let new = new_key.trim().to_lowercase();
        if old.is_empty() || old == new || Keystroke::parse(&old).is_err() {
            return;
        }
        cx.bind_keys([KeyBinding::new(
            &old,
            crate::ui::actions::ShortcutNoOp,
            None,
        )]);
    }

}
