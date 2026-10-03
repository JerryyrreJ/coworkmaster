# Rust backend

This directory is the incremental Rust replacement for the existing TypeScript backend.

The first migration slice intentionally exposes only a health endpoint so both backends can run side by side.

## Run

```bash
cd rust-backend
cargo run
```

The Rust service listens on `127.0.0.1:8790` by default.

```bash
curl http://127.0.0.1:8790/health
```

Expected response:

```json
{"ok":true,"service":"opportunity-autopilot-rust"}
```

Set `RUST_PORT` to use another port.

The existing TypeScript/Fastify backend remains unchanged on port 8787.
