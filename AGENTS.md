# AGENTS.md — ChannelZero

Guidance for AI coding agents working on the **ChannelZero** repository.

## Project Purpose

ChannelZero is a locally hosted streaming dashboard that highlights new releases across major streaming providers. It runs locally via a single OCI container (optimized for ease-of-use on Linux, WSL, and homelab platforms like TrueNAS), stores user profiles, preferences, and program interaction history in SQLite, displays program ratings, supports search/filter/sort, and links directly to the streaming service when a user clicks a program's icon or preview.

## Confirmed Tech Stack & Architecture

- **Backend**: Rust (`axum`, `tokio`, `sqlx` / SQLite, `reqwest`, `serde`) — serves the REST API, background metadata sync, deduplication, config validation, and the compiled React SPA static assets.
- **Frontend**: React + TypeScript + Tailwind CSS + shadcn/ui (clean, free, open-source UI components).
- **Persistence**: SQLite stored on a mounted `/data` volume so container restarts preserve profiles, preferences, history, and cached metadata.
- **Packaging**: Single multi-stage OCI container (`Dockerfile` / `Containerfile` + `compose.yaml`), compatible with Docker, Podman, and TrueNAS Custom Apps.

## Core Requirements

1. **OCI-First Local Hosting & Ease of Use**
   - Package as a single self-contained OCI image with clear, copy-pasteable `docker` / `podman` / `compose.yaml` instructions for Linux, WSL, and TrueNAS.
   - Persist stateful data (SQLite database, caches) via a single `/data` volume mount; optionally mount `/config` for a JSON/TOML config file.
   - Support configuration via environment variables, mounted config file, and secret files (e.g., `*_FILE` env vars for container secrets).

2. **Multi-Source Metadata & Deduplication**
   - Support aggregating new releases, ratings, and watch links from multiple upstream sources, each individually toggleable (`enabled: true/false`) in configuration.
   - **Source priority order**:
     1. **Anonymous, free APIs (no key required)**: e.g., **TVmaze** (TV releases, schedules, ratings, web channels) — works out of the box with zero setup.
     2. **Free-tier APIs (API key required)**: e.g., **TMDB** (movies/TV, streaming provider availability per region, ratings) and **OMDb** (IMDb / Rotten Tomatoes / Metacritic ratings enrichment).
     3. **Paid / limited-tier APIs (optional)**: e.g., **Watchmode** (deep-link enrichment if configured).
   - De-duplicate programs across sources (via external IDs such as IMDb/TMDB/TVDB/TVmaze or normalized title + release year + media type).
   - Refresh metadata on a configurable background cadence (default e.g. every 6 hours) plus an on-demand manual refresh endpoint/button.
   - Handle upstream API rate limits and failures gracefully (SQLite caching, backoff, clear status/error reporting, no silent crashes).

3. **Configuration & Schema Validation**
   - Default region to `US`, with configurable region and streaming providers (starting with major US providers).
   - Validate configuration against external API requirements (e.g., valid ISO 3166-1 region codes, supported provider IDs, valid refresh intervals) at startup.
   - Provide a formal JSON Schema (`config.schema.json`) and document all options and environment variables in `.env.example` and `README.md`.

4. **Dashboard, Ratings & Deep Links**
   - Highlight new releases across configured streaming providers within a configurable lookback window (default `NEW_RELEASE_DAYS=30`), with a filter/toggle for **New Series / Movies** vs. **New Episodes / Returning Seasons**.
   - Display program ratings clearly on cards/previews.
   - **Card interaction & deep-link fallback**:
     - Clicking a **poster play overlay** or **provider badge/icon** launches the streaming service (`rel="noopener noreferrer"`) and records the item as `seen`.
     - Clicking the **card title / info area** opens a lightweight detail modal (full synopsis, all source ratings, genres with quick "Hide genre" actions, and all available providers).
     - Multi-provider titles display all matching configured provider icons; clicking a specific provider icon opens that provider, while the main play action uses the highest-priority configured provider.
     - Use a 3-step deep-link fallback: (1) direct deep link (Watchmode / matching `officialSite`), (2) provider search URL, (3) TMDB/TVmaze page as a last resort.
   - Provide responsive search, filtering (provider, media type, genre, rating, seen/watched/watchlist status), and sorting (release date, rating, title).

5. **Lightweight Profiles, Preferences & Curation**
   - Support lightweight local user profiles (switch/create/manage profiles without a heavy login wall).
   - Persist per-profile preferences and interaction state in SQLite (no reliance on size-limited browser storage for history).
   - Track **"seen"** state (auto-marked on launch/click), allow manual **"mark as watched"** toggling, optional **"watchlist / save for later"** state, and **"not interested"** to hide individual titles.
   - Support **"not interested in genre"** filtering to hide entire genres/categories from the dashboard, with an easy **"Hidden Titles & Genres"** undo/management section in Profile Settings.
   - Provide an **API & Sync Status Panel** in the UI showing active sources, last sync timestamp, item counts, and any API key or rate-limit errors.

6. **Homelab / Container Operations & Security**
   - Support `PORT` (default `8080`), `TZ` (timezone for local release dates), `PUID`/`PGID` or non-root container execution, and a `GET /api/health` endpoint for Docker/Podman/TrueNAS healthchecks.
   - Never hardcode API keys, tokens, or secrets in source files or container images.
   - Read all API keys and private credentials from environment variables or mounted secret files (`*_FILE`).

## Agent Working Guidelines

- **Follow Confirmed Decisions**: Stick to Rust backend, React + shadcn/ui frontend, SQLite persistence, and single-container OCI packaging unless the user explicitly changes direction.
- **Keep Changes Focused**: Make minimal, well-scoped edits. Keep Rust and TypeScript code idiomatic, modular, and cleanly typed.
- **Document as You Build**: Update `README.md`, `.env.example`, and `config.schema.json` whenever you add environment variables, config fields, container options, ports, or build/run commands.
- **Validate Changes**: Run `cargo test`, `cargo clippy`, frontend type-check/build/lint, and container build checks after making changes, and report what was verified.
- **Use Conventional Commits**: Releases are automated with `semantic-release` (`.releaserc.json`), so all commit messages MUST follow Conventional Commits (`feat(scope): ...`, `fix(scope): ...`, `perf(scope): ...`, `docs(scope): ...`, `test(scope): ...`, `chore(scope): ...`). Use `feat:` / `fix:` / `perf:` for changes that should trigger a release.
