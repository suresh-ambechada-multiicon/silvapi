# `libs/net` — Async HTTP Runtime

Tokio-inspired runtime, written in C, sized for a desktop API client:
**~100 concurrent transfers**, per-request progress, timings, and streaming output.

## What we own vs what we delegate

**We own the runtime.** Reactor, scheduler, timers, handle table, channels, backpressure,
connection admission. This is ~2.5k LOC and every hot path is ours.

**We delegate the protocol.** libcurl is driven through `curl_multi_socket_action` — the API
that exists so an application's own event loop drives libcurl. libcurl registers its sockets
with us via `CURLMOPT_SOCKETFUNCTION` and its timeouts via `CURLMOPT_TIMERFUNCTION`; our
reactor calls back in. libcurl never runs an event loop; it is a protocol state machine.

Rationale: HTTP/1.1 chunked+trailers is tractable (~1k LOC), but HTTP/2 is HPACK + flow
control + stream state, and TLS 1.3 is a CVE farm. Writing our own TLS is not on the table.
Delegating buys HTTP/1.1 + HTTP/2 multiplexing + HTTP/3 + TLS + brotli/zstd +
Happy Eyeballs (RFC 8305) from a stack that is already a dependency.

`net.h` exposes zero curl types, so `backend_h1.c` (picohttpparser + OpenSSL) can replace
`backend_curl.c` later without the app noticing.

## Threading

```
UI thread                          IO thread                    CPU pool (work-stealing)
    |                                  |                                |
    |--- cmd ring (SPSC) ------------->|  reactor: epoll_wait           |
    |                                  |  timers:  wheel tick           |
    |                                  |  admit:   <=8/host, <=64 total  |
    |                                  |                                |
    |<-- event ring (SPSC) ------------|  HEADERS/CHUNK/PROGRESS/DONE   |
    |    drained in ui_begin_frame     |                                |
    |                                  |---- json format / decompress ->|
    |<---------------------------------|<--- result -------------------|
```

**One IO thread, not a work-stealing IO pool.** Work-stealing solves cross-core contention
across thousands of tasks; a single epoll thread handles 10k+ connections and 100
network-bound transfers do not saturate it. Work-stealing is used where it actually pays:
the CPU pool, for pretty-printing large JSON, decompression, and syntax tokenizing.

**epoll, not io_uring.** Benchmarks are workload-dependent: io_uring wins request-response
ping-pong but is slower on streaming, and at 10-100 concurrent clients the two are roughly
equal. HTTP bodies stream. `reactor.h` is abstracted so io_uring can be added and *measured*.

## Handles

```c
typedef struct { uint32_t index; uint32_t generation; } NetHandle;
```

Slot reuse bumps `generation`. A stale handle is detectably stale, not a use-after-free.
This is what today's `http_client_cancel(const char* request_id)` should have been —
it currently sets an `is_cancelled` flag that nothing ever reads.

## Event model

```c
typedef enum {
    NET_EV_QUEUED, NET_EV_DNS_DONE, NET_EV_CONNECTED, NET_EV_TLS_DONE,
    NET_EV_REQ_SENT, NET_EV_HEADERS, NET_EV_BODY_CHUNK,
    NET_EV_PROGRESS, NET_EV_DONE, NET_EV_ERROR, NET_EV_CANCELLED,
} NetEventKind;

typedef struct {
    double dns_ms, tcp_ms, tls_ms, ttfb_ms, download_ms, total_ms;
} NetTimings;
```

- **Backpressure**: rings are bounded. When the event ring is full the IO thread *coalesces*
  `PROGRESS` (keep only the latest per request) instead of blocking. Progress is a sample,
  not a log — dropping intermediate samples is correct. `HEADERS`/`DONE`/`ERROR` are never
  dropped; if those cannot be enqueued the IO thread parks on an eventfd until the UI drains.
- **Ownership**: the body accumulates in the request slot's arena. `NET_EV_DONE` transfers
  the pointer to the UI thread, which owns and frees it. No memory is touched by both threads.
- **Streaming**: `NET_EV_BODY_CHUNK` fires as bytes arrive, so a large response renders
  progressively instead of appearing all at once at the end.

## Admission & pooling

FIFO queue with a priority lane: the request the user just clicked preempts a batch run.
Caps are `max_per_host` (default 8) and `max_inflight` (default 64), both configurable.
Connection reuse and HTTP/2 multiplexing come from the shared `CURLM` connection cache.

## Bug inventory this replaces

Current `src/http_client.c`:

1. `task->callback(&resp, ...)` runs on the **worker thread** and `on_http_response_received`
   writes `g_app` directly while the render thread reads it. Data race.
2. `task->ws / col / folder` are borrowed raw pointers into UI-thread-mutable state. UAF.
3. `http_client_cancel` sets `is_cancelled`; nothing reads it. Cancel does not work.
4. Thread per request (`pthread_create` + `pthread_detach`), capped at 32. No connection
   reuse across requests — every request pays DNS + TCP + TLS again.
5. `char final_url[4096]` — silent truncation on long query strings.
6. No streaming: the whole body buffers before the UI sees anything.
