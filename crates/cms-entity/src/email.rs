//! Email entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Email request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct EmailRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub html_body: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
}

/// Email template
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct EmailTemplate {
    pub name: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub html_body: Option<String>,
}

/// Predefined email template types
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EmailTemplateType {
    InvitationAccepted,
    InvitationSent,
    PasswordReset,
    WelcomeEmail,
    ProjectInvitation,
    ReaderInvitation,
}
