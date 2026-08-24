# Silvapi - Pure C & OpenGL Frontend

A high-performance, lightweight, hardware-accelerated API client written in **pure C (C11)** with **modern OpenGL (3.3 Core)** rendering and **libcurl** asynchronous networking.

Matches the complete UI/UX, aesthetics, and workflow of Silvapi with zero web bloat, sub-millisecond startup, and low memory usage.

---

## Features

- **2D OpenGL Batch Renderer**: Dynamic vertex batching, signed distance field (SDF) smooth anti-aliased rounded rectangles, scissor clipping stack, and procedural vector icons.
- **TrueType Font Engine**: Glyph atlas generation via `stb_truetype` with support for system UI and monospace fonts (`Hack`, `Adwaita`, `Droid Sans`).
- **Full Theme System**: Pixel-accurate color schemes for:
  - Ayu Dark (default `#0D1016`)
  - Ayu Light
  - Catppuccin Mocha
  - Gruvbox Dark
- **Multi-Tab Request Strip**: Reorderable tab strip with HTTP method pills (`GET`, `POST`, `PUT`, `DELETE`, etc.), dirty indicators, and close buttons.
- **Request Builder**:
  - HTTP Method selector dropdown
  - URL bar with dynamic variable interpolation (`{{baseUrl}}`, `{{$guid}}`, `{{$timestamp}}`, `{{$randomInt}}`) and variable pill highlighting
  - Sub-tabs: **Body** (No Body, JSON, Form Data, URL Encoded, Raw, Binary File), **Params** (Query & Path), **Headers**, **Auth** (Bearer, Basic, API Key, AWS Sig v4, JWT, OAuth 2.0), and **Info**
  - Interactive JSON Code Editor with line numbering and real-time syntax token highlighting
- **Response Inspector**:
  - Status code badge (`200 OK`, `201 Created`, `404 Not Found`, `500 Error`)
  - Execution latency (`ms` / `s`) & payload size (`B`, `KB`, `MB`)
  - Response Code viewer with line numbers & syntax highlighting
  - **Response Table View**: Searchable columns, row selection, **Copy CSV**, and bottom resizable cell inspector with Markdown preview
  - **Timeline Tab**: Detailed waterfall bar chart breaking down DNS Lookup, TCP Connect, TLS Handshake, TTFB, and Content Download
  - **Response History**: Dropdown popover to view and restore snapshots of previous executions
- **SQLite Persistence**: Automatic workspace sync, settings persistence, and history storage in `~/.local/share/silvapi/silvapi.sqlite`.
- **Import & Export**:
  - OpenAPI 3.0 / Swagger JSON import
  - Postman Collection v2 / v2.1 JSON import
  - Export request to `cURL` command (and copy to clipboard)
- **Command Palette (`Ctrl + P`)**: Quick search modal across all collections, folders, and endpoints with breadcrumb navigation.

---

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl + Enter` | Send active request |
| `Ctrl + P` | Open Command Palette / Quick Search API |
| `Ctrl + W` | Close active tab |
| `Ctrl + S` | Save workspace to SQLite database |
| `Ctrl + ,` | Open Appearance & Settings dialog |

---

## Building and Running

### Prerequisites
- GCC or Clang (supporting C11)
- OpenGL 3.3+ runtime
- GLFW 3
- libcurl
- SQLite3

### Build with Make
```bash
cd c-frontend
make
./bin/silvapi-c
```

### Build with CMake
```bash
cd c-frontend
mkdir build && cd build
cmake ..
make
./silvapi-c
```

---

## Project Structure

```
c-frontend/
├── bin/
│   └── silvapi-c             # Compiled executable
├── include/
│   ├── silvapi.h             # Core types, vectors, rects, color math
│   ├── theme.h               # Theme palettes (Ayu, Catppuccin, Gruvbox)
│   ├── model.h               # API models, collections, workspaces, JSON serializers
│   ├── gl_backend.h          # 2D OpenGL batch renderer & vector icons
│   ├── font.h                # stb_truetype font rasterizer and atlas
│   ├── json_utils.h          # JSON tokenizer, formatter, table parser, CSV exporter
│   ├── storage.h             # SQLite storage and history snapshots
│   ├── http_client.h         # Asynchronous libcurl HTTP dispatch & timings
│   ├── import_export.h       # OpenAPI, Postman, and cURL converters
│   └── ui.h                  # Complete immediate-mode UI widget suite
├── src/
│   ├── theme.c
│   ├── model.c
│   ├── gl_backend.c
│   ├── font.c
│   ├── json_utils.c
│   ├── storage.c
│   ├── http_client.c
│   ├── import_export.c
│   ├── ui.c
│   ├── views/
│   │   ├── views.h
│   │   ├── top_bar.c         # Title, OpenAPI/Postman import, Variables, Settings
│   │   ├── sidebar.c         # Search, Collections tree, Method badges, Status badges
│   │   ├── request_view.c    # Tabs, Method dropdown, URL, Body, Params, Headers, Auth
│   │   ├── response_view.c   # Status, Code editor, Data table, CSV export, Timeline
│   │   └── dialogs.c         # Modals for Variables, Settings, Command Palette, cURL
│   └── main.c                # GLFW window, event dispatch, and layout orchestration
├── vendor/
│   ├── cJSON.h / cJSON.c
│   ├── stb_truetype.h
│   └── GLFW / curl headers
├── Makefile
└── CMakeLists.txt
```
