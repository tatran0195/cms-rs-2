//! Language entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, PaginatedResponse};

/// A content language associated with a project.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Language {
    pub id: Id,
    pub project_id: Id,
    /// Canonicalized BCP-47 language tag (for example, `en`, `pt-BR`, or `zh-Hant`).
    pub code: String,
    pub name: String,
    pub is_default: bool,
    pub is_rtl: bool,
    pub enabled: bool,
    pub position: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Language create request.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateLanguageRequest {
    #[serde(default)]
    pub project_id: Id,
    pub code: String,
    #[serde(alias = "label", default)]
    pub name: String,
    #[serde(alias = "isRtl", default)]
    pub is_rtl: bool,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(alias = "isDefault", default)]
    pub is_default: Option<bool>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub position: Option<i32>,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

/// Language update request.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateLanguageRequest {
    #[serde(alias = "label", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(alias = "isRtl", skip_serializing_if = "Option::is_none")]
    pub is_rtl: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(alias = "isDefault", skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    #[serde(
        default,
        deserialize_with = "deserialize_config_patch",
        skip_serializing_if = "Option::is_none"
    )]
    pub config: Option<Option<serde_json::Value>>,
}

fn deserialize_config_patch<'de, D>(
    deserializer: D,
) -> Result<Option<Option<serde_json::Value>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<serde_json::Value>::deserialize(deserializer).map(Some)
}

/// Translation coverage compared with the default language on the same branch.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LanguageCoverage {
    #[serde(rename = "pageCount")]
    pub page_count: u64,
    #[serde(rename = "sourcePageCount")]
    pub source_page_count: u64,
    #[serde(rename = "matchedPages")]
    pub matched_pages: u64,
    #[serde(rename = "missingPages")]
    pub missing_pages: u64,
    #[serde(rename = "extraPages")]
    pub extra_pages: u64,
    pub percentage: Option<u8>,
}

/// Language response. The wire format intentionally follows the frontend's
/// camelCase contract while retaining a few snake_case keys for older clients.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct LanguageResponse {
    pub id: Id,
    #[serde(alias = "projectId")]
    pub project_id: Id,
    pub code: String,
    #[serde(alias = "label")]
    pub name: String,
    #[serde(alias = "isDefault")]
    pub is_default: bool,
    #[serde(alias = "isRtl", default)]
    pub is_rtl: bool,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    #[serde(default)]
    pub translation: Option<ProjectTranslationResponse>,
    #[serde(default)]
    pub coverage: Option<LanguageCoverage>,
    #[serde(alias = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(alias = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

fn default_enabled() -> bool {
    true
}

impl serde::Serialize for LanguageResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("projectId", &self.project_id)?;
        map.serialize_entry("project_id", &self.project_id)?;
        map.serialize_entry("code", &self.code)?;
        map.serialize_entry("label", &self.name)?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("isDefault", &self.is_default)?;
        map.serialize_entry("is_default", &self.is_default)?;
        map.serialize_entry("direction", if self.is_rtl { "RTL" } else { "LTR" })?;
        map.serialize_entry("isRtl", &self.is_rtl)?;
        map.serialize_entry("is_rtl", &self.is_rtl)?;
        map.serialize_entry("enabled", &self.enabled)?;
        map.serialize_entry("position", &self.position)?;
        map.serialize_entry("config", &self.config)?;
        map.serialize_entry("translation", &self.translation)?;
        map.serialize_entry("coverage", &self.coverage)?;
        map.serialize_entry("createdAt", &self.created_at)?;
        map.serialize_entry("created_at", &self.created_at)?;
        map.serialize_entry("updatedAt", &self.updated_at)?;
        map.serialize_entry("updated_at", &self.updated_at)?;
        map.end()
    }
}

impl From<Language> for LanguageResponse {
    fn from(lang: Language) -> Self {
        Self {
            id: lang.id,
            project_id: lang.project_id,
            code: lang.code,
            name: lang.name,
            is_default: lang.is_default,
            is_rtl: lang.is_rtl,
            enabled: lang.enabled,
            position: lang.position,
            config: lang.config,
            translation: None,
            coverage: None,
            created_at: lang.created_at,
            updated_at: lang.updated_at,
        }
    }
}

/// Project translation entity.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectTranslation {
    pub id: Id,
    pub project_id: Id,
    pub language_id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Project translation response.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectTranslationResponse {
    pub id: Id,
    #[serde(rename = "projectId")]
    pub project_id: Id,
    #[serde(rename = "languageId")]
    pub language_id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

impl From<ProjectTranslation> for ProjectTranslationResponse {
    fn from(trans: ProjectTranslation) -> Self {
        Self {
            id: trans.id,
            project_id: trans.project_id,
            language_id: trans.language_id,
            name: trans.name,
            description: trans.description,
            created_at: trans.created_at,
            updated_at: trans.updated_at,
        }
    }
}

/// List languages query parameters.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListLanguagesQuery {
    pub project_id: Id,
}

/// List languages response.
pub type ListLanguagesResponse = PaginatedResponse<LanguageResponse>;

/// Set default language request.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct SetDefaultLanguageRequest {
    pub language_id: Id,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_language() -> Language {
        Language {
            id: "lang-1".to_string(),
            project_id: "proj-1".to_string(),
            code: "ja".to_string(),
            name: "Japanese".to_string(),
            is_default: true,
            is_rtl: false,
            enabled: true,
            position: 0,
            config: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_language_response_conversion() {
        let response: LanguageResponse = sample_language().into();
        assert_eq!(response.code, "ja");
        assert_eq!(response.name, "Japanese");
        assert!(!response.is_rtl);
        assert!(response.enabled);
        assert_eq!(response.position, 0);
    }

    #[test]
    fn test_create_language_request_from_frontend_json() {
        let req: CreateLanguageRequest =
            serde_json::from_str(r#"{"code":"ja","label":"Japanese","direction":"LTR"}"#).unwrap();
        assert_eq!(req.code, "ja");
        assert_eq!(req.name, "Japanese");
        assert_eq!(req.direction.as_deref(), Some("LTR"));
    }

    #[test]
    fn test_update_language_request_from_frontend_json() {
        let req: UpdateLanguageRequest = serde_json::from_str(r#"{"isDefault":true}"#).unwrap();
        assert_eq!(req.is_default, Some(true));
        assert_eq!(req.config, None);

        let clear_config: UpdateLanguageRequest =
            serde_json::from_str(r#"{"config":null}"#).unwrap();
        assert_eq!(clear_config.config, Some(None));

        let set_config: UpdateLanguageRequest =
            serde_json::from_str(r#"{"config":{"seo":{"allowIndex":false}}}"#).unwrap();
        assert_eq!(
            set_config.config,
            Some(Some(serde_json::json!({"seo":{"allowIndex":false}})))
        );
    }

    #[test]
    fn test_language_response_serialization_uses_persisted_metadata() {
        let mut language: LanguageResponse = sample_language().into();
        language.enabled = false;
        language.position = 4;
        language.config = Some(serde_json::json!({"name":"Arabic"}));
        let value = serde_json::to_value(&language).unwrap();
        assert_eq!(value["projectId"], "proj-1");
        assert_eq!(value["label"], "Japanese");
        assert_eq!(value["direction"], "LTR");
        assert_eq!(value["isDefault"], true);
        assert_eq!(value["enabled"], false);
        assert_eq!(value["position"], 4);
        assert_eq!(value["config"]["name"], "Arabic");
    }
}
