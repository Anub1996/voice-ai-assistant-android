# Changelog

## 0.2.0

### Usability
- Running `llmo` with no arguments (or double-clicking it) creates a config on
  first run, starts the server and opens the web UI. Errors keep the window
  open.
- If the port is taken by another llmo, llmo opens that one instead. If it's
  taken by a different program, llmo uses the next free port.
- `llmo chat` goes through a running server instead of launching a second
  copy of a managed runtime.
- Windows: hardware detection, no console window for managed runtimes.

### Security
- Named API keys with `admin`/`user` roles and namespace scopes
  (`[[server.keys]]`), stored as SHA-256 digests (`llmo key new`).
- Refuse to bind beyond loopback without authentication.
- Audit log (`llmo::audit`) for privileged actions, denials and failed
  authentication.
- Internal errors no longer expose details to clients.

### Reliability
- Load shedding (`max_concurrent_requests`, 503 + `Retry-After`).
- Upstream total and stream-idle timeouts.
- Graceful drain on SIGTERM/Ctrl+C (up to 30 s), with `/ready` flipping to 503.
- Managed runtimes that fail to launch now report `failed` with a hint.

### Observability
- `x-request-id` on every response, access log, JSON logs
  (`LLMO_LOG_FORMAT=json`).
- Prometheus `/metrics`, `/ready`, `llmo healthcheck`.

### Data
- Versioned schema with forward migrations. Refuses newer databases.
- `llmo memory backup` (online, consistent).
- Assistant replies aren't stored by default (`memory.store_assistant`).

### Packaging
- Dockerfile (non-root, healthcheck), hardened systemd unit, release workflow
  for Linux/macOS/Windows binaries and a GHCR image, CI on all three OSes with
  `cargo audit`.

## 0.1.0
- Initial release: OpenAI-compatible proxy with long-term memory, runtime
  orchestration, web UI and hardware-aware model planner.
