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

## Install

```sh
cargo install --path .        # or: cargo build --release
llmo init                     # writes ~/.config/llmo/config.toml
$EDITOR ~/.config/llmo/config.toml
llmo serve
```

Requires Rust 1.88+. SQLite is bundled; no system libraries needed.

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
| `GET  /health`                         | Liveness (never requires auth)           |

Set `server.api_key` to require `Authorization: Bearer <key>` on everything
except `/health`. Errors use the OpenAI shape: `{"error":{"message","type"}}`.

## How it works

- **Memory ranking** — cosine similarity against memories embedded by the
  *same* embedder, filtered by `min_score`, then weighted by recency
  (`0.5 + 0.5·2^(−age/half_life)`); pinned memories skip decay. Identical
  content in a namespace is de-duplicated. Injected context is capped by
  `max_context_chars`.
- **Storage** — turns shorter than 16 characters ("ok", "thanks!") are not
  stored. Streaming replies are stored only if the client received them in
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
