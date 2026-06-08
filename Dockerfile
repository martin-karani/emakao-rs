# ── chef ───────────────────────────────────────────────────────────────────────
FROM lukemathwalker/cargo-chef:latest-rust-1.78-slim-bookworm AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev libpq-dev

# ── planner ────────────────────────────────────────────────────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── builder ────────────────────────────────────────────────────────────────────
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release

# ── runtime ────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/emakao      ./emakao
COPY --from=builder /app/target/release/migrate      ./migrate
COPY --from=builder /app/migrations                  ./migrations
COPY --from=builder /app/templates                   ./templates
COPY --from=builder /app/resources                   ./resources

EXPOSE 3000
ENV RUST_LOG=info

CMD ["./emakao"]
