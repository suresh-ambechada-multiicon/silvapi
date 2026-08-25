# Silvapi C Frontend — Architecture & Roadmap

## Goal

Turn `c-frontend/` from an 11k-line monolith into a **set of independent C libraries**
plus a thin application layer, so that:

1. A future frontend can `#include <ui/ui.h>` and build a new app without touching app code.
2. Each library depends only *downward*. No cycles. No library knows about Silvapi.
3. Text, icons, and animation are real subsystems with their own files — not scattered helpers.
4. The network layer is a standalone async runtime usable outside a GUI entirely.

Non-goals: rewriting protocol stacks (HTTP/2, TLS), matching GPUI feature-for-feature
before the foundation is right.

---

## Layering

```
                        app/          (silvapi: model, views, import/export, main)
                          |
   +--------+--------+----+----+--------+--------+
   |        |        |         |        |        |
  ui/     text/    icons/    gfx/     net/     json/
   |        |        |         |        |        |
   +--------+--------+----+----+--------+--------+
                          |
                         ds/          (ring, slab, timer wheel)
                          |
                        base/         (arena, String8, log, os, atomics, geom)
```

Hard rules:

- `base/` has zero dependencies beyond libc + pthreads.
- `net/` depends on `base/` + `ds/` ONLY. It must link into a headless binary.
  If `net/` ever includes a GUI header, the layering is broken.
- `gfx/` knows nothing about icons. It draws textured quads. `icons/` owns `IconType`.
  (Today `gl_backend.h` declares `IconType` and `r_draw_icon` — a layering violation to fix.)
- `text/` owns all shaping, wrapping, alignment, and caret math. Nothing else measures text.
- `app/` is the only layer allowed to know what an "API request" is.

---

## Library responsibilities

### `base/` — foundation
`arena.h` (arena + String8, already exists), `log.h`, `os.h` (fs/time/thread/mutex/cond),
`atomic.h` (C11 atomics wrappers), `geom.h` (Vec2/Rect/Color — moved out of `silvapi.h`).

### `ds/` — data structures
- `ring.h` — Vyukov bounded SPSC ring (cache-line-padded head/tail, acquire/release, no CAS)
  and a bounded MPMC ring for the CPU pool.
- `slab.h` — generation-counted slab. Handles are `{u32 index, u32 generation}` so a stale
  handle is *detectably* stale instead of a use-after-free.
- `timerwheel.h` — 4-level hierarchical timing wheel. O(1) insert/cancel/tick.
  Chosen over a binary heap because cancel is the hot path: every request arms a timeout
  that is normally cancelled on completion.

### `net/` — async HTTP runtime (see NET.md)
Own reactor, own scheduler, own timers, own channels. libcurl is driven via
`curl_multi_socket_action` as a passive protocol state machine, hidden behind the interface.

### `text/` — text engine
`fontdb.c` (fontconfig discovery + fallback chain), `font.c` (FreeType face + atlas),
`shape.c` (HarfBuzz + **shaped-run cache** — today `hb_buffer_create()` runs per draw call),
`layout.c` (UTF-8-correct line breaking, alignment, ellipsis, caret hit-test).

Correctness bug being fixed: wrapping currently sums `font_measure_char` per byte, which
ignores kerning and shaping, so wrap width != draw width. Layout must measure via shaped runs.

### `gfx/` — 2D renderer
Batch renderer, SDF rounded rects, scissor stack, dynamic texture atlas allocator
(shelf-packing, replaces the fixed 64px icon grid and the ad-hoc font atlas cursor).

### `icons/` — icon system
nanosvg rasterization at the *requested* size into the shared atlas, keyed by (icon, px, dpi).
Today icons rasterize once into fixed 64px cells and blur at every other size.

### `ui/` — widgets, layout, animation, theme
Clay as the box-layout engine (flex/sizing/scroll/clip) wired into the frame loop —
it is currently vendored and `clay_renderer.c` is written but **never called**.
Clay never wraps text; we feed it measured blocks from `text/`.
Plus: `spring.c` (animation), `dock.c` (panel layout), `editbuf.c` (piece table),
retained widget state, input state.

### `json/`, `app/`
`json/` is standalone. `app/` holds model, storage, import/export, views, main.

---

## Phases

- **Phase 0** — Create the tree. Move files, namespace headers, fix includes. Zero behavior
  change. Build must stay green. *(mechanical, verifiable)*
- **Phase 1** — Break layering violations: `IconType` out of `gfx`, `geom.h` out of `silvapi.h`,
  `-Wall -Wextra -Werror` clean (26 warnings today).
- **Phase 2** — `net/`: reactor, timer wheel, slab, SPSC rings, event model, curl backend.
  Delete the worker-thread callback race and the borrowed-pointer UAF.
- **Phase 3** — `text/`: shaped-run cache, fontconfig + fallback chain, UTF-8 correct
  wrap/align/caret, ellipsis. Wrap width == draw width.
- **Phase 4** — `ui/`: wire Clay into the frame loop, port the five views off manual rect math.
- **Phase 5** — `icons/` + `gfx/`: dynamic atlas, DPI-correct icon rasterization.
- **Phase 6** — Animation pass: spring-driven transitions on real UI state.

Each phase ends with: builds clean, app runs, no regression.
