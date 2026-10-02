use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::component::{
    ActiveTheme, Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
    button::{Button, ButtonGroup, ButtonVariants as _},
    form::{Field, Form},
    h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    list::{List, ListState},
    progress::{Progress, ProgressCircle},
    scroll::ScrollableElement,
    switch::Switch,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::component::dialog::Dialog;
mod playlist_list;
mod manager;
mod download_list;
use download_list::DownloadDelegate;
use playlist_list::PlaylistDelegate;
use silvadl::{analyze, AnalysisEvent, AppConfig, DesignTokens, DownloadController, DownloadEvent, MediaInspection, MediaKind, start};
use std::{collections::HashMap, sync::mpsc, time::Duration};

#[derive(Clone)]
struct TaskSnapshot {
    url: String,
    title: String,
    downloaded: u64,
    total: u64,
    speed: Option<f64>,
    eta: Option<u64>,
    state: String,
    error: Option<String>,
    thumbnail: Option<String>,
    uploader: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TaskFilter { All, Running, Queued, Completed, Failed }

pub struct DownloaderView {
    config: AppConfig,
    tokens: DesignTokens,
    urls: Entity<InputState>,
    output: Entity<InputState>,
    running: bool,
    completed: usize,
    total: usize,
    status: String,
    logs: Vec<String>,
    inspection: Option<MediaInspection>,
    analyzing: bool,
    analysis_generation: u64,
    _subscriptions: Vec<Subscription>,
    controller: Option<DownloadController>,
    tasks: HashMap<String, TaskSnapshot>,
    task_order: Vec<String>,
    task_filter: TaskFilter,
    add_tab: usize,
    show_add: bool,
    playlist_list: Entity<ListState<PlaylistDelegate>>,
    download_list: Entity<ListState<DownloadDelegate>>,
}

impl DownloaderView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = AppConfig::load(&AppConfig::user_path()).unwrap_or_else(|_| AppConfig::defaults().expect("built-in config"));
        let tokens = DesignTokens::load().expect("built-in design tokens");
        let urls = cx.new(|cx| InputState::new(window, cx).placeholder("Paste a YouTube video or playlist link"));
        let output = input_with_value(&config.download.output_directory.to_string_lossy(), "Output directory", window, cx);
        let subscription = cx.subscribe_in(&urls, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) { this.schedule_analysis(cx); }
        });
        let playlist_list = cx.new(|cx| ListState::new(PlaylistDelegate::empty(), window, cx).searchable(true));
        let download_list = cx.new(|cx| ListState::new(DownloadDelegate::empty(tokens.clone()), window, cx));
        Self { config, tokens, urls, output, running: false, completed: 0, total: 0, status: "Ready".into(), logs: vec![], inspection: None, analyzing: false, analysis_generation: 0, _subscriptions: vec![subscription], controller: None, tasks: HashMap::new(), task_order: vec![], task_filter: TaskFilter::All, add_tab: 0, show_add: false, playlist_list, download_list }
    }

    fn schedule_analysis(&mut self, cx: &mut Context<Self>) {
        self.analysis_generation += 1;
        let generation = self.analysis_generation;
        self.inspection = None;
        self.config.download.selected_format_id = None;
        self.status = "Waiting for URL…".into();
        cx.spawn(async move |view, cx| {
            cx.background_executor().timer(Duration::from_millis(550)).await;
            let request = view.update(cx, |this, cx| {
                if this.analysis_generation != generation { return None; }
                let url = this.urls.read(cx).value().trim().to_owned();
                if url.is_empty() { this.analyzing = false; this.status = "Paste a URL to inspect it".into(); cx.notify(); return None; }
                this.analyzing = true; this.status = "Inspecting media and available formats…".into(); cx.notify();
                Some((url, this.config.download.output_directory.clone()))
            }).ok().flatten();
            let Some((url, output)) = request else { return; };
            let (tx, rx) = mpsc::channel();
            analyze(url, output, tx);
            loop {
                cx.background_executor().timer(Duration::from_millis(35)).await;
                let mut events = Vec::new();
                loop { match rx.try_recv() { Ok(event) => events.push(event), Err(mpsc::TryRecvError::Empty) => break, Err(_) => return } }
                if events.is_empty() { continue; }
                let done = events.iter().any(|event| matches!(event, AnalysisEvent::Finished | AnalysisEvent::Failed(_)));
                let _ = view.update(cx, |this, cx| {
                    if this.analysis_generation != generation { return; }
                    for event in events { match event {
                        AnalysisEvent::Started => this.analyzing = true,
                        AnalysisEvent::PlaylistStarted { title, uploader } => {
                            this.inspection = Some(MediaInspection { title, thumbnail: None, uploader, is_playlist: true, total_items: 0, items: vec![] });
                            this.playlist_list.update(cx, |state, cx| { state.delegate_mut().replace(vec![]); cx.notify(); });
                            this.status = "Loading playlist…".into();
                        }
                        AnalysisEvent::ItemDiscovered(item) => {
                            if let Some(inspection) = &mut this.inspection {
                                inspection.items.push(item.clone()); inspection.total_items = inspection.items.len();
                                this.status = format!("{} items available", inspection.total_items);
                            }
                            this.playlist_list.update(cx, |state, cx| { state.delegate_mut().push(item); cx.notify(); });
                        }
                        AnalysisEvent::Ready(inspection) => {
                            this.analyzing = false;
                            this.status = if inspection.is_playlist { format!("{} items selected", inspection.total_items) } else { "Ready".into() };
                            if let Some(item) = inspection.items.first() {
                                let want_audio = this.config.download.media_kind == MediaKind::Audio;
                                this.config.download.selected_format_id = item.formats.iter().rev().find(|format| if want_audio { format.has_audio } else { format.has_video }).map(|format| format.id.clone());
                            }
                            this.playlist_list.update(cx, |state, cx| { state.delegate_mut().replace(inspection.items.clone()); cx.notify(); });
                            this.inspection = Some(inspection);
                        }
                        AnalysisEvent::ThumbnailCached { id, path } => {
                            if let Some(inspection) = &mut this.inspection {
                                if inspection.thumbnail.is_none() { inspection.thumbnail = Some(path.clone()); }
                                if let Some(item) = inspection.items.iter_mut().find(|item| item.id == id) { item.thumbnail = Some(path.clone()); }
                            }
                            this.playlist_list.update(cx, |state, cx| { state.delegate_mut().update_thumbnail(&id, path); cx.notify(); });
                        }
                        AnalysisEvent::Finished => this.analyzing = false,
                        AnalysisEvent::Failed(error) => { this.analyzing = false; this.status = format!("Inspection failed: {error}"); this.logs.push(error); }
                    } }
                    cx.notify();
                });
                if done { break; }
            }
        }).detach();
    }

    fn begin(&mut self, cx: &mut Context<Self>) {
        let mut urls: Vec<String> = self.urls.read(cx).value().lines().map(str::trim)
            .filter(|line| !line.is_empty()).map(str::to_owned).collect();
        if urls.is_empty() || self.inspection.is_none() { self.status = "Inspect the URL before downloading".into(); cx.notify(); return; }
        self.config.download.output_directory = self.output.read(cx).value().trim().into();
        if self.inspection.as_ref().is_some_and(|inspection| inspection.is_playlist) {
            let delegate = self.playlist_list.read(cx).delegate();
            urls = delegate.selected_urls();
            self.config.download.download_playlist = false;
        }
        if let Err(error) = self.config.save(&AppConfig::user_path()) {
            self.logs.push(format!("Config warning: {error}"));
        }
        let (tx, rx) = mpsc::channel();
        let queued = if self.inspection.as_ref().is_some_and(|item| item.is_playlist) && !self.config.download.download_playlist {
            self.playlist_list.read(cx).delegate().selected_urls()
        } else {
            self.inspection.as_ref().map(|item| item.items.iter().map(|entry| entry.url.clone()).collect()).unwrap_or_else(Vec::new)
        };
        for url in queued {
            let media = self.inspection.as_ref().and_then(|inspection| inspection.items.iter().find(|item| item.url == url));
            let title = media.map(|item| item.title.clone()).unwrap_or_else(|| url.clone());
            let thumbnail = media.and_then(|item| item.thumbnail.clone());
            let uploader = media.and_then(|item| item.uploader.clone());
            if !self.tasks.contains_key(&url) { self.task_order.push(url.clone()); }
            self.tasks.insert(url.clone(), TaskSnapshot { url, title, downloaded: 0, total: 0, speed: None, eta: None, state: "Queued".into(), error: None, thumbnail, uploader });
        }
        self.sync_download_list(cx);
        self.controller = Some(start(urls, self.config.download.clone(), tx));
        self.running = true; self.completed = 0; self.total = 0; self.status = "Preparing download…".into();
        cx.spawn(async move |view, cx| loop {
            cx.background_executor().timer(Duration::from_millis(100)).await;
            let mut finished = false;
            let disconnected = view.update(cx, |this, cx| {
                let mut disconnected = false;
                let mut changed = false;
                loop {
                    match rx.try_recv() {
                        Ok(event) => { changed = true; if matches!(event, DownloadEvent::Finished { .. }) { finished = true; } this.handle_event(event); }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => { disconnected = true; break; }
                    }
                }
                if changed { this.sync_download_list(cx); cx.notify(); }
                disconnected
            }).unwrap_or(true);
            if finished || disconnected { break; }
        }).detach();
        cx.notify();
    }

    fn handle_event(&mut self, event: DownloadEvent) {
        match event {
            DownloadEvent::QueueReady { items } => { self.total = items; self.status = format!("0 of {items} complete"); self.logs.push(format!("Queue contains {items} item(s)")); }
            DownloadEvent::ItemStarted { worker: _, url } => {
                let media = self.inspection.as_ref().and_then(|inspection| inspection.items.iter().find(|item| item.url == url || url.contains(&item.id)));
                let title = media.map(|item| item.title.clone()).unwrap_or_else(|| url.clone());
                let thumbnail = media.and_then(|item| item.thumbnail.clone());
                let uploader = media.and_then(|item| item.uploader.clone());
                if !self.tasks.contains_key(&url) { self.task_order.push(url.clone()); }
                self.tasks.insert(url.clone(), TaskSnapshot { url, title, downloaded: 0, total: 0, speed: None, eta: None, state: "Starting".into(), error: None, thumbnail, uploader });
            }
            DownloadEvent::ItemProgress { url, downloaded, total, speed, eta } => {
                if let Some(task) = self.tasks.get_mut(&url) { task.downloaded = downloaded; task.total = total; task.speed = speed; task.eta = eta; task.state = "Downloading".into(); }
            }
            DownloadEvent::ItemFinished { url, success } => { self.completed += 1; self.status = format!("{} of {} complete", self.completed, self.total); if let Some(task) = self.tasks.get_mut(&url) { if success { task.state = "Complete".into(); } else if task.state != "Canceled" && task.error.is_none() { task.state = "Failed".into(); } } }
            DownloadEvent::ItemError { url, message } => { if let Some(task) = self.tasks.get_mut(&url) { task.state = "Failed".into(); task.error = Some(message); } }
            DownloadEvent::ItemCanceled { url } => { if let Some(task) = self.tasks.get_mut(&url) { task.state = "Canceled".into(); } }
            DownloadEvent::Log(line) => self.logs.push(line),
            DownloadEvent::Finished { succeeded, failed } => { self.running = false; self.status = format!("Finished — {succeeded} succeeded, {failed} failed"); }
        }
        if self.logs.len() > 200 { self.logs.drain(..self.logs.len() - 200); }
    }

    fn sync_download_list(&mut self, cx: &mut Context<Self>) {
        let items = self.task_order.iter().filter_map(|url| self.tasks.get(url).cloned()).collect();
        let filter = self.task_filter;
        let controller = self.controller.clone();
        self.download_list.update(cx, |state, cx| {
            state.delegate_mut().replace(items, filter, controller);
            cx.notify();
        });
    }

}

