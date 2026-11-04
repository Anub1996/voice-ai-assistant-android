# syntax=docker/dockerfile:1
# Multi-stage build: static-ish release binary on a slim, non-root base.
FROM rust:1.97-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
COPY config.example.toml ./
RUN cargo build --release --locked && strip target/release/llmo

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --home-dir /data --create-home llmo
COPY --from=build /src/target/release/llmo /usr/local/bin/llmo
USER llmo
WORKDIR /data
# Binding beyond loopback requires authentication: pass LLMO_API_KEY or
# mount a config with [[server.keys]] at /data/.config/llmo/config.toml.
ENV HOME=/data \
    LLMO_BIND=0.0.0.0:8765 \
    LLMO_DB_PATH=/data/memory.db \
    LLMO_LOG_FORMAT=json
VOLUME ["/data"]
EXPOSE 8765
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
  CMD ["llmo", "healthcheck", "--url", "http://127.0.0.1:8765"]
STOPSIGNAL SIGTERM
ENTRYPOINT ["llmo"]
CMD ["serve"]
