//! Language business logic.
//!
//! Languages are project-scoped content namespaces. Their tags, default flag,
//! direction, visibility, order, chrome configuration, translations, and page
//! coverage are all derived from persisted data.

use std::collections::{HashMap, HashSet};

use cms_db::{
    branch::BranchQueries,
    language::{LanguageQueries, ProjectTranslationQueries},
    page::PageQueries,
    project::ProjectQueries,
};
use cms_entity::{
    common::{MemberRole, PaginatedResponse},
    language::{
        CreateLanguageRequest, Language, LanguageCoverage, LanguageResponse, ListLanguagesQuery,
        ListLanguagesResponse, ProjectTranslationResponse, SetDefaultLanguageRequest,
        UpdateLanguageRequest,
    },
};

use crate::{AppError, BizContext};

/// Language service.
pub struct LanguageService;

impl LanguageService {
    /// Create a project language using a validated, canonical BCP-47 tag.
    pub async fn create_language(
        ctx: &BizContext,
        user_id: &str,
        request: CreateLanguageRequest,
    ) -> Result<LanguageResponse, AppError> {
        ProjectQueries::get_by_id(&ctx.pool, &request.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, &request.project_id, MemberRole::Admin)
            .await?;

        let code = canonical_language_tag(&request.code)?;
        let name = request.name.trim();
        if name.is_empty() {
            return Err(AppError::Validation(
                "Language name is required".to_string(),
            ));
        }
        if name.chars().count() > 120 {
            return Err(AppError::Validation(
                "Language name must be 120 characters or fewer".to_string(),
            ));
        }
        let is_rtl = parse_direction(request.direction.as_deref(), request.is_rtl)?;
        let position = validate_position(request.position)?;
        validate_config(request.config.as_ref())?;

        if LanguageQueries::get_by_code(&ctx.pool, &request.project_id, &code)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(
                "Language with this code already exists for this project".to_string(),
            ));
        }

        let existing_count =
            LanguageQueries::count_by_project(&ctx.pool, &request.project_id).await?;
        let is_default = request.is_default.unwrap_or(false) || existing_count == 0;
        let enabled = request.enabled.unwrap_or(true) || is_default;
        let language = LanguageQueries::create_with_metadata(
            &ctx.pool,
            &request.project_id,
            &code,
            name,
            is_default,
            is_rtl,
            enabled,
            position,
            request.config.as_ref(),
        )
        .await?;

        Ok(language.into())
    }

    /// Read a language after checking project membership.
    pub async fn get_language(
        ctx: &BizContext,
        user_id: &str,
        language_id: &str,
    ) -> Result<LanguageResponse, AppError> {
        let language = LanguageQueries::get_by_id(&ctx.pool, language_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, &language.project_id, MemberRole::Viewer)
            .await?;
        Ok(language.into())
    }

    /// Update a language's metadata. Omitted config is unchanged; explicit JSON
    /// null clears its language-specific chrome overrides.
    pub async fn update_language(
        ctx: &BizContext,
        user_id: &str,
        language_id: &str,
        request: UpdateLanguageRequest,
    ) -> Result<LanguageResponse, AppError> {
        let language = LanguageQueries::get_by_id(&ctx.pool, language_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, &language.project_id, MemberRole::Admin)
            .await?;

        if let Some(name) = request.name.as_deref() {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > 120 {
                return Err(AppError::Validation(
                    "Language name must contain 1 to 120 characters".to_string(),
                ));
            }
        }
        let is_rtl = request
            .direction
            .as_deref()
            .map(|direction| parse_direction(Some(direction), false))
            .transpose()?
            .or(request.is_rtl);
        let position = validate_position(request.position)?;
        if let Some(Some(config)) = request.config.as_ref() {
            validate_config(Some(config))?;
        }

        let config_patch = request.config.as_ref().map(|value| value.as_ref());
        let updated = LanguageQueries::update_with_metadata(
            &ctx.pool,
            language_id,
            request.name.as_deref().map(str::trim),
            is_rtl,
            request.is_default,
            request.enabled,
            position,
            config_patch,
        )
        .await?;
        Ok(updated.into())
    }

    /// Delete a non-default language. Its pages and project-translation row are
    /// intentionally removed by the database's ON DELETE CASCADE constraints.
    pub async fn delete_language(
        ctx: &BizContext,
        user_id: &str,
        language_id: &str,
    ) -> Result<bool, AppError> {
        let language = LanguageQueries::get_by_id(&ctx.pool, language_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, &language.project_id, MemberRole::Admin)
            .await?;
        LanguageQueries::delete(&ctx.pool, language_id).await
    }

    /// List languages with real translations and default-branch coverage data.
    pub async fn list_languages(
        ctx: &BizContext,
        user_id: &str,
        query: ListLanguagesQuery,
        page: u64,
        page_size: u64,
    ) -> Result<ListLanguagesResponse, AppError> {
        ProjectQueries::get_by_id(&ctx.pool, &query.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, &query.project_id, MemberRole::Viewer)
            .await?;

        let offset = page.saturating_sub(1).saturating_mul(page_size);
        let languages = LanguageQueries::get_by_project(
            &ctx.pool,
            &query.project_id,
            Some(page_size.min(i64::MAX as u64) as i64),
            Some(offset.min(i64::MAX as u64) as i64),
        )
        .await?;
        let total = LanguageQueries::count_by_project(&ctx.pool, &query.project_id).await?;

        let mut responses: Vec<LanguageResponse> = languages.into_iter().map(Into::into).collect();
        let translations = ProjectTranslationQueries::get_by_project(&ctx.pool, &query.project_id)
            .await?
            .into_iter()
            .map(|translation| (translation.language_id.clone(), translation.into()))
            .collect::<HashMap<String, ProjectTranslationResponse>>();
        for response in &mut responses {
            response.translation = translations.get(&response.id).cloned();
        }

        if let Some(default_language) =
            LanguageQueries::get_default(&ctx.pool, &query.project_id).await?
        {
            if let Some(branch) = BranchQueries::get_default(&ctx.pool, &query.project_id).await? {
                // Reuse the same scoped page-list API used by the editor. Coverage
                // is calculated from all PAGE records on the default branch.
                let pages = PageQueries::get_by_project_branch_and_language(
                    &ctx.pool,
                    &query.project_id,
                    &branch.id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await?;
                apply_coverage(&mut responses, &default_language.id, &pages);
            }
        }

        Ok(PaginatedResponse::new(
            responses,
            total.max(0) as u64,
            page,
            page_size,
        ))
    }

    /// Set a language as the project default.
    pub async fn set_default_language(
        ctx: &BizContext,
        user_id: &str,
        request: SetDefaultLanguageRequest,
    ) -> Result<LanguageResponse, AppError> {
        let language = LanguageQueries::get_by_id(&ctx.pool, &request.language_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, &language.project_id, MemberRole::Admin)
            .await?;
        Ok(
            LanguageQueries::set_default(&ctx.pool, &request.language_id)
                .await?
                .into(),
        )
    }

    /// Get project translations.
    pub async fn get_project_translations(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<Vec<ProjectTranslationResponse>, AppError> {
        ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;
        Ok(
            ProjectTranslationQueries::get_by_project(&ctx.pool, project_id)
                .await?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    /// Create or update the localized name/description for a language.
    pub async fn upsert_project_translation(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        language_id: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<ProjectTranslationResponse, AppError> {
        ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;
        let language = LanguageQueries::get_by_id(&ctx.pool, language_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
        if language.project_id != project_id {
            return Err(AppError::Conflict(
                "Language does not belong to this project".to_string(),
            ));
        }
        if name.is_some_and(|value| value.chars().count() > 120)
            || description.is_some_and(|value| value.chars().count() > 500)
        {
            return Err(AppError::Validation(
                "Project translation exceeds the supported name/description length".to_string(),
            ));
        }

        Ok(
            ProjectTranslationQueries::upsert(
                &ctx.pool,
                project_id,
                language_id,
                name,
                description,
            )
            .await?
            .into(),
        )
    }

    /// Delete a project translation.
    pub async fn delete_project_translation(
        ctx: &BizContext,
        user_id: &str,
        translation_id: &str,
    ) -> Result<bool, AppError> {
        let translation = ProjectTranslationQueries::get_by_id(&ctx.pool, translation_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Translation not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, &translation.project_id, MemberRole::Admin)
            .await?;
        ProjectTranslationQueries::delete(&ctx.pool, translation_id).await
    }
}

fn validate_position(position: Option<i32>) -> Result<Option<i32>, AppError> {
    if position.is_some_and(|position| !(0..=1_000_000).contains(&position)) {
        return Err(AppError::Validation(
            "Language position must be between 0 and 1000000".to_string(),
        ));
    }
    Ok(position)
}

fn validate_config(config: Option<&serde_json::Value>) -> Result<(), AppError> {
    if let Some(config) = config {
        if !config.is_object() {
            return Err(AppError::Validation(
                "Language config must be a JSON object".to_string(),
            ));
        }
        if config.to_string().len() > 32 * 1024 {
            return Err(AppError::Validation(
                "Language config must be 32 KB or smaller".to_string(),
            ));
        }
    }
    Ok(())
}

fn parse_direction(direction: Option<&str>, fallback_rtl: bool) -> Result<bool, AppError> {
    let Some(direction) = direction else {
        return Ok(fallback_rtl);
    };
    if direction.eq_ignore_ascii_case("rtl") {
        Ok(true)
    } else if direction.eq_ignore_ascii_case("ltr") {
        Ok(false)
    } else {
        Err(AppError::Validation(
            "Language direction must be either LTR or RTL".to_string(),
        ))
    }
}

/// Canonicalize and validate a practical BCP-47 language-tag subset. It accepts
/// language/script/region/variant/extension/private-use subtags and rejects
/// separators, empty subtags, non-ASCII tags, and malformed extension chains.
fn canonical_language_tag(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 63 || !value.is_ascii() {
        return Err(AppError::Validation(
            "Language code must be a valid BCP-47 tag of at most 63 ASCII characters".to_string(),
        ));
    }
    let parts: Vec<&str> = value.split('-').collect();
    if parts.iter().any(|part| part.is_empty())
        || parts.iter().any(|part| {
            !part
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        })
    {
        return Err(AppError::Validation(
            "Language code must be a valid BCP-47 tag".to_string(),
        ));
    }

    if parts[0].eq_ignore_ascii_case("x") {
        if parts.len() < 2 || parts.iter().skip(1).any(|part| part.len() > 8) {
            return Err(AppError::Validation(
                "Private-use language tags require one or more subtags".to_string(),
            ));
        }
        return Ok(parts
            .iter()
            .map(|part| part.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join("-"));
    }

    let language = parts[0];
    if !(2..=8).contains(&language.len())
        || !language
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        return Err(AppError::Validation(
            "Language code must start with a 2- to 8-letter language subtag".to_string(),
        ));
    }

    let mut canonical = vec![language.to_ascii_lowercase()];
    let mut index = 1;
    if language.len() <= 3 {
        let mut extlang_count = 0;
        while index < parts.len()
            && parts[index].len() == 3
            && parts[index]
                .chars()
                .all(|character| character.is_ascii_alphabetic())
            && extlang_count < 3
        {
            canonical.push(parts[index].to_ascii_lowercase());
            index += 1;
            extlang_count += 1;
        }
    }
    if index < parts.len()
        && parts[index].len() == 4
        && parts[index]
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        let mut script = parts[index].to_ascii_lowercase();
        script[..1].make_ascii_uppercase();
        canonical.push(script);
        index += 1;
    }
    if index < parts.len()
        && ((parts[index].len() == 2
            && parts[index]
                .chars()
                .all(|character| character.is_ascii_alphabetic()))
            || (parts[index].len() == 3
                && parts[index]
                    .chars()
                    .all(|character| character.is_ascii_digit())))
    {
        canonical.push(parts[index].to_ascii_uppercase());
        index += 1;
    }

    let mut variants = HashSet::new();
    while index < parts.len() {
        let part = parts[index];
        if part.len() == 1 {
            break;
        }
        let is_variant = (5..=8).contains(&part.len())
            || (part.len() == 4 && part.as_bytes()[0].is_ascii_digit());
        if !is_variant || !variants.insert(part.to_ascii_lowercase()) {
            return Err(AppError::Validation(
                "Language code contains an invalid or repeated BCP-47 subtag".to_string(),
            ));
        }
        canonical.push(part.to_ascii_lowercase());
        index += 1;
    }

    let mut extensions = HashSet::new();
    while index < parts.len() {
        let singleton = parts[index];
        if singleton.len() != 1 {
            return Err(AppError::Validation(
                "Language code contains an invalid extension".to_string(),
            ));
        }
        if singleton.eq_ignore_ascii_case("x") {
            if index + 1 == parts.len() || parts[index + 1..].iter().any(|part| part.len() > 8) {
                return Err(AppError::Validation(
                    "Private-use language tags require one or more subtags".to_string(),
                ));
            }
            canonical.push("x".to_string());
            canonical.extend(
                parts[index + 1..]
                    .iter()
                    .map(|part| part.to_ascii_lowercase()),
            );
            index = parts.len();
            break;
        }
        let normalized_singleton = singleton.to_ascii_lowercase();
        if !extensions.insert(normalized_singleton.clone()) {
            return Err(AppError::Validation(
                "Language code contains a repeated extension singleton".to_string(),
            ));
        }
        canonical.push(normalized_singleton);
        index += 1;
        let start = index;
        while index < parts.len() && parts[index].len() >= 2 {
            if parts[index].len() > 8 {
                return Err(AppError::Validation(
                    "Language code contains an overlong extension subtag".to_string(),
                ));
            }
            canonical.push(parts[index].to_ascii_lowercase());
            index += 1;
        }
        if index == start {
            return Err(AppError::Validation(
                "Each language extension must contain a subtag".to_string(),
            ));
        }
    }

    Ok(canonical.join("-"))
}

fn apply_coverage(
    languages: &mut [LanguageResponse],
    default_language_id: &str,
    pages: &[cms_entity::page::PageListItem],
) {
    fn is_page(kind: Option<&str>) -> bool {
        kind.unwrap_or("PAGE").eq_ignore_ascii_case("PAGE")
    }

    let source_paths: HashSet<&str> = pages
        .iter()
        .filter(|page| page.language_id.as_deref() == Some(default_language_id))
        .filter(|page| is_page(page.kind.as_deref()))
        .map(|page| page.path.as_str())
        .collect();
    let source_page_count = pages
        .iter()
        .filter(|page| page.language_id.as_deref() == Some(default_language_id))
        .filter(|page| is_page(page.kind.as_deref()))
        .count() as u64;

    for language in languages {
        let page_rows: Vec<_> = pages
            .iter()
            .filter(|page| page.language_id.as_deref() == Some(language.id.as_str()))
            .filter(|page| is_page(page.kind.as_deref()))
            .collect();
        let matched = page_rows
            .iter()
            .filter(|page| source_paths.contains(page.path.as_str()))
            .count() as u64;
        let page_count = page_rows.len() as u64;
        let missing = source_page_count.saturating_sub(matched);
        let extra = page_count.saturating_sub(matched);
        let percentage = matched
            .saturating_mul(100)
            .saturating_add(source_page_count / 2)
            .checked_div(source_page_count)
            .map(|percentage| percentage.min(100) as u8);
        language.coverage = Some(LanguageCoverage {
            page_count,
            source_page_count,
            matched_pages: matched,
            missing_pages: missing,
            extra_pages: extra,
            percentage,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_common_bcp47_tags() {
        assert_eq!(canonical_language_tag("en").unwrap(), "en");
        assert_eq!(canonical_language_tag("pt-br").unwrap(), "pt-BR");
        assert_eq!(canonical_language_tag("ZH-hant-tw").unwrap(), "zh-Hant-TW");
        assert_eq!(
            canonical_language_tag("ar-u-nu-latn").unwrap(),
            "ar-u-nu-latn"
        );
        assert_eq!(canonical_language_tag("x-company").unwrap(), "x-company");
    }

    #[test]
    fn rejects_malformed_language_tags_and_directions() {
        for invalid in ["", "_", "en--US", "e", "english_US", "en-123456789", "é"] {
            assert!(
                canonical_language_tag(invalid).is_err(),
                "accepted {invalid:?}"
            );
        }
        assert!(parse_direction(Some("SIDEWAYS"), false).is_err());
        assert!(parse_direction(Some("rtl"), false).unwrap());
    }

    #[test]
    fn coverage_counts_page_kind_and_matches_only_identical_paths() {
        use chrono::Utc;
        use cms_entity::page::PageListItem;

        fn page(
            language_id: &str,
            path: &str,
            kind: &str,
            translation_key: Option<&str>,
        ) -> PageListItem {
            PageListItem {
                id: format!("{language_id}:{path}"),
                project_id: "project".to_string(),
                branch_id: "default".to_string(),
                parent_id: None,
                language_id: Some(language_id.to_string()),
                kind: Some(kind.to_string()),
                path: path.to_string(),
                slug: path.trim_matches('/').to_string(),
                title: path.to_string(),
                description: None,
                content: None,
                icon: None,
                config: None,
                translation_key: translation_key.map(str::to_string),
                position: 0,
                is_published: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }

        let pages = vec![
            page("en", "/guide", "PAGE", Some("source-key")),
            page("en", "/section", "GROUP", None),
            // Translation keys deliberately disagree: identical paths are the
            // matching rule for coverage.
            page("ar", "/guide", "PAGE", Some("different-key")),
            // Matching translation keys must not make a different path count.
            page("ar", "/guide-ar", "PAGE", Some("source-key")),
        ];
        let mut languages = vec![
            LanguageResponse {
                id: "en".to_string(),
                project_id: "project".to_string(),
                code: "en".to_string(),
                name: "English".to_string(),
                is_default: true,
                is_rtl: false,
                enabled: true,
                position: 0,
                config: None,
                translation: None,
                coverage: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            LanguageResponse {
                id: "ar".to_string(),
                project_id: "project".to_string(),
                code: "ar".to_string(),
                name: "Arabic".to_string(),
                is_default: false,
                is_rtl: true,
                enabled: true,
                position: 1,
                config: None,
                translation: None,
                coverage: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        apply_coverage(&mut languages, "en", &pages);
        let source = languages[0].coverage.as_ref().unwrap();
        assert_eq!(source.source_page_count, 1);
        assert_eq!(source.matched_pages, 1);
        assert_eq!(source.percentage, Some(100));

        let translated = languages[1].coverage.as_ref().unwrap();
        assert_eq!(translated.source_page_count, 1);
        assert_eq!(translated.page_count, 2);
        assert_eq!(translated.matched_pages, 1);
        assert_eq!(translated.missing_pages, 0);
        assert_eq!(translated.extra_pages, 1);
        assert_eq!(translated.percentage, Some(100));
    }
}