fn input_with_value(value: &str, placeholder: &str, window: &mut Window, cx: &mut Context<DownloaderView>) -> Entity<InputState> {
    let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
    input.update(cx, |state, cx| state.set_value(value, window, cx)); input
}

impl Render for DownloaderView {
    #[allow(unreachable_code)]
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        return self.render_manager(_window, cx);
        let gap = px(self.tokens.control_gap);
        let kind = self.config.download.media_kind;
        let playlist_selected = self.playlist_list.read(cx).delegate().selected_count();
        let progress = if self.total == 0 { 0.0 } else { self.completed as f32 / self.total as f32 * 100.0 };

        let left = v_flex().gap(px(self.tokens.section_gap)).flex_1().min_w_0()
            .child(Form::vertical()
                .child(Field::new().label("Link").child(Input::new(&self.urls)))
                .child(Field::new().label("Media type").child(ButtonGroup::new("media-kind").outline().small()
                    .child(Button::new("video").icon(IconName::Play).label("Video").selected(kind == MediaKind::Video))
                    .child(Button::new("audio").icon(IconName::File).label("Audio").selected(kind == MediaKind::Audio))
                    .on_click(cx.listener(|this, selected: &Vec<usize>, _, cx| { this.config.download.media_kind = if selected.contains(&1) { MediaKind::Audio } else { MediaKind::Video }; this.config.download.selected_format_id = None; cx.notify(); }))))
                .child(Field::new().label("Save to").child(Input::new(&self.output)))
                .child(Field::new().child(h_flex().gap(gap).flex_wrap()
                    .child(Switch::new("metadata").label("Metadata").checked(self.config.download.embed_metadata).on_click(cx.listener(|this, value: &bool, _, cx| { this.config.download.embed_metadata = *value; cx.notify(); })))
                    .child(Switch::new("thumbnail").label("Thumbnail").checked(self.config.download.embed_thumbnail).on_click(cx.listener(|this, value: &bool, _, cx| { this.config.download.embed_thumbnail = *value; cx.notify(); })))
                    .child(Switch::new("subtitles").label("Subtitles").checked(self.config.download.download_subtitles).on_click(cx.listener(|this, value: &bool, _, cx| { this.config.download.download_subtitles = *value; cx.notify(); }))))) )
            .child(Progress::new("download-progress").loading(self.running && self.total == 0).value(progress))
            .child(h_flex().items_center().gap(gap)
                .child(Button::new("download").icon(IconName::ArrowRight).label(if self.running { "Downloading" } else { "Download" }).primary().disabled(self.running || self.inspection.is_none() || self.inspection.as_ref().is_some_and(|inspection| inspection.is_playlist && playlist_selected == 0)).on_click(cx.listener(|this, _, _, cx| this.begin(cx))))
                .child(Button::new("settings").icon(IconName::Settings2).ghost())
                .child(Label::new(if self.inspection.as_ref().is_some_and(|inspection| inspection.is_playlist) { format!("{playlist_selected} selected") } else { self.status.clone() })));

