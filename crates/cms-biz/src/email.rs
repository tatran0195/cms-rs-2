//! Email business logic and delivery adapters.

use std::sync::Arc;

use async_trait::async_trait;
use cms_entity::email::{EmailRequest, EmailTemplate};
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::AppError;

/// Email sender trait — implementations can be SMTP, SES, SendGrid, etc.
#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError>;
}

/// Build the configured mail adapter. Missing SMTP is represented explicitly and
/// fails closed on a send request; it never logs an OTP or silently discards mail.
pub fn create_mailer(
    config: Option<&cms_config::MailerConfig>,
) -> Result<Arc<dyn Mailer>, AppError> {
    match config.and_then(|config| config.smtp_host.as_deref().map(|_| config)) {
        Some(config) => Ok(Arc::new(SmtpMailer::new(config.clone())?)),
        None => Ok(Arc::new(UnconfiguredMailer)),
    }
}

/// SMTP mailer implementation shared by the HTTP API and background worker.
pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpMailer {
    pub fn new(config: cms_config::MailerConfig) -> Result<Self, AppError> {
        let host = config
            .smtp_host
            .as_deref()
            .ok_or_else(|| AppError::InvalidInput("smtp_host is required".to_string()))?;
        let from_email = config
            .from_email
            .as_deref()
            .unwrap_or("noreply@example.com");
        let from_name = config.from_name.as_deref().unwrap_or("CMS");
        let from: Mailbox = format!("{} <{}>", from_name, from_email)
            .parse()
            .map_err(|error| AppError::InvalidInput(format!("Invalid from_email: {error}")))?;

        let builder = if config.smtp_use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(host)
        } else if config.smtp_plain_no_tls {
            // Plain unencrypted SMTP — used for local dev tools like Mailpit.
            // lettre's `builder_dangerous` disables all TLS negotiation.
            Ok(AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host))
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
        }
        .map_err(|error| AppError::ProviderError(format!("SMTP configuration error: {error}")))?;

        let builder = match (&config.smtp_username, &config.smtp_password) {
            (Some(username), Some(password)) => {
                builder.credentials(Credentials::new(username.clone(), password.clone()))
            }
            (None, None) => builder,
            _ => {
                return Err(AppError::InvalidInput(
                    "smtp_username and smtp_password must be configured together".to_string(),
                ));
            }
        };

        Ok(Self {
            transport: builder.port(config.smtp_port).build(),
            from,
        })
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        let to_mailbox: Mailbox = to
            .parse()
            .map_err(|error| AppError::InvalidInput(format!("Invalid recipient address: {error}")))?;
        let email = Message::builder()
            .from(self.from.clone())
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|error| AppError::Internal(error.into()))?;

        self.transport.send(email).await.map_err(|error| {
            tracing::error!("SMTP delivery failed: {error}");
            AppError::ProviderError("Email delivery failed".to_string())
        })?;
        Ok(())
    }
}

/// Explicit fail-closed adapter used when SMTP is not configured.
pub struct UnconfiguredMailer;

#[async_trait]
impl Mailer for UnconfiguredMailer {
    async fn send_email(&self, _to: &str, _subject: &str, _body: &str) -> Result<(), AppError> {
        Err(AppError::ProviderError(
            "Email delivery is not configured; set CMS_MAILER__SMTP_HOST".to_string(),
        ))
    }
}

/// Email service
pub struct EmailService;

impl EmailService {
    /// Send an email
    pub async fn send_email(
        mailer: Arc<dyn Mailer>,
        request: EmailRequest,
    ) -> Result<(), AppError> {
        mailer
            .send_email(&request.to, &request.subject, &request.body)
            .await
    }

    /// Send a templated email
    pub async fn send_templated_email(
        mailer: Arc<dyn Mailer>,
        template: EmailTemplate,
        to: &str,
        variables: serde_json::Value,
    ) -> Result<(), AppError> {
        let (subject, body) = Self::render_template(template, variables)?;
        mailer.send_email(to, &subject, &body).await
    }

    /// Render an email template
    fn render_template(
        template: EmailTemplate,
        variables: serde_json::Value,
    ) -> Result<(String, String), AppError> {
        let mut subject = template.subject.clone();
        let mut body = template.body.clone();

        if let serde_json::Value::Object(map) = variables {
            for (key, value) in map {
                let placeholder = format!("{{{}}}", key);
                let replacement = value.as_str().unwrap_or("");

                subject = subject.replace(&placeholder, replacement);
                body = body.replace(&placeholder, replacement);
            }
        }

        Ok((subject, body))
    }
}

/// Process email job (for worker queue)
pub async fn process_email_job(
    mailer: Arc<dyn Mailer>,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let to = payload
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing to address".to_string()))?;
    let subject = payload
        .get("subject")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing subject".to_string()))?;
    let body = payload
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing body".to_string()))?;

    mailer.send_email(to, subject, body).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unconfigured_mailer_fails_instead_of_dropping_security_codes() {
        let error = UnconfiguredMailer
            .send_email("user@example.com", "OTP", "123456")
            .await
            .unwrap_err();
        assert!(error.to_string().contains("Email delivery is not configured"));
    }
}
