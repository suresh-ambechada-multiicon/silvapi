use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    v_flex,
};

use crate::{
    models::Variable,
    state::{AppEvent, AppState},
};

pub struct TopBar {
    app_state: Entity<AppState>,
    _subs: Vec<Subscription>,
}

struct VariableRow {
    id: String,
    name: Entity<InputState>,
    value: Entity<InputState>,
}

struct VariablesDialog {
    app_state: Entity<AppState>,
    rows: Vec<VariableRow>,
}

impl TopBar {
    pub fn new(app_state: Entity<AppState>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            app_state,
            _subs: vec![],
        }
    }
}

impl VariablesDialog {
    fn new(
        app_state: Entity<AppState>,
        vars: Vec<Variable>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = Self {
            app_state,
            rows: Vec::new(),
        };

        for var in vars {
            this.add_row(&var.name, &var.value, window, cx);
        }
        if this.rows.is_empty() {
            this.add_row("", "", window, cx);
        }

        this
    }

    fn add_row(&mut self, name: &str, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let value_input = cx.new(|cx| InputState::new(window, cx).placeholder("Value"));
        name_input.update(cx, |state, cx| {
            state.set_value(name.to_string(), window, cx);
        });
        value_input.update(cx, |state, cx| {
            state.set_value(value.to_string(), window, cx);
        });
        self.rows.push(VariableRow {
            id: uuid::Uuid::new_v4().to_string(),
            name: name_input,
            value: value_input,
        });
    }

    fn save(&self, cx: &mut Context<Self>) {
        let new_vars: Vec<Variable> = self
            .rows
            .iter()
            .filter_map(|row| {
                let name = row.name.read(cx).value().trim().to_string();
                let value = row.value.read(cx).value().to_string();
                (!name.is_empty()).then(|| Variable {
                    id: row.id.clone(),
                    name,
                    value,
                    enabled: true,
                })
            })
            .collect();

        self.app_state.update(cx, |state, cx| {
            state.workspace.variables = new_vars;
            cx.emit(AppEvent::SaveNeeded);
        });
    }
}

impl Render for VariablesDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows: Vec<AnyElement> = Vec::new();

        rows.push(
            h_flex()
                .gap_2()
                .pb_1()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .flex_1()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Variable Name"),
                )
                .child(
                    div()
                        .flex_1()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Value"),
                )
                .child(div().w(px(76.)))
                .into_any_element(),
        );

        for row in &self.rows {
            let id = row.id.clone();
            rows.push(
                h_flex()
                    .gap_2()
                    .items_center()
                    .py_0p5()
                    .child(div().flex_1().child(Input::new(&row.name)))
                    .child(div().flex_1().child(Input::new(&row.value)))
                    .child(
                        Button::new(SharedString::from(format!("remove-var-{}", id)))
                            .label("Delete")
                            .danger()
                            .small()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.rows.retain(|row| row.id != id);
                                cx.notify();
                            })),
                    )
                    .into_any_element(),
            );
        }

        v_flex()
            .p_4()
            .gap_2()
            .children(rows)
            .child(div().h(px(8.)))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("add-var-btn")
                            .icon(IconName::Plus)
                            .label("Add")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_row("", "", window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("save-vars-btn")
                            .label("Save Variables")
                            .primary()
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save(cx);
                                window.close_dialog(cx);
                                window.push_notification(
                                    gpui_component::notification::Notification::success(
                                        "Variables saved successfully!",
                                    ),
                                    cx,
                                );
                            })),
                    ),
            )
    }
}

