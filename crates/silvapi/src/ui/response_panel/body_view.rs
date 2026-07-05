#![allow(unused_imports)]
use std::collections::HashSet;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::PopupMenuItem,
    resizable::{ResizableState, h_resizable, resizable_panel, v_resizable},
    scroll::ScrollableElement,
    spinner::Spinner,
    tab::{Tab, TabBar},
    text::TextView,
    v_flex,
};

use crate::state::{AppEvent, AppState};

use super::*;

impl ResponsePanel {
    /// Set the response editor's text and force the wrap layout to recompute in
    /// the same frame. `set_value` alone only refreshes `wrap_width` on the next
    /// paint, so wrapped text otherwise appears stale until an interaction (e.g.
    /// moving the cursor) forces a relayout. The response editor keeps its
    /// measured width across updates, so forcing wrap here is safe.
    pub(super) fn set_response_text(&self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let soft_wrap = self.soft_wrap && !self.response_editor_plain;
        self.response_editor.update(cx, |state, cx| {
            state.set_value(text, window, cx);
            state.set_soft_wrap(soft_wrap, window, cx);
        });
    }

    pub(super) fn ensure_response_editor_mode(
        &mut self,
        plain: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.response_editor_plain == plain {
            return;
        }

        self.response_editor_plain = plain;
        let soft_wrap = self.soft_wrap && !plain;
        self.response_editor.update(cx, |state, cx| {
            let mut next = InputState::new(window, cx)
                .multi_line(true)
                .searchable(true)
                .replaceable(false)
                .placeholder("Response will appear here... (Ctrl+F to search)")
                .soft_wrap(soft_wrap);
            if !plain {
                next = next.code_editor("json");
            }
            *state = next;
        });
    }

