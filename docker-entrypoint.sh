#!/bin/sh
set -e

PUID=${PUID:-1000}
PGID=${PGID:-1000}

# Create or align group and user for homelab PUID/PGID support
if ! getent group channelzero >/dev/null 2>&1; then
    addgroup -g "$PGID" channelzero 2>/dev/null || addgroup channelzero
fi

if ! getent passwd channelzero >/dev/null 2>&1; then
    adduser -D -u "$PUID" -G channelzero -h /app channelzero 2>/dev/null || adduser -D -G channelzero -h /app channelzero
fi

# Ensure data directory has proper permissions
mkdir -p "$DATA_DIR"
chown -R "$PUID:$PGID" "$DATA_DIR" 2>/dev/null || true

# Execute ChannelZero binary as non-root user
exec su-exec "$PUID:$PGID" /app/channelzero