impl Render for TopBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.app_state.clone();

        h_flex()
            .px_4()
            .py_2()
            .gap_3()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().title_bar)
            .child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child("Silvapi"),
            )
            .child(div().flex_1())
            .child(
                Button::new("import-openapi-btn")
                    .label("Import OpenAPI")
                    .ghost()
                    .small()
                    .on_click({
                        let app_state = app_state.clone();
                        cx.listener(move |_this, _, window, cx| {
                            let window_handle = window.window_handle();
                            let app_state = app_state.clone();
                            let rx = cx.prompt_for_paths(PathPromptOptions {
                                files: true,
                                directories: false,
                                multiple: false,
                                prompt: Some("Select OpenAPI spec (JSON or YAML)".into()),
                            });

                            cx.spawn(async move |_, cx| match rx.await {
                                Ok(Ok(Some(paths))) => {
                                    if let Some(path) = paths.first() {
                                        crate::ui::top_bar::handle_openapi_file(
                                            path,
                                            &app_state,
                                            cx,
                                            window_handle,
                                        );
                                    }
                                }
                                Ok(Err(e)) => {
                                    let msg = e.to_string();
                                    if let Some(path) =
                                        fallback_file_picker("Select OpenAPI spec (JSON or YAML)")
                                    {
                                        crate::ui::top_bar::handle_openapi_file(
                                            &path,
                                            &app_state,
                                            cx,
                                            window_handle,
                                        );
                                    } else {
                                        let _ = cx.update_window(
                                            window_handle,
                                            move |_, window, cx| {
                                                window.push_notification(
                                                    format!("Native dialog failed: {}", msg),
                                                    cx,
                                                );
                                            },
                                        );
                                    }
                                }
                                Err(e) => {
                                    let msg = e.to_string();
                                    if let Some(path) =
                                        fallback_file_picker("Select OpenAPI spec (JSON or YAML)")
                                    {
                                        crate::ui::top_bar::handle_openapi_file(
                                            &path,
                                            &app_state,
                                            cx,
                                            window_handle,
                                        );
                                    } else {
                                        let _ = cx.update_window(
                                            window_handle,
                                            move |_, window, cx| {
                                                window.push_notification(
                                                    format!("Native dialog failed: {}", msg),
                                                    cx,
                                                );
                                            },
                                        );
                                    }
                                }
                                _ => {}
                            })
                            .detach();
                        })
                    }),
            )
            .child(
                Button::new("import-postman-btn")
                    .label("Import Postman")
                    .ghost()
                    .small()
                    .on_click({
                        let app_state = app_state.clone();
                        cx.listener(move |_this, _, window, cx| {
                            let window_handle = window.window_handle();
                            let app_state = app_state.clone();
                            let rx = cx.prompt_for_paths(PathPromptOptions {
                                files: true,
                                directories: false,
                                multiple: false,
                                prompt: Some("Select Postman Collection JSON".into()),
                            });

                            cx.spawn(async move |_, cx| match rx.await {
                                Ok(Ok(Some(paths))) => {
                                    if let Some(path) = paths.first() {
                                        crate::ui::top_bar::handle_postman_file(
                                            path,
                                            &app_state,
                                            cx,
                                            window_handle,
                                        );
                                    }
                                }
                                Ok(Err(e)) => {
                                    let msg = e.to_string();
                                    if let Some(path) =
                                        fallback_file_picker("Select Postman Collection JSON")
                                    {
                                        crate::ui::top_bar::handle_postman_file(
                                            &path,
                                            &app_state,
                                            cx,
                                            window_handle,
                                        );
                                    } else {
                                        let _ = cx.update_window(
                                            window_handle,
                                            move |_, window, cx| {
                                                window.push_notification(
                                                    format!("Native dialog failed: {}", msg),
                                                    cx,
                                                );
                                            },
                                        );
                                    }
                                }
                                Err(e) => {
                                    let msg = e.to_string();
                                    if let Some(path) =
                                        fallback_file_picker("Select Postman Collection JSON")
                                    {
                                        crate::ui::top_bar::handle_postman_file(
                                            &path,
                                            &app_state,
                                            cx,
                                            window_handle,
                                        );
                                    } else {
                                        let _ = cx.update_window(
                                            window_handle,
                                            move |_, window, cx| {
                                                window.push_notification(
                                                    format!("Native dialog failed: {}", msg),
                                                    cx,
                                                );
                                            },
                                        );
                                    }
                                }
                                _ => {}
                            })
                            .detach();
                        })
                    }),
            )
            .child(
                Button::new("vars-btn")
                    .label("Variables")
                    .ghost()
                    .small()
                    .on_click({
                        let app_state = app_state.clone();
                        cx.listener(move |_this, _, window, cx| {
                            let vars = app_state.read(cx).workspace.variables.clone();
                            let app_state2 = app_state.clone();
                            let editor = cx.new(|cx| {
                                VariablesDialog::new(app_state2.clone(), vars.clone(), window, cx)
                            });

                            window.open_dialog(cx, move |dialog, _window, _cx| {
                                dialog
                                    .close_button(true)
                                    .w(px(700.))
                                    .title(
                                        div()
                                            .font_weight(FontWeight::BOLD)
                                            .child("Global Variables"),
                                    )
                                    .content({
                                        let editor = editor.clone();
                                        move |content, _window, _cx| content.child(editor.clone())
                                    })
                            });
                        })
                    }),
            )
            .child(
                Button::new("settings-btn")
                    .icon(IconName::Settings)
                    .label("Settings")
                    .ghost()
                    .small()
                    .on_click(cx.listener(|_this, _, window, cx| {
                        window.dispatch_action(Box::new(crate::ui::actions::OpenSettings), cx);
                    })),
            )
    }
}

