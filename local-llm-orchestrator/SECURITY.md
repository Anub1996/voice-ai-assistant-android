# Security policy

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub's **"Report a
vulnerability"** (Security → Advisories) on this repository rather than in a
public issue. Include the version (`llmo --version`), configuration (with
secrets removed), and steps to reproduce. We aim to acknowledge within 3
business days.

## Security model

llmo is designed to be safe by default on a personal machine and hardened when
shared:

- **Local by default**: binds `127.0.0.1`. Binding beyond loopback without
  authentication is refused at start-up.
- **No outbound traffic**: llmo only talks to the runtimes you configure. It
  ignores system proxies for that traffic and sends no telemetry. The web UI's
  Content-Security-Policy blocks every request to other origins.
- **Authentication**: bearer API keys, stored as SHA-256 digests and compared
  in constant time. Keys must be at least 16 characters, and generated keys
  have 256 bits of entropy.
- **Authorization**: `admin` and `user` roles, with per-key namespace scopes
  enforced on every chat and memory operation. Memories outside the caller's
  scope look like "not found".
- **Auditability**: privileged and destructive actions, permission denials and
  authentication failures are logged to `llmo::audit`.
- **Data minimisation**: secrets and email addresses are redacted before
  storage. Assistant replies aren't stored by default. The database file is
  `0600` on Unix.
- **Error hygiene**: internal errors return a generic message; details go only
  to server logs.
- **Resource limits**: request body limit, concurrency limit with load
  shedding, and upstream timeouts.

## Supply chain (built-in model runner)

- The engine comes only from the configured llama.cpp GitHub repository
  (`ggml-org/llama.cpp` by default) and is verified against the SHA-256
  digest GitHub publishes for each release asset. Pin `llama_cpp_version`
  for reproducible deployments, or provide your own vetted binary with
  `engine.llama_server`.
- Model files are verified against Hugging Face's published SHA-256. GGUF
  headers are parsed defensively, with bounded allocations and no panics on
  malformed input.
- Downloads, installs and process launches need an admin key and are
  audited. `engine.downloads = false` turns all of it off.
- Model weights from third parties can be biased or wrong. Treat their
  output as untrusted.

## Out of scope / your responsibility

- **TLS**: terminate it at a reverse proxy when exposing llmo beyond
  localhost.
- **Encryption at rest**: use full-disk encryption.
- **Runtime security**: model servers llmo launches run with llmo's
  privileges. Only configure commands you trust.
- **Redaction**: it is best-effort pattern matching, not a guarantee.
