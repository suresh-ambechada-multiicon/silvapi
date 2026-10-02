use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, IndexPath, Selectable,
    label::Label,
    list::{ListDelegate, ListItem, ListState},
    h_flex, v_flex,
};
use silvadl::MediaItem;
use std::collections::HashSet;

#[derive(IntoElement)]
pub struct PlaylistRow {
    base: ListItem,
    item: MediaItem,
    number: usize,
    selected: bool,
}

impl Selectable for PlaylistRow {
    fn selected(mut self, selected: bool) -> Self { self.selected = selected; self }
    fn is_selected(&self) -> bool { self.selected }
}

impl RenderOnce for PlaylistRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let thumbnail = self.item.thumbnail.clone().map(|source| {
            if source.starts_with("http://") || source.starts_with("https://") {
                img(source).w(px(72.)).h(px(42.)).rounded(px(4.)).into_any_element()
            } else {
                img(std::path::PathBuf::from(source)).w(px(72.)).h(px(42.)).rounded(px(4.)).into_any_element()
            }
        }).unwrap_or_else(|| div().w(px(72.)).h(px(42.)).rounded(px(4.)).bg(cx.theme().secondary).child(Icon::new(IconName::Play)).into_any_element());
        self.base.px_3().pr_5().py_2().child(
            h_flex().items_center().gap_3()
                .child(Label::new(format!("{:02}", self.number)).text_color(cx.theme().muted_foreground))
                .child(thumbnail)
                .child(v_flex().flex_1().min_w_0().child(Label::new(self.item.title).whitespace_nowrap()).when_some(self.item.uploader, |view, uploader| view.child(Label::new(uploader).text_size(px(12.)).text_color(cx.theme().muted_foreground))))
                .when_some(self.item.duration_seconds, |row, seconds| row.child(Label::new(duration(seconds)).text_color(cx.theme().muted_foreground)))
                .child(Icon::new(if self.selected { IconName::CircleCheck } else { IconName::CircleX }).text_color(if self.selected { cx.theme().primary } else { cx.theme().muted_foreground }))
        )
    }
}

pub struct PlaylistDelegate {
    items: Vec<MediaItem>,
    visible: Vec<usize>,
    selected: HashSet<String>,
    cursor: Option<IndexPath>,
}

impl PlaylistDelegate {
    pub fn empty() -> Self { Self { items: vec![], visible: vec![], selected: HashSet::new(), cursor: None } }
    pub fn replace(&mut self, items: Vec<MediaItem>) {
        self.selected = items.iter().map(|item| item.id.clone()).collect();
        self.visible = (0..items.len()).collect(); self.items = items; self.cursor = None;
    }
    pub fn push(&mut self, item: MediaItem) {
        self.selected.insert(item.id.clone());
        self.visible.push(self.items.len());
        self.items.push(item);
    }
    pub fn select_all(&mut self) { self.selected.extend(self.items.iter().map(|item| item.id.clone())); }
    pub fn clear(&mut self) { self.selected.clear(); }
    pub fn selected_count(&self) -> usize { self.selected.len() }
    pub fn selected_urls(&self) -> Vec<String> { self.items.iter().filter(|item| self.selected.contains(&item.id)).map(|item| item.url.clone()).collect() }
    pub fn update_thumbnail(&mut self, id: &str, path: String) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) { item.thumbnail = Some(path); }
    }
}

impl ListDelegate for PlaylistDelegate {
    type Item = PlaylistRow;
    fn items_count(&self, _: usize, _: &App) -> usize { self.visible.len() }
    fn perform_search(&mut self, query: &str, _: &mut Window, _: &mut Context<ListState<Self>>) -> Task<()> {
        let query = query.trim().to_lowercase();
        self.visible = self.items.iter().enumerate().filter(|(_, item)| query.is_empty() || item.title.to_lowercase().contains(&query) || item.uploader.as_deref().unwrap_or("").to_lowercase().contains(&query)).map(|(index, _)| index).collect();
        Task::ready(())
    }
    fn set_selected_index(&mut self, index: Option<IndexPath>, _: &mut Window, cx: &mut Context<ListState<Self>>) { self.cursor = index; cx.notify(); }
    fn confirm(&mut self, _: bool, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        if let Some(cursor) = self.cursor && let Some(index) = self.visible.get(cursor.row) && let Some(item) = self.items.get(*index) {
            if !self.selected.remove(&item.id) { self.selected.insert(item.id.clone()); }
            cx.notify();
        }
    }
    fn render_item(&mut self, index: IndexPath, _: &mut Window, _: &mut Context<ListState<Self>>) -> Option<Self::Item> {
        let source = *self.visible.get(index.row)?; let item = self.items.get(source)?.clone();
        Some(PlaylistRow { base: ListItem::new(("playlist-row", source)).selected(self.selected.contains(&item.id)), item, number: source + 1, selected: self.selected.contains(&self.items[source].id) })
    }
}

fn duration(seconds: f64) -> String { let total = seconds.max(0.0) as u64; format!("{}:{:02}", total / 60, total % 60) }
