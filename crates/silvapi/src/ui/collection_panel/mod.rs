use std::collections::HashSet;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt as _, PopupMenuItem},
    spinner::Spinner,
    v_flex,
};

use silvapi_core::models::{AuthType, CollectionItem, Folder, KeyValue};
use crate::{state::{AppEvent, AppState}};

use super::actions::{FocusActiveRequest, RenameSelected};

// ── Drag payload ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct DragItem {
    id: String,
    display: String,
}

struct DragPreview(String);
impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .text_sm()
            .text_color(cx.theme().foreground)
            .child(self.0.clone())
    }
}

// ── Flat list item ───────────────────────────────────────────────────────────

#[derive(Clone)]
struct FlatItem {
    id: String,
    display: String,
    method_str: String,
    method_color: Hsla,
    depth: usize,
    is_folder: bool,
    is_collection_root: bool,
    is_expanded: bool,
}

#[derive(Clone, Copy)]
enum RequestRowStatus {
    Loading,
    Status(u16),
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FolderSettingsTab {
    General,
    Headers,
    Auth,
    Variables,
}

struct KvRow {
    key: Entity<InputState>,
    value: Entity<InputState>,
    enabled: bool,
}

struct FolderAuthInputs {
    bearer_token: Entity<InputState>,
    bearer_prefix: Entity<InputState>,
    basic_username: Entity<InputState>,
    basic_password: Entity<InputState>,
    api_key_name: Entity<InputState>,
    api_key_value: Entity<InputState>,
    aws_access_key_id: Entity<InputState>,
    aws_secret_access_key: Entity<InputState>,
    aws_service: Entity<InputState>,
    aws_region: Entity<InputState>,
    aws_session_token: Entity<InputState>,
    jwt_secret: Entity<InputState>,
    jwt_payload: Entity<InputState>,
    oauth_client_id: Entity<InputState>,
    oauth_client_secret: Entity<InputState>,
    oauth_authorization_url: Entity<InputState>,
    oauth_access_token_url: Entity<InputState>,
    oauth_redirect_uri: Entity<InputState>,
    oauth_state: Entity<InputState>,
    oauth_audience: Entity<InputState>,
    oauth_username: Entity<InputState>,
    oauth_password: Entity<InputState>,
    oauth_scope: Entity<InputState>,
    oauth_header_name: Entity<InputState>,
    oauth_header_prefix: Entity<InputState>,
}

impl FolderAuthInputs {
    fn all(&self) -> Vec<Entity<InputState>> {
        vec![
            self.bearer_token.clone(),
            self.bearer_prefix.clone(),
            self.basic_username.clone(),
            self.basic_password.clone(),
            self.api_key_name.clone(),
            self.api_key_value.clone(),
            self.aws_access_key_id.clone(),
            self.aws_secret_access_key.clone(),
            self.aws_service.clone(),
            self.aws_region.clone(),
            self.aws_session_token.clone(),
            self.jwt_secret.clone(),
            self.jwt_payload.clone(),
            self.oauth_client_id.clone(),
            self.oauth_client_secret.clone(),
            self.oauth_authorization_url.clone(),
            self.oauth_access_token_url.clone(),
            self.oauth_redirect_uri.clone(),
            self.oauth_state.clone(),
            self.oauth_audience.clone(),
            self.oauth_username.clone(),
            self.oauth_password.clone(),
            self.oauth_scope.clone(),
            self.oauth_header_name.clone(),
            self.oauth_header_prefix.clone(),
        ]
    }
}

// ── Panel ────────────────────────────────────────────────────────────────────

pub struct CollectionPanel {
    focus_handle: FocusHandle,
    app_state: Entity<AppState>,
    expanded: HashSet<String>,
    selected_id: Option<String>,
    renaming_id: Option<String>,
    rename_input: Entity<InputState>,
    search_input: Entity<InputState>,
    flat_cache: Vec<FlatItem>,
    flat_dirty: bool,
    list_scroll: gpui::UniformListScrollHandle,
    folder_settings_id: Option<String>,
    folder_settings_tab: FolderSettingsTab,
    folder_name_input: Entity<InputState>,
    folder_description_input: Entity<InputState>,
    folder_header_rows: Vec<KvRow>,
    folder_variable_rows: Vec<KvRow>,
    folder_auth_enabled: bool,
    folder_auth_type: AuthType,
    folder_auth_inputs: FolderAuthInputs,

