# SilvaDL

SilvaDL is a GPUI Component desktop frontend for `yt-dlp`. It supports video,
audio, individual URLs, playlists, parallel playlist workers, and concurrent
DASH/HLS fragments.

## Requirements

- `yt-dlp` on `PATH`, or its executable path entered in the application.
- FFmpeg on `PATH` for stream merging, remuxing, audio conversion, embedded
  thumbnails, metadata, and subtitles. A custom location can be entered.

## Run

```sh
cargo run -p silvadl
```

Application defaults and available choices are defined in
`resources/default-config.json`. Layout values are defined in
`resources/design-tokens.json`. User changes are saved to
`$XDG_CONFIG_HOME/silvadl/config.json` (or the platform-equivalent config
directory).

## Parallelism

`playlist_workers` controls how many playlist entries run in parallel.
`concurrent_fragments` is passed to yt-dlp's `--concurrent-fragments` option for
parallel DASH/HLS fragment downloads inside every worker. Raising both values
can create many simultaneous network connections.
