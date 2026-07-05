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

    pub(super) fn load_auth_inputs(&self, auth: &AuthConfig, window: &mut Window, cx: &mut Context<Self>) {
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
            .child(Self::auth_type_dropdown(
                self.app_state.clone(),
                &auth.auth_type,
            ))
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

    pub(super) fn auth_type_dropdown(app_state: Entity<AppState>, current: &AuthType) -> impl IntoElement {
        let current = current.clone();
        let current_label = Self::auth_type_label(&current);
        let current_for_items = current.clone();
        let item = move |target: AuthType, label: &'static str| {
            let app_state = app_state.clone();
            let selected = current_for_items == target;
            PopupMenuItem::new(label)
                .checked(selected)
                .on_click(move |_, _, cx| {
                    app_state.update(cx, |state, cx| {
                        if let Some(req) = &mut state.active_request {
                            req.auth.auth_type = target.clone();
                            Self::ensure_auth_defaults(&mut req.auth);
                        }
                        state.save_active_request();
                        cx.emit(AppEvent::WorkspaceChanged);
                        cx.emit(AppEvent::SaveNeeded);
                    });
                })
        };

        DropdownButton::new("auth-type-dropdown")
            .button(
                Button::new("auth-type-btn")
                    .label(current_label)
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                menu.item(item(AuthType::ApiKey, "API Key"))
                    .item(item(AuthType::AwsV4, "AWS Signature"))
                    .item(item(AuthType::Basic, "Basic Auth"))
                    .item(item(AuthType::Bearer, "Bearer Token"))
                    .item(item(AuthType::Jwt, "JWT Bearer"))
                    .item(item(AuthType::OAuth2, "OAuth 2.0"))
                    .separator()
                    .item(item(AuthType::None, "Inherit from Parent"))
                    .item(item(AuthType::None, "No Auth"))
            })
    }

    pub(super) fn render_api_key_auth(&self, auth: &AuthConfig, cx: &mut Context<Self>) -> impl IntoElement {
        let api_key_in_header = auth.api_key_in_header;
        let behavior_label = if auth.api_key_in_header {
            "Insert Header"
        } else {
            "Insert Query Param"
        };
        v_flex()
            .gap_3()
            .child(Self::auth_select_field(
                "Behavior",
                DropdownButton::new("api-key-behavior")
                    .button(
                        Button::new("api-key-behavior-btn")
                            .label(behavior_label)
                            .ghost()
                            .small(),
                    )
                    .dropdown_menu({
                        let app_state = self.app_state.clone();
                        move |menu, _, _| {
                            let header_state = app_state.clone();
                            let query_state = app_state.clone();
                            menu.item(
                                PopupMenuItem::new("Insert Header")
                                    .checked(api_key_in_header)
                                    .on_click(move |_, _, cx| {
                                        Self::update_auth_config(&header_state, cx, |auth| {
                                            auth.api_key_in_header = true;
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("Insert Query Param")
                                    .checked(!api_key_in_header)
                                    .on_click(move |_, _, cx| {
                                        Self::update_auth_config(&query_state, cx, |auth| {
                                            auth.api_key_in_header = false;
                                        });
                                    }),
                            )
                        }
                    }),
                cx,
            ))
            .child(Self::auth_input_field(
                if auth.api_key_in_header {
                    "Header Name*"
                } else {
                    "Query Param Name*"
                },
                &self.auth_inputs.api_key_name,
                cx,
            ))
            .child(Self::auth_input_field(
                "API Key",
                &self.auth_inputs.api_key_value,
                cx,
            ))
    }

    pub(super) fn render_aws_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(Self::auth_input_field(
                "Access Key ID*",
                &self.auth_inputs.aws_access_key_id,
                cx,
            ))
            .child(Self::auth_input_field(
                "Secret Access Key*",
                &self.auth_inputs.aws_secret_access_key,
                cx,
            ))
            .child(Self::auth_input_field(
                "Service Name*",
                &self.auth_inputs.aws_service,
                cx,
            ))
            .child(Self::auth_input_field(
                "Region",
                &self.auth_inputs.aws_region,
                cx,
            ))
            .child(Self::auth_input_field(
                "Session Token",
                &self.auth_inputs.aws_session_token,
                cx,
            ))
    }

    pub(super) fn render_basic_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(Self::auth_input_field(
                "Username",
                &self.auth_inputs.basic_username,
                cx,
            ))
            .child(Self::auth_input_field(
                "Password",
                &self.auth_inputs.basic_password,
                cx,
            ))
    }

    pub(super) fn render_bearer_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(Self::auth_input_field(
                "Token",
                &self.auth_inputs.bearer_token,
                cx,
            ))
            .child(Self::auth_input_field(
                "Prefix",
                &self.auth_inputs.bearer_prefix,
                cx,
            ))
    }

    pub(super) fn render_jwt_auth(&self, auth: &AuthConfig, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                DropdownButton::new("jwt-algorithm")
                    .button(
                        Button::new("jwt-algorithm-btn")
                            .label(if auth.jwt_algorithm.is_empty() {
                                "HS256"
                            } else {
                                auth.jwt_algorithm.as_str()
                            })
                            .ghost()
                            .small(),
                    )
                    .dropdown_menu({
                        let app_state = self.app_state.clone();
                        let current = auth.jwt_algorithm.clone();
                        move |menu, _, _| {
                            ["HS256", "HS384", "HS512", "RS256", "RS384", "RS512"]
                                .into_iter()
                                .fold(menu, |menu, algorithm| {
                                    let app_state = app_state.clone();
                                    menu.item(
                                        PopupMenuItem::new(algorithm)
                                            .checked(current == algorithm)
                                            .on_click(move |_, _, cx| {
                                                Self::update_auth_config(&app_state, cx, |auth| {
                                                    auth.jwt_algorithm = algorithm.to_string();
                                                });
                                            }),
                                    )
                                })
                        }
                    }),
            )
            .child(Self::auth_input_field(
                "Secret or Private Key",
                &self.auth_inputs.jwt_secret,
                cx,
            ))
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

    pub(super) fn render_oauth_auth(&self, auth: &AuthConfig, cx: &mut Context<Self>) -> impl IntoElement {
        let grant = Self::oauth_grant_label(&auth.oauth_grant_type);
        let is_authorization_code = grant == "Authorization Code";
        let is_implicit = grant == "Implicit";
        let is_resource_owner = grant == "Resource Owner Password Credential";
        let is_client_credentials = grant == "Client Credentials";

        v_flex()
            .gap_3()
            .child(Self::oauth_grant_dropdown(
                self.app_state.clone(),
                &auth.oauth_grant_type,
            ))
            .when(is_authorization_code, |el| {
                el.child(Self::auth_input_field(
                    "Client ID",
                    &self.auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Client Secret",
                    &self.auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Authorization URL",
                    &self.auth_inputs.oauth_authorization_url,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Access Token URL",
                    &self.auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Redirect URI",
                    &self.auth_inputs.oauth_redirect_uri,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "State",
                    &self.auth_inputs.oauth_state,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Audience",
                    &self.auth_inputs.oauth_audience,
                    cx,
                ))
                .child(Self::auth_select_field(
                    "Token for authorization",
                    Self::oauth_token_target_dropdown(self.app_state.clone(), auth),
                    cx,
                ))
                .child(Self::oauth_pkce_checkbox(auth, cx))
            })
            .when(is_implicit, |el| {
                el.child(Self::auth_input_field(
                    "Client ID",
                    &self.auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Authorization URL",
                    &self.auth_inputs.oauth_authorization_url,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Redirect URI",
                    &self.auth_inputs.oauth_redirect_uri,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "State",
                    &self.auth_inputs.oauth_state,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Audience",
                    &self.auth_inputs.oauth_audience,
                    cx,
                ))
                .child(Self::auth_select_field(
                    "Token for authorization",
                    Self::oauth_token_target_dropdown(self.app_state.clone(), auth),
                    cx,
                ))
                .child(Self::auth_select_field(
                    "Response Type",
                    Self::oauth_response_type_dropdown(self.app_state.clone(), auth),
                    cx,
                ))
            })
            .when(is_resource_owner, |el| {
                el.child(Self::auth_input_field(
                    "Client ID",
                    &self.auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Client Secret",
                    &self.auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Access Token URL",
                    &self.auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Audience",
                    &self.auth_inputs.oauth_audience,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Username",
                    &self.auth_inputs.oauth_username,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Password",
                    &self.auth_inputs.oauth_password,
                    cx,
                ))
            })
            .when(is_client_credentials, |el| {
                el.child(Self::auth_input_field(
                    "Client ID",
                    &self.auth_inputs.oauth_client_id,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Client Secret",
                    &self.auth_inputs.oauth_client_secret,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Access Token URL",
                    &self.auth_inputs.oauth_access_token_url,
                    cx,
                ))
                .child(Self::auth_input_field(
                    "Audience",
                    &self.auth_inputs.oauth_audience,
                    cx,
                ))
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

    pub(super) fn oauth_grant_dropdown(app_state: Entity<AppState>, current: &str) -> impl IntoElement {
        let current = Self::oauth_grant_label(current);
        let app_state_for_items = app_state.clone();
        let item = move |grant: &'static str| {
            let app_state = app_state_for_items.clone();
            PopupMenuItem::new(grant)
                .checked(current == grant)
                .on_click(move |_, _, cx| {
                    Self::update_auth_config(&app_state, cx, |auth| {
                        auth.oauth_grant_type = grant.to_string();
                    });
                })
        };

        DropdownButton::new("oauth-grant")
            .button(
                Button::new("oauth-grant-btn")
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

    pub(super) fn oauth_token_target_dropdown(
        app_state: Entity<AppState>,
        auth: &AuthConfig,
    ) -> impl IntoElement {
        let current = default_if_empty(&auth.oauth_token_target, "access_token");
        DropdownButton::new("oauth-token-target")
            .button(
                Button::new("oauth-token-target-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["access_token", "id_token"]
                    .into_iter()
                    .fold(menu, |menu, token| {
                        let app_state = app_state.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(token)
                                .checked(current == token)
                                .on_click(move |_, _, cx| {
                                    Self::update_auth_config(&app_state, cx, |auth| {
                                        auth.oauth_token_target = token.to_string();
                                    });
                                }),
                        )
                    })
            })
    }

    pub(super) fn oauth_response_type_dropdown(
        app_state: Entity<AppState>,
        auth: &AuthConfig,
    ) -> impl IntoElement {
        let current = default_if_empty(&auth.oauth_response_type, "Access Token");
        DropdownButton::new("oauth-response-type")
            .button(
                Button::new("oauth-response-type-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["Access Token", "ID Token", "Access Token + ID Token"]
                    .into_iter()
                    .fold(menu, |menu, response_type| {
                        let app_state = app_state.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(response_type)
                                .checked(current == response_type)
                                .on_click(move |_, _, cx| {
                                    Self::update_auth_config(&app_state, cx, |auth| {
                                        auth.oauth_response_type = response_type.to_string();
                                    });
                                }),
                        )
                    })
            })
    }

    pub(super) fn oauth_send_credentials_dropdown(
        app_state: Entity<AppState>,
        auth: &AuthConfig,
    ) -> impl IntoElement {
        let current = default_if_empty(&auth.oauth_send_credentials, "In Request Body");
        DropdownButton::new("oauth-send-credentials")
            .button(
                Button::new("oauth-send-credentials-btn")
                    .label(current.clone())
                    .ghost()
                    .small(),
            )
            .dropdown_menu(move |menu, _, _| {
                ["In Request Body", "As Basic Authentication"]
                    .into_iter()
                    .fold(menu, |menu, target| {
                        let app_state = app_state.clone();
                        let current = current.clone();
                        menu.item(
                            PopupMenuItem::new(target)
                                .checked(current == target)
                                .on_click(move |_, _, cx| {
                                    Self::update_auth_config(&app_state, cx, |auth| {
                                        auth.oauth_send_credentials = target.to_string();
                                    });
                                }),
                        )
                    })
            })
    }

    pub(super) fn oauth_pkce_checkbox(auth: &AuthConfig, cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(super) fn render_oauth_advanced(&self, auth: &AuthConfig, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .p_3()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child(div().text_sm().child("Advanced"))
            .child(Self::auth_input_field(
                "Scope",
                &self.auth_inputs.oauth_scope,
                cx,
            ))
            .child(Self::auth_input_field(
                "Header Name*",
                &self.auth_inputs.oauth_header_name,
                cx,
            ))
            .child(Self::auth_input_field(
                "Header Prefix",
                &self.auth_inputs.oauth_header_prefix,
                cx,
            ))
            .child(Self::auth_select_field(
                "Send Credentials",
                Self::oauth_send_credentials_dropdown(self.app_state.clone(), auth),
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
        label: &'static str,
        input: &Entity<InputState>,
        cx: &App,
    ) -> impl IntoElement {
        Self::auth_select_field(label, Input::new(input).w_full(), cx)
    }

    pub(super) fn auth_multiline_field(
        label: &'static str,
        input: &Entity<InputState>,
        cx: &App,
    ) -> impl IntoElement {
        Self::auth_select_field(label, Input::new(input).h(px(82.)).w_full(), cx)
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