fn fallback_file_picker(title: &str) -> Option<std::path::PathBuf> {
    if let Ok(output) = std::process::Command::new("kdialog")
        .args(["--getopenfilename", ".", "--title", title])
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(std::path::PathBuf::from(path));
            }
        }
    }
    if let Ok(output) = std::process::Command::new("zenity")
        .args(["--file-selection", "--title", title])
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(std::path::PathBuf::from(path));
            }
        }
    }
    None
}

pub(crate) fn handle_openapi_file(
    path: &std::path::Path,
    app_state: &Entity<AppState>,
    cx: &mut AsyncApp,
    window_handle: AnyWindowHandle,
) {
    match std::fs::read_to_string(path) {
        Ok(content) => match crate::import::import_openapi(&content) {
            Ok(col) => {
                let name = col.name.clone();
                let _ = cx.update(|cx| {
                    app_state.update(cx, |state, cx| {
                        state.import_collection(col);
                        cx.emit(AppEvent::WorkspaceChanged);
                    });
                });
                let _ = cx.update_window(window_handle, |_, window, cx| {
                    window.push_notification(format!("Imported: {}", name), cx);
                });
            }
            Err(e) => {
                let _ = cx.update_window(window_handle, |_, window, cx| {
                    window.push_notification(format!("Import error: {}", e), cx);
                });
            }
        },
        Err(e) => {
            let _ = cx.update_window(window_handle, |_, window, cx| {
                window.push_notification(format!("Failed to read file: {}", e), cx);
            });
        }
    }
}

pub(crate) fn handle_postman_file(
    path: &std::path::Path,
    app_state: &Entity<AppState>,
    cx: &mut AsyncApp,
    window_handle: AnyWindowHandle,
) {
    match std::fs::read_to_string(path) {
        Ok(content) => match crate::import::import_postman(&content) {
            Ok(col) => {
                let name = col.name.clone();
                let _ = cx.update(|cx| {
                    app_state.update(cx, |state, cx| {
                        state.import_collection(col);
                        cx.emit(AppEvent::WorkspaceChanged);
                    });
                });
                let _ = cx.update_window(window_handle, |_, window, cx| {
                    window.push_notification(format!("Imported: {}", name), cx);
                });
            }
            Err(e) => {
                let _ = cx.update_window(window_handle, |_, window, cx| {
                    window.push_notification(format!("Import error: {}", e), cx);
                });
            }
        },
        Err(e) => {
            let _ = cx.update_window(window_handle, |_, window, cx| {
                window.push_notification(format!("Failed to read file: {}", e), cx);
            });
        }
    }
}
