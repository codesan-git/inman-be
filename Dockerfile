FROM rust:latest as builder

WORKDIR /app

# Cache dependensi
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release

# Copy source code
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs && \
    cargo build --release

FROM debian:buster-slim

RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rustrest /usr/local/bin/
COPY --from=builder /app/migrations /app/migrations

ENV RUST_LOG=info
ENV PORT=8080

CMD ["rustrest"]
