# Operating llmo in production

This guide covers deploying, securing, observing and maintaining llmo for a
team or an organisation. For a personal machine, double-clicking the binary
(or running `llmo`) is enough.

## Deployment options

| Option | When | How |
|---|---|---|
| Single binary | Workstations, small servers | Download a release archive, verify its `.sha256`, run `llmo serve` |
| systemd | Linux servers | [`deploy/llmo.service`](../deploy/llmo.service), hardened (no new privileges, read-only system, private tmp) |
| Docker | Containers, Kubernetes | `docker run -e LLMO_API_KEY=… -p 8765:8765 -v llmo:/data ghcr.io/<owner>/llmo` |

The container runs as uid 10001, writes only to `/data`, logs JSON, and has a
`HEALTHCHECK` (`llmo healthcheck`). It **refuses to start without an API key**
because it binds `0.0.0.0`.

**Kubernetes**: use `/health` as the liveness probe and `/ready` as the
readiness probe. `/ready` returns 503 while draining after SIGTERM, so set
`terminationGracePeriodSeconds` ≥ 35 (llmo drains for up to 30 s). Keep
`replicas: 1` per memory database: SQLite is single-writer, so scale out by
giving each team its own instance and volume.

**TLS**: llmo serves plain HTTP. Put it behind a reverse proxy (Caddy, nginx,
Traefik, an ingress) for TLS when it is reachable from other machines, and
pass the `x-request-id` header through.

## Configuration

Precedence: command-line flag > environment variable > config file > default.

| Env var | Meaning |
|---|---|
| `LLMO_CONFIG` | Config file path (default `~/.config/llmo/config.toml`, `%APPDATA%\llmo\config.toml` on Windows) |
| `LLMO_BIND` | Listen address |
| `LLMO_API_KEY` | Single admin API key |
| `LLMO_DB_PATH` | Memory database path |
| `LLMO_LOG` | Log filter, e.g. `llmo=debug`, `llmo::access=warn` |
| `LLMO_LOG_FORMAT` | `text` or `json` |

The config is validated at start-up (duplicate names, unknown fields, short
keys, user keys without namespaces, runtimes over the memory budget). An
invalid config stops the process with a clear message instead of running half
configured.

## Authentication and authorization

- **Local mode** (no keys): every caller is an anonymous admin, and llmo only
  allows loopback binds. Overriding that takes
  `server.allow_unauthenticated_remote = true`; don't.
- **Keys**: `Authorization: Bearer <key>`. Generate keys with
  `llmo key new --name <who> [--role admin] [--namespace ns]…`. The config
  stores only the SHA-256 (`key_sha256`), and comparison is constant-time.
  Rotate a key by adding the new entry, rolling clients over, then removing the
  old entry and restarting.
- **Roles**:
  - `admin` can do everything: runtime start/stop, reindex, forget-all, any
    namespace.
  - `user` can chat and manage memory only in its `namespaces`. Exact names or
    prefixes like `team-*` are allowed.
  - Memories in a namespace the key cannot access return 404, so their
    existence isn't revealed.
- `GET /llmo/whoami` shows the caller's identity.

## Observability

- **Request IDs**: every response carries `x-request-id`. A valid incoming
  value is reused, so IDs can be traced through proxies and clients. Every log
  line for a request includes it.
- **Access log**: target `llmo::access`, with method, path, matched route,
  status, latency (ms) and principal.
- **Audit log**: target `llmo::audit`. It records runtime start/stop, memory
  deletes and forgets, reindexes, permission denials and failed
  authentication (with client IP). Route it to a separate sink with
  `LLMO_LOG_FORMAT=json` and filter on `target`.
- **Metrics**: `GET /metrics` in Prometheus format. It requires a valid key
  when auth is on.

  | Metric | Type |
  |---|---|
  | `llmo_http_requests_total{method,route,status}` | counter |
  | `llmo_http_request_duration_seconds{route}` | histogram |
  | `llmo_http_inflight_requests` | gauge |
  | `llmo_http_rejected_total` (load shed) | counter |
  | `llmo_auth_failures_total` | counter |
  | `llmo_runtime_up{runtime,managed}`, `llmo_runtime_inflight{runtime}` | gauge |
  | `llmo_runtime_events_total{runtime,event=start\|start_failure\|eviction\|idle_stop}` | counter |
  | `llmo_memories`, `llmo_memories_recalled_total`, `llmo_memories_stored_total` | gauge / counter |
  | `llmo_build_info{version}` | gauge |

  Suggested alerts:
  - `llmo_runtime_up == 0` for 5 minutes
  - `rate(llmo_http_rejected_total[5m]) > 0`
  - `rate(llmo_auth_failures_total[5m]) > 1`
  - p95 latency of `/v1/chat/completions` above your SLO

## Reliability

- **Load shedding**: at most `max_concurrent_requests` `/v1/*` requests are
  served at once, with streams counted until they finish. Extra requests get an
  immediate `503` with `Retry-After: 1` instead of piling up and exhausting
  RAM/VRAM.
