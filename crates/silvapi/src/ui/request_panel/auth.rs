#![allow(unused_imports)]
use std::time::Duration;
use instant::Instant;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_kit::component::{
    ActiveTheme, IconName, Sizable as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
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
    pub(super) fn make_auth_inputs(window: &mut Window, cx: &mut Context<Self>) -> AuthInputs {
        AuthInputs {
            bearer_token: Self::new_input(window, cx, "Token"),
            bearer_prefix: Self::new_input(window, cx, "Bearer"),
            basic_username: Self::new_input(window, cx, "Username"),
            basic_password: Self::new_input(window, cx, "Password"),
            api_key_name: Self::new_input(window, cx, "Header name"),
            api_key_value: Self::new_input(window, cx, "API key"),
            aws_access_key_id: Self::new_input(window, cx, "Access key ID"),
            aws_secret_access_key: Self::new_input(window, cx, "Secret access key"),
            aws_service: Self::new_input(window, cx, "sts"),
            aws_region: Self::new_input(window, cx, "us-east-1"),
            aws_session_token: Self::new_input(window, cx, "Session token"),
            jwt_secret: Self::new_input(window, cx, "Secret or private key"),
            jwt_payload: Self::new_multiline_input(window, cx, "Payload", "json"),
            oauth_client_id: Self::new_input(window, cx, "Client ID"),
            oauth_client_secret: Self::new_input(window, cx, "Client secret"),
            oauth_authorization_url: Self::new_input(window, cx, "Authorization URL"),
            oauth_access_token_url: Self::new_input(window, cx, "Access token URL"),
            oauth_redirect_uri: Self::new_input(window, cx, "Redirect URI"),
            oauth_state: Self::new_input(window, cx, "State"),
            oauth_audience: Self::new_input(window, cx, "Audience"),
            oauth_username: Self::new_input(window, cx, "Username"),
            oauth_password: Self::new_input(window, cx, "Password"),
            oauth_scope: Self::new_input(window, cx, "Scope"),
            oauth_header_name: Self::new_input(window, cx, "Authorization"),
            oauth_header_prefix: Self::new_input(window, cx, "Bearer"),
        }
    }

    pub(super) fn load_auth_inputs(
        &self,
        auth: &AuthConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.auth_inputs.bearer_token.update(cx, |s, cx| {
            s.set_value(auth.bearer_token.clone(), window, cx)
        });
        self.auth_inputs.bearer_prefix.update(cx, |s, cx| {
            s.set_value(default_if_empty(&auth.bearer_prefix, "Bearer"), window, cx)
        });
        self.auth_inputs.basic_username.update(cx, |s, cx| {
            s.set_value(auth.basic_username.clone(), window, cx)
        });
        self.auth_inputs.basic_password.update(cx, |s, cx| {
            s.set_value(auth.basic_password.clone(), window, cx)
        });
        self.auth_inputs.api_key_name.update(cx, |s, cx| {
            s.set_value(auth.api_key_name.clone(), window, cx)
        });
        self.auth_inputs.api_key_value.update(cx, |s, cx| {
            s.set_value(auth.api_key_value.clone(), window, cx)
        });
        self.auth_inputs.aws_access_key_id.update(cx, |s, cx| {
            s.set_value(auth.aws_access_key_id.clone(), window, cx)
        });
        self.auth_inputs.aws_secret_access_key.update(cx, |s, cx| {
            s.set_value(auth.aws_secret_access_key.clone(), window, cx)
        });
        self.auth_inputs.aws_service.update(cx, |s, cx| {
            s.set_value(default_if_empty(&auth.aws_service, "sts"), window, cx)
        });
        self.auth_inputs.aws_region.update(cx, |s, cx| {
            s.set_value(default_if_empty(&auth.aws_region, "us-east-1"), window, cx)
        });
        self.auth_inputs.aws_session_token.update(cx, |s, cx| {
            s.set_value(auth.aws_session_token.clone(), window, cx)
        });
        self.auth_inputs
            .jwt_secret
            .update(cx, |s, cx| s.set_value(auth.jwt_secret.clone(), window, cx));
        self.auth_inputs.jwt_payload.update(cx, |s, cx| {
            s.set_value(
                default_if_empty(&auth.jwt_payload, "{\n  \"foo\": \"bar\"\n}"),
                window,
                cx,
            )
        });
        self.auth_inputs.oauth_client_id.update(cx, |s, cx| {
            s.set_value(auth.oauth_client_id.clone(), window, cx)
        });
        self.auth_inputs.oauth_client_secret.update(cx, |s, cx| {
            s.set_value(auth.oauth_client_secret.clone(), window, cx)
        });
        self.auth_inputs
            .oauth_authorization_url
            .update(cx, |s, cx| {
                s.set_value(
                    default_if_empty(
                        &auth.oauth_authorization_url,
                        "https://github.com/login/oauth/authorize",
                    ),
                    window,
                    cx,
                )
            });
        self.auth_inputs.oauth_access_token_url.update(cx, |s, cx| {
            s.set_value(
                default_if_empty(
                    &auth.oauth_access_token_url,
                    "https://github.com/login/oauth/access_token",
                ),
                window,
                cx,
            )
        });
        self.auth_inputs.oauth_redirect_uri.update(cx, |s, cx| {
            s.set_value(auth.oauth_redirect_uri.clone(), window, cx)
        });
        self.auth_inputs.oauth_state.update(cx, |s, cx| {
            s.set_value(auth.oauth_state.clone(), window, cx)
        });
        self.auth_inputs.oauth_audience.update(cx, |s, cx| {
            s.set_value(auth.oauth_audience.clone(), window, cx)
        });
        self.auth_inputs.oauth_username.update(cx, |s, cx| {
            s.set_value(auth.oauth_username.clone(), window, cx)
        });
        self.auth_inputs.oauth_password.update(cx, |s, cx| {
            s.set_value(auth.oauth_password.clone(), window, cx)
        });
        self.auth_inputs.oauth_scope.update(cx, |s, cx| {
            s.set_value(auth.oauth_scope.clone(), window, cx)
        });
        self.auth_inputs.oauth_header_name.update(cx, |s, cx| {
            s.set_value(
                default_if_empty(&auth.oauth_header_name, "Authorization"),
                window,
                cx,
            )
        });
        self.auth_inputs.oauth_header_prefix.update(cx, |s, cx| {
            s.set_value(
                default_if_empty(&auth.oauth_header_prefix, "Bearer"),
                window,
                cx,
            )
        });
    }

    pub(super) fn sync_auth_inputs_to_config(&self, auth: &mut AuthConfig, cx: &App) {
        auth.bearer_token = self.auth_inputs.bearer_token.read(cx).value().to_string();
        auth.bearer_prefix = self.auth_inputs.bearer_prefix.read(cx).value().to_string();
        auth.basic_username = self.auth_inputs.basic_username.read(cx).value().to_string();
        auth.basic_password = self.auth_inputs.basic_password.read(cx).value().to_string();
        auth.api_key_name = self.auth_inputs.api_key_name.read(cx).value().to_string();
        auth.api_key_value = self.auth_inputs.api_key_value.read(cx).value().to_string();
        auth.aws_access_key_id = self
            .auth_inputs
            .aws_access_key_id
            .read(cx)
            .value()
            .to_string();
        auth.aws_secret_access_key = self
            .auth_inputs
            .aws_secret_access_key
            .read(cx)
            .value()
            .to_string();
        auth.aws_service = self.auth_inputs.aws_service.read(cx).value().to_string();
        auth.aws_region = self.auth_inputs.aws_region.read(cx).value().to_string();
        auth.aws_session_token = self
            .auth_inputs
            .aws_session_token
            .read(cx)
            .value()
            .to_string();
        auth.jwt_secret = self.auth_inputs.jwt_secret.read(cx).value().to_string();
        auth.jwt_payload = self.auth_inputs.jwt_payload.read(cx).value().to_string();
        auth.oauth_client_id = self
            .auth_inputs
            .oauth_client_id
            .read(cx)
            .value()
            .to_string();
        auth.oauth_client_secret = self
            .auth_inputs
            .oauth_client_secret
            .read(cx)
            .value()
            .to_string();
        auth.oauth_authorization_url = self
            .auth_inputs
            .oauth_authorization_url
            .read(cx)
            .value()
            .to_string();
        auth.oauth_access_token_url = self
            .auth_inputs
            .oauth_access_token_url
            .read(cx)
            .value()
            .to_string();
        auth.oauth_redirect_uri = self
            .auth_inputs
            .oauth_redirect_uri
            .read(cx)
            .value()
            .to_string();
        auth.oauth_state = self.auth_inputs.oauth_state.read(cx).value().to_string();
        auth.oauth_audience = self.auth_inputs.oauth_audience.read(cx).value().to_string();
        auth.oauth_username = self.auth_inputs.oauth_username.read(cx).value().to_string();
        auth.oauth_password = self.auth_inputs.oauth_password.read(cx).value().to_string();
        auth.oauth_scope = self.auth_inputs.oauth_scope.read(cx).value().to_string();
        auth.oauth_header_name = self
            .auth_inputs
            .oauth_header_name
            .read(cx)
            .value()
            .to_string();
        auth.oauth_header_prefix = self
            .auth_inputs
            .oauth_header_prefix
            .read(cx)
            .value()
            .to_string();
    }

    pub(super) fn render_auth_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let auth = self
            .app_state
            .read(cx)
            .active_request
            .as_ref()
            .map(|r| r.auth.clone())
            .unwrap_or_default();
        let auth_type = auth.auth_type.clone();

        div()
            .id("auth-scroll")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_3()
            .gap_3()
            .child(
                div()
                    .w(px(160.))
                    .child(
                        Select::new(&self.auth_type_select)
                            .small()
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Checkbox::new("auth-enabled")
                            .checked(auth.enabled)
                            .on_click(cx.listener(|this, enabled: &bool, _, cx| {
                                Self::update_auth_config(&this.app_state, cx, |auth| {
                                    auth.enabled = *enabled;
                                });
                            })),
                    )
                    .child(div().text_sm().child("Enabled")),
            )
            .when(matches!(auth_type, AuthType::ApiKey), |el| {
                el.child(self.render_api_key_auth(&auth, cx))
            })
            .when(matches!(auth_type, AuthType::AwsV4), |el| {
                el.child(self.render_aws_auth(cx))
            })
            .when(matches!(auth_type, AuthType::Basic), |el| {
                el.child(self.render_basic_auth(cx))
            })
            .when(matches!(auth_type, AuthType::Bearer), |el| {
                el.child(self.render_bearer_auth(cx))
            })
            .when(matches!(auth_type, AuthType::Jwt), |el| {
                el.child(self.render_jwt_auth(&auth, cx))
            })
            .when(matches!(auth_type, AuthType::OAuth2), |el| {
                el.child(self.render_oauth_auth(&auth, cx))
            })
            .when(matches!(auth_type, AuthType::None), |el| {
                el.child(
                    div()
                        .flex()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("No Auth"),
                )
            })
    }

    pub(super) fn auth_type_label(auth_type: &AuthType) -> &'static str {
        match auth_type {
            AuthType::ApiKey => "API Key",
            AuthType::AwsV4 => "AWS v4",
            AuthType::Basic => "Basic",
            AuthType::Bearer => "Bearer",
            AuthType::Jwt => "JWT",
            AuthType::OAuth2 => "OAuth 2",
            AuthType::None => "No Auth",
        }
    }

    pub(super) fn render_api_key_auth(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(Self::auth_select_field(
                "Behavior",
                div()
                    .w(px(160.))
                    .child(
                        Select::new(&self.api_key_placement_select)
                            .small()
                    ),
                cx,
            ))
            .child(self.auth_input_field(
                if auth.api_key_in_header {
                    "Header Name*"
                } else {
                    "Query Param Name*"
                },
                &self.auth_inputs.api_key_name,
                cx,
            ))
            .child(self.auth_input_field("API Key", &self.auth_inputs.api_key_value, cx))
    }

    pub(super) fn render_aws_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(self.auth_input_field("Access Key ID*", &self.auth_inputs.aws_access_key_id, cx))
            .child(self.auth_input_field(
                "Secret Access Key*",
                &self.auth_inputs.aws_secret_access_key,
                cx,
            ))
            .child(self.auth_input_field("Service Name*", &self.auth_inputs.aws_service, cx))
            .child(self.auth_input_field("Region", &self.auth_inputs.aws_region, cx))
            .child(self.auth_input_field("Session Token", &self.auth_inputs.aws_session_token, cx))
    }

    pub(super) fn render_basic_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(self.auth_input_field("Username", &self.auth_inputs.basic_username, cx))
            .child(self.auth_input_field("Password", &self.auth_inputs.basic_password, cx))
    }

    pub(super) fn render_bearer_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(self.auth_input_field("Token", &self.auth_inputs.bearer_token, cx))
            .child(self.auth_input_field("Prefix", &self.auth_inputs.bearer_prefix, cx))
    }

    pub(super) fn render_jwt_auth(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(Self::auth_select_field(
                "Algorithm",
                div()
                    .w(px(120.))
                    .child(
                        Select::new(&self.jwt_alg_select)
                            .small()
                    ),
                cx,
            ))
            .child(self.auth_input_field("Secret or Private Key", &self.auth_inputs.jwt_secret, cx))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Checkbox::new("jwt-secret-base64")
                            .checked(auth.jwt_secret_base64)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                Self::update_auth_config(&this.app_state, cx, |auth| {
                                    auth.jwt_secret_base64 = *checked;
                                });
                            })),
                    )
                    .child(div().text_sm().child("Secret is base64 encoded")),
            )
            .child(Self::auth_multiline_field(
                "Payload* (Json)",
                &self.auth_inputs.jwt_payload,
                cx,
            ))
    }

    pub(super) fn render_oauth_auth(
        &self,
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let grant = Self::oauth_grant_label(&auth.oauth_grant_type);
        let is_authorization_code = grant == "Authorization Code";
        let is_implicit = grant == "Implicit";
        let is_resource_owner = grant == "Resource Owner Password Credential";
        let is_client_credentials = grant == "Client Credentials";

        v_flex()
            .gap_3()
            .child(Self::auth_select_field(
                "Grant Type",
                div()
                    .w(px(220.))
                    .child(
                        Select::new(&self.oauth_grant_select)
                            .small()
                    ),
                cx,
            ))
            .when(is_authorization_code, |el| {
                el.child(self.auth_input_field("Client ID", &self.auth_inputs.oauth_client_id, cx))
                    .child(self.auth_input_field(
                        "Client Secret",
                        &self.auth_inputs.oauth_client_secret,
                        cx,
                    ))
                    .child(self.auth_input_field(
                        "Authorization URL",
                        &self.auth_inputs.oauth_authorization_url,
                        cx,
                    ))
                    .child(self.auth_input_field(
                        "Access Token URL",
                        &self.auth_inputs.oauth_access_token_url,
                        cx,
                    ))
                    .child(self.auth_input_field(
                        "Redirect URI",
                        &self.auth_inputs.oauth_redirect_uri,
                        cx,
                    ))
                    .child(self.auth_input_field("State", &self.auth_inputs.oauth_state, cx))
                    .child(self.auth_input_field("Audience", &self.auth_inputs.oauth_audience, cx))
                    .child(Self::auth_select_field(
                        "Token for authorization",
                        div()
                            .w(px(140.))
                            .child(
                                Select::new(&self.oauth_token_target_select)
                                    .small()
                            ),
                        cx,
                    ))
                    .child(Self::oauth_pkce_checkbox(auth, cx))
            })
            .when(is_implicit, |el| {
                el.child(self.auth_input_field("Client ID", &self.auth_inputs.oauth_client_id, cx))
                    .child(self.auth_input_field(
                        "Authorization URL",
                        &self.auth_inputs.oauth_authorization_url,
                        cx,
                    ))
                    .child(self.auth_input_field(
                        "Redirect URI",
                        &self.auth_inputs.oauth_redirect_uri,
                        cx,
                    ))
                    .child(self.auth_input_field("State", &self.auth_inputs.oauth_state, cx))
                    .child(self.auth_input_field("Audience", &self.auth_inputs.oauth_audience, cx))
                    .child(Self::auth_select_field(
                        "Token for authorization",
                        div()
                            .w(px(140.))
                            .child(
                                Select::new(&self.oauth_token_target_select)
                                    .small()
                            ),
                        cx,
                    ))
                    .child(Self::auth_select_field(
                        "Response Type",
                        div()
                            .w(px(220.))
                            .child(
                                Select::new(&self.oauth_response_type_select)
                                    .small()
                            ),
                        cx,
                    ))
            })
            .when(is_resource_owner, |el| {
                el.child(self.auth_input_field("Client ID", &self.auth_inputs.oauth_client_id, cx))
                    .child(self.auth_input_field(
                        "Client Secret",
                        &self.auth_inputs.oauth_client_secret,
                        cx,
                    ))
                    .child(self.auth_input_field(
                        "Access Token URL",
                        &self.auth_inputs.oauth_access_token_url,
                        cx,
                    ))
                    .child(self.auth_input_field("Audience", &self.auth_inputs.oauth_audience, cx))
                    .child(self.auth_input_field("Username", &self.auth_inputs.oauth_username, cx))
                    .child(self.auth_input_field("Password", &self.auth_inputs.oauth_password, cx))
            })
            .when(is_client_credentials, |el| {
                el.child(self.auth_input_field("Client ID", &self.auth_inputs.oauth_client_id, cx))
                    .child(self.auth_input_field(
                        "Client Secret",
                        &self.auth_inputs.oauth_client_secret,
                        cx,
                    ))
                    .child(self.auth_input_field(
                        "Access Token URL",
                        &self.auth_inputs.oauth_access_token_url,
                        cx,
                    ))
                    .child(self.auth_input_field("Audience", &self.auth_inputs.oauth_audience, cx))
            })
            .child(self.render_oauth_advanced(auth, cx))
    }

    pub(super) fn oauth_grant_label(value: &str) -> &'static str {
        match value {
            "Implicit" => "Implicit",
            "Resource Owner Password Credential" | "Password" => {
                "Resource Owner Password Credential"
            }
            "Client Credentials" => "Client Credentials",
            _ => "Authorization Code",
        }
    }

    pub(super) fn oauth_pkce_checkbox(
        auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(
                Checkbox::new("oauth-use-pkce")
                    .checked(auth.oauth_use_pkce)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        Self::update_auth_config(&this.app_state, cx, |auth| {
                            auth.oauth_use_pkce = *checked;
                        });
                    })),
            )
            .child(div().text_sm().child("Use PKCE"))
    }

    pub(super) fn render_oauth_advanced(
        &self,
        _auth: &AuthConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_3()
            .p_3()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child(div().text_sm().child("Advanced"))
            .child(self.auth_input_field("Scope", &self.auth_inputs.oauth_scope, cx))
            .child(self.auth_input_field("Header Name*", &self.auth_inputs.oauth_header_name, cx))
            .child(self.auth_input_field(
                "Header Prefix",
                &self.auth_inputs.oauth_header_prefix,
                cx,
            ))
            .child(Self::auth_select_field(
                "Send Credentials",
                div()
                    .w(px(200.))
                    .child(
                        Select::new(&self.oauth_send_credentials_select)
                            .small()
                    ),
                cx,
            ))
    }

    pub(super) fn auth_select_field(
        label: &'static str,
        child: impl IntoElement,
        cx: &App,
    ) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(label),
            )
            .child(child)
    }

    pub(super) fn auth_input_field(
        &self,
        label: &'static str,
        input: &Entity<InputState>,
        cx: &App,
    ) -> impl IntoElement {
        let variables = self.auth_variable_suggestions(cx);
        let app_state = self.app_state.clone();
        let input_for_menu = input.clone();
        let input_for_add = input.clone();
        let add_name = Self::auth_variable_name_for_field(label, input.read(cx).value().as_ref());

        Self::auth_select_field(
            label,
            h_flex()
                .gap_1()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .child(Input::new(input).w_full()),
                )
                .child(
                    DropdownButton::new(SharedString::from(format!(
                        "auth-var-menu-{}",
                        label.replace(|ch: char| !ch.is_ascii_alphanumeric(), "-")
                    )))
                    .button(
                        Button::new(SharedString::from(format!(
                            "auth-var-btn-{}",
                            label.replace(|ch: char| !ch.is_ascii_alphanumeric(), "-")
                        )))
                        .icon(IconName::Plus)
                        .ghost()
                        .xsmall()
                        .tooltip("Insert or create variable"),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        let menu = variables.iter().take(20).fold(menu, |menu, (name, value)| {
                            let name_for_label = name.clone();
                            let value_preview = Self::input_line(value);
                            let label = if value_preview.is_empty() {
                                format!("{{{{{name_for_label}}}}}")
                            } else {
                                format!(
                                    "{{{{{name_for_label}}}}} = {}",
                                    truncate_middle(&value_preview, 44)
                                )
                            };
                            let input = input_for_menu.clone();
                            let insert_name = name.clone();
                            menu.item(PopupMenuItem::new(label).on_click(move |_, window, cx| {
                                input.update(cx, |state, cx| {
                                    let next = insert_variable_reference(
                                        state.value().as_ref(),
                                        &insert_name,
                                    );
                                    state.set_value(next, window, cx);
                                });
                            }))
                        });

                        let input = input_for_add.clone();
                        let app_state = app_state.clone();
                        let add_name_for_label = add_name.clone();
                        let add_name_for_click = add_name.clone();
                        menu.separator().item(
                            PopupMenuItem::new(format!(
                                "Add variable: {{{{{add_name_for_label}}}}}"
                            ))
                            .on_click(move |_, window, cx| {
                                let add_name = add_name_for_click.clone();
                                let current_value = input.read(cx).value().to_string();
                                app_state.update(cx, |state, cx| {
                                    if let Some(var) = state
                                        .workspace
                                        .variables
                                        .iter_mut()
                                        .find(|var| var.name == add_name)
                                    {
                                        if var.value.is_empty() && !current_value.starts_with("{{")
                                        {
                                            var.value = current_value.clone();
                                        }
                                        var.enabled = true;
                                    } else {
                                        let value = if current_value.starts_with("{{") {
                                            String::new()
                                        } else {
                                            current_value.clone()
                                        };
                                        state.workspace.variables.push(
                                            silvapi_core::models::Variable::new(
                                                add_name.clone(),
                                                value,
                                            ),
                                        );
                                    }
                                    cx.emit(AppEvent::SaveNeeded);
                                });
                                input.update(cx, |state, cx| {
                                    state.set_value(format!("{{{{{add_name}}}}}"), window, cx);
                                });
                            }),
                        )
                    }),
                ),
            cx,
        )
    }

    pub(super) fn auth_multiline_field(
        label: &'static str,
        input: &Entity<InputState>,
        cx: &App,
    ) -> impl IntoElement {
        Self::auth_select_field(label, Input::new(input).h(px(82.)).w_full(), cx)
    }

    pub(super) fn auth_variable_suggestions(&self, cx: &App) -> Vec<(String, String)> {
        let state = self.app_state.read(cx);
        let mut suggestions = Vec::new();
        let mut push_unique = |name: &str, value: &str| {
            if !name.is_empty() && !suggestions.iter().any(|(existing, _)| existing == name) {
                suggestions.push((name.to_string(), value.to_string()));
            }
        };

        if let Some(request_id) = state.active_request_id.as_deref() {
            for collection in &state.workspace.collections {
                if collection_contains_request(collection, request_id) {
                    for var in &collection.variables {
                        if var.enabled {
                            push_unique(&var.name, &var.value);
                        }
                    }
                }
            }
        }

        for var in &state.workspace.variables {
            if var.enabled {
                push_unique(&var.name, &var.value);
            }
        }

        let _ = state;
        for name in self.request_variable_names(cx) {
            let value = self.variable_value(&name, cx).unwrap_or_default();
            push_unique(&name, &value);
        }

        suggestions.sort_by(|a, b| a.0.cmp(&b.0));
        suggestions
    }

    pub(super) fn auth_variable_name_for_field(label: &str, value: &str) -> String {
        if let Some(name) = Self::extract_variable_names(value).first() {
            return name.clone();
        }

        let mut out = String::new();
        let mut uppercase_next = false;
        for ch in label.trim_end_matches('*').chars() {
            if ch.is_ascii_alphanumeric() {
                if out.is_empty() {
                    out.push(ch.to_ascii_lowercase());
                } else if uppercase_next {
                    out.push(ch.to_ascii_uppercase());
                    uppercase_next = false;
                } else {
                    out.push(ch.to_ascii_lowercase());
                }
            } else {
                uppercase_next = !out.is_empty();
            }
        }
        if out.is_empty() {
            "variable".to_string()
        } else {
            out
        }
    }

    pub(super) fn update_auth_config<F>(app_state: &Entity<AppState>, cx: &mut App, f: F)
    where
        F: FnOnce(&mut AuthConfig),
    {
        app_state.update(cx, |state, cx| {
            if let Some(req) = &mut state.active_request {
                f(&mut req.auth);
                Self::ensure_auth_defaults(&mut req.auth);
            }
            state.save_active_request();
            cx.emit(AppEvent::WorkspaceChanged);
            cx.emit(AppEvent::SaveNeeded);
        });
    }

    pub(super) fn ensure_auth_defaults(auth: &mut AuthConfig) {
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
            auth.oauth_access_token_url = "https://github.com/login/oauth/access_token".to_string();
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
}

fn insert_variable_reference(current: &str, name: &str) -> String {
    let reference = format!("{{{{{name}}}}}");
    if current.trim().is_empty() {
        return reference;
    }
    if current.contains(&reference) {
        return current.to_string();
    }
    if let Some(start) = current.rfind("{{") {
        if current[start + 2..].find("}}").is_none() {
            let mut out = current[..start].to_string();
            out.push_str(&reference);
            return out;
        }
    }
    format!("{current}{reference}")
}

fn truncate_middle(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let head = keep / 2;
    let tail = keep - head;
    let start: String = value.chars().take(head).collect();
    let end: String = value
        .chars()
        .rev()
        .take(tail)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{start}…{end}")
}
