use gpui_kit::component::{
    h_flex,
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement as _,
    text::TextView,
    tooltip::Tooltip,
    v_flex, ActiveTheme, Root, Theme, ThemeRegistry,
};
use gpui_kit::*;
use std::path::PathBuf;

mod anthropic;
use anthropic::{ClaudeConfig, ClaudeStreamEvent, UsageLimitInfo, send_prompt_stream};

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Extra {
    think: String,
    tools: Vec<(String, String, String)>,
}

struct Msg {
    user: bool,
    text: String,
    extra: Extra,
    open: bool,
}

struct Chat {
    input: Entity<InputState>,
    msgs: Vec<Msg>,
    loading: bool,
    parent: Option<String>,
    usage: UsageLimitInfo,
    pad: f32,
    focus: FocusHandle,
    scroll: ScrollHandle,

    config: ClaudeConfig,
    config_open: bool,
    cookie_input: Entity<InputState>,
    org_id_input: Entity<InputState>,
    conv_id_input: Entity<InputState>,
    device_id_input: Entity<InputState>,
    model_input: Entity<InputState>,
}

impl Chat {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Message"));
        cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. }) {
                this.submit(window, cx);
            }
        })
        .detach();
        input.update(cx, |s, cx| s.focus(window, cx));
        let (msgs, parent) = load_history();
        let config = load_config();
        let cookie_input = cx.new(|cx| InputState::new(window, cx).placeholder("Cookie").default_value(&config.cookie));
        let org_id_input = cx.new(|cx| InputState::new(window, cx).placeholder("Organization ID").default_value(&config.org_id));
        let conv_id_input = cx.new(|cx| InputState::new(window, cx).placeholder("Conversation ID").default_value(&config.conv_id));
        let device_id_input = cx.new(|cx| InputState::new(window, cx).placeholder("Device ID").default_value(&config.device_id));
        let model_input = cx.new(|cx| InputState::new(window, cx).placeholder("Model").default_value(&config.model));

        Self {
            input,
            msgs,
            loading: false,
            parent,
            usage: UsageLimitInfo::default(),
            pad: load_pad(),
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            config,
            config_open: false,
            cookie_input,
            org_id_input,
            conv_id_input,
            device_id_input,
            model_input,
        }
    }

    fn open_config(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cfg = self.config.clone();
        self.cookie_input.update(cx, |s, cx| s.set_value(&cfg.cookie, window, cx));
        self.org_id_input.update(cx, |s, cx| s.set_value(&cfg.org_id, window, cx));
        self.conv_id_input.update(cx, |s, cx| s.set_value(&cfg.conv_id, window, cx));
        self.device_id_input.update(cx, |s, cx| s.set_value(&cfg.device_id, window, cx));
        self.model_input.update(cx, |s, cx| s.set_value(&cfg.model, window, cx));
        self.config_open = true;
        self.cookie_input.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }

    fn close_config(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.config_open = false;
        self.input.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }

    fn save_config_from_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cookie = self.cookie_input.read(cx).value().trim().to_string();
        let org_id = clean_id(&self.org_id_input.read(cx).value());
        let conv_id = clean_id(&self.conv_id_input.read(cx).value());
        let device_id = self.device_id_input.read(cx).value().trim().to_string();
        let model = self.model_input.read(cx).value().trim().to_string();

        let old_conv = self.config.conv_id.clone();
        self.config = ClaudeConfig {
            cookie,
            org_id,
            conv_id,
            device_id,
            model: if model.is_empty() { "claude-haiku-5-5".to_string() } else { model },
        };
        if self.config.conv_id != old_conv {
            self.parent = None;
        }
        save_config(&self.config);
        self.close_config(window, cx);
    }

    fn reset_config_defaults(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let def = ClaudeConfig::default();
        self.cookie_input.update(cx, |s, cx| s.set_value(&def.cookie, window, cx));
        self.org_id_input.update(cx, |s, cx| s.set_value(&def.org_id, window, cx));
        self.conv_id_input.update(cx, |s, cx| s.set_value(&def.conv_id, window, cx));
        self.device_id_input.update(cx, |s, cx| s.set_value(&def.device_id, window, cx));
        self.model_input.update(cx, |s, cx| s.set_value(&def.model, window, cx));
        cx.notify();
    }

    fn at_bottom(&self) -> bool {
        let max: f32 = self.scroll.max_offset().y.into();
        let off: f32 = self.scroll.offset().y.into();
        max + off <= 4.0
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = self.input.read(cx).value().trim().to_string();
        if prompt.is_empty() || self.loading {
            return;
        }
        self.input.update(cx, |s, cx| s.set_value("", window, cx));
        self.msgs.push(Msg { user: true, text: prompt.clone(), extra: Extra::default(), open: false });
        self.msgs.push(Msg { user: false, text: String::new(), extra: Extra::default(), open: false });
        self.loading = true;
        save_history(&self.msgs, &self.parent);
        self.scroll.scroll_to_bottom();
        cx.notify();

        let parent = self.parent.clone();
        let config = self.config.clone();
        cx.spawn_in(window, async move |this, window| {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    rt.block_on(async move {
                        let _ = send_prompt_stream(&prompt, parent.as_deref(), &config, move |ev| {
                            let _ = tx.send(ev);
                        })
                        .await;
                    });
                }
            });
            while let Some(ev) = rx.recv().await {
                let _ = window.update(|_, cx| {
                    let _ = this.update(cx, |c, cx| {
                        let stick = c.at_bottom();
                        match ev {
                            ClaudeStreamEvent::Delta(t) => {
                                if let Some(m) = c.msgs.last_mut() {
                                    m.text.push_str(&t);
                                }
                            }
                            ClaudeStreamEvent::Think(t) => {
                                if let Some(m) = c.msgs.last_mut() {
                                    m.extra.think.push_str(&t);
                                }
                            }
                            ClaudeStreamEvent::ToolStart(n) => {
                                if let Some(m) = c.msgs.last_mut() {
                                    m.extra.tools.push((n, String::new(), String::new()));
                                }
                            }
                            ClaudeStreamEvent::ToolInput(t) => {
                                if let Some(m) = c.msgs.last_mut() {
                                    if let Some(tool) = m.extra.tools.last_mut() {
                                        tool.1.push_str(&t);
                                    }
                                }
                            }
                            ClaudeStreamEvent::ToolResult(t) => {
                                if let Some(m) = c.msgs.last_mut() {
                                    if let Some(tool) = m.extra.tools.iter_mut().find(|t| t.2.is_empty()) {
                                        tool.2 = t;
                                    }
                                }
                            }
                            ClaudeStreamEvent::Parent(id) => c.parent = Some(id),
                            ClaudeStreamEvent::MessageLimit(u) => c.usage = u,
                            ClaudeStreamEvent::Error(e) => {
                                if let Some(m) = c.msgs.last_mut() {
                                    m.text.push_str(&format!("\n\n_Error: {e}_"));
                                }
                            }
                            ClaudeStreamEvent::Done => {
                                c.loading = false;
                                save_history(&c.msgs, &c.parent);
                            }
                        }
                        if stick {
                            c.scroll.scroll_to_bottom();
                        }
                        cx.notify();
                    });
                });
            }
        })
        .detach();
    }
}

