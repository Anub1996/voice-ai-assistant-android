# Changelog

## 0.3.0

### Built-in model runner: run Hugging Face models with nothing else installed
- **Models** tab:
  - Recommended models for this PC
  - Hugging Face search, with fit and speed shown for every quantization
  - One-click **Download & run**, live progress, speed, ETA and cancel
  - My models: start, stop, chat, delete
- Downloads and installs the official llama.cpp build for the machine (CUDA,
  Metal, Vulkan or CPU) on first use. It is verified against GitHub's
  SHA-256, and Windows CUDA builds include their runtime.
- Model downloads are resumable and retried, and verified against Hugging
  Face's SHA-256. Split GGUFs are supported, vision projectors are skipped,
  and free disk space is checked first.
- Downloaded models become managed runtimes on automatically chosen ports.
  They appear in `/v1/models`, start on first chat, stop when idle and are
  evicted LRU-first under memory pressure.
- GGUF header reader that gives the planner exact layer, KV-cache and
  mixture-of-experts numbers for any downloaded model.
- CLI: `llmo model search|info|pull|list|rm`, `llmo engine status|install`.
- Enterprise settings: Hugging Face mirror (`HF_ENDPOINT`), token, GitHub
  API mirror, version pinning, a configured binary, extra CA bundle, OS
  proxy support, `engine.downloads = false` for air-gapped sites. All
  actions are admin-only and audited.

### Other
- The memory budget defaults to automatic (detected RAM plus VRAM).
- Model loading no longer blocks other runtimes from starting or stopping,
  and a runtime can be stopped while it loads.
- Truncated downloads are retried instead of failing.


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
