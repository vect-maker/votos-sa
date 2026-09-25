# syntax=docker/dockerfile:1
FROM rust:alpine AS builder
WORKDIR /workspace

# Install fast linker and build dependencies (including git for git dependencies)
RUN apk add --no-cache musl-dev lld git
ENV RUSTFLAGS="-C link-arg=-fuse-ld=lld"

COPY . .

# Debug build with persistent caches for fast incremental rebuilds
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/workspace/target \
    cargo build && \
    cp /workspace/target/debug/backend /tmp/backend_bin

# Runtime Stage
FROM alpine:3.21 AS runtime
WORKDIR /app
RUN apk add --no-cache ca-certificates tzdata
COPY --from=builder /tmp/backend_bin /usr/local/bin/backend

ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000

CMD ["backend", "serve"]
