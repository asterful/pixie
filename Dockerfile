# syntax=docker/dockerfile:1

FROM rust:latest AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /data

COPY --from=builder /app/target/release/Pixie /usr/local/bin/pixie

EXPOSE 8080
VOLUME ["/data"]

CMD ["/usr/local/bin/pixie"]