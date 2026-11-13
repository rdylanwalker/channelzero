# Stage 1: Build Frontend
FROM node:26-alpine AS frontend-builder
WORKDIR /build/frontend

COPY frontend/package*.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust Backend
FROM rust:1.100-alpine AS backend-builder
WORKDIR /build

RUN apk add --no-cache musl-dev sqlite-dev openssl-dev openssl-libs-static

# Cache dependencies
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release || true
RUN rm -rf src

COPY src/ ./src/
RUN touch src/main.rs && cargo build --release

# Stage 3: Minimal Runtime Container
FROM alpine:3.22

RUN apk add --no-cache \
    ca-certificates \
    tzdata \
    sqlite-libs \
    su-exec

ENV PORT=8080 \
    TZ=UTC \
    PUID=1000 \
    PGID=1000 \
    DATA_DIR=/data \
    STATIC_DIR=/app/static \
    CONFIG_FILE=/config/config.json

WORKDIR /app

COPY --from=backend-builder /build/target/release/channelzero /app/channelzero
COPY --from=frontend-builder /build/frontend/dist /app/static
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

RUN mkdir -p /data /config

EXPOSE 8080

VOLUME ["/data", "/config"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD wget -qO- http://127.0.0.1:${PORT:-8080}/api/health || exit 1

ENTRYPOINT ["/app/docker-entrypoint.sh"]
