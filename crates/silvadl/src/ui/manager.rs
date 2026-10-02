use super::*;

impl DownloaderView {
    pub(super) fn render_manager(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity().clone();
        let content = if self.tasks.is_empty() {
            div().max_w(px(1320.)).w_full().h_full().mx_auto().overflow_hidden()
                .child(self.render_general_options(cx, view.clone())).into_any_element()
        } else {
            h_flex().max_w(px(1600.)).w_full().h_full().mx_auto().items_start().overflow_hidden()
                .child(div().p(px(18.)).child(self.render_quick_add(cx, view.clone())))
                .child(div().flex_1().min_w_0().h_full().child(self.render_download_manager(cx)))
                .into_any_element()
        };

        let dialog = self.show_add.then(|| {
            self.build_add_dialog(Dialog::new(cx), window, cx, cx.entity().clone()).into_any_element()
        });
        v_flex().size_full().relative().bg(linear_gradient(
                110.,
                linear_color_stop(cx.theme().background, 0.38),
                linear_color_stop(cx.theme().secondary.opacity(0.72), 1.0),
            ))
            .child(div().flex_1().min_h_0().p(px(self.tokens.page_padding)).child(content))
            .children(dialog)
            .into_any_element()
    }

    fn render_download_manager(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let counts = [
            (TaskFilter::All, "All", self.tasks.len()),
            (TaskFilter::Running, "Running", self.tasks.values().filter(|t| matches!(t.state.as_str(), "Starting" | "Downloading")).count()),
            (TaskFilter::Queued, "Queued", self.tasks.values().filter(|t| t.state == "Queued").count()),
            (TaskFilter::Completed, "Completed", self.tasks.values().filter(|t| t.state == "Complete").count()),
            (TaskFilter::Failed, "Failed", self.tasks.values().filter(|t| matches!(t.state.as_str(), "Failed" | "Canceled")).count()),
        ];
        let nav = counts.into_iter().enumerate().map(|(index, (filter, label, count))| {
            let icon = match filter { TaskFilter::All => IconName::LayoutDashboard, TaskFilter::Running => IconName::LoaderCircle, TaskFilter::Queued => IconName::Inbox, TaskFilter::Completed => IconName::CircleCheck, TaskFilter::Failed => IconName::TriangleAlert };
            Button::new(SharedString::from(format!("queue-filter-{index}"))).icon(icon).label(format!("{label} {count}")).small().selected(self.task_filter == filter).on_click(cx.listener(move |this, _, _, cx| { this.task_filter = filter; this.sync_download_list(cx); cx.notify(); }))
        }).collect::<Vec<_>>();
        let folder = self.config.download.output_directory.to_string_lossy().into_owned();

        let active = self.task_order.iter().filter_map(|url| self.tasks.get(url)).find(|task| matches!(task.state.as_str(), "Downloading" | "Starting")).or_else(|| self.task_order.first().and_then(|url| self.tasks.get(url))).cloned();
        let detail = if let Some(task) = active {
            let art = task.thumbnail.clone().map(|path| img(std::path::PathBuf::from(path)).w(px(220.)).h(px(220.)).rounded(px(110.)).into_any_element()).unwrap_or_else(|| div().w(px(220.)).h(px(220.)).rounded(px(110.)).bg(cx.theme().secondary).child(Icon::new(IconName::GalleryVerticalEnd).large()).into_any_element());
            let percentage = if task.total == 0 { 0.0 } else { task.downloaded as f32 / task.total as f32 * 100.0 };
            v_flex().w(px(290.)).h_full().items_center().justify_center().p(px(18.)).gap(px(12.)).border_l_1().border_color(cx.theme().border)
                .child(art)
                .child(Label::new(task.title).font_weight(FontWeight::BOLD).text_size(px(18.)))
                .when_some(task.uploader, |view, uploader| view.child(Label::new(uploader).text_color(cx.theme().muted_foreground)))
                .child(Progress::new("active-detail-progress").loading(task.total == 0 && matches!(task.state.as_str(), "Downloading" | "Starting")).value(percentage))
                .child(h_flex().w_full().justify_between().child(Label::new(task.state)).when_some(task.speed, |row, speed| row.child(Label::new(format!("{}/s", format_bytes(speed as u64))))))
                .child(h_flex().w_full().gap(px(8.)).child(Icon::new(IconName::Folder)).child(Label::new(folder).text_size(px(self.tokens.caption_size)).text_color(cx.theme().muted_foreground)))
                .into_any_element()
        } else { div().w(px(290.)).h_full().border_l_1().border_color(cx.theme().border).into_any_element() };

        let cancel_controller = self.controller.clone();
        h_flex().w_full().h_full().overflow_hidden()
            .child(v_flex().flex_1().min_w_0().h_full().gap(px(10.)).p(px(14.))
                .child(h_flex().justify_between().items_center().gap(px(12.))
                    .child(h_flex().gap(px(5.)).children(nav))
                    .child(h_flex().gap(px(10.)).items_center()
                        .child(Label::new(self.status.clone()).text_color(cx.theme().muted_foreground))
                        .child(Button::new("cancel-all-downloads").icon(IconName::CircleX).label("Cancel all").small().ghost().disabled(!self.running).on_click(move |_, _, _| { if let Some(controller) = &cancel_controller { controller.cancel_all(); } }))))
                .child(div().flex_1().min_h_0().child(List::new(&self.download_list))))
            .child(detail)
            .into_any_element()
    }

