#![allow(unused_imports)]
use std::collections::HashSet;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_kit::component::{
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

fn html_to_preview_markdown(html: &str) -> String {
    let sanitized = sanitize_html_for_preview(html);
    let mut text = sanitized;
    text = text.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");
    text = text.replace("<h1>", "\n# ").replace("</h1>", "\n");
    text = text.replace("<h2>", "\n## ").replace("</h2>", "\n");
    text = text.replace("<h3>", "\n### ").replace("</h3>", "\n");
    text = text.replace("<h4>", "\n#### ").replace("</h4>", "\n");
    text = text.replace("<h5>", "\n##### ").replace("</h5>", "\n");
    text = text.replace("<h6>", "\n###### ").replace("</h6>", "\n");
    text = text.replace("<p>", "\n\n").replace("</p>", "\n");
    text = text.replace("<b>", "**").replace("</b>", "**");
    text = text.replace("<strong>", "**").replace("</strong>", "**");
    text = text.replace("<i>", "*").replace("</i>", "*");
    text = text.replace("<em>", "*").replace("</em>", "*");
    text = text.replace("<code>", "`").replace("</code>", "`");
    text = text.replace("<pre>", "\n```\n").replace("</pre>", "\n```\n");
    text = text.replace("<li>", "\n- ").replace("</li>", "");
    text = text.replace("<ul>", "\n").replace("</ul>", "\n");
    text = text.replace("<ol>", "\n").replace("</ol>", "\n");
    text = text.replace("<hr>", "\n---\n").replace("<hr/>", "\n---\n").replace("<hr />", "\n---\n");

    let mut clean = String::with_capacity(text.len());
    let mut inside_tag = false;
    for ch in text.chars() {
        if ch == '<' {
            inside_tag = true;
        } else if ch == '>' {
            inside_tag = false;
        } else if !inside_tag {
            clean.push(ch);
        }
    }

    clean
}

impl ResponsePanel {
    pub(super) fn render_preview_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body = self
            .app_state
            .read(cx)
            .response
            .as_ref()
            .map(|r| r.body.clone())
            .unwrap_or_default();
        let md = html_to_preview_markdown(&body);
        div()
            .id("preview-scroll")
            .size_full()
            .overflow_y_scrollbar()
            .p_4()
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .child(TextView::markdown("preview-md-view", md).selectable(true)),
            )
    }
}
