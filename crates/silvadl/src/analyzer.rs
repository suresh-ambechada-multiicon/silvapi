use crate::model::{MediaFormat, MediaInspection, MediaItem};
use std::{env, path::{Path, PathBuf}, sync::{Arc, mpsc::Sender}, thread};
use yt_dlp::{Downloader, client::deps::Libraries, model::Video};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Clone, Debug)]
pub enum AnalysisEvent {
    Started,
    PlaylistStarted { title: String, uploader: Option<String> },
    ItemDiscovered(MediaItem),
    Ready(MediaInspection),
    ThumbnailCached { id: String, path: String },
    Finished,
    Failed(String),
}

pub fn analyze(url: String, output_directory: PathBuf, tx: Sender<AnalysisEvent>) {
    thread::spawn(move || {
        let _ = tx.send(AnalysisEvent::Started);
        let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(error) => { let _ = tx.send(AnalysisEvent::Failed(error.to_string())); return; }
        };
        let normalized = normalize_url(&url);
        if is_playlist_url(&normalized) {
            if let Err(error) = runtime.block_on(analyze_playlist_stream(&normalized, tx.clone())) {
                let _ = tx.send(AnalysisEvent::Failed(error.to_string()));
            }
            return;
        }
        let result = runtime.block_on(analyze_async(&url, output_directory));
        match result {
            Ok(inspection) => {
                let _ = tx.send(AnalysisEvent::Ready(inspection.clone()));
                runtime.block_on(cache_item_thumbnails(&inspection, tx.clone()));
                let _ = tx.send(AnalysisEvent::Finished);
            }
            Err(error) => { let _ = tx.send(AnalysisEvent::Failed(error.to_string())); }
        }
    });
}

async fn analyze_playlist_stream(url: &str, tx: Sender<AnalysisEvent>) -> anyhow::Result<()> {
    let executable = find_on_path(executable_name("yt-dlp")).ok_or_else(|| anyhow::anyhow!("yt-dlp is not installed"))?;
    let mut child = tokio::process::Command::new(executable)
        .args(["--flat-playlist", "--ignore-errors", "--dump-json", url])
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn()?;
    let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("yt-dlp did not provide playlist output"))?;
    let mut lines = BufReader::new(stdout).lines();
    let mut items = Vec::new();
    let mut announced = false;
    let directory = std::env::temp_dir().join("silvadl-thumbnails");
    let _ = tokio::fs::create_dir_all(&directory).await;
    let client = reqwest::Client::new();
    let limit = Arc::new(tokio::sync::Semaphore::new(24));
    let mut thumbnails = tokio::task::JoinSet::new();
    while let Some(line) = lines.next_line().await? {
        let Ok(value) = serde_json::from_str::<Value>(&line) else { continue; };
        if !announced {
            let title = string(&value, "playlist_title").or_else(|| string(&value, "playlist")).unwrap_or_else(|| "Playlist".into());
            let uploader = string(&value, "playlist_uploader").or_else(|| string(&value, "channel"));
            let _ = tx.send(AnalysisEvent::PlaylistStarted { title, uploader });
            announced = true;
        }
        let item = raw_item(&value);
        if item.id.is_empty() || matches!(item.availability.as_deref(), Some("private" | "premium_only" | "subscriber_only")) { continue; }
        items.push(item.clone());
        let _ = tx.send(AnalysisEvent::ItemDiscovered(item));
        let thumbnail_item = items.last().cloned().expect("item was just inserted");
        let thumbnail_tx = tx.clone();
        let thumbnail_client = client.clone();
        let thumbnail_limit = limit.clone();
        let thumbnail_directory = directory.clone();
        thumbnails.spawn(async move {
            if let Some((id, path)) = cache_one_thumbnail(&thumbnail_item, thumbnail_client, thumbnail_limit, thumbnail_directory).await {
                let _ = thumbnail_tx.send(AnalysisEvent::ThumbnailCached { id, path: path.to_string_lossy().into_owned() });
            }
        });
    }
    let status = child.wait().await?;
    if !status.success() && items.is_empty() { return Err(anyhow::anyhow!("yt-dlp could not inspect this playlist")); }
    while thumbnails.join_next().await.is_some() {}
    let _ = tx.send(AnalysisEvent::Finished);
    Ok(())
}

