# Build stage — compiles the release binary inside an official Rust image.
FROM rust:1.86-slim AS builder

WORKDIR /app

# Cache dependencies in a separate layer so incremental builds are fast.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "pub fn main() {}" > src/lib.rs && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/palettify*

# Copy the real source and palette files, then compile the final binary.
COPY src       ./src
COPY palettes  ./palettes
RUN cargo build --release

# Runtime stage — minimal Debian image, no Rust toolchain.
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/palettify ./palettify
COPY --from=builder /app/palettes                  ./palettes

ENV PORT=3000
ENV ENVIRONMENT=production

EXPOSE 3000

CMD ["./palettify"]