impl Render for Chat {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let used = self.usage.five_hour_utilization.unwrap_or(0.0) * 100.0;
        let left = self
            .usage
            .five_hour_resets_at
            .map(|ts| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                let m = ((ts - now) / 60).max(0);
                format!("{}h {}m", m / 60, m % 60)
            })
            .unwrap_or_default();

        // Keep at least MIN_CONTENT px for text: the side padding shrinks
        // automatically when the window is narrow.
        const MIN_CONTENT: f32 = 320.;
        let win_w: f32 = window.viewport_size().width.into();
        let pad = self.pad.min(((win_w - 24. - MIN_CONTENT) / 2.).max(0.));

        let mut list = v_flex().w_full().min_w_0().gap_3();
        for (i, m) in self.msgs.iter().enumerate() {
            if m.user {
                let view = TextView::markdown(SharedString::from(format!("m{i}")), m.text.clone()).selectable(true);
                list = list.child(v_flex().w_full().items_end().child(
                    div().max_w_full().min_w_0().px_3().py_1().border_1().border_color(cx.theme().primary).child(view),
                ));
                continue;
            }
            let mut col = v_flex().w_full().min_w_0().gap_1();
            if !m.extra.think.is_empty() || !m.extra.tools.is_empty() {
                col = col.child(
                    div()
                        .id(SharedString::from(format!("think{i}")))
                        .cursor_pointer()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{} Thinking ({} tools)", if m.open { "v" } else { ">" }, m.extra.tools.len()))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.msgs[i].open = !this.msgs[i].open;
                            cx.notify();
                        })),
                );
                if m.open {
                    let mut inner = v_flex().w_full().min_w_0().gap_1().pl_3().text_color(cx.theme().muted_foreground);
                    if !m.extra.think.is_empty() {
                        inner = inner.child(TextView::markdown(SharedString::from(format!("t{i}")), m.extra.think.clone()));
                    }
                    for (j, (name, input, output)) in m.extra.tools.iter().enumerate() {
                        let tip = format!("input: {}\n\noutput: {}", clip(input), clip(output));
                        inner = inner.child(
                            div()
                                .id(SharedString::from(format!("tool{i}-{j}")))
                                .child(format!("tool: {name}"))
                                .tooltip(move |window, cx| wrapped_tip(tip.clone()).build(window, cx)),
                        );
                    }
                    col = col.child(inner);
                }
            }
            col = col.child(TextView::markdown(SharedString::from(format!("m{i}")), m.text.clone()).selectable(true));
            list = list.child(col);
        }

        let modal = if self.config_open {
            Some(
                div()
                    .id("modal-backdrop")
                    .absolute()
                    .inset_0()
                    .bg(cx.theme().background.opacity(0.85))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .child(
                        v_flex()
                            .id("modal-dialog")
                            .w(px(560.))
                            .max_w_full()
                            .p_4()
                            .gap_3()
                            .bg(cx.theme().background)
                            .border_1()
                            .border_color(cx.theme().border)
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .items_center()
                                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Model Configuration"))
                                    .child(
                                        div()
                                            .id("btn-modal-close")
                                            .cursor_pointer()
                                            .text_color(cx.theme().muted_foreground)
                                            .hover(|s| s.text_color(cx.theme().foreground))
                                            .child("[x]")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.close_config(window, cx);
                                            })),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Cookie"))
                                    .child(Input::new(&self.cookie_input)),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Organization ID"))
                                    .child(Input::new(&self.org_id_input)),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Conversation ID"))
                                    .child(Input::new(&self.conv_id_input)),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Device ID"))
                                    .child(Input::new(&self.device_id_input)),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Model"))
                                    .child(Input::new(&self.model_input)),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .items_center()
                                    .pt_2()
                                    .child(
                                        div()
                                            .id("btn-reset-defaults")
                                            .cursor_pointer()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .hover(|s| s.text_color(cx.theme().danger))
                                            .child("Reset defaults")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.reset_config_defaults(window, cx);
                                            })),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .id("btn-config-cancel")
                                                    .cursor_pointer()
                                                    .px_3()
                                                    .py_1()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .hover(|s| s.text_color(cx.theme().foreground))
                                                    .child("Cancel")
                                                    .on_click(cx.listener(|this, _, window, cx| {
                                                        this.close_config(window, cx);
                                                    })),
                                            )
                                            .child(
                                                div()
                                                    .id("btn-config-save")
                                                    .cursor_pointer()
                                                    .px_3()
                                                    .py_1()
                                                    .border_1()
                                                    .border_color(cx.theme().primary)
                                                    .text_color(cx.theme().primary)
                                                    .hover(|s| s.bg(cx.theme().primary).text_color(cx.theme().background))
                                                    .child("Save")
                                                    .on_click(cx.listener(|this, _, window, cx| {
                                                        this.save_config_from_inputs(window, cx);
                                                    })),
                                            ),
                                    ),
                            ),
                    ),
            )
        } else {
            None
        };

        v_flex()
            .size_full()
            .relative()
            .p_3()
            .gap_2()
            .id("root")
            .track_focus(&self.focus)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                if !this.config_open {
                    window.focus(&this.focus, cx);
                }
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                if ev.keystroke.key == "escape" && this.config_open {
                    this.close_config(window, cx);
                    return;
                }
                if ev.keystroke.key == "/" && !this.config_open && !this.input.read(cx).focus_handle(cx).is_focused(window) {
                    this.input.update(cx, |s, cx| s.focus(window, cx));
                }
            }))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                h_flex()
                    .group("top-bar")
                    .h(px(24.))
                    .w_full()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .id("btn-config")
                            .opacity(0.)
                            .group_hover("top-bar", |s| s.opacity(1.))
                            .cursor_pointer()
                            .text_color(cx.theme().muted_foreground)
                            .hover(|s| s.text_color(cx.theme().foreground))
                            .child("[config]")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_config(window, cx);
                            })),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .opacity(0.)
                            .group_hover("top-bar", |s| s.opacity(1.))
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                div().id("pad-less").cursor_pointer().child("<").on_click(cx.listener(|this, _, _, cx| {
                                    this.pad = (this.pad - 40.).max(0.);
                                    save_pad(this.pad);
                                    cx.notify();
                                })),
                            )
                            .child(format!("{}px", self.pad as i32))
                            .child(
                                div().id("pad-more").cursor_pointer().child(">").on_click(cx.listener(|this, _, _, cx| {
                                    this.pad = (this.pad + 40.).min(600.);
                                    save_pad(this.pad);
                                    cx.notify();
                                })),
                            ),
                    ),
            )
            .child(
                div()
                    .id("chat")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .px(px(pad))
                    .overflow_y_scrollbar()
                    .child(list),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child({
                        let tip = usage_details(&self.usage);
                        div()
                            .id("usage")
                            .child(format!("{used:.0}% {left}"))
                            .tooltip(move |window, cx| wrapped_tip(tip.clone()).build(window, cx))
                    })
                    .child(div().flex_1().child(Input::new(&self.input))),
            )
            .children(modal)
    }
}

