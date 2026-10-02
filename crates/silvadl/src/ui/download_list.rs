use super::*;

#[derive(IntoElement)]
pub struct DownloadRow {
    base: gpui_kit::component::list::ListItem,
    task: TaskSnapshot,
    controller: Option<DownloadController>,
    tokens: DesignTokens,
}

impl gpui_kit::component::Selectable for DownloadRow {
    fn selected(self, _: bool) -> Self { self }
    fn is_selected(&self) -> bool { false }
}

impl RenderOnce for DownloadRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let task = self.task;
        let percentage = if task.state == "Complete" { 100.0 } else if task.total == 0 { 0.0 } else { task.downloaded as f32 / task.total as f32 * 100.0 };
        let running = matches!(task.state.as_str(), "Starting" | "Downloading");
        let cancel_url = task.url.clone();
        let controller = self.controller.clone();
        let artwork = task.thumbnail.clone().map(|path| img(std::path::PathBuf::from(path)).w(px(72.)).h(px(46.)).rounded(px(6.)).into_any_element()).unwrap_or_else(|| div().w(px(72.)).h(px(46.)).rounded(px(6.)).bg(cx.theme().secondary).child(Icon::new(IconName::File)).into_any_element());
        self.base.child(
            v_flex().gap(px(8.)).px(px(12.)).py(px(10.)).border_b_1().border_color(cx.theme().border)
                .child(h_flex().items_center().gap(px(12.))
                    .child(Icon::new(if task.state == "Complete" { IconName::CircleCheck } else if task.state == "Failed" { IconName::CircleX } else if task.state == "Queued" { IconName::Inbox } else { IconName::LoaderCircle }))
                    .child(artwork)
                    .child(v_flex().flex_1().min_w_0().child(Label::new(task.title)).child(Label::new(task.uploader.unwrap_or(task.url.clone())).text_size(px(self.tokens.caption_size)).text_color(cx.theme().muted_foreground)))
                    .child(Label::new(task.state.clone()))
                    .child(Button::new(SharedString::from(format!("virtual-cancel-{}", task.url))).icon(IconName::Close).ghost().small().disabled(!running && task.state != "Queued").on_click(move |_, _, _| { if let Some(controller) = &controller { controller.cancel(&cancel_url); } })))
                .child(Progress::new(SharedString::from(format!("virtual-progress-{}", task.url))).loading(running && task.total == 0).value(percentage))
                .child(h_flex().gap(px(14.))
                    .child(Label::new(if task.state == "Complete" { "Downloaded".into() } else if task.total == 0 { "Waiting for data…".into() } else { format!("{} / {}", format_bytes(task.downloaded), format_bytes(task.total)) }).text_size(px(self.tokens.caption_size)))
                    .when_some(task.speed, |row, speed| row.child(Label::new(format!("{}/s", format_bytes(speed as u64))).text_size(px(self.tokens.caption_size))))
                    .when_some(task.eta, |row, eta| row.child(Label::new(format!("{}:{:02} remaining", eta / 60, eta % 60)).text_size(px(self.tokens.caption_size))))
                    .when_some(task.error, |row, error| row.child(Label::new(error).text_color(cx.theme().danger))))
        )
    }
}

pub struct DownloadDelegate {
    items: Vec<TaskSnapshot>,
    visible: Vec<usize>,
    controller: Option<DownloadController>,
    tokens: DesignTokens,
}

impl DownloadDelegate {
    pub fn empty(tokens: DesignTokens) -> Self { Self { items: vec![], visible: vec![], controller: None, tokens } }
    pub fn replace(&mut self, items: Vec<TaskSnapshot>, filter: TaskFilter, controller: Option<DownloadController>) {
        self.visible = items.iter().enumerate().filter(|(_, task)| match filter {
            TaskFilter::All => true,
            TaskFilter::Running => matches!(task.state.as_str(), "Starting" | "Downloading"),
            TaskFilter::Queued => task.state == "Queued",
            TaskFilter::Completed => task.state == "Complete",
            TaskFilter::Failed => matches!(task.state.as_str(), "Failed" | "Canceled"),
        }).map(|(index, _)| index).collect();
        self.items = items;
        self.controller = controller;
    }
}

impl gpui_kit::component::list::ListDelegate for DownloadDelegate {
    type Item = DownloadRow;
    fn items_count(&self, _: usize, _: &App) -> usize { self.visible.len() }
    fn set_selected_index(&mut self, _: Option<gpui_kit::component::IndexPath>, _: &mut Window, _: &mut Context<ListState<Self>>) {}
    fn render_item(&mut self, index: gpui_kit::component::IndexPath, _: &mut Window, _: &mut Context<ListState<Self>>) -> Option<Self::Item> {
        let source = *self.visible.get(index.row)?;
        Some(DownloadRow { base: gpui_kit::component::list::ListItem::new(("download-row", source)), task: self.items.get(source)?.clone(), controller: self.controller.clone(), tokens: self.tokens.clone() })
    }
}
