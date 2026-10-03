use crate::models::{
    ConnectionStatus, MailMessage, MailServerConfig, MailboxConfig, OutgoingMail, SendResult,
};
use async_native_tls::TlsConnector;
use futures_util::TryStreamExt;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
use mail_parser::MessageParser;
use thiserror::Error;
use tokio::net::TcpStream;

#[derive(Debug, Error)]
pub enum MailError {
    #[error("invalid mail configuration: {0}")]
    InvalidConfig(String),
    #[error("IMAP error: {0}")]
    Imap(String),
    #[error("SMTP error: {0}")]
    Smtp(String),
    #[error("message parse error: {0}")]
    Parse(String),
}

pub async fn test_connection(config: &MailboxConfig) -> ConnectionStatus {
    let (imap, smtp) = tokio::join!(test_imap(&config.imap), test_smtp(&config.smtp));
    ConnectionStatus {
        imap: imap.is_ok(),
        smtp: smtp.is_ok(),
        message: format!(
            "{}；{}",
            imap.map(|_| "IMAP 连接成功".to_string())
                .unwrap_or_else(|e| format!("IMAP: {e}")),
            smtp.map(|_| "SMTP 连接成功".to_string())
                .unwrap_or_else(|e| format!("SMTP: {e}")),
        ),
    }
}

pub async fn list_unread(config: &MailboxConfig) -> Result<Vec<MailMessage>, MailError> {
    read_messages(config, "UNSEEN").await
}

pub async fn list_messages(config: &MailboxConfig) -> Result<Vec<MailMessage>, MailError> {
    read_messages(config, "ALL").await
}

pub async fn get_message(
    config: &MailboxConfig,
    id: &str,
) -> Result<Option<MailMessage>, MailError> {
    let messages = read_messages(config, "ALL").await?;
    Ok(messages.into_iter().find(|message| message.id == id))
}

pub async fn send(config: &MailboxConfig, mail: &OutgoingMail) -> Result<SendResult, MailError> {
    let server = &config.smtp;
    let from = server.login();
    if from.is_empty() {
        return Err(MailError::InvalidConfig("SMTP username is required".into()));
    }

    let mut builder = Message::builder()
        .from(
            from.parse()
                .map_err(|e| MailError::Smtp(format!("invalid from address: {e}")))?,
        )
        .to(mail
            .to
            .parse()
            .map_err(|e| MailError::Smtp(format!("invalid recipient address: {e}")))?)
        .subject(&mail.subject);

    if let Some(in_reply_to) = &mail.in_reply_to {
        builder = builder.in_reply_to(in_reply_to.clone());
    }

    let message = builder
        .body(mail.text.clone())
        .map_err(|e| MailError::Smtp(e.to_string()))?;

    let mailer = smtp_transport(server)?;
    let response = mailer
        .send(message)
        .await
        .map_err(|e| MailError::Smtp(e.to_string()))?;

    Ok(SendResult {
        message_id: response.message().next().unwrap_or("sent").to_string(),
    })
}

async fn test_imap(server: &MailServerConfig) -> Result<(), MailError> {
    let mut session = imap_session(server).await?;
    session
        .noop()
        .await
        .map_err(|e| MailError::Imap(e.to_string()))?;
    session.logout().await.ok();
    Ok(())
}

async fn test_smtp(server: &MailServerConfig) -> Result<(), MailError> {
    let mailer = smtp_transport(server)?;
    mailer
        .test_connection()
        .await
        .map_err(|e| MailError::Smtp(e.to_string()))?;
    Ok(())
}

async fn imap_session(
    server: &MailServerConfig,
) -> Result<async_imap::Session<async_native_tls::TlsStream<TcpStream>>, MailError> {
    if server.secure == Some(false) {
        return Err(MailError::InvalidConfig(
            "Rust IMAP adapter currently requires implicit TLS (secure=true)".into(),
        ));
    }

    let login = server.login();
    if login.is_empty() {
        return Err(MailError::InvalidConfig("IMAP username is required".into()));
    }

    let tcp = TcpStream::connect((server.host.as_str(), server.port.unwrap_or(993)))
        .await
        .map_err(|e| MailError::Imap(e.to_string()))?;
    let tls = TlsConnector::new();
    let tls_stream = tls
        .connect(&server.host, tcp)
        .await
        .map_err(|e| MailError::Imap(e.to_string()))?;
    let client = async_imap::Client::new(tls_stream);

    client
        .login(login, &server.password)
        .await
        .map_err(|(e, _)| MailError::Imap(e.to_string()))
}

async fn read_messages(
    config: &MailboxConfig,
    criteria: &str,
) -> Result<Vec<MailMessage>, MailError> {
    let mut session = imap_session(&config.imap).await?;
    session
        .select("INBOX")
        .await
        .map_err(|e| MailError::Imap(e.to_string()))?;

    let mut uids: Vec<_> = session
        .uid_search(criteria)
        .await
        .map_err(|e| MailError::Imap(e.to_string()))?
        .into_iter()
        .collect();
    uids.sort_unstable();

    if uids.len() > 50 {
        uids = uids.split_off(uids.len() - 50);
    }

    if uids.is_empty() {
        session.logout().await.ok();
        return Ok(Vec::new());
    }

    let sequence = uids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");

    let fetches: Vec<_> = session
        .uid_fetch(sequence, "(UID FLAGS INTERNALDATE BODY.PEEK[])")
        .await
        .map_err(|e| MailError::Imap(e.to_string()))?
        .try_collect()
        .await
        .map_err(|e| MailError::Imap(e.to_string()))?;

    let account_id = config.imap.login().to_string();
    let mut messages = Vec::with_capacity(fetches.len());

    for fetch in fetches {
        let uid = fetch
            .uid
            .ok_or_else(|| MailError::Parse("IMAP response did not include UID".into()))?;
        let raw = fetch
            .body()
            .ok_or_else(|| MailError::Parse(format!("message {uid} had no body")))?;
        let parsed = MessageParser::default()
            .parse(raw)
            .ok_or_else(|| MailError::Parse(format!("could not parse message {uid}")))?;

        let from = parsed
            .from()
            .and_then(|value| value.first())
            .and_then(|mailbox| mailbox.address())
            .unwrap_or("")
            .to_string();
        let to = parsed
            .to()
            .and_then(|value| value.first())
            .and_then(|mailbox| mailbox.address())
            .unwrap_or(account_id.as_str())
            .to_string();
        let subject = parsed.subject().unwrap_or("(no subject)").to_string();
        let text = parsed
            .body_text(0)
            .map(|value| value.into_owned())
            .unwrap_or_default();
        let received_at = fetch
            .internal_date()
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        let unread = !fetch
            .flags()
            .any(|flag| matches!(flag, async_imap::types::Flag::Seen));

        messages.push(MailMessage {
            id: uid.to_string(),
            account_id: account_id.clone(),
            from,
            to,
            subject,
            text,
            received_at,
            unread,
        });
    }

    session.logout().await.ok();
    messages.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(messages)
}

fn smtp_transport(
    server: &MailServerConfig,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, MailError> {
    let login = server.login();
    if login.is_empty() {
        return Err(MailError::InvalidConfig("SMTP username is required".into()));
    }

    let creds = Credentials::new(login.to_string(), server.password.clone());
    let secure = server.secure.unwrap_or(true);

    let builder = if secure {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&server.host)
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&server.host)
    }
    .map_err(|e| MailError::Smtp(e.to_string()))?;

    Ok(builder
        .port(server.port.unwrap_or(if secure { 465 } else { 587 }))
        .credentials(creds)
        .build())
}
