//! Theme entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Theme settings for a project
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Theme {
    pub id: Id,
    pub project_id: Id,
    pub name: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub background_color: String,
    pub text_color: String,
    pub font_family: Option<String>,
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
    /// JSON config blob (alternative to individual color fields)
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    /// Whether this theme is globally applied (not project-specific)
    #[serde(default)]
    pub is_global: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Theme response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ThemeResponse {
    pub id: Id,
    pub project_id: Id,
    pub name: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub background_color: String,
    pub text_color: String,
    pub font_family: Option<String>,
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Theme> for ThemeResponse {
    fn from(theme: Theme) -> Self {
        Self {
            id: theme.id,
            project_id: theme.project_id,
            name: theme.name,
            primary_color: theme.primary_color,
            secondary_color: theme.secondary_color,
            background_color: theme.background_color,
            text_color: theme.text_color,
            font_family: theme.font_family,
            logo_url: theme.logo_url,
            favicon_url: theme.favicon_url,
            created_at: theme.created_at,
            updated_at: theme.updated_at,
        }
    }
}

/// Create theme request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateThemeRequest {
    pub project_id: Id,
    pub name: String,
    #[serde(default = "default_primary")]
    pub primary_color: String,
    #[serde(default = "default_secondary")]
    pub secondary_color: String,
    #[serde(default = "default_background")]
    pub background_color: String,
    #[serde(default = "default_text")]
    pub text_color: String,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub favicon_url: Option<String>,
    /// JSON config blob for all theme settings
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    /// Whether this is a globally applied theme
    #[serde(default)]
    pub is_global: Option<bool>,
}

fn default_primary() -> String {
    "#3b82f6".to_string() // blue-500
}

fn default_secondary() -> String {
    "#10b981".to_string() // emerald-500
}

fn default_background() -> String {
    "#ffffff".to_string() // white
}

fn default_text() -> String {
    "#1f2937".to_string() // gray-800
}

/// Update theme request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateThemeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon_url: Option<String>,
    /// JSON config blob for theme settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    /// Whether this is a globally applied theme
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_global: Option<bool>,
}

/// Theme CSS variables
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ThemeCssVariables {
    pub primary_color: String,
    pub secondary_color: String,
    pub background_color: String,
    pub text_color: String,
    pub font_family: Option<String>,
}

impl From<Theme> for ThemeCssVariables {
    fn from(theme: Theme) -> Self {
        Self {
            primary_color: theme.primary_color,
            secondary_color: theme.secondary_color,
            background_color: theme.background_color,
            text_color: theme.text_color,
            font_family: theme.font_family,
        }
    }
}

/// List themes query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListThemesQuery {
    #[serde(default)]
    pub project_id: Option<Id>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}
