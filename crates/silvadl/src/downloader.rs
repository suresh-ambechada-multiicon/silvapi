use anyhow::{bail, Context, Result};
use crate::model::{DownloadConfig, DownloadEvent, MediaKind};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::{BufRead, BufReader, Read},
    process::{Child, Command, Stdio},
    sync::{mpsc::Sender, Arc, Mutex},
    thread,
    time::Duration,
};

#[derive(Clone, Default)]
pub struct DownloadController {
    children: Arc<Mutex<HashMap<String, Arc<Mutex<Child>>>>>,
    canceled: Arc<Mutex<HashSet<String>>>,
    cancel_all_requested: Arc<Mutex<bool>>,
}

impl DownloadController {
    pub fn cancel(&self, url: &str) {
        self.canceled.lock().expect("canceled set poisoned").insert(url.to_owned());
        if let Some(child) = self.children.lock().expect("children poisoned").get(url).cloned() {
            let _ = child.lock().expect("child poisoned").kill();
        }
    }

    pub fn cancel_all(&self) {
        *self.cancel_all_requested.lock().expect("cancel flag poisoned") = true;
        let children: Vec<_> = self.children.lock().expect("children poisoned").values().cloned().collect();
        for child in children { let _ = child.lock().expect("child poisoned").kill(); }
    }
}

impl DownloadConfig {
    pub fn args_for(&self, url: &str) -> Vec<String> {
        let mut args = vec![
            "--newline".into(), "--continue".into(),
            "--no-overwrites".into(), "--retries".into(), self.retries.to_string(),
            "--fragment-retries".into(), self.retries.to_string(),
            "--concurrent-fragments".into(), self.concurrent_fragments.max(1).to_string(),
            "--paths".into(), self.output_directory.to_string_lossy().into_owned(),
            "--output".into(),
            "%(title)s [%(id)s].%(ext)s".into(),
        ];
        args.extend(["--progress-template".into(), "download:silvadl:%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s".into()]);
        args.push(if self.download_playlist { "--yes-playlist" } else { "--no-playlist" }.into());
        if let Some(path) = &self.ffmpeg_path { args.extend(["--ffmpeg-location".into(), path.clone()]); }
        match self.media_kind {
            MediaKind::Audio => {
                args.extend(["--extract-audio".into(), "--audio-format".into(), self.audio_format.clone(),
                    "--audio-quality".into(), "0".into()]);
            }
            MediaKind::Video => {
                let h = self.resolution.map(|v| format!("[height<={v}]")).unwrap_or_default();
                args.extend(["--format".into(), format!("bv*{h}+ba/b{h}"),
                    "--merge-output-format".into(), self.video_container.clone()]);
            }
        }
        if let Some(format_id) = &self.selected_format_id {
            args.extend(["--format".into(), format_id.clone()]);
        }
        if self.embed_metadata { args.push("--embed-metadata".into()); args.push("--embed-chapters".into()); }
        if self.embed_thumbnail { args.push("--embed-thumbnail".into()); }
        if self.download_subtitles { args.extend(["--write-subs".into(), "--write-auto-subs".into(), "--embed-subs".into()]); }
        if self.use_download_archive {
            args.extend(["--download-archive".into(), self.output_directory.join(".silvadl-archive.txt").to_string_lossy().into_owned()]);
        }
        args.push(url.into());
        args
    }
}