async fn analyze_async(url: &str, output_directory: PathBuf) -> anyhow::Result<MediaInspection> {
    let normalized_url = normalize_url(url);
    let yt_dlp = find_on_path(executable_name("yt-dlp"))
        .ok_or_else(|| anyhow::anyhow!("yt-dlp is not installed yet; managed dependency installation is required"))?;
    let ffmpeg = find_on_path(executable_name("ffmpeg"))
        .ok_or_else(|| anyhow::anyhow!("FFmpeg is not installed yet; managed dependency installation is required"))?;
    let downloader = Downloader::builder(Libraries::new(yt_dlp.clone(), ffmpeg), output_directory).build().await?;

    let mut inspection = if is_playlist_url(&normalized_url) {
        let playlist = match downloader.fetch_playlist_infos(&normalized_url).await {
            Ok(playlist) => playlist,
            Err(_) => return analyze_raw(&yt_dlp, &normalized_url).await,
        };
        let items = playlist.available_entries().into_iter().map(|entry| MediaItem {
            id: entry.id.clone(), title: entry.title.clone(), url: entry.url.clone(),
            thumbnail: entry.thumbnail.clone(), duration_seconds: entry.duration,
            uploader: entry.uploader.clone(), availability: entry.availability.clone(), formats: vec![],
        }).collect::<Vec<_>>();
        MediaInspection {
            title: playlist.title, thumbnail: items.first().and_then(|item| item.thumbnail.clone()),
            uploader: playlist.uploader, is_playlist: true, total_items: items.len(), items,
        }
    } else {
        let video = match downloader.fetch_video_infos(&normalized_url).await {
            Ok(video) => video,
            Err(_) => return analyze_raw(&yt_dlp, &normalized_url).await,
        };
        let item = video_item(&video, &normalized_url);
        MediaInspection {
            title: video.title.clone(), thumbnail: video.thumbnail.clone(), uploader: video.channel.clone().or(video.uploader.clone()),
            is_playlist: false, total_items: 1, items: vec![item],
        }
    };
    cache_thumbnail(&mut inspection).await;
    Ok(inspection)
}

async fn analyze_raw(executable: &Path, url: &str) -> anyhow::Result<MediaInspection> {
    let output = tokio::process::Command::new(executable)
        .args(["--dump-single-json", "--flat-playlist", "--ignore-errors", url])
        .output().await?;
    if !output.status.success() { return Err(anyhow::anyhow!(String::from_utf8_lossy(&output.stderr).trim().to_owned())); }
    let value: Value = serde_json::from_slice(&output.stdout)?;
    let entries = value.get("entries").and_then(Value::as_array);
    let is_playlist = entries.is_some();
    let mut items = if let Some(entries) = entries {
        entries.iter().filter(|entry| entry.get("id").and_then(Value::as_str).is_some()).map(raw_item).collect()
    } else { vec![raw_item(&value)] };
    items.retain(|item| !matches!(item.availability.as_deref(), Some("private" | "premium_only" | "subscriber_only")));
    let mut inspection = MediaInspection {
        title: string(&value, "title").unwrap_or_else(|| "Untitled".into()),
        thumbnail: string(&value, "thumbnail").or_else(|| items.first().and_then(|item| item.thumbnail.clone())),
        uploader: string(&value, "channel").or_else(|| string(&value, "uploader")),
        is_playlist, total_items: items.len(), items,
    };
    cache_thumbnail(&mut inspection).await;
    Ok(inspection)
}

fn raw_item(value: &Value) -> MediaItem {
    let formats = value.get("formats").and_then(Value::as_array).into_iter().flatten().filter_map(|format| {
        let id = string(format, "format_id")?;
        let video_codec = non_none_string(format, "vcodec");
        let audio_codec = non_none_string(format, "acodec");
        Some(MediaFormat {
            id, label: string(format, "format").unwrap_or_default(), extension: string(format, "ext").unwrap_or_default(),
            resolution: string(format, "resolution").or_else(|| format.get("height").and_then(Value::as_u64).map(|height| format!("{height}p"))),
            video_codec: video_codec.clone(), audio_codec: audio_codec.clone(),
            size: format.get("filesize").or_else(|| format.get("filesize_approx")).and_then(Value::as_u64),
            has_video: video_codec.is_some(), has_audio: audio_codec.is_some(),
        })
    }).collect();
    let id = string(value, "id").unwrap_or_default();
    let raw_url = string(value, "webpage_url").or_else(|| string(value, "url")).unwrap_or_default();
    let url = if raw_url.starts_with("http") { raw_url } else if !id.is_empty() { format!("https://www.youtube.com/watch?v={id}") } else { raw_url };
    MediaItem {
        id, title: string(value, "title").unwrap_or_else(|| "Unavailable item".into()), url,
        thumbnail: string(value, "thumbnail"), duration_seconds: value.get("duration").and_then(Value::as_f64),
        uploader: string(value, "channel").or_else(|| string(value, "uploader")), availability: string(value, "availability"), formats,
    }
}

fn string(value: &Value, key: &str) -> Option<String> { value.get(key).and_then(Value::as_str).filter(|v| !v.is_empty()).map(str::to_owned) }
fn non_none_string(value: &Value, key: &str) -> Option<String> { string(value, key).filter(|value| value != "none") }

