# Rust backend

This directory is the incremental Rust replacement for the existing TypeScript backend.

The migration follows a strangler pattern: the existing Fastify service keeps running while selected capabilities move behind this Rust service. The current slice owns mailbox protocol work while higher-level Agent and Opportunity logic stays in TypeScript.

## Stack

- Axum + Tokio for the HTTP service
- `async-imap` for IMAP
- `mail-parser` for RFC 5322 / MIME parsing
- `lettre` for SMTP
- Serde for the JSON boundary shared with TypeScript

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

## Mailbox endpoints

The Rust service is intentionally internal-only. The TypeScript `RustMailboxAdapter` sends the existing mailbox configuration over loopback to these endpoints:

- `POST /mail/test`
- `POST /mail/unread`
- `POST /mail/messages`
- `POST /mail/message`
- `POST /mail/send`

Passwords are not persisted by the Rust service.

To route CoworkMaster mailbox operations through Rust:

```dotenv
RUST_MAILBOX_ENABLED=true
RUST_MAILBOX_URL=http://127.0.0.1:8790
```

When `RUST_MAILBOX_ENABLED=false`, the existing TypeScript `ImapSmtpMailboxAdapter` remains the fallback.

### Current compatibility note

The Rust IMAP adapter currently supports the same common implicit-TLS setup used by the existing project (`secure=true`, normally port 993). STARTTLS/non-TLS IMAP can be added later if a real account requires it.
