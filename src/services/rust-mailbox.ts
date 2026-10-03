import type { MailMessage } from "../types.js";
import type { MailboxAdapter, MailboxConfig } from "./mailbox.js";

type ConnectionStatus = { imap: boolean; smtp: boolean; message: string };

export class RustMailboxAdapter implements MailboxAdapter {
  constructor(
    private readonly config: MailboxConfig,
    private readonly baseUrl = process.env.RUST_MAILBOX_URL ?? "http://127.0.0.1:8790",
  ) {}

  private async request<T>(path: string, body: Record<string, unknown>): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(35_000),
    });

    if (!response.ok) {
      const payload = await response.json().catch(() => null) as { error?: string } | null;
      throw new Error(payload?.error ?? `Rust mailbox request failed: HTTP ${response.status}`);
    }

    return await response.json() as T;
  }

  testConnection(): Promise<ConnectionStatus> {
    return this.request("/mail/test", { config: this.config });
  }

  listUnread(): Promise<MailMessage[]> {
    return this.request("/mail/unread", { config: this.config });
  }

  listMessages(): Promise<MailMessage[]> {
    return this.request("/mail/messages", { config: this.config });
  }

  getMessage(id: string): Promise<MailMessage | null> {
    return this.request("/mail/message", { config: this.config, id });
  }

  send(message: { to: string; subject: string; text: string; inReplyTo?: string }): Promise<{ messageId: string }> {
    return this.request("/mail/send", { config: this.config, message });
  }
}