async fn cache_thumbnail(inspection: &mut MediaInspection) {
    let Some(url) = inspection.thumbnail.clone().or_else(|| inspection.items.first().and_then(|item| {
        (!item.id.is_empty()).then(|| format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", item.id))
    })) else { return; };
    let Ok(response) = reqwest::get(&url).await else { return; };
    let extension = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok())
        .map(|kind| if kind.contains("webp") { "webp" } else if kind.contains("png") { "png" } else { "jpg" }).unwrap_or("jpg");
    let Ok(bytes) = response.bytes().await else { return; };
    let directory = std::env::temp_dir().join("silvadl-thumbnails");
    if tokio::fs::create_dir_all(&directory).await.is_err() { return; }
    let safe_id = inspection.items.first().map(|item| item.id.replace(['/', '\\'], "_")).unwrap_or_else(|| "preview".into());
    let path = directory.join(format!("{safe_id}.{extension}"));
    if tokio::fs::write(&path, bytes).await.is_ok() { inspection.thumbnail = Some(path.to_string_lossy().into_owned()); }
}

async fn cache_item_thumbnails(inspection: &MediaInspection, tx: Sender<AnalysisEvent>) {
    let directory = std::env::temp_dir().join("silvadl-thumbnails");
    if tokio::fs::create_dir_all(&directory).await.is_err() { return; }
    let client = reqwest::Client::new();
    let limit = Arc::new(tokio::sync::Semaphore::new(24));
    let mut jobs = tokio::task::JoinSet::new();
    for item in &inspection.items {
        let Some(url) = item.thumbnail.clone().or_else(|| (!item.id.is_empty()).then(|| format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", item.id))) else { continue; };
        let id = item.id.clone();
        let path = directory.join(format!("item-{}.jpg", id.replace(['/', '\\'], "_")));
        let client = client.clone();
        let limit = limit.clone();
        jobs.spawn(async move {
            if path.is_file() { return Some((id, path)); }
            let _permit = limit.acquire_owned().await.ok()?;
            let response = client.get(url).send().await.ok()?.error_for_status().ok()?;
            let bytes = response.bytes().await.ok()?;
            tokio::fs::write(&path, bytes).await.ok()?;
            Some((id, path))
        });
    }
    while let Some(result) = jobs.join_next().await {
        if let Ok(Some((id, path))) = result {
            let _ = tx.send(AnalysisEvent::ThumbnailCached { id, path: path.to_string_lossy().into_owned() });
        }
    }
}

async fn cache_one_thumbnail(item: &MediaItem, client: reqwest::Client, limit: Arc<tokio::sync::Semaphore>, directory: PathBuf) -> Option<(String, PathBuf)> {
    let url = item.thumbnail.clone().or_else(|| (!item.id.is_empty()).then(|| format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", item.id)))?;
    let id = item.id.clone();
    let path = directory.join(format!("item-{}.jpg", id.replace(['/', '\\'], "_")));
    if path.is_file() { return Some((id, path)); }
    let _permit = limit.acquire_owned().await.ok()?;
    let response = client.get(url).send().await.ok()?.error_for_status().ok()?;
    let bytes = response.bytes().await.ok()?;
    tokio::fs::write(&path, bytes).await.ok()?;
    Some((id, path))
}

fn video_item(video: &Video, fallback_url: &str) -> MediaItem {
    let formats = video.formats.iter().filter(|format| {
        format.download_info.url.is_some() && !format.format_type().is_storyboard()
    }).map(|format| MediaFormat {
        id: format.format_id.clone(), label: format.format.clone(), extension: format.download_info.ext.to_string(),
        resolution: format.video_resolution.resolution.clone(), video_codec: format.codec_info.video_codec.clone(),
        audio_codec: format.codec_info.audio_codec.clone(),
        size: format.file_info.filesize.or(format.file_info.filesize_approx).and_then(|v| u64::try_from(v).ok()),
        has_video: format.is_video(), has_audio: format.is_audio(),
    }).collect();
    MediaItem {
        id: video.id.clone(), title: video.title.clone(),
        url: video.webpage_url.clone().unwrap_or_else(|| fallback_url.to_owned()),
        thumbnail: video.thumbnail.clone(), duration_seconds: video.duration.map(|v| v as f64),
        uploader: video.channel.clone().or(video.uploader.clone()), availability: video.availability.clone(), formats,
    }
}

fn is_playlist_url(url: &str) -> bool { url.contains("/playlist?") || (!url.contains("/watch?") && url.contains("list=")) }
fn normalize_url(url: &str) -> String {
    let value = url.trim();
    if value.starts_with("?list=") { format!("https://www.youtube.com/playlist{value}") }
    else if value.starts_with("list=") { format!("https://www.youtube.com/playlist?{value}") }
    else { value.to_owned() }
}
fn executable_name(name: &str) -> &str { if cfg!(windows) && name == "yt-dlp" { "yt-dlp.exe" } else if cfg!(windows) && name == "ffmpeg" { "ffmpeg.exe" } else { name } }
fn find_on_path(name: &str) -> Option<PathBuf> {
    let direct = Path::new(name);
    if direct.components().count() > 1 && direct.is_file() { return Some(direct.to_owned()); }
    env::var_os("PATH").and_then(|paths| env::split_paths(&paths).map(|directory| directory.join(name)).find(|candidate| candidate.is_file()))
}
