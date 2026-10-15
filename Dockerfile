# syntax=docker/dockerfile:1.7

FROM rust:1-bookworm AS builder
WORKDIR /src

COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
# Cargo validates every declared [[bin]]/[[bench]] path while parsing the
# manifest even when we build only the standalone server. Copy only the Rust
# harness source files, not the large benchmark datasets.
COPY benches/*.rs ./benches/

RUN cargo build --locked --release --no-default-features --features bins --bin genesis-db-server

FROM debian:bookworm-slim AS runtime

ARG VERSION=dev
ARG VCS_REF=unknown

LABEL org.opencontainers.image.title="GenesisBlockDB" \
      org.opencontainers.image.description="GenesisBlockDB standalone graph + vector database server" \
      org.opencontainers.image.source="https://github.com/Freshair129/GenesisBlock" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.licenses="MIT"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --home-dir /home/genesis genesis \
    && mkdir -p /data \
    && chown -R genesis:genesis /data

COPY --from=builder /src/target/release/genesis-db-server /usr/local/bin/genesis-db-server

ENV GENESIS_HOST=0.0.0.0 \
    GENESIS_PORT=3000 \
    GENESIS_DATA_DIR=/data \
    RUST_LOG=genesis_db_server=info,tower_http=info

VOLUME ["/data"]
EXPOSE 3000
USER genesis

HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=5 CMD \
  if [ -n "${GENESIS_API_KEY:-}" ]; then curl -fsS -H "Authorization: Bearer ${GENESIS_API_KEY}" "http://127.0.0.1:${GENESIS_PORT:-3000}/v1/status" >/dev/null; else curl -fsS "http://127.0.0.1:${GENESIS_PORT:-3000}/v1/status" >/dev/null; fi

ENTRYPOINT ["/usr/local/bin/genesis-db-server"]