        let card = |content: AnyElement| v_flex().p(px(self.tokens.section_padding)).border_1().border_color(cx.theme().border).rounded(px(self.tokens.corner_radius)).child(content);
        let (hero, content): (AnyElement, AnyElement) = if self.analyzing {
            (card(v_flex().h(px(190.)).items_center().justify_center().gap(gap).child(ProgressCircle::new("analyzing").large().loading(true)).child(Label::new("Loading media…")).into_any_element()).into_any_element(),
             card(v_flex().h(px(300.)).into_any_element()).into_any_element())
        } else if let Some(inspection) = &self.inspection {
            let artwork = if let Some(thumbnail) = &inspection.thumbnail {
                img(std::path::PathBuf::from(thumbnail)).w(px(320.)).h(px(180.)).rounded(px(self.tokens.corner_radius)).into_any_element()
            } else { div().w(px(320.)).h(px(180.)).bg(cx.theme().secondary).rounded(px(self.tokens.corner_radius)).child(Icon::new(IconName::GalleryVerticalEnd).large()).into_any_element() };
            let duration = inspection.items.first().and_then(|item| item.duration_seconds).map(format_duration);
            let metadata = v_flex().gap(gap)
                .child(Label::new(if inspection.is_playlist { "PLAYLIST" } else { "MEDIA" }).text_size(px(self.tokens.caption_size)).text_color(cx.theme().primary))
                .child(Label::new(inspection.title.clone()).text_size(px(22.)).font_weight(FontWeight::SEMIBOLD))
                .when_some(inspection.uploader.clone(), |view, uploader| view.child(h_flex().gap(gap).child(Icon::new(IconName::CircleUser)).child(Label::new(uploader))))
                .child(h_flex().gap(gap)
                    .child(Button::new("media-count").icon(if inspection.is_playlist { IconName::GalleryVerticalEnd } else { IconName::Play }).label(if inspection.is_playlist { format!("{} tracks", inspection.total_items) } else { duration.unwrap_or_else(|| "Video".into()) }).small().ghost())
                    .when(inspection.is_playlist, |row| row.child(Button::new("selected-count").icon(IconName::CircleCheck).label(format!("{playlist_selected} selected")).small().ghost())));
            let hero = card(h_flex().gap(px(self.tokens.section_gap)).items_center().child(artwork).child(metadata).into_any_element()).into_any_element();
            let content = if inspection.is_playlist {
                let select_list = self.playlist_list.clone(); let clear_list = self.playlist_list.clone();
                let playlist_actions = h_flex().gap(gap)
                    .child(Button::new("select-all").icon(IconName::CircleCheck).label("All").small().on_click(cx.listener(move |_, _, _, cx| { select_list.update(cx, |state, cx| { state.delegate_mut().select_all(); cx.notify(); }); })))
                    .child(Button::new("clear-all").label("Clear").small().ghost().on_click(cx.listener(move |_, _, _, cx| { clear_list.update(cx, |state, cx| { state.delegate_mut().clear(); cx.notify(); }); })));
                let playlist_header = h_flex().justify_between().items_center()
                    .child(Label::new("Playlist").text_size(px(18.)).font_weight(FontWeight::SEMIBOLD))
                    .child(playlist_actions);
                let playlist_content = v_flex().gap(gap)
                    .child(playlist_header)
                    .child(div().h(px(430.)).child(List::new(&self.playlist_list)));
                card(playlist_content.into_any_element()).into_any_element()
            } else if let Some(item) = inspection.items.first() {
                let want_audio = kind == MediaKind::Audio;
                let rows = item.formats.iter().filter(|format| if want_audio { format.has_audio } else { format.has_video }).map(|format| {
                    let id = format.id.clone(); let selected = self.config.download.selected_format_id.as_ref() == Some(&id);
                    Button::new(SharedString::from(format!("available-{id}"))).label(format!("{}  ·  {}  ·  {}", format.resolution.clone().unwrap_or_else(|| "Audio".into()), format.extension.to_uppercase(), format.size.map(format_bytes).unwrap_or_else(|| "Unknown size".into()))).selected(selected).on_click(cx.listener(move |this, _, _, cx| { this.config.download.selected_format_id = Some(id.clone()); cx.notify(); })).into_any_element()
                }).collect::<Vec<_>>();
                card(v_flex().gap(gap).child(Label::new("Available quality").text_size(px(18.)).font_weight(FontWeight::SEMIBOLD)).child(div().h(px(390.)).overflow_y_scrollbar().child(v_flex().gap(px(2.)).children(rows))).into_any_element()).into_any_element()
            } else { card(Label::new("No available media").into_any_element()).into_any_element() };
            (hero, content)
        } else {
            (card(v_flex().h(px(190.)).items_center().justify_center().gap(gap).child(Icon::new(IconName::ExternalLink).large()).child(Label::new("Paste a link to begin")).into_any_element()).into_any_element(),
             card(v_flex().h(px(300.)).into_any_element()).into_any_element())
        };

