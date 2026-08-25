Intentionally empty until Phase 2.

The previous `http_client.[ch]` moved to `app/` because it takes `ApiRequest`,
`Workspace`, `Collection`, and `Folder` directly — it is application code, not a
library. `libs/net` starts clean so the rule in docs/ARCHITECTURE.md holds from
day one: net/ depends on base/ and ds/ only, and must link into a headless binary.

Phase 2 lands: reactor.h (epoll), timerwheel, slab handle table, SPSC rings,
the NetEvent model, and backend_curl.c driven via curl_multi_socket_action.
See docs/NET.md.
