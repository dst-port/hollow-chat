# Multi-stage build: compiles hollowchat-server against the workspace, then
# ships only the resulting binary in a slim runtime image. Migrations are
# embedded into the binary at compile time by sqlx::migrate!, so the image
# never needs the source tree or a live database at build time.

FROM rust:1.82-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/common crates/common
COPY crates/server crates/server
COPY migrations migrations

RUN cargo build --release -p hollowchat-server

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --shell /usr/sbin/nologin hollowchat

COPY --from=builder /build/target/release/hollowchat-server /usr/local/bin/hollowchat-server

USER hollowchat
WORKDIR /home/hollowchat
ENV ATTACHMENTS_DIR=/home/hollowchat/attachments
ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/hollowchat-server"]
