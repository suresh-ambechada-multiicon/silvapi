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
    pub(super) fn render_preview_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body = self
            .app_state
            .read(cx)
            .response
            .as_ref()
            .map(|r| r.body.clone())
            .unwrap_or_default();
        // TextView::html is a lightweight rich-text renderer, not a browser: it
        // has no CSS/JS engine, so strip script/style/head/comments (otherwise
        // their raw source renders as text) before handing it over. Also collapse
        // CR/LF to spaces — gpui's text shaper panics on a newline within a run.
        let body = sanitize_html_for_preview(&body);
        div()
            .id("preview-scroll")
            .size_full()
            .overflow_scroll()
            .p_4()
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .overflow_hidden()
                    .child(TextView::html("preview-html-view", body)),
            )
    }

}