pub fn check_dependencies(config: &DownloadConfig) -> Result<String> {
    let output = Command::new(&config.yt_dlp_path).arg("--version").output()
        .with_context(|| format!("Could not start '{}'. Install yt-dlp or set its path", config.yt_dlp_path))?;
    if !output.status.success() { bail!("yt-dlp returned an error"); }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn expand_playlist(config: &DownloadConfig, url: &str) -> Vec<String> {
    if !config.download_playlist || config.playlist_workers <= 1 { return vec![url.into()]; }
    let result = Command::new(&config.yt_dlp_path)
        .args(["--flat-playlist", "--ignore-errors", "--print", "webpage_url", url])
        .stdout(Stdio::piped()).stderr(Stdio::null()).output();
    match result {
        Ok(out) if out.status.success() => {
            let urls: Vec<_> = String::from_utf8_lossy(&out.stdout).lines()
                .map(str::trim).filter(|s| s.starts_with("http")).map(str::to_owned).collect();
            if urls.len() > 1 { urls } else { vec![url.into()] }
        }
        _ => vec![url.into()],
    }
}

pub fn start(urls: Vec<String>, config: DownloadConfig, tx: Sender<DownloadEvent>) -> DownloadController {
    let controller = DownloadController::default();
    let background_controller = controller.clone();
    thread::spawn(move || {
        if let Err(error) = std::fs::create_dir_all(&config.output_directory) {
            let _ = tx.send(DownloadEvent::Log(format!("Cannot create output folder: {error}")));
            let _ = tx.send(DownloadEvent::Finished { succeeded: 0, failed: urls.len() });
            return;
        }
        let mut items = Vec::new();
        for url in urls { items.extend(expand_playlist(&config, &url)); }
        let total = items.len();
        let _ = tx.send(DownloadEvent::QueueReady { items: total });
        let queue = Arc::new(Mutex::new(VecDeque::from(items)));
        let results = Arc::new(Mutex::new((0usize, 0usize)));
        let workers = config.playlist_workers.max(1).min(total.max(1));
        let mut handles = Vec::new();
        for worker in 0..workers {
            let queue = queue.clone(); let results = results.clone();
            let tx = tx.clone(); let config = config.clone();
            let controller = background_controller.clone();
            handles.push(thread::spawn(move || loop {
                let url = queue.lock().expect("queue poisoned").pop_front();
                let Some(url) = url else { break };
                let canceled = *controller.cancel_all_requested.lock().expect("cancel flag poisoned")
                    || controller.canceled.lock().expect("canceled set poisoned").contains(&url);
                if canceled {
                    let _ = tx.send(DownloadEvent::ItemCanceled { url: url.clone() });
                    let _ = tx.send(DownloadEvent::ItemFinished { url, success: false });
                    let mut counts = results.lock().expect("results poisoned"); counts.1 += 1;
                    continue;
                }
                let _ = tx.send(DownloadEvent::ItemStarted { worker: worker + 1, url: url.clone() });
                let mut command = Command::new(&config.yt_dlp_path);
                command.args(config.args_for(&url)).stdout(Stdio::piped()).stderr(Stdio::piped());
                let child = match command.spawn() {
                    Ok(child) => Arc::new(Mutex::new(child)),
                    Err(error) => {
                        let _ = tx.send(DownloadEvent::ItemError { url: url.clone(), message: error.to_string() });
                        let mut counts = results.lock().expect("results poisoned"); counts.1 += 1;
                        continue;
                    }
                };
                let (stdout, stderr) = {
                    let mut process = child.lock().expect("child poisoned");
                    (process.stdout.take(), process.stderr.take())
                };
                controller.children.lock().expect("children poisoned").insert(url.clone(), child.clone());
                if let Some(stdout) = stdout {
                    let tx_progress = tx.clone(); let progress_url = url.clone();
                    thread::spawn(move || read_progress(stdout, &progress_url, &tx_progress));
                }
                let error_output = Arc::new(Mutex::new(String::new()));
                if let Some(stderr) = stderr {
                    let errors = error_output.clone();
                    thread::spawn(move || { let mut reader = BufReader::new(stderr); let _ = reader.read_to_string(&mut errors.lock().expect("errors poisoned")); });
                }
                let status = loop {
                    let state = child.lock().expect("child poisoned").try_wait();
                    match state { Ok(Some(status)) => break Some(status), Ok(None) => thread::sleep(Duration::from_millis(100)), Err(_) => break None }
                };
                controller.children.lock().expect("children poisoned").remove(&url);
                let ok = status.is_some_and(|s| s.success());
                if !ok {
                    let message = error_output.lock().expect("errors poisoned").lines().rev().find(|line| !line.trim().is_empty()).unwrap_or("Download canceled").to_owned();
                    let canceled = status.and_then(|s| s.code()).is_none();
                    let _ = tx.send(if canceled { DownloadEvent::ItemCanceled { url: url.clone() } } else { DownloadEvent::ItemError { url: url.clone(), message } });
                }
                let mut counts = results.lock().expect("results poisoned");
                if ok { counts.0 += 1 } else { counts.1 += 1 }
                let _ = tx.send(DownloadEvent::ItemFinished { url, success: ok });
            }));
        }
        for handle in handles { let _ = handle.join(); }
        let (succeeded, failed) = *results.lock().expect("results poisoned");
        let _ = tx.send(DownloadEvent::Finished { succeeded, failed });
    });
    controller
}

fn read_progress(stdout: impl Read, url: &str, tx: &Sender<DownloadEvent>) {
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let Some(values) = line.strip_prefix("silvadl:") else { continue };
        let parts: Vec<_> = values.split('|').collect();
        if parts.len() < 5 { continue; }
        let number = |value: &str| value.trim().parse::<f64>().ok();
        let downloaded = number(parts[0]).unwrap_or(0.0) as u64;
        let total = number(parts[1]).or_else(|| number(parts[2])).unwrap_or(0.0) as u64;
        let speed = number(parts[3]);
        let eta = number(parts[4]).map(|value| value as u64);
        let _ = tx.send(DownloadEvent::ItemProgress { url: url.to_owned(), downloaded, total, speed, eta });
    }
}