    _subs: Vec<Subscription>,
    _folder_auth_subs: Vec<Subscription>,
}

impl Focusable for CollectionPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}


mod folder_settings;
mod tree;

impl CollectionPanel {
    pub fn new(app_state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let rename_input = cx.new(|cx| InputState::new(window, cx).placeholder("Rename..."));
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let folder_name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Folder name"));
        let folder_description_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .soft_wrap(true)
                .placeholder("Folder description")
        });

        let folder_auth_inputs = Self::make_folder_auth_inputs(window, cx);
        let folder_auth_subs = folder_auth_inputs
            .all()
            .into_iter()
            .map(|input| {
                cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, _, cx| {
                    if matches!(ev, InputEvent::Change) {
                        this.save_folder_settings(cx);
                    }
                })
            })
            .collect::<Vec<_>>();

        // Expand all collections by default
        let mut expanded = HashSet::new();
        for col in &app_state.read(cx).workspace.collections {
            expanded.insert(col.id.clone());
        }

        let ws_sub = cx.subscribe_in(
            &app_state,
            window,
            |this, app_state, ev: &AppEvent, window, cx| {
                match ev {
                    AppEvent::WorkspaceChanged => {
                        // Auto-expand new collections
                        for col in &app_state.read(cx).workspace.collections {
                            this.expanded.insert(col.id.clone());
                        }
                        this.flat_dirty = true;
                        cx.notify();
                    }
                    AppEvent::RequestSelected => {
                        this.reveal_active_request(false, window, cx);
                    }
                    _ => {}
                }
            },
        );

        let rename_sub = cx.subscribe_in(
            &rename_input,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { .. } => this.commit_rename(window, cx),
                InputEvent::Blur => this.commit_rename(window, cx),
                _ => {}
            },
        );

        let search_sub = cx.subscribe_in(
            &search_input,
            window,
            |this, _, ev: &InputEvent, _, cx| match ev {
                InputEvent::Change => {
                    this.flat_dirty = true;
                    cx.notify();
                }
                _ => {}
            },
        );

        Self {
            focus_handle,
            app_state,
            expanded,
            selected_id: None,
            renaming_id: None,
            rename_input,
            search_input,
            flat_cache: Vec::new(),
            flat_dirty: true,
            list_scroll: gpui::UniformListScrollHandle::new(),
            folder_settings_id: None,
            folder_settings_tab: FolderSettingsTab::General,
            folder_name_input,
            folder_description_input,
            folder_header_rows: Vec::new(),
            folder_variable_rows: Vec::new(),
            folder_auth_enabled: false,
            folder_auth_type: AuthType::None,
            folder_auth_inputs,
            _subs: vec![ws_sub, rename_sub, search_sub],
            _folder_auth_subs: folder_auth_subs,
        }
    }

    fn make_kv_row(
        &self,
        key: &str,
        value: &str,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> KvRow {
        let key_input = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let value_input = cx.new(|cx| InputState::new(window, cx).placeholder("Value"));
        key_input.update(cx, |state, cx| {
            state.set_value(key.to_string(), window, cx);
        });
        value_input.update(cx, |state, cx| {
            state.set_value(value.to_string(), window, cx);
        });
        KvRow {
            key: key_input,
            value: value_input,
            enabled,
        }
    }

}


