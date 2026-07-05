#![allow(unused_imports)]
use std::collections::HashSet;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex,
    input::{Input, InputEvent},
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
    pub(super) fn render_timeline_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let timeline = self
            .app_state
            .read(cx)
            .response
            .as_ref()
            .map(|r| r.timeline.clone())
            .unwrap_or_default();

        let selected_idx = self.selected_timeline_event;
        let events_len = timeline.len();

        let list = uniform_list(
            "timeline-scroll",
            events_len,
            cx.processor(move |_this: &mut Self, range, _window, cx| {
                let mut children = Vec::new();
                for idx in range {
                    if let Some(ev) = timeline.get(idx) {
                        let selected = selected_idx == Some(idx);

                        let (icon, color) = match ev.icon {
                            silvapi_core::models::TimelineIcon::Setting => {
                                (IconName::Settings, cx.theme().muted_foreground)
                            }
                            silvapi_core::models::TimelineIcon::Request => {
                                (IconName::ArrowUp, cx.theme().primary)
                            }
                            silvapi_core::models::TimelineIcon::Response => {
                                (IconName::ArrowDown, cx.theme().primary)
                            }
                            silvapi_core::models::TimelineIcon::Info => {
                                (IconName::Info, cx.theme().muted_foreground)
                            }
                        };

                        let bg_color = if selected {
                            cx.theme().primary.opacity(0.1)
                        } else {
                            gpui::transparent_black()
                        };
                        let border = if selected {
                            gpui::transparent_black()
                        } else {
                            cx.theme().border.opacity(0.5)
                        };

                        children.push(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .py_1p5()
                                .px_2()
                                .rounded_md()
                                .bg(bg_color)
                                .border_b_1()
                                .border_color(border)
                                .cursor_pointer()
                                .id(idx)
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        if this.selected_timeline_event == Some(idx) {
                                            this.selected_timeline_event = None;
                                        } else {
                                            this.selected_timeline_event = Some(idx);
                                        }
                                        cx.notify();
                                    }),
                                )
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .items_center()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .flex_none()
                                                .flex()
                                                .items_center()
                                                .child(Icon::new(icon).small().text_color(color)),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_family("monospace")
                                                .text_color(cx.theme().foreground)
                                                .child(single_line(&ev.name)),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_family("monospace")
                                        .text_color(cx.theme().muted_foreground)
                                        .child(ev.timestamp.clone()),
                                )
                                .into_any_element(),
                        );
                    }
                }
                children
            }),
        )
        .track_scroll(&self.timeline_scroll)
        .size_full();

        let mut container = v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if events_len == 0 {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "up" => {
                        let mut curr = this.selected_timeline_event.unwrap_or(events_len - 1);
                        if curr > 0 {
                            curr -= 1;
                        } else {
                            curr = events_len - 1;
                        }
                        this.selected_timeline_event = Some(curr);
                        this.timeline_scroll
                            .scroll_to_item(curr, ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    "down" => {
                        let mut curr = this.selected_timeline_event.unwrap_or(0);
                        curr = (curr + 1) % events_len;
                        this.selected_timeline_event = Some(curr);
                        this.timeline_scroll
                            .scroll_to_item(curr, ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(list);

        if let Some(idx) = selected_idx {
            if let Some(ev) = self
                .app_state
                .read(cx)
                .response
                .as_ref()
                .map(|r| r.timeline.clone())
                .unwrap_or_default()
                .get(idx)
            {
                if let Some(detail) = &ev.detail {
                    let pretty_detail = if let Ok(parsed) =
                        serde_json::from_str::<serde_json::Value>(detail)
                    {
                        serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| detail.to_string())
                    } else {
                        detail.to_string()
                    };
                    let md_content = if serde_json::from_str::<serde_json::Value>(detail).is_ok() {
                        format!("```json\n{}\n```", pretty_detail)
                    } else {
                        format!("```text\n{}\n```", pretty_detail)
                    };

                    let detail_view = v_flex()
                        .h(px(250.))
                        .w_full()
                        .bg(cx.theme().background)
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .p_3()
                        .child(
                            h_flex()
                                .justify_between()
                                .w_full()
                                .items_center()
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_lg()
                                        .child(single_line(&ev.name)),
                                )
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .items_center()
                                        .child(
                                            div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(ev.timestamp.clone()),
                                        )
                                        .child(
                                            Button::new("close-detail")
                                                .icon(IconName::Close)
                                                .tooltip("Close detail")
                                                .ghost()
                                                .xsmall()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.selected_timeline_event = None;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .mt_3()
                                .size_full()
                                .id("timeline-detail-scroll")
                                .overflow_y_scroll()
                                .child(
                                    TextView::markdown("timeline-md-view", md_content)
                                        .selectable(true),
                                ),
                        );

                    container = container.child(detail_view);
                }
            }
        }

        container
    }
    pub(super) fn render_sse_tab(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text = self.response_text.clone();
        let mut events = Vec::new();
        for block in text.split("\n\n") {
            if events.len() >= SSE_MAX_EVENTS {
                break;
            }
            if block.is_empty() {
                continue;
            }
            let mut event_type = "message";
            let mut data = String::new();
            for line in block.split('\n') {
                if let Some(rest) = line.strip_prefix("event: ") {
                    event_type = rest;
                } else if let Some(rest) = line.strip_prefix("data: ") {
                    if !data.is_empty() {
                        append_sse_detail_text(&mut data, "\n");
                    }
                    append_sse_detail_text(&mut data, rest);
                }
            }
            if !data.is_empty() {
                events.push((event_type.to_string(), data));
            }
        }

        let selected_idx = self.selected_sse_event;
        let events_len = events.len();
        let selected_event_data =
            selected_idx.and_then(|idx| events.get(idx).map(|(_, data)| data.clone()));

        let list = uniform_list(
            "sse-scroll",
            events_len,
            cx.processor(move |_this: &mut Self, range, _window, cx| {
                let mut children = Vec::new();
                for idx in range {
                    if let Some((etype, data)) = events.get(idx) {
                        let selected = selected_idx == Some(idx);
                        let bg_color = if selected {
                            cx.theme().primary.opacity(0.1)
                        } else {
                            gpui::transparent_black()
                        };
                        let border = if selected {
                            gpui::transparent_black()
                        } else {
                            cx.theme().border.opacity(0.5)
                        };
                        let etype = etype.clone();
                        let data_preview = single_line_preview(data, SSE_ROW_PREVIEW_BYTES);

                        children.push(
                            h_flex()
                                .w_full()
                                .gap_3()
                                .py_1()
                                .px_2()
                                .rounded_md()
                                .bg(bg_color)
                                .border_b_1()
                                .border_color(border)
                                .cursor_pointer()
                                .id(idx)
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        if this.selected_sse_event == Some(idx) {
                                            this.selected_sse_event = None;
                                        } else {
                                            this.selected_sse_event = Some(idx);
                                        }
                                        cx.notify();
                                    }),
                                )
                                .child(
                                    div()
                                        .bg(cx.theme().muted)
                                        .px_2()
                                        .py_0p5()
                                        .rounded_md()
                                        .text_sm()
                                        .font_family("monospace")
                                        .child(idx.to_string()),
                                )
                                .child(
                                    div()
                                        .bg(cx.theme().muted)
                                        .px_2()
                                        .py_0p5()
                                        .rounded_md()
                                        .text_sm()
                                        .font_family("monospace")
                                        .child(single_line(etype)),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_sm()
                                        .font_family("monospace")
                                        .text_color(cx.theme().foreground)
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .child(single_line(data_preview)),
                                )
                                .into_any_element(),
                        );
                    }
                }
                children
            }),
        )
        .track_scroll(&self.sse_scroll)
        .size_full();

        let container = v_flex()
            .size_full()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if events_len == 0 {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "up" => {
                        let mut curr = this.selected_sse_event.unwrap_or(events_len - 1);
                        if curr > 0 {
                            curr -= 1;
                        } else {
                            curr = events_len - 1;
                        }
                        this.selected_sse_event = Some(curr);
                        this.sse_scroll
                            .scroll_to_item(curr, ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    "down" => {
                        let mut curr = this.selected_sse_event.unwrap_or(0);
                        curr = (curr + 1) % events_len;
                        this.selected_sse_event = Some(curr);
                        this.sse_scroll
                            .scroll_to_item(curr, ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(list);

        if let Some(data) = selected_event_data {
            let allow_column_detail =
                self.sse_detail_column_layout && Self::has_room_for_sse_side_detail(window);
            let detail_text = format_sse_detail_text(&data);
            let sse_wrap = self.sse_detail_soft_wrap;
            self.sse_detail_editor.update(cx, |state, cx| {
                if state.value() != detail_text {
                    // Reuse the editor (don't rebuild it) so it keeps its
                    // measured width; then force the wrap layout to recompute
                    // in the same frame — same pattern as the main response
                    // editor. Rebuilding would reset the layout to width 0 and
                    // wrap one character per line.
                    state.set_value(detail_text, window, cx);
                    state.set_soft_wrap(sse_wrap, window, cx);
                }
            });
            let detail_view = v_flex()
                .size_full()
                .bg(cx.theme().background)
                .child(
                    h_flex()
                        .h(px(44.))
                        .px_3()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .justify_between()
                        .w_full()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_lg()
                                .child("Message Received"),
                        )
                        .child(
                            h_flex()
                                .flex_none()
                                .gap_2()
                                .items_center()
                                .child(
                                    Button::new("sse-detail-layout-toggle")
                                        .icon(if allow_column_detail {
                                            IconName::PanelBottomOpen
                                        } else {
                                            IconName::PanelRightOpen
                                        })
                                        .tooltip(if allow_column_detail {
                                            "Stack message below"
                                        } else {
                                            "Show message beside list"
                                        })
                                        .ghost()
                                        .xsmall()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            let wide_enough =
                                                Self::has_room_for_sse_side_detail(window);
                                            this.sse_detail_column_layout =
                                                wide_enough && !this.sse_detail_column_layout;
                                            cx.notify();
                                            // The editor's wrap layout lags a
                                            // width change by a frame, so after
                                            // the new panel width settles, force
                                            // it to re-wrap — otherwise it briefly
                                            // shows the text unwrapped.
                                            let editor = this.sse_detail_editor.clone();
                                            let soft_wrap = this.sse_detail_soft_wrap;
                                            cx.spawn_in(window, async move |_, cx| {
                                                cx.background_executor()
                                                    .timer(std::time::Duration::from_millis(48))
                                                    .await;
                                                let _ = cx.update(|window, cx| {
                                                    editor.update(cx, |state, cx| {
                                                        state.set_soft_wrap(soft_wrap, window, cx);
                                                    });
                                                });
                                            })
                                            .detach();
                                        })),
                                )
                                .child(
                                    Button::new("sse-detail-wrap-toggle")
                                        .icon(if self.sse_detail_soft_wrap {
                                            IconName::ArrowLeft
                                        } else {
                                            IconName::ChevronsUpDown
                                        })
                                        .tooltip(if self.sse_detail_soft_wrap {
                                            "Disable wrap"
                                        } else {
                                            "Enable wrap"
                                        })
                                        .ghost()
                                        .xsmall()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            // Toggle wrap in place — recompute
                                            // the layout instead of rebuilding
                                            // the editor and re-setting its value.
                                            this.sse_detail_soft_wrap = !this.sse_detail_soft_wrap;
                                            let soft_wrap = this.sse_detail_soft_wrap;
                                            this.sse_detail_editor.update(cx, |state, cx| {
                                                state.set_soft_wrap(soft_wrap, window, cx);
                                            });
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("close-sse-detail")
                                        .icon(IconName::Close)
                                        .tooltip("Close message")
                                        .ghost()
                                        .xsmall()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.selected_sse_event = None;
                                            cx.notify();
                                        })),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        // min_w_0 so this column is bounded by its panel width
                        // immediately rather than being measured at the editor's
                        // (unwrapped) content width for a frame during a layout
                        // switch — which briefly shows the text unwrapped.
                        .min_w_0()
                        .p_3()
                        .id("sse-detail-scroll")
                        .child(Input::new(&self.sse_detail_editor).min_w_0().h_full().w_full()),
                );

            if allow_column_detail {
                return h_flex()
                    .size_full()
                    .track_focus(&self.focus_handle)
                    .child(
                        h_resizable("sse-detail-column-split")
                            .with_state(&self.sse_detail_column_resize)
                            .child(
                                resizable_panel()
                                    .size(px(440.))
                                    .size_range(px(220.)..px(1400.))
                                    .child(container),
                            )
                            .child(
                                resizable_panel()
                                    .size(px(420.))
                                    .size_range(px(280.)..px(900.))
                                    .child(detail_view),
                            ),
                    )
                    .into_any_element();
            } else {
                return div()
                    .size_full()
                    .track_focus(&self.focus_handle)
                    .child(
                        v_resizable("sse-detail-row-split")
                            .with_state(&self.sse_detail_row_resize)
                            .child(
                                resizable_panel()
                                    .size(px(440.))
                                    .size_range(px(160.)..px(1200.))
                                    .child(container),
                            )
                            .child(
                                resizable_panel()
                                    .size(px(320.))
                                    .size_range(px(180.)..px(800.))
                                    .child(detail_view),
                            ),
                    )
                    .into_any_element();
            }
        }

        container.into_any_element()
    }

    pub(super) fn current_sse_detail_text(&self) -> Option<String> {
        let selected_idx = self.selected_sse_event?;
        let mut idx = 0;
        for block in self.response_text.split("\n\n") {
            if block.is_empty() {
                continue;
            }
            if idx == selected_idx {
                let mut data = String::new();
                for line in block.split('\n') {
                    if let Some(rest) = line.strip_prefix("data: ") {
                        if !data.is_empty() {
                            data.push('\n');
                        }
                        data.push_str(rest);
                    }
                }
                return Some(format_sse_detail_text(&data));
            }
            idx += 1;
        }
        None
    }

    pub(super) fn has_room_for_sse_side_detail(window: &Window) -> bool {
        let logical_width = window.bounds().size.width / window.scale_factor();
        logical_width >= px(720.)
    }

}
