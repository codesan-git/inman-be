FROM rust:latest as builder

WORKDIR /app

# Cache dependensi
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release

# Copy source code
COPY src ./src
COPY .sqlx ./.sqlx
RUN touch src/main.rs && \
    cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rustrest /usr/local/bin/

ENV RUST_LOG=info
ENV PORT=8080

CMD ["rustrest"]