impl Render for CollectionPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let row_count = self.flat_items(cx).len();

        let app_new_http = self.app_state.clone();
        let app_new_folder = self.app_state.clone();
        let selected_for_new = self.selected_id.clone();

        v_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "up" | "down" => {
                        let flat = this.flat_items(cx);
                        if flat.is_empty() {
                            return;
                        }
                        let current_pos = this
                            .selected_id
                            .as_ref()
                            .and_then(|sid| flat.iter().position(|f| f.id == *sid));
                        let new_pos = match event.keystroke.key.as_str() {
                            "up" => match current_pos {
                                Some(0) => flat.len() - 1,
                                Some(pos) => pos - 1,
                                None => flat.len() - 1,
                            },
                            _ => match current_pos {
                                Some(pos) => (pos + 1) % flat.len(),
                                None => 0,
                            },
                        };
                        let item = &flat[new_pos];
                        let item_id = item.id.clone();
                        let is_folder = item.is_folder;
                        this.selected_id = Some(item_id.clone());
                        if !is_folder {
                            this.app_state.update(cx, |state, cx| {
                                if state.select_request(&item_id) {
                                    cx.emit(AppEvent::RequestSelected);
                                }
                            });
                        }
                        this.list_scroll
                            .scroll_to_item(new_pos, gpui::ScrollStrategy::Nearest);
                        cx.notify();
                    }
                    "space" => {
                        let sid = this.selected_id.clone();
                        if let Some(sid) = sid {
                            let is_folder = this
                                .flat_items(cx)
                                .iter()
                                .any(|f| f.id == sid && f.is_folder);
                            if is_folder {
                                if this.expanded.contains(&sid) {
                                    this.expanded.remove(&sid);
                                } else {
                                    this.expanded.insert(sid.clone());
                                }
                                this.flat_dirty = true;
                                cx.notify();
                            }
                        }
                    }
                    "right" => {
                        let sid = this.selected_id.clone();
                        if let Some(sid) = sid {
                            let is_folder = this
                                .flat_items(cx)
                                .iter()
                                .any(|f| f.id == sid && f.is_folder);
                            if is_folder {
                                this.expanded.insert(sid.clone());
                                this.flat_dirty = true;
                                cx.notify();
                            }
                        }
                    }
                    "left" => {
                        let sid = this.selected_id.clone();
                        if let Some(sid) = sid {
                            let is_folder = this
                                .flat_items(cx)
                                .iter()
                                .any(|f| f.id == sid && f.is_folder);
                            if is_folder {
                                this.expanded.remove(&sid);
                                this.flat_dirty = true;
                                cx.notify();
                            }
                        }
                    }
                    "escape" => {
                        if this.renaming_id.is_some() {
                            this.renaming_id = None;
                            let fh = this.focus_handle.clone();
                            cx.defer_in(window, move |_, window, cx| {
                                fh.focus(window, cx);
                            });
                            cx.notify();
                        }
                    }
                    _ => {}
                }
            }))
            .on_action(cx.listener(|this, _: &RenameSelected, window, cx| {
                if window.has_focused_input(cx) {
                    return;
                }
                if this.renaming_id.is_some() {
                    this.commit_rename(window, cx);
                    return;
                }
                if let Some(id) = this.selected_id.clone() {
                    let name = find_name_by_id(this.app_state.read(cx), &id).unwrap_or_default();
                    this.rename_input
                        .update(cx, |s, cx| s.set_value(name, window, cx));
                    this.renaming_id = Some(id);
                    // focus the rename input
                    let fh = this.rename_input.read(cx).focus_handle(cx);
                    fh.focus(window, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(
                |this, _: &crate::ui::actions::FocusActiveRequest, window, cx| {
                    this.reveal_active_request(true, window, cx);
                },
            ))
            // Header
            .child(
                h_flex()
                    .px_3()
                    .py_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .items_center()
                    .child(Input::new(&self.search_input).flex_1().h(px(28.))),
            )
            // List
            .child(
                div()
                    .id("collection-list-container")
                    .flex_1()
                    .relative()
                    // Background to catch right-clicks on empty space
                    .child(div().absolute().inset_0().context_menu(move |menu, _, _| {
                        let app_new_http = app_new_http.clone();
                        let app_new_folder = app_new_folder.clone();
                        let target_http = selected_for_new.clone();
                        let target_folder = selected_for_new.clone();

                        menu.item(
                            PopupMenuItem::new("New HTTP").on_click(move |_, window, cx| {
                                app_new_http.update(cx, |state, cx| {
                                    if let Some(new_id) =
                                        state.add_request_to_target(target_http.as_deref())
                                    {
                                        state.select_request(&new_id);
                                        cx.emit(AppEvent::RequestSelected);
                                    }
                                    cx.emit(AppEvent::WorkspaceChanged);
                                });
                                window.dispatch_action(Box::new(FocusActiveRequest), cx);
                            }),
                        )
                        .item(PopupMenuItem::new("New Folder").on_click(move |_, _, cx| {
                            app_new_folder.update(cx, |state, cx| {
                                state.add_folder_to_target(target_folder.as_deref());
                                cx.emit(AppEvent::WorkspaceChanged);
                            });
                        }))
                    }))
                    // The actual list over it
                    .child(
                        uniform_list(
                            "collection-list-scroll",
                            row_count,
                            cx.processor(move |this: &mut Self, range, _window, cx| {
                                this.render_rows(range, cx)
                            }),
                        )
                        .track_scroll(&self.list_scroll)
                        .size_full(),
                    ),
            )
    }
}

fn start_rename(panel: &Entity<CollectionPanel>, id: &str, window: &mut Window, cx: &mut App) {
    let id = id.to_string();
    panel.update(cx, |this, cx| {
        let name = find_name_by_id(this.app_state.read(cx), &id).unwrap_or_default();
        this.rename_input
            .update(cx, |s, cx| s.set_value(name, window, cx));
        this.renaming_id = Some(id);
        this.selected_id = this.renaming_id.clone();
        let fh = this.rename_input.read(cx).focus_handle(cx);
        fh.focus(window, cx);
        cx.notify();
    });
}

fn format_curl(req: &silvapi_core::models::ApiRequest) -> String {
    let mut parts = vec![
        "curl".to_string(),
        "-X".to_string(),
        shell_quote(req.method.as_str()),
        shell_quote(&req.url),
    ];

    for header in &req.headers {
        if header.enabled && !header.key.is_empty() {
            parts.push("-H".to_string());
            parts.push(shell_quote(&format!("{}: {}", header.key, header.value)));
        }
    }

    let body = if matches!(req.body.body_type, silvapi_core::models::BodyType::UrlEncoded)
        && !req.body.urlencoded.is_empty()
    {
        crate::http::build_urlencoded_body(&req.body.urlencoded)
    } else {
        req.body.content.clone()
    };

    if !body.is_empty() {
        parts.push("-d".to_string());
        parts.push(shell_quote(&body));
    }

    parts.join(" ")
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/:._?=&-%{}".contains(c))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn find_name_by_id(state: &AppState, id: &str) -> Option<String> {
    for col in &state.workspace.collections {
        if col.id == id {
            return Some(col.name.clone());
        }
        if let Some(n) = find_name_in_items(&col.items, id) {
            return Some(n);
        }
    }
    None
}

fn find_name_in_items(items: &[CollectionItem], id: &str) -> Option<String> {
    for item in items {
        match item {
            CollectionItem::Request(r) if r.id == id => return Some(r.name.clone()),
            CollectionItem::Folder(f) if f.id == id => return Some(f.name.clone()),
            CollectionItem::Folder(f) => {
                if let Some(n) = find_name_in_items(&f.items, id) {
                    return Some(n);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_request_path(
    items: &[CollectionItem],
    request_id: &str,
    current_path: &mut Vec<String>,
) -> bool {
    for item in items {
        match item {
            CollectionItem::Request(request) if request.id == request_id => return true,
            CollectionItem::Folder(folder) => {
                current_path.push(folder.id.clone());
                if find_request_path(&folder.items, request_id, current_path) {
                    return true;
                }
                current_path.pop();
            }
            _ => {}
        }
    }
    false
}

fn find_folder_in_items<'a>(items: &'a [CollectionItem], id: &str) -> Option<&'a Folder> {
    for item in items {
        match item {
            CollectionItem::Folder(folder) if folder.id == id => return Some(folder),
            CollectionItem::Folder(folder) => {
                if let Some(found) = find_folder_in_items(&folder.items, id) {
                    return Some(found);
                }
            }
            CollectionItem::Request(_) => {}
        }
    }
    None
}

fn find_folder_in_items_mut<'a>(
    items: &'a mut [CollectionItem],
    id: &str,
) -> Option<&'a mut Folder> {
    for item in items {
        if let CollectionItem::Folder(folder) = item {
            if folder.id == id {
                return Some(folder);
            }
            {
                if let Some(found) = find_folder_in_items_mut(&mut folder.items, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn collect_kv_rows(rows: &[KvRow], cx: &App) -> Vec<KeyValue> {
    rows.iter()
        .filter_map(|row| {
            let key = row.key.read(cx).value().to_string();
            let value = row.value.read(cx).value().to_string();
            (!key.is_empty() || !value.is_empty()).then(|| KeyValue {
                id: uuid::Uuid::new_v4().to_string(),
                key,
                value,
                enabled: row.enabled,
                description: String::new(),
            })
        })
        .collect()
}

fn label_text(label: &'static str, cx: &mut Context<CollectionPanel>) -> AnyElement {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(label)
        .into_any_element()
}

fn folder_default_if_empty(value: &str, default: &str) -> String {
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_string()
    }
}

fn folder_oauth_grant_label(value: &str) -> &'static str {
    match value {
        "Implicit" => "Implicit",
        "Resource Owner Password Credential" | "Password" => {
            "Resource Owner Password Credential"
        }
        "Client Credentials" => "Client Credentials",
        _ => "Authorization Code",
    }
}

fn folder_auth_select_field(
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

fn folder_auth_input_field(
    label: &'static str,
    input: &Entity<InputState>,
    cx: &App,
) -> impl IntoElement {
    folder_auth_select_field(label, Input::new(input).w_full(), cx)
}

fn folder_auth_multiline_field(
    label: &'static str,
    input: &Entity<InputState>,
    cx: &App,
) -> impl IntoElement {
    folder_auth_select_field(label, Input::new(input).h(px(82.)).w_full(), cx)
}

fn folder_kv_header(cx: &mut Context<CollectionPanel>) -> AnyElement {
    h_flex()
        .gap_2()
        .items_center()
        .px_1()
        .child(div().w(px(18.)))
        .child(
            div()
                .flex_1()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Name"),
        )
        .child(
            div()
                .flex_1()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Value"),
        )
        .child(div().w(px(34.)))
        .into_any_element()
}

fn render_folder_kv_row(
    row: &KvRow,
    index: usize,
    variables: bool,
    cx: &mut Context<CollectionPanel>,
) -> AnyElement {
    let key = row.key.clone();
    let value = row.value.clone();
    let id = if variables {
        format!("remove-folder-var-{index}")
    } else {
        format!("remove-folder-header-{index}")
    };

    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .w(px(18.))
                .h(px(18.))
                .rounded_sm()
                .border_1()
                .border_color(cx.theme().primary)
                .bg(cx.theme().primary)
                .child(
                    Icon::new(IconName::Check)
                        .xsmall()
                        .text_color(cx.theme().background),
                ),
        )
        .child(div().flex_1().child(Input::new(&key).h(px(32.))))
        .child(div().flex_1().child(Input::new(&value).h(px(32.))))
        .child(
            Button::new(SharedString::from(id))
                .icon(IconName::Close)
                .ghost()
                .small()
                .on_click(cx.listener(move |this, _, _, cx| {
                    if variables {
                        if index < this.folder_variable_rows.len() {
                            this.folder_variable_rows.remove(index);
                        }
                    } else if index < this.folder_header_rows.len() {
                        this.folder_header_rows.remove(index);
                    }
                    cx.notify();
                })),
        )
        .into_any_element()
}

fn request_row_status(state: &AppState, request_id: &str) -> Option<RequestRowStatus> {
    let activity = state.request_activity.get(request_id);
    if activity
        .map(|activity| activity.is_loading())
        .unwrap_or(false)
    {
        return Some(RequestRowStatus::Loading);
    }

    if let Some(status) = activity.and_then(|activity| activity.last_status) {
        return Some(RequestRowStatus::Status(status));
    }

    if activity
        .and_then(|activity| activity.last_error.as_ref())
        .is_some()
    {
        return Some(RequestRowStatus::Error);
    }

    state
        .workspace
        .response_cache
        .get(request_id)
        .and_then(|history| history.last())
        .map(|response| RequestRowStatus::Status(response.status))
}

fn render_request_row_status(
    status: RequestRowStatus,
    cx: &mut Context<CollectionPanel>,
) -> AnyElement {
    match status {
        RequestRowStatus::Loading => Spinner::new()
            .xsmall()
            .color(cx.theme().primary)
            .into_any_element(),
        RequestRowStatus::Status(code) => {
            let color: Hsla = if (200..300).contains(&code) {
                rgb(0x00C853).into()
            } else if (300..400).contains(&code) {
                rgb(0xFF9800).into()
            } else {
                rgb(0xFF4081).into()
            };
            div()
                .flex_none()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(color)
                .child(code.to_string())
                .into_any_element()
        }
        RequestRowStatus::Error => div()
            .flex_none()
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .text_color(Hsla::from(rgb(0xFF4081)))
            .child("ERR")
            .into_any_element(),
    }
}

fn method_color(method: &str) -> Hsla {
    match method {
        "GET" => rgb(0x4CAF50).into(),
        "POST" => rgb(0x2196F3).into(),
        "PUT" => rgb(0xFF9800).into(),
        "PATCH" => rgb(0x9C27B0).into(),
        "DELETE" => rgb(0xF44336).into(),
        _ => rgb(0x607D8B).into(),
    }
}

