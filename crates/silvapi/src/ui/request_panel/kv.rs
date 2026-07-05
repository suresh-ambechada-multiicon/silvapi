#![allow(unused_imports)]
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, Sizable as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::PopupMenuItem,
    tab::{Tab, TabBar},
    v_flex,
};

use silvapi_core::{models::{AuthConfig, AuthType, BodyType, FormDataPart, FormDataPartKind, HttpMethod, KeyValue,}};
use crate::{state::{AppEvent, AppState}};

use super::*;

impl RequestPanel {
    pub(super) fn make_kv_row(
        &mut self,
        key: &str,
        value: &str,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<InputState>, Entity<InputState>, bool) {
        let k = cx.new(|cx| InputState::new(window, cx).placeholder("Key"));
        let v = cx.new(|cx| InputState::new(window, cx).placeholder("Value"));
        k.update(cx, |s, cx| s.set_value(Self::input_line(key), window, cx));
        v.update(cx, |s, cx| s.set_value(Self::input_line(value), window, cx));

        let k_sub = cx.subscribe_in(&k, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        });
        let v_sub = cx.subscribe_in(&v, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        });
        self.row_subs.push(k_sub);
        self.row_subs.push(v_sub);
        (k, v, enabled)
    }

    pub(super) fn render_params_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, (key, val, enabled)) in self.param_rows.iter().enumerate() {
            rows.push(kv_input_row(i, key, val, *enabled, false, cx).into_any_element());
        }
        div()
            .id("params-scroll")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .child(kv_header(cx))
            .children(rows)
            .child(
                Button::new("add-param")
                    .icon(IconName::Plus)
                    .label("Add")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|this, _, window, cx| {
                        let row = this.make_kv_row("", "", true, window, cx);
                        this.param_rows.push(row);
                        this.sync_active_request(window, cx);
                        cx.notify();
                    })),
            )
    }

    pub(super) fn render_headers_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, (key, val, enabled)) in self.header_rows.iter().enumerate() {
            rows.push(kv_input_row(i, key, val, *enabled, true, cx).into_any_element());
        }
        div()
            .id("headers-scroll")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .child(kv_header(cx))
            .children(rows)
            .child(
                Button::new("add-header")
                    .icon(IconName::Plus)
                    .label("Add")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|this, _, window, cx| {
                        let row = this.make_kv_row("", "", true, window, cx);
                        this.header_rows.push(row);
                        this.sync_active_request(window, cx);
                        cx.notify();
                    })),
            )
    }

    pub(super) fn collect_kv(
        rows: &[(Entity<InputState>, Entity<InputState>, bool)],
        cx: &App,
    ) -> Vec<KeyValue> {
        rows.iter()
            .map(|(k, v, enabled)| KeyValue {
                id: uuid::Uuid::new_v4().to_string(),
                key: k.read(cx).value().to_string(),
                value: v.read(cx).value().to_string(),
                enabled: *enabled,
                description: String::new(),
            })
            .filter(|kv| !kv.key.is_empty())
            .collect()
    }

}