fn chat_db() -> Option<rusqlite::Connection> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?
        .join("claude-chat");
    std::fs::create_dir_all(&base).ok()?;
    let conn = rusqlite::Connection::open(base.join("chat.sqlite")).ok()?;
    conn.execute_batch(
        "create table if not exists msgs2(id integer primary key autoincrement, user integer not null, text text not null, extra text not null default '{}');
         create table if not exists meta(key text primary key, value text not null);",
    )
    .ok()?;
    Some(conn)
}

fn load_history() -> (Vec<Msg>, Option<String>) {
    let Some(conn) = chat_db() else { return (Vec::new(), None) };
    let msgs = conn
        .prepare("select user, text, extra from msgs2 order by id")
        .and_then(|mut st| {
            st.query_map([], |r| Ok(Msg {
                user: r.get::<_, i64>(0)? != 0,
                text: r.get(1)?,
                extra: serde_json::from_str(&r.get::<_, String>(2)?).unwrap_or_default(),
                open: false,
            }))
                .map(|rows| rows.flatten().collect())
        })
        .unwrap_or_default();
    let parent = conn.query_row("select value from meta where key='parent'", [], |r| r.get(0)).ok();
    (msgs, parent)
}

fn save_history(msgs: &[Msg], parent: &Option<String>) {
    let Some(mut conn) = chat_db() else { return };
    let Ok(tx) = conn.transaction() else { return };
    let _ = tx.execute("delete from msgs2", []);
    for m in msgs {
        let _ = tx.execute(
            "insert into msgs2(user, text, extra) values(?1, ?2, ?3)",
            rusqlite::params![m.user as i64, m.text, serde_json::to_string(&m.extra).unwrap_or_default()],
        );
    }
    if let Some(p) = parent {
        let _ = tx.execute("insert or replace into meta(key, value) values('parent', ?1)", [p]);
    }
    let _ = tx.commit();
}