        let task_cards = self.tasks.values().cloned().map(|task| {
            let percentage = if task.total == 0 { 0.0 } else { task.downloaded as f32 / task.total as f32 * 100.0 };
            let controller = self.controller.clone(); let cancel_url = task.url.clone();
            v_flex().gap(px(6.)).p(px(12.)).border_1().border_color(cx.theme().border).rounded(px(self.tokens.corner_radius))
                .child(h_flex().items_center().gap(gap).child(Icon::new(if task.state == "Complete" { IconName::CircleCheck } else if task.state == "Failed" { IconName::CircleX } else { IconName::LoaderCircle })).child(Label::new(task.title)).child(div().flex_1()).child(Label::new(task.state.clone())).child(Button::new(SharedString::from(format!("cancel-{}", task.url))).icon(IconName::CircleX).ghost().small().disabled(matches!(task.state.as_str(), "Complete" | "Failed" | "Canceled")).on_click(cx.listener(move |_, _, _, _| { if let Some(controller) = &controller { controller.cancel(&cancel_url); } }))))
                .child(Progress::new(SharedString::from(format!("task-progress-{}", task.url))).loading(task.total == 0 && task.state == "Downloading").value(percentage))
                .child(h_flex().gap(gap).child(Label::new(format!("{} / {}", format_bytes(task.downloaded), if task.total == 0 { "—".into() } else { format_bytes(task.total) })).text_size(px(self.tokens.caption_size))).when_some(task.speed, |row, speed| row.child(Label::new(format!("{}/s", format_bytes(speed as u64))).text_size(px(self.tokens.caption_size)))).when_some(task.eta, |row, eta| row.child(Label::new(format!("{}:{:02} left", eta / 60, eta % 60)).text_size(px(self.tokens.caption_size)))).when_some(task.error, |row, error| row.child(Label::new(error).text_color(cx.theme().danger))))
        }).collect::<Vec<_>>();
        let cancel_controller = self.controller.clone();
        let task_header = h_flex().justify_between()
            .child(Label::new("Downloads").text_size(px(18.)).font_weight(FontWeight::SEMIBOLD))
            .child(Button::new("cancel-all").icon(IconName::CircleX).label("Cancel all").small().ghost().on_click(cx.listener(move |_, _, _, _| { if let Some(controller) = &cancel_controller { controller.cancel_all(); } })));
        let tasks = v_flex().gap(gap).when(!task_cards.is_empty(), |view| view.child(task_header)).children(task_cards);
        let body = v_flex().gap(px(self.tokens.section_gap)).max_w(px(self.tokens.content_max_width)).w_full().child(hero).child(h_flex().items_start().gap(px(self.tokens.section_gap)).child(div().w(px(390.)).child(left)).child(div().flex_1().min_w_0().child(content))).child(tasks);

        div().size_full().overflow_y_scrollbar().bg(cx.theme().background).child(v_flex().items_center().p(px(self.tokens.page_padding)).gap(px(self.tokens.section_gap))
            .child(v_flex().max_w(px(self.tokens.content_max_width)).w_full().gap(px(4.)).child(Label::new("SilvaDL").text_size(px(self.tokens.title_size)).font_weight(FontWeight::BOLD)).child(Label::new("Fast parallel video and music downloads powered by yt-dlp").text_size(px(self.tokens.body_size)).text_color(cx.theme().muted_foreground)))
            .child(body)).into_any_element()
    }
}

fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    if bytes as f64 >= GIB { format!("{:.1} GB", bytes as f64 / GIB) } else { format!("{:.1} MB", bytes as f64 / MIB) }
}

fn format_duration(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{}:{:02}", total / 60, total % 60)
}
