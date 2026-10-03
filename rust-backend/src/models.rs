use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailServerConfig {
    pub host: String,
    pub port: Option<u16>,
    pub secure: Option<bool>,
    pub user: Option<String>,
    pub username: Option<String>,
    pub password: String,
}

impl MailServerConfig {
    pub fn login(&self) -> &str {
        self.user
            .as_deref()
            .or(self.username.as_deref())
            .unwrap_or("")
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct MailboxConfig {
    pub imap: MailServerConfig,
    pub smtp: MailServerConfig,
}

#[derive(Debug, Deserialize)]
pub struct ConfigRequest {
    pub config: MailboxConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRequest {
    pub config: MailboxConfig,
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingMail {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub in_reply_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendRequest {
    pub config: MailboxConfig,
    pub message: OutgoingMail,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailMessage {
    pub id: String,
    pub account_id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub text: String,
    pub received_at: String,
    pub unread: bool,
}

#[derive(Debug, Serialize)]
pub struct ConnectionStatus {
    pub imap: bool,
    pub smtp: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendResult {
    pub message_id: String,
}