fn load_pad() -> f32 {
    chat_db()
        .and_then(|c| c.query_row("select value from meta where key='pad'", [], |r| r.get::<_, String>(0)).ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0)
}

fn save_pad(pad: f32) {
    if let Some(c) = chat_db() {
        let _ = c.execute("insert or replace into meta(key, value) values('pad', ?1)", [pad.to_string()]);
    }
}

fn load_config() -> ClaudeConfig {
    let Some(conn) = chat_db() else { return ClaudeConfig::default() };
    if let Ok(val) = conn.query_row("select value from meta where key='claude_config'", [], |r| r.get::<_, String>(0)) {
        if let Ok(cfg) = serde_json::from_str::<ClaudeConfig>(&val) {
            return cfg;
        }
    }
    ClaudeConfig::default()
}

fn save_config(cfg: &ClaudeConfig) {
    let Some(conn) = chat_db() else { return };
    if let Ok(val) = serde_json::to_string(cfg) {
        let _ = conn.execute("insert or replace into meta(key, value) values('claude_config', ?1)", [val]);
    }
}

fn clean_id(s: &str) -> String {
    let s = s.trim();
    if let Some(pos) = s.rfind('/') {
        let after = &s[pos + 1..];
        if after == "completion" {
            let before = &s[..pos];
            if let Some(p2) = before.rfind('/') {
                return before[p2 + 1..].to_string();
            }
        }
        return after.to_string();
    }
    s.to_string()
}

