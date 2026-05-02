# ── builder ────────────────────────────────────────────────────────────────────
FROM rust:1.78-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies before copying source
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null; rm -rf src

# Build real source
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs && cargo build --release

# ── runtime ────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/emakao-backend   ./emakao-backend
COPY --from=builder /app/target/release/migrate          ./migrate
COPY --from=builder /app/migrations                      ./migrations

EXPOSE 3000
ENV RUST_LOG=info

CMD ["./emakao-backend"]