- **Timeouts**: connections to runtimes time out after 5 s. Non-streaming
  generations have a total limit (`upstream_timeout_secs`). Streams fail if a
  runtime goes silent for `stream_idle_timeout_secs`.
- **Runtime supervision**: managed runtimes are health-checked on start, their
  stderr tail is included in start errors, and they are evicted
  least-recently-used first under memory pressure and stopped when idle. A
  runtime in use is never evicted.
- **Graceful shutdown**: on SIGTERM or Ctrl+C, `/ready` flips to 503, new
  connections stop, in-flight requests get up to 30 s, and then managed
  runtimes are stopped.

## Data

- **Location**: `memory.db` (SQLite, WAL mode) in the platform data directory
  or `LLMO_DB_PATH`, created with mode `0600` on Unix.
- **Backups**: `llmo memory backup /backups/llmo-$(date +%F).db` takes a
  consistent online copy (`VACUUM INTO`) while the server is running and never
  overwrites an existing file. Restore by stopping llmo and replacing
  `memory.db` (delete the `-wal` and `-shm` files) with the backup. For
  portable exports, use `llmo memory export > dump.jsonl` and
  `llmo memory import`.
- **Schema migrations**: the schema version is stored in `PRAGMA user_version`.
  Upgrades migrate forward automatically in a transaction. An older binary
  refuses to open a newer database instead of corrupting it. Take a backup
  before upgrading.
- **Privacy**: emails, API keys, tokens, JWTs and private keys are redacted
  before anything is stored, and assistant replies aren't stored unless
  `memory.store_assistant = true`. Users can delete by id, session or
  namespace. Encryption at rest is delegated to the disk (BitLocker,
  FileVault, LUKS).

## Built-in model runner

llmo can download GGUF models from Hugging Face and run them with a
llama.cpp build it installs itself (the **Models** tab, `/llmo/hub/*`,
`llmo model …`).

- **Who can use it**: downloading, installing the engine, starting,
  stopping and deleting models need an **admin** key. Searching and listing
  are open to any key. Every action is audited (`model_download`,
  `engine_install`, `model_run`, `model_delete`, …). Chatting with a
  downloaded model starts it on demand for any user allowed to chat.
- **Integrity**: model files are checked against the SHA-256 that Hugging
  Face publishes for LFS objects. Engine archives are checked against the
  digest GitHub publishes for release assets. A mismatch deletes the partial
  file and fails the job. Downloads resume after interruptions (HTTP Range)
  and retry with backoff. Free disk space is checked first. Unsafe paths in
  repositories or archives are rejected.
- **Choosing the engine build**: llmo picks one automatically (NVIDIA on
  Windows → CUDA 12 build plus its runtime; Apple Silicon → Metal; otherwise
  CPU, or Vulkan on Linux with NVIDIA). Override with `engine.backend`, pin
  the version with `engine.llama_cpp_version`, or point
  `engine.llama_server` at a binary you provide.
- **Corporate networks**: downloads honour the OS proxy settings and
  `HTTPS_PROXY`/`NO_PROXY`, and trust the OS certificate store. Add
  `engine.ca_bundle` for a TLS-inspecting proxy's root CA.
- **Mirrors and air-gapped sites**: set `engine.hf_endpoint` to an internal
  Hugging Face mirror (e.g. an Artifactory "huggingface" remote) and
  `engine.github_api` to a mirror of the GitHub releases API. To allow no
  downloads at all, set `engine.downloads = false`, set
  `engine.llama_server`, and place model files plus a manifest under
  `models_dir` (or run `llmo model pull` on a connected machine and copy the
  `models` directory across).
- **Storage**: `<data dir>/llmo/models/<owner>__<repo>/` (files) and
  `models/manifests/*.json` (registry), `<data dir>/llmo/engine/<tag>-<backend>/`.
  Change them with `engine.models_dir` and `engine.engine_dir`.
- **Resources**: each model's memory claim (weights, KV cache and buffers)
  counts against `[budget] memory_mb`. The default `0` means automatic:
  detected RAM minus an OS reserve, plus GPU VRAM. Idle models stop after
  `engine.idle_timeout_secs`, and under memory pressure the least recently
  used idle model is stopped first.

## Upgrades

1. `llmo memory backup <file>`
2. Replace the binary or image.
3. Restart. Migrations run automatically, and `/ready` reports
   `schema_version`.
4. To roll back, restore the backup and run the previous binary.

## Known limits

- One process per memory database, because SQLite has a single writer. This
  is fine for thousands of users chatting, not for a multi-region cluster.
- Memory search is brute force over a namespace. It's fast up to roughly
  100k memories per namespace; beyond that, split namespaces.
- The built-in `hash` embedder matches words, not meaning. Configure a
  `runtime` embedder (for example `nomic-embed-text`) for semantic recall.
- Planner speeds are estimates, typically within about ±25%.
