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
    pub(super) fn render_headers_tab(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (sections, source_text) = {
            let state = self.app_state.read(cx);
            let resolved_url = state
                .active_request
                .as_ref()
                .map(|req| state.interpolate_variables(&req.url));
            let sections = build_header_sections(
                state.active_request.as_ref(),
                state.response.as_ref(),
                resolved_url.as_deref(),
            );
            let source_text = if self.headers_source_view {
                format_headers_dump(
                    state.active_request.as_ref(),
                    state.response.as_ref(),
                    resolved_url.as_deref(),
                )
            } else {
                String::new()
            };
            (sections, source_text)
        };

        if self.headers_source_view {
            self.headers_editor.update(cx, |state, cx| {
                if state.value() != source_text {
                    state.set_value(source_text, window, cx);
                }
            });
        }

        let table = v_flex()
            .size_full()
            .overflow_y_scrollbar()
            .px_3()
            .pt_2()
            // Extra bottom padding so the last row clears the panel edge and
            // isn't clipped in half.
            .pb_8()
            .gap_3()
            .children(sections.into_iter().map(|section| {
                self.render_header_section(section.title, section.rows, cx)
                    .into_any_element()
            }));

        v_flex()
            .id("resp-headers-tab")
            .size_full()
            .child(
                h_flex()
                    .justify_end()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .when(self.headers_source_view, |el| {
                        el.child(
                            Button::new("headers-wrap-toggle")
                                .icon(if self.headers_soft_wrap {
                                    IconName::ArrowLeft
                                } else {
                                    IconName::ChevronsUpDown
                                })
                                .tooltip(if self.headers_soft_wrap {
                                    "Disable wrap"
                                } else {
                                    "Enable wrap"
                                })
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.headers_soft_wrap = !this.headers_soft_wrap;
                                    let wrap = this.headers_soft_wrap;
                                    this.headers_editor.update(cx, |state, cx| {
                                        state.set_soft_wrap(wrap, window, cx);
                                    });
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        Button::new("headers-source-toggle")
                            .label(if self.headers_source_view {
                                "view parsed"
                            } else {
                                "view source"
                            })
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.headers_source_view = !this.headers_source_view;
                                cx.notify();
                            })),
                    ),
            )
            .child(if self.headers_source_view {
                div()
                    .id("resp-headers-editor")
                    .flex_1()
                    .p_1()
                    .child(Input::new(&self.headers_editor).h_full().w_full())
                    .into_any_element()
            } else {
                table.into_any_element()
            })
    }

    pub(super) fn current_headers_source_text(&self, cx: &mut Context<Self>) -> String {
        let state = self.app_state.read(cx);
        let resolved_url = state
            .active_request
            .as_ref()
            .map(|req| state.interpolate_variables(&req.url));

        format_headers_dump(
            state.active_request.as_ref(),
            state.response.as_ref(),
            resolved_url.as_deref(),
        )
    }

    pub(super) fn render_header_section(
        &self,
        title: &'static str,
        rows: Vec<(String, String)>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let collapsed = self.collapsed_header_sections.contains(title);

        v_flex()
            .w_full()
            .border_b_1()
            .border_color(cx.theme().border)
            .pb_2()
            .child(
                h_flex()
                    .h(px(26.))
                    .items_center()
                    .gap_1()
                    .cursor_pointer()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().foreground)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            if !this.collapsed_header_sections.remove(title) {
                                this.collapsed_header_sections.insert(title);
                            }
                            cx.notify();
                        }),
                    )
                    .child(
                        Icon::new(if collapsed {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        })
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                    )
                    .child(title),
            )
            .when(!collapsed, |el| {
                el.children(rows.into_iter().enumerate().map(|(idx, (key, value))| {
                    h_flex()
                        .w_full()
                        .items_start()
                        .gap_4()
                        .py_1()
                        .px_2()
                        .border_b_1()
                        .border_color(cx.theme().border.opacity(0.28))
                        .when(idx % 2 == 1, |row| row.bg(cx.theme().muted.opacity(0.12)))
                        .child(
                            div()
                                .w(px(190.))
                                .flex_shrink_0()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(single_line(key)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .whitespace_normal()
                                .child(single_line(value)),
                        )
                }))
            })
    }

}
