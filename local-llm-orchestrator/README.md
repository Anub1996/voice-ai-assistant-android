# llmo — Local LLM Memory & Runtime Orchestrator

**One private, OpenAI-compatible endpoint for all your local models — with
long-term memory and on-demand runtimes. Built for local-AI power users and
privacy-focused developers.**

```
 your apps / OpenAI SDKs / editors / agents
                 │  http://127.0.0.1:8765/v1
                 ▼
 ┌───────────────────────────────────────────────┐
 │ llmo                                          │
 │  1. recall   relevant memories (SQLite, local)│
 │  2. inject   them into the system prompt      │
 │  3. route    model → runtime                  │
 │  4. launch   runtime if needed (RAM budget,   │
 │              LRU eviction, idle shutdown)     │
 │  5. forward  request, stream reply back       │
 │  6. remember the turn (secrets redacted)      │
 └───────────────────────────────────────────────┘
        │                │                 │
     Ollama        llama-server         vLLM / LM Studio / LocalAI
   (external)   (managed by llmo)      (anything OpenAI-compatible)
```

## Why

Running models locally usually means juggling several servers, ports and
model names, manually starting/stopping them to fit in VRAM, and losing all
context between sessions. `llmo` fixes that without sending a single byte off
your machine:

- **Persistent memory** — every conversation can recall relevant facts from
  earlier ones. Memories live in a single SQLite file (`0600` permissions)
  on your disk, partitioned by namespace, searchable, exportable, deletable.
- **Runtime orchestration** — route by model name to Ollama, llama.cpp
  `llama-server`, vLLM, LM Studio, LocalAI… Managed runtimes are launched on
  first use, evicted least-recently-used-first when the memory budget is
  tight, and stopped after an idle timeout.
- **Drop-in** — speaks the OpenAI Chat Completions API (including streaming),
  so existing clients just change their `base_url`.
- **Private by default** — binds to loopback, no telemetry, no cloud calls,
  the HTTP client ignores system proxies, the default embedder runs
  in-process with no model download, and emails/API keys/tokens/private keys
  are scrubbed before anything is written to disk.

## Run Hugging Face models with nothing else installed

Open the **Models** tab:

1. Pick a **Recommended for this PC** card, or search Hugging Face and choose
   a file. Each file shows whether it fits your RAM/VRAM and its estimated
   speed on your hardware.
