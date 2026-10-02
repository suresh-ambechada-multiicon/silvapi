#![allow(unused_imports)]
use std::time::Duration;
use instant::Instant;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_kit::component::{
    ActiveTheme, IconName, Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    checkbox::Checkbox,
    h_flex,
    input::{Editor, Input, InputEvent, InputState},
    menu::PopupMenuItem,
    tab::{Tab, TabBar},
    v_flex,
};

use crate::state::{AppEvent, AppState};
use silvapi_core::models::{
    AuthConfig, AuthType, BodyType, FormDataPart, FormDataPartKind, HttpMethod, KeyValue,
};

use super::*;

impl RequestPanel {
    pub(super) fn make_urlencoded_row(
        &mut self,
        key: &str,
        value: &str,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<InputState>, Entity<InputState>, bool) {
        let k = cx.new(|cx| InputState::new(window, cx).placeholder("entry_name"));
        let v = cx.new(|cx| InputState::new(window, cx).placeholder("value"));
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

    pub(super) fn make_multipart_row(
        &mut self,
        part: &FormDataPart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> MultipartRow {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("name"));
        let value_placeholder = match part.kind {
            FormDataPartKind::Text => "value",
            FormDataPartKind::File => "file path",
        };
        let value = cx.new(|cx| InputState::new(window, cx).placeholder(value_placeholder));
        name.update(cx, |s, cx| {
            s.set_value(Self::input_line(&part.name), window, cx)
        });
        value.update(cx, |s, cx| {
            s.set_value(Self::input_line(&part.value), window, cx)
        });

        let kind_idx = match part.kind {
            FormDataPartKind::Text => 0,
            FormDataPartKind::File => 1,
        };
        let kind_select = cx.new(|cx| {
            SelectState::new(
                vec!["Text", "File"],
                Some(IndexPath::new(kind_idx)),
                window,
                cx,
            )
            .searchable(false)
        });

        let name_sub = cx.subscribe_in(&name, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        });
        let value_sub = cx.subscribe_in(&value, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.sync_active_request(window, cx);
            }
        });

        let part_id = part.id.clone();
        let kind_sub = cx.subscribe_in(&kind_select, window, {
            let part_id = part_id.clone();
            move |this, _, ev: &SelectEvent<Vec<&'static str>>, window, cx| {
                if let SelectEvent::Confirm(Some(kind_str)) = ev {
                    let kind = match *kind_str {
                        "File" => FormDataPartKind::File,
                        _ => FormDataPartKind::Text,
                    };
                    Self::update_multipart_part(&this.app_state, &part_id, cx, |part| {
                        part.kind = kind;
                    });
                    this.sync_active_request(window, cx);
                }
            }
        });

        self.row_subs.push(name_sub);
        self.row_subs.push(value_sub);
        self.row_subs.push(kind_sub);

        MultipartRow {
            id: part.id.clone(),
            name,
            value,
            kind_select,
        }
    }

    pub(super) fn body_type_label(body_type: &BodyType) -> &'static str {
        match body_type {
            BodyType::FormData => "Multi-Part",
            BodyType::UrlEncoded => "Url Encoded",
            BodyType::Json => "JSON",
            BodyType::Raw => "Other",
            BodyType::BinaryFile => "Binary File",
            BodyType::None => "No Body",
        }
    }

    pub(super) fn is_text_body_type(body_type: &BodyType) -> bool {
        matches!(body_type, BodyType::Json | BodyType::Raw)
    }

    pub(super) fn empty_body_label(body_type: &BodyType) -> &'static str {
        match body_type {
            BodyType::FormData => "No multipart fields",
            BodyType::BinaryFile => "No file selected",
            _ => "No body",
        }
    }


    pub(super) fn render_body_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body_type = self
            .app_state
            .read(cx)
            .active_request
            .as_ref()
            .map(|r| r.body.body_type.clone())
            .unwrap_or(BodyType::None);
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .px_3()
                    .py_1p5()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .w(px(130.))
                            .child(
                                Select::new(&self.body_type_select)
                                    .small()
                            ),
                    )
                    .when(Self::is_text_body_type(&body_type), |el| {
                        let wrap_on = self.body_wrap;
                        el.child(
                            h_flex()
                                .flex_1()
                                .justify_end()
                                .gap_1()
                                .child(
                                    Button::new("wrap-body-btn")
                                        .icon(IconName::Menu)
                                        .tooltip(if wrap_on {
                                            "Word wrap: on"
                                        } else {
                                            "Word wrap: off"
                                        })
                                        .ghost()
                                        .xsmall()
                                        .when(wrap_on, |b| b.selected(true))
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.body_wrap = !this.body_wrap;
                                            let wrap = this.body_wrap;
                                            this.body_editor.update(cx, |editor, cx| {
                                                editor.set_soft_wrap(wrap, window, cx);
                                            });
                                            cx.notify();
                                        })),
                                )
                                .when(matches!(body_type, BodyType::Json), |el| {
                                    el.child(
                                        Button::new("format-json-btn")
                                            .label("Format JSON")
                                            .ghost()
                                            .xsmall()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let text =
                                                    this.body_editor.read(cx).value().to_string();
                                                if let Ok(parsed) =
                                                    serde_json::from_str::<serde_json::Value>(&text)
                                                {
                                                    if let Ok(pretty) =
                                                        serde_json::to_string_pretty(&parsed)
                                                    {
                                                        this.body_editor.update(
                                                            cx,
                                                            |editor, cx| {
                                                                editor
                                                                    .set_value(pretty, window, cx);
                                                            },
                                                        );
                                                        this.sync_active_request(window, cx);
                                                        cx.notify();
                                                    }
                                                }
                                            })),
                                    )
                                }),
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .min_h(px(0.))
                    .overflow_hidden()
                    .when(matches!(body_type, BodyType::FormData), |el| {
                        el.child(self.render_multipart_form(cx))
                    })
                    .when(matches!(body_type, BodyType::UrlEncoded), |el| {
                        el.child(self.render_urlencoded_form(cx))
                    })
                    .when(Self::is_text_body_type(&body_type), |el| {
                        el.child(
                            Editor::new(&self.body_editor)
                                .text_size(cx.theme().font_size)
                                .h(relative(1.0))
                                .bordered(false)
                                .appearance(false)
                                .context_menu(crate::ui::editor_utils::editor_context_menu),
                        )
                    })
                    .when(
                        !Self::is_text_body_type(&body_type)
                            && !matches!(body_type, BodyType::UrlEncoded)
                            && !matches!(body_type, BodyType::FormData),
                        |el| {
                            el.child(
                                div()
                                    .flex()
                                    .size_full()
                                    .items_center()
                                    .justify_center()
                                    .text_color(cx.theme().muted_foreground)
                                    .text_sm()
                                    .child(Self::empty_body_label(&body_type)),
                            )
                        },
                    ),
            )
    }

    pub(super) fn render_urlencoded_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, (key, val, enabled)) in self.urlencoded_rows.iter().enumerate() {
            rows.push(
                self.render_urlencoded_row(i, key, val, *enabled, cx)
                    .into_any_element(),
            );
        }

        div()
            .id("urlencoded-scroll")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .child(urlencoded_header(cx))
            .children(rows)
            .child(
                Button::new("add-urlencoded")
                    .icon(IconName::Plus)
                    .label("Add")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|this, _, window, cx| {
                        let row = this.make_urlencoded_row("", "", true, window, cx);
                        this.urlencoded_rows.push(row);
                        this.sync_active_request(window, cx);
                        cx.notify();
                    })),
            )
    }

    pub(super) fn render_urlencoded_row(
        &self,
        i: usize,
        key: &Entity<InputState>,
        val: &Entity<InputState>,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .py_0p5()
            .child(
                Checkbox::new(format!("chk-urlencoded-{}", i))
                    .checked(enabled)
                    .on_click(cx.listener(move |this, enabled: &bool, window, cx| {
                        if let Some(row) = this.urlencoded_rows.get_mut(i) {
                            row.2 = *enabled;
                            this.sync_active_request(window, cx);
                            cx.notify();
                        }
                    })),
            )
            .child(div().flex_1().child(Input::new(key)))
            .child(div().flex_1().child(Input::new(val)))
            .child(
                Button::new(format!("del-urlencoded-row-{}", i))
                    .icon(IconName::Close)
                    .tooltip("Remove row")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if i < this.urlencoded_rows.len() {
                            this.urlencoded_rows.remove(i);
                        }
                        this.sync_active_request(window, cx);
                        cx.notify();
                    })),
            )
    }

    pub(super) fn render_multipart_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let parts = self
            .app_state
            .read(cx)
            .active_request
            .as_ref()
            .map(|req| req.body.form_data.clone())
            .unwrap_or_default();

        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, row) in self.multipart_rows.iter().enumerate() {
            let part = parts
                .iter()
                .find(|part| part.id == row.id)
                .cloned()
                .unwrap_or_else(|| FormDataPart {
                    id: row.id.clone(),
                    name: row.name.read(cx).value().to_string(),
                    value: row.value.read(cx).value().to_string(),
                    enabled: true,
                    kind: FormDataPartKind::Text,
                    content_type: String::new(),
                });
            rows.push(
                self.render_multipart_row(i, row, &part, cx)
                    .into_any_element(),
            );
        }

        div()
            .id("multipart-scroll")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .child(multipart_header(cx))
            .children(rows)
            .child(
                Button::new("add-multipart")
                    .icon(IconName::Plus)
                    .label("Add")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|this, _, window, cx| {
                        let part = FormDataPart::empty();
                        this.app_state.update(cx, |state, cx| {
                            if let Some(req) = &mut state.active_request {
                                req.body.body_type = BodyType::FormData;
                                req.body.form_data.push(part.clone());
                            }
                            state.save_active_request();
                            cx.emit(AppEvent::SaveNeeded);
                        });
                        let row = this.make_multipart_row(&part, window, cx);
                        this.multipart_rows.push(row);
                        cx.notify();
                    })),
            )
    }

    pub(super) fn render_multipart_row(
        &self,
        i: usize,
        row: &MultipartRow,
        part: &FormDataPart,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let row_id = row.id.clone();
        let enabled = part.enabled;
        let kind = part.kind.clone();
        let is_file = matches!(kind, FormDataPartKind::File);
        let app_state_for_menu = self.app_state.clone();
        let menu_row_id = row.id.clone();
        let content_type = part.content_type.clone();
        let picker_app_state = self.app_state.clone();
        let picker_row_id = row.id.clone();
        let picker_value = row.value.clone();
        let delete_row_id = row.id.clone();

        h_flex()
            .gap_2()
            .items_center()
            .py_0p5()
            .child(
                Checkbox::new(format!("chk-multipart-{}", row_id))
                    .checked(enabled)
                    .on_click(cx.listener(move |this, enabled: &bool, _, cx| {
                        Self::update_multipart_part(&this.app_state, &row_id, cx, |part| {
                            part.enabled = *enabled;
                        });
                    })),
            )
            .child(div().flex_1().child(Input::new(&row.name)))
            .child(
                h_flex()
                    .flex_1()
                    .gap_1()
                    .child(div().flex_1().child(Input::new(&row.value)))
                    .when(is_file, |el| {
                        el.child(
                            Button::new(format!("multipart-file-picker-{}", i))
                                .icon(IconName::FolderOpen)
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    let rx = cx.prompt_for_paths(PathPromptOptions {
                                        files: true,
                                        directories: false,
                                        multiple: false,
                                        prompt: Some("Select multipart file".into()),
                                    });
                                    let app_state = picker_app_state.clone();
                                    let row_id = picker_row_id.clone();
                                    let value_input = picker_value.clone();
                                    cx.spawn_in(window, async move |_, window| {
                                        let path = rx.await.ok()?.ok()??.iter().next()?.clone();
                                        let path = path.to_string_lossy().to_string();
                                        window
                                            .update(|window, cx| {
                                                value_input.update(cx, |state, cx| {
                                                    state.set_value(path.clone(), window, cx);
                                                });
                                                app_state.update(cx, |state, cx| {
                                                    if let Some(req) = &mut state.active_request {
                                                        if let Some(part) =
                                                            req.body.form_data.iter_mut().find(
                                                                |part| part.id == row_id.as_str(),
                                                            )
                                                        {
                                                            part.kind = FormDataPartKind::File;
                                                            part.value = path.clone();
                                                        }
                                                    }
                                                    state.save_active_request();
                                                    cx.emit(AppEvent::SaveNeeded);
                                                });
                                            })
                                            .ok();

                                        Some(())
                                    })
                                    .detach();
                                })),
                        )
                    }),
            )
            .when(!content_type.is_empty(), |el| {
                el.child(
                    div()
                        .max_w(px(150.))
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(content_type.replace(['\r', '\n'], " ")),
                )
            })
            .child(
                div()
                    .w(px(85.))
                    .child(
                        Select::new(&row.kind_select)
                            .small()
                    ),
            )
            .child(
                DropdownButton::new(format!("multipart-opts-{}", i))
                    .button(
                        Button::new(format!("multipart-opts-btn-{}", i))
                            .icon(IconName::Ellipsis)
                            .ghost()
                            .xsmall()
                            .tooltip("Options"),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        let set_content_type_state = app_state_for_menu.clone();
                        let set_content_type_id = menu_row_id.clone();
                        let unset_content_type_state = app_state_for_menu.clone();
                        let unset_content_type_id = menu_row_id.clone();
                        let unset_file_state = app_state_for_menu.clone();
                        let unset_file_id = menu_row_id.clone();

                        menu.item(
                            PopupMenuItem::new("Set Content-Type").on_click(move |_, _, cx| {
                                Self::update_multipart_part(
                                    &set_content_type_state,
                                    &set_content_type_id,
                                    cx,
                                    |part| {
                                        part.content_type = match part.kind {
                                            FormDataPartKind::Text => "text/plain".to_string(),
                                            FormDataPartKind::File => {
                                                "application/octet-stream".to_string()
                                            }
                                        };
                                    },
                                );
                            }),
                        )
                        .item(
                            PopupMenuItem::new("Unset Content-Type").on_click(move |_, _, cx| {
                                Self::update_multipart_part(
                                    &unset_content_type_state,
                                    &unset_content_type_id,
                                    cx,
                                    |part| {
                                        part.content_type.clear();
                                    },
                                );
                            }),
                        )
                        .item(PopupMenuItem::new("Unset File").on_click(move |_, _, cx| {
                            Self::update_multipart_part(
                                &unset_file_state,
                                &unset_file_id,
                                cx,
                                |part| {
                                    if matches!(part.kind, FormDataPartKind::File) {
                                        part.value.clear();
                                    }
                                },
                            );
                        }))
                    }),
            )
            .child(
                Button::new(format!("del-multipart-row-{}", i))
                    .icon(IconName::Close)
                    .tooltip("Remove row")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        Self::delete_multipart_part(&this.app_state, &delete_row_id, cx);
                    })),
            )
    }

    pub(super) fn update_multipart_part<F>(
        app_state: &Entity<AppState>,
        id: &str,
        cx: &mut App,
        f: F,
    ) where
        F: FnOnce(&mut FormDataPart),
    {
        app_state.update(cx, |state, cx| {
            if let Some(req) = &mut state.active_request {
                if let Some(part) = req.body.form_data.iter_mut().find(|part| part.id == id) {
                    f(part);
                    if part.kind == FormDataPartKind::File && part.content_type == "text/plain" {
                        part.content_type.clear();
                    }
                }
            }
            state.save_active_request();
            cx.emit(AppEvent::WorkspaceChanged);
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    pub(super) fn delete_multipart_part(app_state: &Entity<AppState>, id: &str, cx: &mut App) {
        app_state.update(cx, |state, cx| {
            if let Some(req) = &mut state.active_request {
                req.body.form_data.retain(|part| part.id != id);
            }
            state.save_active_request();
            cx.emit(AppEvent::WorkspaceChanged);
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    pub(super) fn urlencoded_fields_for_body(
        body: &silvapi_core::models::RequestBody,
    ) -> Vec<KeyValue> {
        if !body.urlencoded.is_empty() {
            body.urlencoded.clone()
        } else if matches!(body.body_type, BodyType::UrlEncoded) {
            Self::parse_urlencoded_content(&body.content)
        } else {
            Vec::new()
        }
    }

    pub(super) fn parse_urlencoded_content(content: &str) -> Vec<KeyValue> {
        if content.contains(['\r', '\n']) {
            return Vec::new();
        }

        content
            .split('&')
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                let (key, value) = entry.split_once('=').unwrap_or((entry, ""));
                KeyValue::new(
                    Self::input_line(Self::decode_urlencoded_component(key)),
                    Self::input_line(Self::decode_urlencoded_component(value)),
                )
            })
            .filter(|kv| !kv.key.is_empty() || !kv.value.is_empty())
            .collect()
    }

    pub(super) fn decode_urlencoded_component(value: &str) -> String {
        let bytes = value.as_bytes();
        let mut decoded = Vec::with_capacity(bytes.len());
        let mut i = 0;

        while i < bytes.len() {
            match bytes[i] {
                b'+' => {
                    decoded.push(b' ');
                    i += 1;
                }
                b'%' if i + 2 < bytes.len() => {
                    if let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2]))
                    {
                        decoded.push((hi << 4) | lo);
                        i += 3;
                    } else {
                        decoded.push(bytes[i]);
                        i += 1;
                    }
                }
                byte => {
                    decoded.push(byte);
                    i += 1;
                }
            }
        }

        String::from_utf8_lossy(&decoded).to_string()
    }

    pub(super) fn collect_multipart(
        rows: &[MultipartRow],
        body: &silvapi_core::models::RequestBody,
        cx: &App,
    ) -> Vec<FormDataPart> {
        rows.iter()
            .map(|row| {
                let existing = body.form_data.iter().find(|part| part.id == row.id);
                FormDataPart {
                    id: row.id.clone(),
                    name: row.name.read(cx).value().to_string(),
                    value: row.value.read(cx).value().to_string(),
                    enabled: existing.map(|part| part.enabled).unwrap_or(true),
                    kind: existing
                        .map(|part| part.kind.clone())
                        .unwrap_or(FormDataPartKind::Text),
                    content_type: existing
                        .map(|part| part.content_type.clone())
                        .unwrap_or_default(),
                }
            })
            .filter(|part| !part.name.is_empty() || !part.value.is_empty())
            .collect()
    }

    pub(super) fn append_stream_batch(
        app_state: &Entity<AppState>,
        request_id: &Option<String>,
        run_id: u64,
        body: String,
        chunk_count: usize,
        cx: &mut AsyncApp,
    ) {
        if body.is_empty() || chunk_count == 0 {
            return;
        }

        let byte_count = body.len();
        let request_id = request_id.clone();
        let _ = cx.update(|cx| {
            app_state.update(cx, |state, cx| {
                if let Some(id) = &request_id {
                    if !state.is_request_run_current(id, run_id) {
                        return;
                    }
                }
                if !state.is_active_response_target(&request_id) {
                    return;
                }
                if let Some(resp) = &mut state.response {
                    resp.body.push_str(&body);
                    resp.size_bytes = resp.body.len();
                    resp.timeline.push(silvapi_core::models::TimelineEvent {
                        name: format!("{} chunks received ({} B)", chunk_count, byte_count),
                        timestamp: chrono::Local::now().format("%H:%M:%S.%3f").to_string(),
                        icon: silvapi_core::models::TimelineIcon::Info,
                        detail: None,
                    });
                    cx.emit(AppEvent::StreamChunk(body));
                }
            });
        });
    }
}