    fn render_quick_add(&self, cx: &App, view: Entity<Self>) -> AnyElement {
        let kind = self.config.download.media_kind;
        let kind_view = view.clone();
        let download_view = view.clone();
        let format_view = view.clone();
        let selected = self.playlist_list.read(cx).delegate().selected_count();
        let preview = self.inspection.as_ref().map(|media| {
            let art = media.thumbnail.as_ref().map(|path| img(std::path::PathBuf::from(path)).w(px(112.)).h(px(64.)).rounded(px(7.)).into_any_element()).unwrap_or_else(|| Icon::new(IconName::GalleryVerticalEnd).into_any_element());
            h_flex().gap(px(10.)).child(art).child(v_flex().gap(px(4.)).child(Label::new(media.title.clone()).font_weight(FontWeight::SEMIBOLD)).child(Label::new(if media.is_playlist { format!("{} selected", selected) } else { "Ready".into() }).text_color(cx.theme().muted_foreground))).into_any_element()
        });
        let resolution = self.config.download.resolution;
        let format_control = if kind == MediaKind::Video {
            ButtonGroup::new("workspace-resolution").outline().small()
                .child(Button::new("workspace-best").label("Best").selected(resolution.is_none()))
                .child(Button::new("workspace-1080").label("1080p").selected(resolution == Some(1080)))
                .child(Button::new("workspace-720").label("720p").selected(resolution == Some(720)))
                .child(Button::new("workspace-480").label("480p").selected(resolution == Some(480)))
                .on_click(move |selected: &Vec<usize>, _, cx| { let value = match selected.first().copied().unwrap_or(0) { 1 => Some(1080), 2 => Some(720), 3 => Some(480), _ => None }; format_view.update(cx, |this, cx| { this.config.download.resolution = value; this.config.download.selected_format_id = None; cx.notify(); }); }).into_any_element()
        } else {
            ButtonGroup::new("workspace-audio-format").outline().small()
                .child(Button::new("workspace-mp3").label("MP3").selected(self.config.download.audio_format == "mp3"))
                .child(Button::new("workspace-m4a").label("M4A").selected(self.config.download.audio_format == "m4a"))
                .child(Button::new("workspace-opus").label("Opus").selected(self.config.download.audio_format == "opus"))
                .on_click(move |selected: &Vec<usize>, _, cx| { let value = match selected.first().copied().unwrap_or(0) { 1 => "m4a", 2 => "opus", _ => "mp3" }; format_view.update(cx, |this, cx| { this.config.download.audio_format = value.into(); this.config.download.selected_format_id = None; cx.notify(); }); }).into_any_element()
        };
        v_flex().w(px(360.)).h_full().gap(px(14.)).pr(px(20.)).border_r_1().border_color(cx.theme().border)
            .child(Label::new("Add media").font_weight(FontWeight::BOLD).text_size(px(18.)))
            .child(Form::vertical()
                .child(Field::new().label("Link").child(div().w_full().child(Input::new(&self.urls))))
                .child(Field::new().label("Save folder").child(div().w_full().child(Input::new(&self.output))))
                .child(Field::new().label("File type").child(ButtonGroup::new("workspace-kind").outline().small()
                    .child(Button::new("workspace-video").icon(IconName::Play).label("Video").selected(kind == MediaKind::Video))
                    .child(Button::new("workspace-audio").icon(IconName::File).label("Audio").selected(kind == MediaKind::Audio))
                    .on_click(move |selected: &Vec<usize>, _, cx| { kind_view.update(cx, |this, cx| { this.config.download.media_kind = if selected.contains(&1) { MediaKind::Audio } else { MediaKind::Video }; cx.notify(); }); })))
                .child(Field::new().label(if kind == MediaKind::Video { "Resolution" } else { "Audio format" }).child(format_control)))
            .when_some(preview, |panel, preview| panel.child(preview))
            .child(div().flex_1())
            .child(Button::new("workspace-download").icon(IconName::ArrowDown).label(if self.running { "Downloading".into() } else if self.inspection.as_ref().is_some_and(|m| m.is_playlist) { format!("Download {selected}") } else { "Download".into() }).primary().w_full().disabled(self.running || self.inspection.is_none() || selected == 0).on_click(move |_, _, cx| { download_view.update(cx, |this, cx| this.begin(cx)); }))
            .into_any_element()
    }

