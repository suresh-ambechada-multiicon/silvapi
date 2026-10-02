use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind { Video, Audio }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DownloadConfig {
    pub yt_dlp_path: String,
    pub ffmpeg_path: Option<String>,
    pub output_directory: PathBuf,
    pub media_kind: MediaKind,
    pub video_container: String,
    pub audio_format: String,
    pub selected_format_id: Option<String>,
    pub resolution: Option<u16>,
    pub download_playlist: bool,
    pub playlist_workers: usize,
    pub concurrent_fragments: usize,
    pub retries: usize,
    pub embed_metadata: bool,
    pub embed_thumbnail: bool,
    pub download_subtitles: bool,
    pub use_download_archive: bool,
}

#[derive(Clone, Debug)]
pub enum DownloadEvent {
    QueueReady { items: usize },
    ItemStarted { worker: usize, url: String },
    ItemFinished { url: String, success: bool },
    ItemProgress { url: String, downloaded: u64, total: u64, speed: Option<f64>, eta: Option<u64> },
    ItemError { url: String, message: String },
    ItemCanceled { url: String },
    Log(String),
    Finished { succeeded: usize, failed: usize },
}

#[derive(Clone, Debug)]
pub struct MediaFormat {
    pub id: String,
    pub label: String,
    pub extension: String,
    pub resolution: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub size: Option<u64>,
    pub has_video: bool,
    pub has_audio: bool,
}

#[derive(Clone, Debug)]
pub struct MediaItem {
    pub id: String,
    pub title: String,
    pub url: String,
    pub thumbnail: Option<String>,
    pub duration_seconds: Option<f64>,
    pub uploader: Option<String>,
    pub availability: Option<String>,
    pub formats: Vec<MediaFormat>,
}

#[derive(Clone, Debug)]
pub struct MediaInspection {
    pub title: String,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    pub is_playlist: bool,
    pub total_items: usize,
    pub items: Vec<MediaItem>,
}
