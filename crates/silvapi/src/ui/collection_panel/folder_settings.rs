#![allow(unused_imports)]
use std::{collections::HashSet, ops::Range};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt as _, PopupMenuItem},
    spinner::Spinner,
    v_flex,
};

use silvapi_core::{models::{AuthConfig, AuthType, CollectionItem, Folder, KeyValue, Variable}};
use crate::{state::{AppEvent, AppState}};
use crate::ui::actions::{FocusActiveRequest, RenameSelected, SendRequest};

use super::*;

impl CollectionPanel {
    pub(super) fn new_auth_input(
        window: &mut Window,
        cx: &mut Context<Self>,
        placeholder: &'static str,
    ) -> Entity<InputState> {
        cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
    }

    pub(super) fn make_folder_auth_inputs(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> FolderAuthInputs {
        FolderAuthInputs {
            bearer_token: Self::new_auth_input(window, cx, "Token"),
            bearer_prefix: Self::new_auth_input(window, cx, "Bearer"),
            basic_username: Self::new_auth_input(window, cx, "Username"),
            basic_password: Self::new_auth_input(window, cx, "Password"),
            api_key_name: Self::new_auth_input(window, cx, "Header name"),
            api_key_value: Self::new_auth_input(window, cx, "API key"),
            aws_access_key_id: Self::new_auth_input(window, cx, "Access key ID"),
            aws_secret_access_key: Self::new_auth_input(window, cx, "Secret access key"),
            aws_service: Self::new_auth_input(window, cx, "sts"),
            aws_region: Self::new_auth_input(window, cx, "us-east-1"),
            aws_session_token: Self::new_auth_input(window, cx, "Session token"),
            jwt_secret: Self::new_auth_input(window, cx, "Secret or private key"),
            jwt_payload: cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .code_editor("json")
                    .placeholder("Payload")
            }),
            oauth_client_id: Self::new_auth_input(window, cx, "Client ID"),
            oauth_client_secret: Self::new_auth_input(window, cx, "Client secret"),
            oauth_authorization_url: Self::new_auth_input(window, cx, "Authorization URL"),
            oauth_access_token_url: Self::new_auth_input(window, cx, "Access token URL"),
            oauth_redirect_uri: Self::new_auth_input(window, cx, "Redirect URI"),
            oauth_state: Self::new_auth_input(window, cx, "State"),
            oauth_audience: Self::new_auth_input(window, cx, "Audience"),
            oauth_username: Self::new_auth_input(window, cx, "Username"),
            oauth_password: Self::new_auth_input(window, cx, "Password"),
            oauth_scope: Self::new_auth_input(window, cx, "Scope"),
            oauth_header_name: Self::new_auth_input(window, cx, "Authorization"),
            oauth_header_prefix: Self::new_auth_input(window, cx, "Bearer"),
        }
    }

    pub(super) fn current_folder_auth(&self, cx: &App) -> AuthConfig {
        let Some(folder_id) = self.folder_settings_id.clone() else {
            return AuthConfig::default();
        };
        self.app_state
            .read(cx)
            .workspace
            .collections
            .iter()
            .find_map(|collection| find_folder_in_items(&collection.items, &folder_id))
            .map(|folder| folder.auth.clone())
            .unwrap_or_default()
    }

    pub(super) fn update_folder_auth(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut AuthConfig)) {
        let Some(folder_id) = self.folder_settings_id.clone() else {
            return;
        };
        self.app_state.update(cx, |state, cx| {
            for collection in &mut state.workspace.collections {
                if let Some(folder) = find_folder_in_items_mut(&mut collection.items, &folder_id) {
                    f(&mut folder.auth);
                    Self::ensure_folder_auth_defaults(&mut folder.auth);
                    break;
                }
            }
            cx.emit(AppEvent::WorkspaceChanged);
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    pub(super) fn set_folder_auth(
        &mut self,
        enabled: bool,
        auth_type: AuthType,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.folder_auth_enabled = enabled;
        self.folder_auth_type = auth_type.clone();
        self.folder_settings_tab = FolderSettingsTab::Auth;
        self.update_folder_auth(cx, |auth| {
            auth.enabled = enabled;
            auth.auth_type = auth_type.clone();
        });
        let auth = self.current_folder_auth(cx);
        self.load_folder_auth_inputs(&auth, window, cx);
        cx.notify();
    }

    pub(super) fn ensure_folder_auth_defaults(auth: &mut AuthConfig) {
        if auth.bearer_prefix.is_empty() {
            auth.bearer_prefix = "Bearer".to_string();
        }
        if auth.aws_service.is_empty() {
            auth.aws_service = "sts".to_string();
        }
        if auth.aws_region.is_empty() {
            auth.aws_region = "us-east-1".to_string();
        }
        if auth.jwt_algorithm.is_empty() {
            auth.jwt_algorithm = "HS256".to_string();
        }
        if auth.jwt_payload.is_empty() {
            auth.jwt_payload = "{\n  \"foo\": \"bar\"\n}".to_string();
        }
        if auth.oauth_grant_type.is_empty() {
            auth.oauth_grant_type = "Authorization Code".to_string();
        } else if auth.oauth_grant_type == "Password" {
            auth.oauth_grant_type = "Resource Owner Password Credential".to_string();
        }
        if auth.oauth_authorization_url.is_empty() {
            auth.oauth_authorization_url = "https://github.com/login/oauth/authorize".to_string();
        }
        if auth.oauth_access_token_url.is_empty() {
            auth.oauth_access_token_url =
                "https://github.com/login/oauth/access_token".to_string();
        }
        if auth.oauth_token_target.is_empty() {
            auth.oauth_token_target = "access_token".to_string();
        }
        if auth.oauth_response_type.is_empty() {
            auth.oauth_response_type = "Access Token".to_string();
        }
        if auth.oauth_header_name.is_empty() {
            auth.oauth_header_name = "Authorization".to_string();
        }
        if auth.oauth_header_prefix.is_empty() {
            auth.oauth_header_prefix = "Bearer".to_string();
        }
        if auth.oauth_send_credentials.is_empty() {
            auth.oauth_send_credentials = "In Request Body".to_string();
        } else if auth.oauth_send_credentials == "As Basic Auth Header" {
            auth.oauth_send_credentials = "As Basic Authentication".to_string();
        }
    }

    pub(super) fn load_folder_auth_inputs(
        &self,
        auth: &AuthConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let i = &self.folder_auth_inputs;
        i.bearer_token
            .update(cx, |s, cx| s.set_value(auth.bearer_token.clone(), window, cx));
        i.bearer_prefix.update(cx, |s, cx| {
            s.set_value(folder_default_if_empty(&auth.bearer_prefix, "Bearer"), window, cx)
        });
        i.basic_username.update(cx, |s, cx| {
            s.set_value(auth.basic_username.clone(), window, cx)
        });
        i.basic_password.update(cx, |s, cx| {
            s.set_value(auth.basic_password.clone(), window, cx)
        });
        i.api_key_name
            .update(cx, |s, cx| s.set_value(auth.api_key_name.clone(), window, cx));
        i.api_key_value
            .update(cx, |s, cx| s.set_value(auth.api_key_value.clone(), window, cx));
        i.aws_access_key_id.update(cx, |s, cx| {
            s.set_value(auth.aws_access_key_id.clone(), window, cx)
        });
        i.aws_secret_access_key.update(cx, |s, cx| {
            s.set_value(auth.aws_secret_access_key.clone(), window, cx)
        });
        i.aws_service.update(cx, |s, cx| {
            s.set_value(folder_default_if_empty(&auth.aws_service, "sts"), window, cx)
        });
        i.aws_region.update(cx, |s, cx| {
            s.set_value(folder_default_if_empty(&auth.aws_region, "us-east-1"), window, cx)
        });
        i.aws_session_token.update(cx, |s, cx| {
            s.set_value(auth.aws_session_token.clone(), window, cx)
        });
        i.jwt_secret
            .update(cx, |s, cx| s.set_value(auth.jwt_secret.clone(), window, cx));
        i.jwt_payload.update(cx, |s, cx| {
            s.set_value(
                folder_default_if_empty(&auth.jwt_payload, "{\n  \"foo\": \"bar\"\n}"),
                window,
                cx,
            )
        });
        i.oauth_client_id.update(cx, |s, cx| {
            s.set_value(auth.oauth_client_id.clone(), window, cx)
        });
        i.oauth_client_secret.update(cx, |s, cx| {
            s.set_value(auth.oauth_client_secret.clone(), window, cx)
        });
        i.oauth_authorization_url.update(cx, |s, cx| {
            s.set_value(
                folder_default_if_empty(
                    &auth.oauth_authorization_url,
                    "https://github.com/login/oauth/authorize",
                ),
                window,
                cx,
            )
        });
        i.oauth_access_token_url.update(cx, |s, cx| {
            s.set_value(
                folder_default_if_empty(
                    &auth.oauth_access_token_url,
                    "https://github.com/login/oauth/access_token",
                ),
                window,
                cx,
            )
        });
        i.oauth_redirect_uri.update(cx, |s, cx| {
            s.set_value(auth.oauth_redirect_uri.clone(), window, cx)
        });
        i.oauth_state
            .update(cx, |s, cx| s.set_value(auth.oauth_state.clone(), window, cx));
        i.oauth_audience.update(cx, |s, cx| {
            s.set_value(auth.oauth_audience.clone(), window, cx)
        });
        i.oauth_username.update(cx, |s, cx| {
            s.set_value(auth.oauth_username.clone(), window, cx)
        });
        i.oauth_password.update(cx, |s, cx| {
            s.set_value(auth.oauth_password.clone(), window, cx)
        });
        i.oauth_scope
            .update(cx, |s, cx| s.set_value(auth.oauth_scope.clone(), window, cx));
        i.oauth_header_name.update(cx, |s, cx| {
            s.set_value(
                folder_default_if_empty(&auth.oauth_header_name, "Authorization"),
                window,
                cx,
            )
        });
        i.oauth_header_prefix.update(cx, |s, cx| {
            s.set_value(
                folder_default_if_empty(&auth.oauth_header_prefix, "Bearer"),
                window,
                cx,
            )
        });
    }

    pub(super) fn sync_folder_auth_inputs_to_config(&self, auth: &mut AuthConfig, cx: &App) {
        let i = &self.folder_auth_inputs;
        auth.bearer_token = i.bearer_token.read(cx).value().to_string();
        auth.bearer_prefix = i.bearer_prefix.read(cx).value().to_string();
        auth.basic_username = i.basic_username.read(cx).value().to_string();
        auth.basic_password = i.basic_password.read(cx).value().to_string();
        auth.api_key_name = i.api_key_name.read(cx).value().to_string();
        auth.api_key_value = i.api_key_value.read(cx).value().to_string();
        auth.aws_access_key_id = i.aws_access_key_id.read(cx).value().to_string();
        auth.aws_secret_access_key = i.aws_secret_access_key.read(cx).value().to_string();
        auth.aws_service = i.aws_service.read(cx).value().to_string();
        auth.aws_region = i.aws_region.read(cx).value().to_string();
        auth.aws_session_token = i.aws_session_token.read(cx).value().to_string();
        auth.jwt_secret = i.jwt_secret.read(cx).value().to_string();
        auth.jwt_payload = i.jwt_payload.read(cx).value().to_string();
        auth.oauth_client_id = i.oauth_client_id.read(cx).value().to_string();
        auth.oauth_client_secret = i.oauth_client_secret.read(cx).value().to_string();
        auth.oauth_authorization_url = i.oauth_authorization_url.read(cx).value().to_string();
        auth.oauth_access_token_url = i.oauth_access_token_url.read(cx).value().to_string();
        auth.oauth_redirect_uri = i.oauth_redirect_uri.read(cx).value().to_string();
        auth.oauth_state = i.oauth_state.read(cx).value().to_string();
        auth.oauth_audience = i.oauth_audience.read(cx).value().to_string();
        auth.oauth_username = i.oauth_username.read(cx).value().to_string();
        auth.oauth_password = i.oauth_password.read(cx).value().to_string();
        auth.oauth_scope = i.oauth_scope.read(cx).value().to_string();
        auth.oauth_header_name = i.oauth_header_name.read(cx).value().to_string();
        auth.oauth_header_prefix = i.oauth_header_prefix.read(cx).value().to_string();
    }

    pub(super) fn open_folder_settings(
        &mut self,
        folder_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let folder = self
            .app_state
            .read(cx)
            .workspace
            .collections
            .iter()
            .find_map(|collection| find_folder_in_items(&collection.items, &folder_id))
            .cloned();

        let Some(folder) = folder else {
            return;
        };

        self.folder_settings_id = Some(folder.id.clone());
        self.folder_settings_tab = FolderSettingsTab::General;
        self.folder_auth_enabled = folder.auth.enabled;
        self.folder_auth_type = folder.auth.auth_type.clone();
        self.load_folder_auth_inputs(&folder.auth, window, cx);
        self.folder_name_input.update(cx, |state, cx| {
            state.set_value(folder.name.clone(), window, cx);
        });
        self.folder_description_input.update(cx, |state, cx| {
            state.set_value(folder.description.clone(), window, cx);
        });
        self.folder_header_rows = folder
            .headers
            .iter()
            .map(|header| self.make_kv_row(&header.key, &header.value, header.enabled, window, cx))
            .collect();
        self.folder_variable_rows = folder
            .variables
            .iter()
            .map(|var| self.make_kv_row(&var.name, &var.value, var.enabled, window, cx))
            .collect();
        cx.notify();
    }

    pub(super) fn close_folder_settings(&mut self, cx: &mut Context<Self>) {
        self.save_folder_settings(cx);
        self.folder_settings_id = None;
        cx.notify();
    }

    pub(super) fn save_folder_settings(&mut self, cx: &mut Context<Self>) {
        let Some(folder_id) = self.folder_settings_id.clone() else {
            return;
        };

        let name = self.folder_name_input.read(cx).value().to_string();
        let description = self.folder_description_input.read(cx).value().to_string();
        let headers = collect_kv_rows(&self.folder_header_rows, cx);
        let variables = self
            .folder_variable_rows
            .iter()
            .filter_map(|row| {
                let name = row.key.read(cx).value().to_string();
                let value = row.value.read(cx).value().to_string();
                (!name.is_empty() || !value.is_empty()).then(|| Variable {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    value,
                    enabled: row.enabled,
                })
            })
            .collect::<Vec<_>>();
        let mut auth = self.current_folder_auth(cx);
        auth.enabled = self.folder_auth_enabled;
        auth.auth_type = self.folder_auth_type.clone();
        self.sync_folder_auth_inputs_to_config(&mut auth, cx);
        Self::ensure_folder_auth_defaults(&mut auth);

        self.app_state.update(cx, |state, cx| {
            for collection in &mut state.workspace.collections {
                if let Some(folder) = find_folder_in_items_mut(&mut collection.items, &folder_id) {
                    folder.name = if name.trim().is_empty() {
                        folder.name.clone()
                    } else {
                        name.clone()
                    };
                    folder.description = description.clone();
                    folder.headers = headers.clone();
                    folder.variables = variables.clone();
                    folder.auth = auth.clone();
                    break;
                }
            }
            cx.emit(AppEvent::WorkspaceChanged);
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    pub(super) fn add_folder_header_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let row = self.make_kv_row("", "", true, window, cx);
        self.folder_header_rows.push(row);
        cx.notify();
    }

    pub(super) fn add_folder_variable_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let row = self.make_kv_row("", "", true, window, cx);
        self.folder_variable_rows.push(row);
        cx.notify();
    }

    pub fn folder_settings_open(&self) -> bool {
        self.folder_settings_id.is_some()
    }

    pub fn folder_settings_modal(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.render_folder_settings_modal(window, cx)
    }

    pub(super) fn render_folder_settings_modal(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let title = self.folder_name_input.read(cx).value().to_string();

        // Size the modal to ~60% of the viewport, centered.
        let viewport = window.viewport_size();
        let modal_w = viewport.width * 0.6;
        let modal_h = viewport.height * 0.6;

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().background.opacity(0.72))
            // Backdrop: block clicks reaching the app behind, and close on click.
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.close_folder_settings(cx);
                }),
            )
            .on_mouse_down(MouseButton::Right, |_, _, _| {})
            .child(
                v_flex()
                    .w(modal_w)
                    .h(modal_h)
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .shadow_lg()
                    .overflow_hidden()
                    // Swallow clicks inside the panel so they don't close the modal.
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .on_mouse_down(MouseButton::Right, |_, _, _| {})
                    .child(
                        h_flex()
                            .h(px(58.))
                            .px_5()
                            .items_center()
                            .gap_3()
                            .child(
                                Icon::new(IconName::Folder).text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(cx.theme().foreground)
                                    .child(if title.trim().is_empty() {
                                        "Folder Settings".to_string()
                                    } else {
                                        title
                                    }),
                            )
                            .child(
                                Button::new("folder-settings-close")
                                    .icon(IconName::Close)
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.close_folder_settings(cx);
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .flex_1()
                            .min_h_0()
                            .items_start()
                            .child(self.render_folder_settings_sidebar(cx))
                            .child(self.render_folder_settings_content(window, cx)),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn render_folder_settings_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w(px(188.))
            .h_full()
            .px_4()
            .py_4()
            .gap_1()
            .border_r_1()
            .border_color(cx.theme().border)
            .child(self.render_folder_settings_tab("General", FolderSettingsTab::General, cx))
            .child(self.render_folder_settings_tab("Headers", FolderSettingsTab::Headers, cx))
            .child(self.render_folder_auth_dropdown(cx))
            .child(self.render_folder_settings_tab("Variables", FolderSettingsTab::Variables, cx))
            .into_any_element()
    }

    pub(super) fn folder_auth_label(enabled: bool, auth_type: &AuthType) -> &'static str {
        if !enabled {
            return "Inherit from Parent";
        }
        match auth_type {
            AuthType::None => "No Auth",
            AuthType::ApiKey => "API Key",
            AuthType::AwsV4 => "AWS Signature",
            AuthType::Basic => "Basic Auth",
            AuthType::Bearer => "Bearer Token",
            AuthType::Jwt => "JWT Bearer",
            AuthType::OAuth2 => "OAuth 2.0",
        }
    }

    pub(super) fn folder_auth_short_label(enabled: bool, auth_type: &AuthType) -> &'static str {
        if !enabled {
            return "Inherit";
        }
        match auth_type {
            AuthType::None => "No Auth",
            AuthType::ApiKey => "API Key",
            AuthType::AwsV4 => "AWS v4",
            AuthType::Basic => "Basic",
            AuthType::Bearer => "Bearer",
            AuthType::Jwt => "JWT",
            AuthType::OAuth2 => "OAuth 2",
        }
    }

    pub(super) fn render_folder_auth_dropdown(&self, cx: &mut Context<Self>) -> AnyElement {
        let active = self.folder_settings_tab == FolderSettingsTab::Auth;
        let label = Self::folder_auth_short_label(self.folder_auth_enabled, &self.folder_auth_type);
        let cur_enabled = self.folder_auth_enabled;
        let cur_type = self.folder_auth_type.clone();
        let entity = cx.entity();

        let item = move |menu_label: &'static str, enabled: bool, auth_type: AuthType| {
            let selected = cur_enabled == enabled && cur_type == auth_type;
            let entity = entity.clone();
            PopupMenuItem::new(menu_label)
                .checked(selected)
                .on_click(move |_, window, cx| {
                    let auth_type = auth_type.clone();
                    entity.update(cx, |this, cx| {
                        this.set_folder_auth(enabled, auth_type, window, cx);
                    });
                })
        };

        div()
            .w_full()
            .overflow_hidden()
            .child(
                DropdownButton::new("folder-auth-dropdown")
            .button(
                Button::new("folder-auth-btn")
                    .label(label)
                    .ghost()
                    .small()
                    .when(active, |b| b.primary())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.folder_settings_tab = FolderSettingsTab::Auth;
                        cx.notify();
                    })),
            )
            .dropdown_menu(move |menu, _, _| {
                menu.item(item("API Key", true, AuthType::ApiKey))
                    .item(item("AWS Signature", true, AuthType::AwsV4))
                    .item(item("Basic Auth", true, AuthType::Basic))
                    .item(item("Bearer Token", true, AuthType::Bearer))
                    .item(item("JWT Bearer", true, AuthType::Jwt))
                    .item(item("OAuth 2.0", true, AuthType::OAuth2))
                    .separator()
                    .item(item("Inherit from Parent", false, AuthType::None))
                    .item(item("No Auth", true, AuthType::None))
            }),
            )
            .into_any_element()
    }

    pub(super) fn render_folder_settings_tab(
        &self,
        label: &'static str,
        tab: FolderSettingsTab,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = self.folder_settings_tab == tab;
        div()
            .h(px(30.))
            .px_3()
            .rounded_md()
            .flex()
            .items_center()
            .cursor_pointer()
            .text_sm()
            .text_color(if selected {
                cx.theme().foreground
            } else {
                cx.theme().muted_foreground
            })
            .bg(if selected {
                cx.theme().secondary
            } else {
                cx.theme().background
            })
            .hover(|style| style.bg(cx.theme().secondary))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.folder_settings_tab = tab;
                    cx.notify();
                }),
            )
            .child(label)
            .into_any_element()
    }

    pub(super) fn render_folder_settings_content(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let body = match self.folder_settings_tab {
            FolderSettingsTab::General => self.render_folder_general_settings(cx),
            FolderSettingsTab::Headers => self.render_folder_headers_settings(window, cx),
            FolderSettingsTab::Auth => self.render_folder_auth_settings(cx),
            FolderSettingsTab::Variables => self.render_folder_variables_settings(window, cx),
        };

        div()
            .id("folder-settings-content")
            .flex_1()
            .h_full()
            .min_w_0()
            .min_h_0()
            .p_4()
            .overflow_y_scroll()
            .child(body)
            .into_any_element()
    }

    pub(super) fn render_folder_general_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .size_full()
            .gap_3()
            .child(label_text("Folder Name", cx))
            .child(Input::new(&self.folder_name_input).h(px(34.)))
            .child(Input::new(&self.folder_description_input).flex_1().w_full())
            .into_any_element()
    }

    pub(super) fn render_folder_headers_settings(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(folder_kv_header(cx))
            .children(
                self.folder_header_rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| render_folder_kv_row(row, index, false, cx)),
            )
            .child(
                h_flex().justify_center().child(
                    Button::new("add-folder-header")
                        .icon(IconName::Plus)
                        .label("Add")
                        .small()
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.add_folder_header_row(window, cx);
                        })),
                ),
            )
            .into_any_element()
    }

    pub(super) fn render_folder_variables_settings(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.folder_variable_rows.is_empty() {
            return div()
                .size_full()
                .border_1()
                .border_color(cx.theme().border)
                .rounded_lg()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    v_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_sm()
                                .italic()
                                .text_color(cx.theme().muted_foreground)
                                .child("Override variables for requests within this folder."),
                        )
                        .child(
                            Button::new("create-folder-env")
                                .label("Create Folder Environment")
                                .small()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.add_folder_variable_row(window, cx);
                                })),
                        ),
                )
                .into_any_element();
        }

        v_flex()
            .size_full()
            .gap_2()
            .child(folder_kv_header(cx))
            .children(
                self.folder_variable_rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| render_folder_kv_row(row, index, true, cx)),
            )
            .child(
                h_flex().justify_center().child(
                    Button::new("add-folder-variable")
                        .icon(IconName::Plus)
                        .label("Add")
                        .small()
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.add_folder_variable_row(window, cx);
                        })),
                ),
            )
            .into_any_element()
    }

    pub(super) fn render_folder_auth_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let auth = self.current_folder_auth(cx);
        let enabled = self.folder_auth_enabled;
        let auth_type = self.folder_auth_type.clone();

        v_flex()
            .w_full()
            .gap_3()
            .child(label_text("Authentication", cx))
            .child(
                h_flex()
                    .h(px(34.))
                    .px_3()
                    .items_center()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().primary)
                    .bg(cx.theme().primary.opacity(0.1))
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(Self::folder_auth_label(enabled, &auth_type)),
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Checkbox::new("folder-auth-enabled")
                            .checked(enabled)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.folder_auth_enabled = *checked;
                                this.update_folder_auth(cx, |auth| {
                                    auth.enabled = *checked;
                                });
                                cx.notify();
                            })),
                    )
                    .child(div().text_sm().child("Enabled")),
            )
            .when(!enabled, |el| {
                el.child(
                    div()
                        .text_sm()
                        .italic()
                        .text_color(cx.theme().muted_foreground)
                        .child("Requests in this folder inherit authentication from the parent."),
                )
            })
            .when(enabled && matches!(auth_type, AuthType::None), |el| {
                el.child(
                    div()
                        .text_sm()
                        .italic()
                        .text_color(cx.theme().muted_foreground)
                        .child("Requests in this folder send no authentication."),
                )
            })
            .when(enabled && matches!(auth_type, AuthType::ApiKey), |el| {
                el.child(self.render_folder_api_key_auth(&auth, cx))
            })
            .when(enabled && matches!(auth_type, AuthType::AwsV4), |el| {
                el.child(self.render_folder_aws_auth(cx))
            })
            .when(enabled && matches!(auth_type, AuthType::Basic), |el| {
                el.child(self.render_folder_basic_auth(cx))
            })
            .when(enabled && matches!(auth_type, AuthType::Bearer), |el| {
                el.child(self.render_folder_bearer_auth(cx))
            })
            .when(enabled && matches!(auth_type, AuthType::Jwt), |el| {
                el.child(self.render_folder_jwt_auth(&auth, cx))
            })
            .when(enabled && matches!(auth_type, AuthType::OAuth2), |el| {
                el.child(self.render_folder_oauth_auth(&auth, cx))
            })
            .into_any_element()
    }

    pub(super) fn render_folder_api_key_auth(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let api_key_in_header = auth.api_key_in_header;
        let behavior_label = if api_key_in_header {
            "Insert Header"
        } else {
            "Insert Query Param"
        };
        let entity = cx.entity();
        v_flex()
            .gap_3()
            .child(folder_auth_select_field(
                "Behavior",
                DropdownButton::new("folder-api-key-behavior")
                    .button(
                        Button::new("folder-api-key-behavior-btn")
                            .label(behavior_label)
                            .ghost()
                            .small(),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        let header_entity = entity.clone();
                        let query_entity = entity.clone();
                        menu.item(
                            PopupMenuItem::new("Insert Header")
                                .checked(api_key_in_header)
                                .on_click(move |_, _, cx| {
                                    header_entity.update(cx, |this, cx| {
                                        this.update_folder_auth(cx, |auth| {
                                            auth.api_key_in_header = true;
                                        });
                                    });
                                }),
                        )
                        .item(
                            PopupMenuItem::new("Insert Query Param")
                                .checked(!api_key_in_header)
                                .on_click(move |_, _, cx| {
                                    query_entity.update(cx, |this, cx| {
                                        this.update_folder_auth(cx, |auth| {
                                            auth.api_key_in_header = false;
                                        });
                                    });
                                }),
                        )
                    }),
                cx,
            ))
            .child(folder_auth_input_field(
                if api_key_in_header {
                    "Header Name*"
                } else {
                    "Query Param Name*"
                },
                &self.folder_auth_inputs.api_key_name,
                cx,
            ))
            .child(folder_auth_input_field(
                "API Key",
                &self.folder_auth_inputs.api_key_value,
                cx,
            ))
    }

    pub(super) fn render_folder_aws_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(folder_auth_input_field(
                "Access Key ID*",
                &self.folder_auth_inputs.aws_access_key_id,
                cx,
            ))
            .child(folder_auth_input_field(
                "Secret Access Key*",
                &self.folder_auth_inputs.aws_secret_access_key,
                cx,
            ))
            .child(folder_auth_input_field(
                "Service Name*",
                &self.folder_auth_inputs.aws_service,
                cx,
            ))
            .child(folder_auth_input_field(
                "Region",
                &self.folder_auth_inputs.aws_region,
                cx,
            ))
            .child(folder_auth_input_field(
                "Session Token",
                &self.folder_auth_inputs.aws_session_token,
                cx,
            ))
    }

    pub(super) fn render_folder_basic_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(folder_auth_input_field(
                "Username",
                &self.folder_auth_inputs.basic_username,
                cx,
            ))
            .child(folder_auth_input_field(
                "Password",
                &self.folder_auth_inputs.basic_password,
                cx,
            ))
    }

    pub(super) fn render_folder_bearer_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(folder_auth_input_field(
                "Token",
                &self.folder_auth_inputs.bearer_token,
                cx,
            ))
            .child(folder_auth_input_field(
                "Prefix",
                &self.folder_auth_inputs.bearer_prefix,
                cx,
            ))
    }

    pub(super) fn render_folder_jwt_auth(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let entity = cx.entity();
        let current_algorithm = auth.jwt_algorithm.clone();
        v_flex()
            .gap_3()
            .child(
                DropdownButton::new("folder-jwt-algorithm")
                    .button(
                        Button::new("folder-jwt-algorithm-btn")
                            .label(if auth.jwt_algorithm.is_empty() {
                                "HS256"
                            } else {
                                auth.jwt_algorithm.as_str()
                            })
                            .ghost()
                            .small(),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        ["HS256", "HS384", "HS512", "RS256", "RS384", "RS512"]
                            .into_iter()
                            .fold(menu, |menu, algorithm| {
                                let entity = entity.clone();
                                menu.item(
                                    PopupMenuItem::new(algorithm)
                                        .checked(current_algorithm == algorithm)
                                        .on_click(move |_, _, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.update_folder_auth(cx, |auth| {
                                                    auth.jwt_algorithm = algorithm.to_string();
                                                });
                                            });
                                        }),
                                )
                            })
                    }),
            )
            .child(folder_auth_input_field(
                "Secret or Private Key",
                &self.folder_auth_inputs.jwt_secret,
                cx,
            ))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Checkbox::new("folder-jwt-secret-base64")
                            .checked(auth.jwt_secret_base64)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.update_folder_auth(cx, |auth| {
                                    auth.jwt_secret_base64 = *checked;
                                });
                            })),
                    )
                    .child(div().text_sm().child("Secret is base64 encoded")),
            )
            .child(folder_auth_multiline_field(
                "Payload* (Json)",
                &self.folder_auth_inputs.jwt_payload,
                cx,
            ))
    }

    pub(super) fn render_folder_oauth_auth(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let grant = folder_oauth_grant_label(&auth.oauth_grant_type);
        let is_authorization_code = grant == "Authorization Code";
        let is_implicit = grant == "Implicit";
        let is_resource_owner = grant == "Resource Owner Password Credential";
        let is_client_credentials = grant == "Client Credentials";

        v_flex()
            .gap_3()
            .child(self.folder_oauth_grant_dropdown(&auth.oauth_grant_type, cx))
            .when(is_authorization_code, |el| {
                el.child(folder_auth_input_field(
                    "Client ID",
                    &self.folder_auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Client Secret",
                    &self.folder_auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Authorization URL",
                    &self.folder_auth_inputs.oauth_authorization_url,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Access Token URL",
                    &self.folder_auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Redirect URI",
                    &self.folder_auth_inputs.oauth_redirect_uri,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "State",
                    &self.folder_auth_inputs.oauth_state,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Audience",
                    &self.folder_auth_inputs.oauth_audience,
                    cx,
                ))
                .child(folder_auth_select_field(
                    "Token for authorization",
                    self.folder_oauth_token_target_dropdown(auth, cx),
                    cx,
                ))
                .child(self.folder_oauth_pkce_checkbox(auth, cx))
            })
            .when(is_implicit, |el| {
                el.child(folder_auth_input_field(
                    "Client ID",
                    &self.folder_auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Authorization URL",
                    &self.folder_auth_inputs.oauth_authorization_url,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Redirect URI",
                    &self.folder_auth_inputs.oauth_redirect_uri,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "State",
                    &self.folder_auth_inputs.oauth_state,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Audience",
                    &self.folder_auth_inputs.oauth_audience,
                    cx,
                ))
                .child(folder_auth_select_field(
                    "Token for authorization",
                    self.folder_oauth_token_target_dropdown(auth, cx),
                    cx,
                ))
                .child(folder_auth_select_field(
                    "Response Type",
                    self.folder_oauth_response_type_dropdown(auth, cx),
                    cx,
                ))
            })
            .when(is_resource_owner, |el| {
                el.child(folder_auth_input_field(
                    "Client ID",
                    &self.folder_auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Client Secret",
                    &self.folder_auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Access Token URL",
                    &self.folder_auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Audience",
                    &self.folder_auth_inputs.oauth_audience,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Username",
                    &self.folder_auth_inputs.oauth_username,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Password",
                    &self.folder_auth_inputs.oauth_password,
                    cx,
                ))
            })
            .when(is_client_credentials, |el| {
                el.child(folder_auth_input_field(
                    "Client ID",
                    &self.folder_auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Client Secret",
                    &self.folder_auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Access Token URL",
                    &self.folder_auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(folder_auth_input_field(
                    "Audience",
                    &self.folder_auth_inputs.oauth_audience,
                    cx,
                ))
            })
            .child(self.render_folder_oauth_advanced(auth, cx))
    }

    pub(super) fn folder_oauth_grant_dropdown(
        &self,
        current: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let current = folder_oauth_grant_label(current);
        let entity = cx.entity();
        let item = move |grant: &'static str| {
            let entity = entity.clone();
            PopupMenuItem::new(grant)
                .checked(current == grant)
                .on_click(move |_, _, cx| {
                    entity.update(cx, |this, cx| {
                        this.update_folder_auth(cx, |auth| {
                            auth.oauth_grant_type = grant.to_string();
                        });
                    });
                })
        };
        DropdownButton::new("folder-oauth-grant")
            .button(
                Button::new("folder-oauth-grant-btn")
                    .label(current)
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                menu.item(item("Authorization Code"))
                    .item(item("Implicit"))
                    .item(item("Resource Owner Password Credential"))
                    .item(item("Client Credentials"))
            })
    }

    pub(super) fn folder_oauth_token_target_dropdown(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let current = folder_default_if_empty(&auth.oauth_token_target, "access_token");
        let entity = cx.entity();
        DropdownButton::new("folder-oauth-token-target")
            .button(
                Button::new("folder-oauth-token-target-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["access_token", "id_token"]
                    .into_iter()
                    .fold(menu, |menu, token| {
                        let entity = entity.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(token)
                                .checked(current == token)
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.update_folder_auth(cx, |auth| {
                                            auth.oauth_token_target = token.to_string();
                                        });
                                    });
                                }),
                        )
                    })
            })
    }

    pub(super) fn folder_oauth_response_type_dropdown(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let current = folder_default_if_empty(&auth.oauth_response_type, "Access Token");
        let entity = cx.entity();
        DropdownButton::new("folder-oauth-response-type")
            .button(
                Button::new("folder-oauth-response-type-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["Access Token", "ID Token", "Access Token + ID Token"]
                    .into_iter()
                    .fold(menu, |menu, response_type| {
                        let entity = entity.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(response_type)
                                .checked(current == response_type)
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.update_folder_auth(cx, |auth| {
                                            auth.oauth_response_type = response_type.to_string();
                                        });
                                    });
                                }),
                        )
                    })
            })
    }

    pub(super) fn folder_oauth_send_credentials_dropdown(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let current = folder_default_if_empty(&auth.oauth_send_credentials, "In Request Body");
        let entity = cx.entity();
        DropdownButton::new("folder-oauth-send-credentials")
            .button(
                Button::new("folder-oauth-send-credentials-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["In Request Body", "As Basic Authentication"]
                    .into_iter()
                    .fold(menu, |menu, target| {
                        let entity = entity.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(target)
                                .checked(current == target)
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.update_folder_auth(cx, |auth| {
                                            auth.oauth_send_credentials = target.to_string();
                                        });
                                    });
                                }),
                        )
                    })
            })
    }

    pub(super) fn folder_oauth_pkce_checkbox(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(
                Checkbox::new("folder-oauth-use-pkce")
                    .checked(auth.oauth_use_pkce)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.update_folder_auth(cx, |auth| {
                            auth.oauth_use_pkce = *checked;
                        });
                    })),
            )
            .child(div().text_sm().child("Use PKCE"))
    }

    pub(super) fn render_folder_oauth_advanced(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_3()
            .p_3()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child(div().text_sm().child("Advanced"))
            .child(folder_auth_input_field(
                "Scope",
                &self.folder_auth_inputs.oauth_scope,
                cx,
            ))
            .child(folder_auth_input_field(
                "Header Name*",
                &self.folder_auth_inputs.oauth_header_name,
                cx,
            ))
            .child(folder_auth_input_field(
                "Header Prefix",
                &self.folder_auth_inputs.oauth_header_prefix,
                cx,
            ))
            .child(folder_auth_select_field(
                "Send Credentials",
                self.folder_oauth_send_credentials_dropdown(auth, cx),
                cx,
            ))
    }

}