    fn build_add_dialog(&self, dialog: Dialog, _window: &mut Window, cx: &App, view: Entity<Self>) -> Dialog {
        let close_view = view.clone();
        let tab_view = view.clone();
        let tabs = TabBar::new("add-tabs").segmented().selected_index(self.add_tab)
            .child(Tab::new().label("General").icon(IconName::ArrowDown))
            .child(Tab::new().label("Subtitles").icon(IconName::File))
            .child(Tab::new().label("Advanced").icon(IconName::Settings2))
            .on_click(move |index: &usize, _, cx| { tab_view.update(cx, |this, cx| { this.add_tab = *index; cx.notify(); }); });

        let body = match self.add_tab {
            1 => self.render_subtitle_options(cx, view.clone()),
            2 => self.render_advanced_options(cx, view.clone()),
            _ => self.render_general_options(cx, view.clone()),
        };
        dialog.title("Add download").w(px(1120.)).on_close(move |_, _, cx| {
            close_view.update(cx, |this, cx| { this.show_add = false; cx.notify(); });
        }).child(v_flex().gap(px(self.tokens.section_gap)).child(tabs).child(body))
    }

    fn render_general_options(&self, cx: &App, view: Entity<Self>) -> AnyElement {
        let kind = self.config.download.media_kind;
        let selected = self.playlist_list.read(cx).delegate().selected_count();
        let preview = if self.analyzing && self.inspection.is_none() {
            h_flex().h(px(110.)).items_center().justify_center().gap(px(10.)).child(ProgressCircle::new("dialog-analysis").loading(true)).child(Label::new("Inspecting link…")).into_any_element()
        } else if let Some(media) = &self.inspection {
            let artwork = media.thumbnail.as_ref().map(|path| img(std::path::PathBuf::from(path)).w(px(160.)).h(px(90.)).rounded(px(8.)).into_any_element()).unwrap_or_else(|| Icon::new(IconName::GalleryVerticalEnd).large().into_any_element());
            h_flex().gap(px(14.)).child(artwork).child(v_flex().gap(px(5.)).child(Label::new(media.title.clone()).font_weight(FontWeight::SEMIBOLD)).when_some(media.uploader.clone(), |v, uploader| v.child(Label::new(uploader).text_color(cx.theme().muted_foreground))).child(Label::new(if media.is_playlist { format!("{} items · {selected} selected", media.total_items) } else { "Ready to download".into() }))).into_any_element()
        } else {
            h_flex().h(px(90.)).items_center().justify_center().child(Label::new("Paste a link to inspect available media").text_color(cx.theme().muted_foreground)).into_any_element()
        };
        let playlist = self.inspection.as_ref().filter(|m| m.is_playlist).map(|_| {
            let select_list = self.playlist_list.clone();
            let clear_list = self.playlist_list.clone();
            let select_parent = view.clone();
            let clear_parent = view.clone();
            v_flex().flex_1().min_h_0().mt(px(6.)).gap(px(10.)).p(px(12.)).border_1().border_color(cx.theme().border).rounded(px(self.tokens.corner_radius))
                .child(h_flex().items_center().justify_between()
                    .child(Label::new(format!("Playlist · {selected} selected")).font_weight(FontWeight::SEMIBOLD))
                    .child(h_flex().gap(px(8.))
                        .child(Button::new("dialog-select-all").icon(IconName::CircleCheck).label("All").small().on_click(move |_, _, cx| {
                            select_list.update(cx, |state, cx| { state.delegate_mut().select_all(); cx.notify(); });
                            select_parent.update(cx, |_, cx| cx.notify());
                        }))
                        .child(Button::new("dialog-clear-all").label("Clear").small().ghost().on_click(move |_, _, cx| {
                            clear_list.update(cx, |state, cx| { state.delegate_mut().clear(); cx.notify(); });
                            clear_parent.update(cx, |_, cx| cx.notify());
                        }))))
                .child(div().flex_1().min_h_0().overflow_hidden().rounded(px(6.)).child(List::new(&self.playlist_list)))
                .into_any_element()
        });
        let available_formats = self.inspection.as_ref().and_then(|media| media.items.first()).map(|item| {
            item.formats.iter().filter(|format| if kind == MediaKind::Audio { format.has_audio } else { format.has_video }).take(10).cloned().collect::<Vec<_>>()
        }).unwrap_or_default();
        let quality = if available_formats.is_empty() { None } else {
            Some(h_flex().gap(px(6.)).flex_wrap().children(available_formats.into_iter().map(|format| {
                let id = format.id.clone();
                let active = self.config.download.selected_format_id.as_ref() == Some(&id);
                let format_view = view.clone();
                Button::new(SharedString::from(format!("dialog-format-{id}")))
                    .label(format!("{} · {}", format.resolution.unwrap_or_else(|| "Audio".into()), format.extension.to_uppercase()))
                    .small().selected(active)
                    .on_click(move |_, _, cx| { format_view.update(cx, |this, cx| { this.config.download.selected_format_id = Some(id.clone()); cx.notify(); }); })
            })).into_any_element())
        };
        let kind_view = view.clone();
        let download_view = view.clone();
        let resolution = self.config.download.resolution;
        let resolution_selector = v_flex().w_full().gap(px(6.)).children(
            [
                vec![(None, "Best"), (Some(2160), "2160p"), (Some(1440), "1440p")],
                vec![(Some(1080), "1080p"), (Some(720), "720p"), (Some(480), "480p")],
                vec![(Some(360), "360p")],
            ].into_iter().enumerate().map(|(row, values)| {
                h_flex().gap(px(6.)).children(values.into_iter().enumerate().map(|(column, (value, label))| {
                    let resolution_view = view.clone();
                    Button::new(SharedString::from(format!("resolution-choice-{row}-{column}"))).label(label).small().selected(resolution == value).on_click(move |_, _, cx| {
                        resolution_view.update(cx, |this, cx| { this.config.download.resolution = value; this.config.download.selected_format_id = None; cx.notify(); });
                    })
                }))
            })
        );
        let current_audio = self.config.download.audio_format.clone();
        let audio_selector = v_flex().w_full().gap(px(6.)).children(
            [vec!["mp3", "m4a", "opus"], vec!["flac", "wav"]].into_iter().enumerate().map(|(row, values)| {
                h_flex().gap(px(6.)).children(values.into_iter().enumerate().map(|(column, value)| {
                    let audio_view = view.clone();
                    Button::new(SharedString::from(format!("audio-choice-{row}-{column}"))).label(value.to_uppercase()).small().selected(current_audio == value).on_click(move |_, _, cx| {
                        audio_view.update(cx, |this, cx| { this.config.download.audio_format = value.into(); this.config.download.selected_format_id = None; cx.notify(); });
                    })
                }))
            })
        );
        let options = v_flex().w(px(360.)).h_full().gap(px(14.)).pr(px(22.)).border_r_1().border_color(cx.theme().border)
            .child(Form::vertical()
                .child(Field::new().label("Link").child(Input::new(&self.urls).w_full()))
                .child(Field::new().label("Save folder").child(Input::new(&self.output).w_full()))
                .child(Field::new().label("File type").child(ButtonGroup::new("dialog-kind").outline().child(Button::new("dialog-video").icon(IconName::Play).label("Video").selected(kind == MediaKind::Video)).child(Button::new("dialog-audio").icon(IconName::File).label("Audio").selected(kind == MediaKind::Audio)).on_click(move |selected: &Vec<usize>, _, cx| { kind_view.update(cx, |this, cx| { this.config.download.media_kind = if selected.contains(&1) { MediaKind::Audio } else { MediaKind::Video }; cx.notify(); }); }))))
            .child(Field::new().label(if kind == MediaKind::Video { "Resolution" } else { "Audio format" }).child(if kind == MediaKind::Video { resolution_selector.into_any_element() } else { audio_selector.into_any_element() }))
            .when_some(quality, |view, quality| view.child(Field::new().label("Available format").child(quality)))
            .child(h_flex().justify_end().child(
                Button::new("confirm-download")
                    .icon(IconName::ArrowDown)
                    .label(if self.inspection.as_ref().is_some_and(|m| m.is_playlist) { format!("Download {selected}") } else { "Download".into() })
                    .primary()
                    .disabled(self.inspection.is_none() || selected == 0)
                    .on_click(move |_, _, cx| { download_view.update(cx, |this, cx| { this.begin(cx); this.show_add = false; cx.notify(); }); })
            ));
        let media_panel = playlist.unwrap_or_else(|| v_flex().flex_1().items_center().justify_center().child(Label::new("Media details and available qualities appear here").text_color(cx.theme().muted_foreground)).into_any_element());
        let right = v_flex().flex_1().min_w_0().h_full().gap(px(14.)).p(px(18.))
            .child(preview)
            .child(div().flex_1().min_h_0().child(media_panel));
        h_flex().size_full().items_start()
            .child(options)
            .child(right)
            .into_any_element()
    }

