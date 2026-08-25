# syntax=docker/dockerfile:1

# Builder - Cargo.toml enables openssl's "vendored" feature, but that
# doesn't reliably propagate to every openssl-sys instance in the graph
# (sqlx's native-tls path pulls one in too), so pkg-config still needs a
# real libssl-dev to find. That means the resulting binary is dynamically
# linked against libssl - the runtime stage installs the matching runtime
# package for it below.
FROM rust:slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config perl make libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Both binaries land in the same image - migrate runs as a one-off step in
# the deploy pipeline, fiscal-core is the long-running service.
RUN cargo build --release --bin fiscal-core --bin migrate

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /app app

WORKDIR /app
COPY --from=builder /app/target/release/fiscal-core /app/fiscal-core
COPY --from=builder /app/target/release/migrate /app/migrate

USER app

# Cloud Run assigns the listen port via $PORT at runtime (see the
# deploy workflow, which sets CORE_HTTP_PORT=$PORT to match) - 8080 here is
# just the documented/default container port for local `docker run`.
ENV CORE_HTTP_PORT=8080
EXPOSE 8080

CMD ["/app/fiscal-core"]