2. Click **Download & run**. llmo downloads the GGUF (resumable, verified
   against Hugging Face's SHA-256), then on first use downloads the official
   **llama.cpp** engine build for your machine (CUDA for NVIDIA on Windows,
   Metal on Apple Silicon, Vulkan or CPU elsewhere, verified against
   GitHub's SHA-256) and starts it.
3. Chat. The model appears in the Chat model list and in `/v1/models` for
   any OpenAI client. It stops when idle and starts again on demand.

No Ollama, no llama.cpp install, no terminal. From the command line:

```sh
llmo model search qwen3
llmo model info unsloth/Qwen3-30B-A3B-GGUF     # every quant with fit + speed
llmo model pull unsloth/Qwen3-30B-A3B-GGUF     # best quant for this PC
llmo model list
llmo engine install                            # optional; happens automatically
```

For gated models (e.g. official Gemma/Llama repos), accept the license on
huggingface.co and set `HF_TOKEN` or `engine.hf_token`.

## Web UI

`llmo serve` also serves a UI at **http://127.0.0.1:8765/**: chat with
streaming and memory recall, a memory browser, runtime controls, and a
**"Run a big model" planner**. The page is one self-contained file; its
Content-Security-Policy blocks every request to other origins.

## Run a big model on modest hardware

```sh
llmo plan gemma-3-27b                 # uses detected RAM / CPU / GPU
llmo plan gemma-3-27b --gpu 16,288    # what if I add an RTX 4060 Ti 16GB?
llmo plan --list
```

The planner picks the best GGUF quantization that fits your RAM/VRAM,
estimates generation speed, and prints the exact `llama-server` command
(plus a speculative-decoding variant and an llmo config block). It is honest
about physics: generating a token reads every active weight once, so
**tokens/s ≈ memory bandwidth ÷ model size**.

| 27B-class goal on 16 GB          | Best fit            | Est. speed  |
|----------------------------------|---------------------|-------------|
| Gemma 3 27B, CPU, ~60 GB/s RAM   | IQ3_M, 11.5 GiB     | ~3 tok/s    |
| Gemma 3 27B, RTX 4060 Ti 16GB    | IQ3_XXS, on GPU     | ~21 tok/s   |
| Qwen3 30B-A3B (MoE), CPU only    | IQ3_XXS, 3B active  | ~28 tok/s   |

So 20 tok/s from a 27B *dense* model needs ~200–400 GB/s of bandwidth (a GPU
that holds the model, or an Apple M-series Pro/Max). On an ordinary 16 GB
laptop, a mixture-of-experts model gets you 30B-class quality at 20+ tok/s.

## Install

**Easiest:** download the binary for your OS from Releases and double-click
it (`llmo.exe` on Windows). It creates a config on first run, starts the
server, and opens the UI in your browser. Keep the window open; close it or
press Ctrl+C to stop. If something fails, the window stays open with the
error.

> Windows may show "Windows protected your PC" for unsigned downloads:
> click **More info → Run anyway**.

From a terminal:

```sh
llmo                 # same as double-clicking: serve + open the UI
llmo serve           # server only (services, containers)
llmo init            # write the fully commented example config
```

From source: `cargo install --path .` (Rust 1.88+; SQLite is bundled).
Cross-compile for Windows from Linux with
`cargo build --release --target x86_64-pc-windows-gnu` (needs
`gcc-mingw-w64-x86-64`).

**Production** (systemd, Docker/Kubernetes, API keys and roles, metrics,
audit log, backups, upgrades): see [docs/OPERATIONS.md](docs/OPERATIONS.md)
and [SECURITY.md](SECURITY.md).

## Configure

See [`config.example.toml`](config.example.toml). The essentials:

```toml
default_runtime = "ollama"

[budget]
memory_mb = 16000            # RAM/VRAM managed runtimes may claim at once

[[runtimes]]                 # external: you run it, llmo routes to it
name = "ollama"
base_url = "http://127.0.0.1:11434"
models = ["llama3.2:3b"]

[[runtimes]]                 # managed: llmo starts/stops it
name = "qwen"
base_url = "http://127.0.0.1:8081"
models = ["qwen2.5-7b-instruct"]
[runtimes.process]
command = "llama-server"
args = ["-m", "/models/qwen2.5-7b-instruct-q4_k_m.gguf", "--port", "8081"]
memory_mb = 6000
idle_timeout_secs = 600
```

**Model routing**, in order: an explicit `runtime/model` prefix
(`qwen/qwen2.5-7b-instruct`), a runtime's configured `models`, models a
runtime reports via `/v1/models`, `default_runtime`, the only runtime.

**Embeddings.** The default `hash` embedder is offline and model-free
(feature hashing of words, bigrams and character trigrams) — good at lexical
recall, zero setup. For semantic recall, point it at a real embedding model:

```toml
[memory.embedder]
kind = "runtime"
runtime = "ollama"
model = "nomic-embed-text"
```

then run `llmo memory reindex` to re-embed existing memories.

## Use

Any OpenAI client works:

```python
from openai import OpenAI
client = OpenAI(
    base_url="http://127.0.0.1:8765/v1",
    api_key="unused-unless-server.api_key-is-set",
    default_headers={"x-llmo-namespace": "work", "x-llmo-session": "laptop"},
)
client.chat.completions.create(model="llama3.2:3b",
    messages=[{"role": "user", "content": "What did we decide about the DB schema?"}])
```

```sh
curl -s http://127.0.0.1:8765/v1/chat/completions \
  -H 'content-type: application/json' -H 'x-llmo-namespace: work' \
  -d '{"model":"qwen2.5-7b-instruct","stream":true,
       "messages":[{"role":"user","content":"Summarise yesterday'\''s notes"}]}'
```

### Per-request memory headers

| Header              | Values                            | Default   |
|---------------------|-----------------------------------|-----------|
| `x-llmo-namespace`  | `[A-Za-z0-9._:@-]{1,128}`         | `default` |
| `x-llmo-session`    | same; tagged on stored turns      | none      |
| `x-llmo-memory`     | `on` · `off` · `recall` · `store` | `on`      |

Responses carry `x-llmo-runtime` (who served it) and `x-llmo-memories`
(how many memories were injected). Memory context is merged into your
existing system message (several chat templates reject a second one) and
never duplicates text already in the conversation.

### CLI

```sh
llmo runtimes                                   # status + health of every runtime
llmo chat -m llama3.2:3b -n work "what's our deploy process?"
llmo memory add --pin -t hardware "I run llama.cpp on an RTX 4090 (24GB)"
llmo memory search -n work "deploy process"
llmo memory list --limit 20
llmo memory forget --id a7081420               # id prefix from `list`
llmo memory forget -n work                      # or -s SESSION, or --all
llmo memory export > backup.jsonl && llmo memory import backup.jsonl
llmo memory stats
llmo memory reindex                             # after changing embedder
```

### HTTP API

| Method & path                          | Purpose                                  |
|----------------------------------------|------------------------------------------|
| `POST /v1/chat/completions`            | Orchestrated chat (JSON or SSE stream)   |
| `POST /v1/embeddings`                  | Routed embeddings passthrough            |
| `GET  /v1/models`                      | Models across all runtimes               |
| `GET  /llmo/runtimes?probe=true`       | Runtime status, models, in-flight, idle  |
| `POST /llmo/runtimes/{name}/start`     | Start a managed runtime                  |
| `POST /llmo/runtimes/{name}/stop`      | Stop a managed runtime                   |
| `POST /llmo/memory`                    | Add `{content, namespace?, tags?, pinned?}` |
| `GET  /llmo/memory?namespace&session&limit` | List                                |
| `POST /llmo/memory/search`             | `{query, namespace?, k?, min_score?}`    |
| `GET/DELETE /llmo/memory/{id}`         | Fetch / delete one                       |
| `DELETE /llmo/memory?namespace=…`      | Bulk delete (filter or `all=true` required) |
| `GET  /llmo/memory/stats`              | Counts per namespace                     |
| `POST /llmo/memory/reindex`            | Re-embed stale memories                  |
| `GET  /llmo/hardware`                  | Detected RAM, CPU, GPU, bandwidth        |
| `POST /llmo/plan`                      | `{model, hardware?, ctx?, target_tps?}`  |
| `GET  /llmo/plan/presets`              | Model presets and quantizations          |
| `GET  /`                               | Web UI                                   |
| `GET  /llmo/whoami`                    | Identity and role of the calling key     |
| `GET  /llmo/engine`                    | Built-in engine status and hardware      |
| `POST /llmo/engine/install`            | Install llama.cpp `{backend?}` (admin)   |
| `GET  /llmo/hub/search?q=`             | Search Hugging Face GGUF repos           |
| `GET  /llmo/hub/repo?id=owner/name`    | Quantizations with fit/speed estimates   |
| `GET  /llmo/hub/recommended`           | Best model choices for this machine      |
| `POST /llmo/hub/download`              | `{repo, choice?, quant?, run?}` (admin)  |
| `GET  /llmo/jobs`, `DELETE /llmo/jobs/{id}` | Download/install progress, cancel   |
| `GET  /llmo/local`                     | Downloaded models                        |
| `POST /llmo/local/{id}/run` · `/stop`, `DELETE /llmo/local/{id}` | Manage (admin) |
| `GET  /metrics`                        | Prometheus metrics                       |
| `GET  /ready`                          | Readiness (DB reachable, not draining)   |
| `GET  /health`                         | Liveness (never requires auth)           |

Set `server.api_key` (or `[[server.keys]]` with roles and namespace scopes)
to require `Authorization: Bearer <key>` on everything except `/`, `/health`
and `/ready`. Errors use the OpenAI shape: `{"error":{"message","type"}}`.

## How it works

- **Memory ranking** — cosine similarity against memories embedded by the
  *same* embedder, filtered by `min_score`, then weighted by recency
  (`0.5 + 0.5·2^(−age/half_life)`); pinned memories skip decay. Identical
  content in a namespace is de-duplicated. Injected context is capped by
  `max_context_chars`.
- **Storage** — user turns are stored; assistant replies only with
  `memory.store_assistant = true` (replies restate recalled memories, which
  would otherwise snowball). Turns shorter than 16 characters are skipped. Streaming replies are stored only if the client received them in
  full.
- **Lifecycle** — each request holds a *lease* on its runtime; leased
  runtimes are never evicted or idle-stopped. Starting a managed runtime
  waits for `/v1/models` to answer (up to `startup_timeout_secs`); if the
  process dies, the error includes the tail of its stderr. All managed
  runtimes are stopped on Ctrl-C / SIGTERM.

## Develop

```sh
cargo test          # unit + end-to-end tests (mock runtime, real process launch & eviction)
cargo clippy --all-targets
LLMO_LOG=llmo=debug llmo serve   # includes managed runtimes' stderr
```

## License

MIT
