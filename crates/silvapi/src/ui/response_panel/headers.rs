#![allow(unused_imports)]
use std::collections::HashSet;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Sizable as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex,
    input::{Editor, Input, InputEvent, InputState},
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
    pub(super) fn render_headers_tab(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (sections, source_text) = {
            let state = self.app_state.read(cx);
            let request_info = current_header_request_info(&state);
            let sections = build_header_sections(request_info.clone(), state.response.as_ref());
            let source_text = if self.headers_source_view {
                format_headers_dump(request_info, state.response.as_ref())
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
                                .icon(IconName::FileText)
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
                                    this.headers_editor.update(cx, |editor, cx| {
                                        editor.set_soft_wrap(wrap, window, cx);
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
                    .child(
                        Editor::new(&self.headers_editor)
                            .text_size(cx.theme().font_size)
                            .h(relative(1.0))
                            .bordered(false)
                            .appearance(false)
                            .readonly(true)
                            .context_menu(crate::ui::editor_utils::readonly_editor_context_menu),
                    )
                    .into_any_element()
            } else {
                table.into_any_element()
            })
    }

    pub(super) fn current_headers_source_text(&self, cx: &mut Context<Self>) -> String {
        let state = self.app_state.read(cx);

        format_headers_dump(current_header_request_info(&state), state.response.as_ref())
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

fn current_header_request_info(state: &AppState) -> Option<HeaderRequestInfo> {
    if let Some(snapshot) = state
        .response
        .as_ref()
        .and_then(|response| response.request.as_ref())
    {
        return Some(HeaderRequestInfo {
            method: snapshot.method.as_str().to_string(),
            url: snapshot.url.clone(),
            resolved_url: Some(snapshot.resolved_url.clone()),
            headers: snapshot.headers.clone(),
            body_type: snapshot.body_type.clone(),
            body: snapshot.body.clone(),
            params: snapshot.params.clone(),
            urlencoded: snapshot.urlencoded.clone(),
            form_data: snapshot.form_data.clone(),
            auth: snapshot.auth.clone(),
            content_type: snapshot.content_type.clone(),
            body_bytes: snapshot.body_bytes,
            body_truncated: snapshot.body_truncated,
        });
    }

    // No snapshot yet (nothing sent, or a pre-snapshot build): describe the
    // request as currently configured.
    state
        .active_request
        .as_ref()
        .map(|request| HeaderRequestInfo {
            method: request.method.as_str().to_string(),
            url: request.url.clone(),
            resolved_url: Some(state.interpolate_variables(&request.url)),
            headers: request.headers.clone(),
            body_type: request.body.body_type.clone(),
            body: request.body.content.clone(),
            params: request.params.clone(),
            urlencoded: request.body.urlencoded.clone(),
            form_data: request
                .body
                .form_data
                .iter()
                .map(silvapi_core::models::FormDataPartSnapshot::from_part)
                .collect(),
            auth: Some(silvapi_core::models::AuthSnapshot::from_config(
                &request.auth,
            )),
            content_type: None,
            body_bytes: request.body.content.len(),
            body_truncated: false,
        })
}