    pub(super) fn render_loading_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let elapsed_ms = self.app_state.read(cx).loading_elapsed_ms();
        let received = self
            .app_state
            .read(cx)
            .response
            .as_ref()
            .map(|response| response.formatted_size())
            .unwrap_or_else(|| "0 B".to_string());

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .text_color(cx.theme().muted_foreground)
            .child(Spinner::new().color(cx.theme().primary))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .text_sm()
                    .child("Receiving response")
                    .when_some(elapsed_ms, |el, elapsed_ms| {
                        el.child(dot_separator(cx))
                            .child(format_duration(elapsed_ms))
                    })
                    .child(dot_separator(cx))
                    .child(received),
            )
    }

    pub(super) fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (
            is_loading,
            loading_elapsed_ms,
            loading_size,
            has_stream_body,
            response_summary,
            has_response,
            has_error,
        ) = {
            let state = self.app_state.read(cx);
            let response = state.response.as_ref();
            (
                state.is_loading,
                state.loading_elapsed_ms(),
                response
                    .map(|resp| resp.formatted_size())
                    .unwrap_or_else(|| "0 B".to_string()),
                response.map_or(false, |resp| !resp.body.is_empty()),
                (!state.is_loading).then(|| response).flatten().map(|resp| {
                    (
                        resp.status,
                        resp.status_text.clone(),
                        resp.time_ms,
                        resp.formatted_size(),
                        resp.is_success(),
                        resp.is_redirect(),
                    )
                }),
                response.is_some(),
                state.error.is_some(),
            )
        };
        let has_nothing = !is_loading && !has_response && !has_error;

        h_flex()
            .px_3()
            .py_2()
            .items_center()
            .min_h(px(40.))
            .border_b_1()
            .border_color(cx.theme().border)
            .justify_between()
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .when(is_loading, |el| {
                        el.child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(div().text_sm().text_color(cx.theme().muted_foreground).child(
                                    if has_stream_body {
                                        "Receiving response..."
                                    } else {
                                        "Sending request..."
                                    },
                                ))
                                .when_some(loading_elapsed_ms, |el, elapsed_ms| {
                                    el.child(dot_separator(cx)).child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().foreground)
                                            .child(format_duration(elapsed_ms)),
                                    )
                                })
                                .child(dot_separator(cx))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().foreground)
                                        .child(loading_size),
                                )
                                .child(
                                    Button::new("cancel-resp")
                                        .label("Cancel")
                                        .danger()
                                        .xsmall()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.app_state.update(cx, |state, cx| {
                                                state.cancel_active_request();
                                                cx.emit(AppEvent::LoadingChanged);
                                            });
                                        })),
                                ),
                        )
                    })
                    .when_some(response_summary, |el, resp| {
                        let (status, status_text, time_ms, size, is_success, is_redirect) = resp;
                        let status_color: Hsla = if is_success {
                            rgb(0x4CAF50).into()
                        } else if is_redirect {
                            rgb(0xFF9800).into()
                        } else {
                            rgb(0xF44336).into()
                        };

                        el.child(
                            h_flex()
                                .gap_3()
                                .items_center()
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .font_weight(FontWeight::BOLD)
                                                .text_sm()
                                                .text_color(status_color)
                                                .child(status.to_string()),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(single_line(status_text)),
                                        ),
                                )
                                .child(dot_separator(cx))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().foreground)
                                        .child(format_duration(time_ms)),
                                )
                                .child(dot_separator(cx))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().foreground)
                                        .child(size),
                                )
                        )
                    })
                    .when(has_nothing, |el| {
                        el.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .whitespace_normal()
                                .text_center()
                                .child("Send a request to see the response"),
                        )
                    })
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .when(has_response, |el| {
                        el.child({
                            let app_state_clone = self.app_state.clone();
                            DropdownButton::new("history-dropdown")
                                .button(Button::new("history-btn").label("History").ghost())
                                .dropdown_menu(move |mut menu, _, cx| {
                                    let state = app_state_clone.read(cx);
                                    let id = if let Some(id) = &state.active_request_id {
                                        id.clone()
                                    } else {
                                        return menu;
                                    };
                                    let history_len = state
                                        .workspace
                                        .response_cache
                                        .get(&id)
                                        .map(|history| history.len())
                                        .unwrap_or(0);
                                    let app_state_clone2 = app_state_clone.clone();

                                    menu = menu.item(
                                        PopupMenuItem::new(format!("Delete {} Responses", history_len))
                                            .on_click(move |_, _, cx| {
                                                let mut request_id = None;
                                                app_state_clone2.update(cx, |s, cx| {
                                                    if let Some(id) = &s.active_request_id {
                                                        request_id = Some(id.clone());
                                                        s.workspace.response_cache.remove(id);
                                                        s.response = None;
                                                    }
                                                    cx.emit(AppEvent::ResponseReceived);
                                                });
                                                if let Some(id) = request_id {
                                                    cx.background_executor()
                                                        .spawn(async move {
                                                            let _ = crate::storage::delete_response_history(&id);
                                                        })
                                                        .detach();
                                                }
                                            }),
                                    );

                                    menu = menu.separator();

                                    let app_state_clone3 = app_state_clone.clone();
                                    if let Some(history) = state.workspace.response_cache.get(&id) {
                                        for (idx, resp) in history.iter().enumerate().rev() {
                                            let app_state_clone4 = app_state_clone3.clone();
                                            let label = format!(
                                                "{} -> {}",
                                                resp.status,
                                                format_duration(resp.time_ms)
                                            );
                                            menu = menu.item(PopupMenuItem::new(label).on_click(
                                                move |_, _, cx| {
                                                    app_state_clone4.update(cx, |s, cx| {
                                                        if let Some(id) = &s.active_request_id {
                                                            if let Some(history) =
                                                                s.workspace.response_cache.get(id)
                                                            {
                                                                if let Some(r) = history.get(idx) {
                                                                    s.response = Some(r.clone());
                                                                    cx.emit(AppEvent::ResponseReceived);
                                                                }
                                                            }
                                                        }
                                                    });
                                                },
                                            ));
                                        }
                                    }
                                    menu
                                })
                        })
                        .child(
                            Button::new("response-wrap-toggle")
                                .icon(if self.soft_wrap {
                                    IconName::ArrowLeft
                                } else {
                                    IconName::ChevronsUpDown
                                })
                                .tooltip(if self.soft_wrap {
                                    "Disable wrap"
                                } else {
                                    "Enable wrap"
                                })
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    // Toggle wrap in place: recompute the wrap
                                    // layout directly instead of rebuilding the
                                    // whole editor + re-setting its value.
                                    this.soft_wrap = !this.soft_wrap;
                                    let soft_wrap = this.soft_wrap && !this.response_editor_plain;
                                    this.response_editor.update(cx, |state, cx| {
                                        state.set_soft_wrap(soft_wrap, window, cx);
                                    });
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        Button::new("response-layout-toggle")
                            .icon(IconName::PanelRightOpen)
                            .tooltip("Toggle response layout")
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.app_state.update(cx, |_, cx| {
                                    cx.emit(AppEvent::ToggleLayout);
                                });
                            })),
                    )
            )
    }

}