    fn render_subtitle_options(&self, cx: &App, view: Entity<Self>) -> AnyElement {
        v_flex().gap(px(14.)).child(Switch::new("dialog-subtitles").label("Download subtitles").checked(self.config.download.download_subtitles).on_click(move |value: &bool, _, cx| { view.update(cx, |this, cx| { this.config.download.download_subtitles = *value; cx.notify(); }); })).child(Label::new("Subtitle tracks are selected from the formats available for this media.").text_color(cx.theme().muted_foreground)).into_any_element()
    }

    fn render_advanced_options(&self, cx: &App, view: Entity<Self>) -> AnyElement {
        let metadata_view = view.clone();
        v_flex().gap(px(14.))
            .child(Switch::new("dialog-metadata").label("Embed metadata").checked(self.config.download.embed_metadata).on_click(move |value: &bool, _, cx| { metadata_view.update(cx, |this, cx| { this.config.download.embed_metadata = *value; cx.notify(); }); }))
            .child(Switch::new("dialog-thumbnail").label("Embed thumbnail").checked(self.config.download.embed_thumbnail).on_click(move |value: &bool, _, cx| { view.update(cx, |this, cx| { this.config.download.embed_thumbnail = *value; cx.notify(); }); }))
            .child(Label::new("Parallel workers, fragments, retry policy, and archive behavior use your saved application configuration.").text_color(cx.theme().muted_foreground))
            .into_any_element()
    }
}