fn remaining(ts: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let m = ((ts - now) / 60).max(0);
    format!("{}h {}m", m / 60, m % 60)
}

fn usage_details(u: &UsageLimitInfo) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&u.raw_json_summary) else {
        return "No usage data yet. Send a message.".into();
    };
    let mut out = String::new();
    for (key, label) in [("5h", "5 hour"), ("7d", "7 day")] {
        let w = v.pointer(&format!("/message_limit/windows/{key}"));
        let pct = w.and_then(|w| w.get("utilization")).and_then(|x| x.as_f64()).unwrap_or(0.0) * 100.0;
        let status = w.and_then(|w| w.get("status")).and_then(|x| x.as_str()).unwrap_or("-");
        let reset = w
            .and_then(|w| w.get("resets_at"))
            .and_then(|x| x.as_i64())
            .map(|t| format!("resets in {}", remaining(t)))
            .unwrap_or_default();
        out.push_str(&format!("{label}: {pct:.1}% used, {status} {reset}\n"));
    }
    if let Some(sev) = v.pointer("/message_limit/resolved/limit/severity").and_then(|x| x.as_str()) {
        out.push_str(&format!("severity: {sev}\n"));
    }
    if let Some(at) = v.pointer("/message_limit/resolved/limit/resets_at").and_then(|x| x.as_str()) {
        out.push_str(&format!("session resets at: {at}\n"));
    }
    out
}

fn wrapped_tip(text: String) -> Tooltip {
    Tooltip::element(move |_, _| {
        div().w(px(480.)).whitespace_normal().child(text.clone())
    })
}

fn clip(s: &str) -> String {
    s.chars().take(600).collect()
}

fn silvapi_setting(key: &str) -> Option<String> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    let conn = rusqlite::Connection::open_with_flags(
        base.join("silvapi/silvapi.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .ok()?;
    conn.query_row("select value from settings where key = ?1", [key], |r| r.get(0)).ok()
}

/// Themes load asynchronously; open the window only after the first load so
/// markdown views are never built (and cached) with the default light colors.
fn init_theme(cx: &mut App, open: impl Fn(&mut App) + 'static) {
    let name = silvapi_setting("theme").unwrap_or_else(|| "Default Dark".into());
    let opened = std::rc::Rc::new(std::cell::Cell::new(false));
    let open = std::rc::Rc::new(open);
    let apply = {
        let opened = opened.clone();
        let open = open.clone();
        move |cx: &mut App| {
            if let Some(t) = ThemeRegistry::global(cx).themes().get(name.as_str()).cloned() {
                Theme::global_mut(cx).apply_config(&t);
            }
            let font = silvapi_setting("ui.font_family")
                .filter(|f| !f.trim().is_empty())
                .unwrap_or_else(|| "JetBrainsMono Nerd Font".into());
            let font = SharedString::from(font);
            Theme::global_mut(cx).font_family = font.clone();
            Theme::global_mut(cx).mono_font_family = font;
            // Theme::global_mut edits do not reach the TextView defaults; sync them.
            Theme::sync_base(cx);
            if !opened.replace(true) {
                open(cx);
            }
            cx.refresh_windows();
        }
    };
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(|p| PathBuf::from(p).join("silvapi/theme"))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/silvapi/theme")));
    match dir {
        Some(dir) => {
            let _ = ThemeRegistry::watch_dir(dir, cx, apply);
        }
        None => apply(cx),
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets::new(""))
        .run(|cx| {
            gpui_kit::init(cx);
            init_theme(cx, |cx| {
                cx.spawn(async move |cx| {
                    cx.open_window(WindowOptions::default(), |window, cx| {
                        let view = cx.new(|cx| Chat::new(window, cx));
                        cx.new(|cx| Root::new(view, window, cx))
                    })
                    .expect("window");
                })
                .detach();
            });
        });
}
