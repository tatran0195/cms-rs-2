//! Locale middleware
//!
//! This module provides locale resolution from request headers or cookies.

use std::{collections::HashMap, convert::Infallible};

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderValue},
};

/// Supported locales
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    #[default]
    En, // English
    Ja, // Japanese
    Zh, // Chinese
    Es, // Spanish
    Fr, // French
    De, // German
    Ko, // Korean
    Vi, // Vietnamese
}

impl Locale {
    /// Get locale from language tag
    pub fn from_language_tag(tag: &str) -> Option<Self> {
        let tag = tag.to_lowercase();

        // Check for exact matches first
        match tag.as_str() {
            "en" => return Some(Self::En),
            "ja" => return Some(Self::Ja),
            "zh" => return Some(Self::Zh),
            "es" => return Some(Self::Es),
            "fr" => return Some(Self::Fr),
            "de" => return Some(Self::De),
            "ko" => return Some(Self::Ko),
            "vi" => return Some(Self::Vi),
            _ => {}
        }

        // Check for language-region combinations
        if tag.starts_with("en") {
            return Some(Self::En);
        }
        if tag.starts_with("ja") {
            return Some(Self::Ja);
        }
        if tag.starts_with("zh") {
            return Some(Self::Zh);
        }
        if tag.starts_with("es") {
            return Some(Self::Es);
        }
        if tag.starts_with("fr") {
            return Some(Self::Fr);
        }
        if tag.starts_with("de") {
            return Some(Self::De);
        }
        if tag.starts_with("ko") {
            return Some(Self::Ko);
        }
        if tag.starts_with("vi") {
            return Some(Self::Vi);
        }

        None
    }

    /// Get locale as ISO 639-1 string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ja => "ja",
            Self::Zh => "zh",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::De => "de",
            Self::Ko => "ko",
            Self::Vi => "vi",
        }
    }

    /// Get locale as BCP 47 string (language-region)
    pub fn as_bcp47(&self) -> &'static str {
        match self {
            Self::En => "en-US",
            Self::Ja => "ja-JP",
            Self::Zh => "zh-CN",
            Self::Es => "es-ES",
            Self::Fr => "fr-FR",
            Self::De => "de-DE",
            Self::Ko => "ko-KR",
            Self::Vi => "vi-VN",
        }
    }
}

/// Locale extractor from request
#[derive(Debug, Clone)]
pub struct LocaleExtractor(pub Locale);

impl<S> FromRequestParts<S> for Locale
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get locale from the `x-cms-locale` header the SPA sends.
        if let Some(locale_header) = parts.headers.get("x-cms-locale") {
            if let Ok(locale_str) = locale_header.to_str() {
                if let Some(locale) = Locale::from_language_tag(locale_str.trim()) {
                    return Ok(locale);
                }
            }
        }

        // Try to get locale from Accept-Language header
        if let Some(accept_language) = parts.headers.get(header::ACCEPT_LANGUAGE) {
            if let Ok(languages) = accept_language.to_str() {
                // Parse Accept-Language header (e.g., "en-US,en;q=0.9,ja;q=0.8")
                for lang in languages.split(',') {
                    let lang = lang.trim();
                    // Remove quality factor if present (e.g., "en;q=0.9" -> "en")
                    let lang_code = lang.split(';').next().unwrap_or(lang);
                    if let Some(locale) = Locale::from_language_tag(lang_code) {
                        return Ok(locale);
                    }
                }
            }
        }

        // Try to get locale from cookie
        if let Some(cookies) = parts.headers.get(header::COOKIE) {
            if let Ok(cookie_str) = cookies.to_str() {
                let cookie_map = parse_cookies(cookie_str);

                if let Some(locale_str) = cookie_map.get("locale") {
                    if let Some(locale) = Locale::from_language_tag(locale_str) {
                        return Ok(locale);
                    }
                }

                // Also check for language cookie
                if let Some(locale_str) = cookie_map.get("language") {
                    if let Some(locale) = Locale::from_language_tag(locale_str) {
                        return Ok(locale);
                    }
                }
            }
        }

        // Return default locale
        Ok(Locale::default())
    }
}

/// Parse cookie string into a map
fn parse_cookies(cookie_str: &str) -> HashMap<String, String> {
    let mut cookies = HashMap::new();

    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        if cookie.is_empty() {
            continue;
        }

        let parts: Vec<&str> = cookie.splitn(2, '=').collect();
        if parts.len() == 2 {
            cookies.insert(parts[0].to_string(), parts[1].to_string());
        } else if !cookie.is_empty() {
            cookies.insert(cookie.to_string(), String::new());
        }
    }

    cookies
}

/// Locale layer for Tower middleware
pub struct LocaleLayer;

impl Default for LocaleLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl LocaleLayer {
    pub fn new() -> Self {
        Self
    }
}

/// Set locale cookie middleware
pub struct SetLocaleMiddleware;

impl Default for SetLocaleMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl SetLocaleMiddleware {
    pub fn new() -> Self {
        Self
    }

    /// Build Set-Cookie header for locale
    pub fn set_locale_cookie(locale: Locale) -> HeaderValue {
        HeaderValue::from_str(&format!(
            "locale={}; Path=/; SameSite=Lax; Max-Age=31536000",
            locale.as_str()
        ))
        .expect("Invalid locale cookie header")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_from_language_tag() {
        assert_eq!(Locale::from_language_tag("en"), Some(Locale::En));
        assert_eq!(Locale::from_language_tag("en-US"), Some(Locale::En));
        assert_eq!(Locale::from_language_tag("en-GB"), Some(Locale::En));
        assert_eq!(Locale::from_language_tag("ja"), Some(Locale::Ja));
        assert_eq!(Locale::from_language_tag("ja-JP"), Some(Locale::Ja));
        assert_eq!(Locale::from_language_tag("zh-CN"), Some(Locale::Zh));
        assert_eq!(Locale::from_language_tag("fr"), Some(Locale::Fr));
        assert_eq!(Locale::from_language_tag("xx"), None);
    }

    #[test]
    fn test_locale_as_str() {
        assert_eq!(Locale::En.as_str(), "en");
        assert_eq!(Locale::Ja.as_str(), "ja");
        assert_eq!(Locale::Zh.as_str(), "zh");
    }

    #[test]
    fn test_parse_cookies() {
        let cookies = parse_cookies("locale=en-US; session=abc123; theme=dark");
        assert_eq!(cookies.get("locale"), Some(&"en-US".to_string()));
        assert_eq!(cookies.get("session"), Some(&"abc123".to_string()));
        assert_eq!(cookies.get("theme"), Some(&"dark".to_string()));
    }

    #[tokio::test]
    async fn test_locale_from_cms_header() {
        use axum::http::Request;

        let req = Request::builder()
            .header("x-cms-locale", "ja")
            .body(())
            .unwrap();
        let locale = Locale::from_request_parts(&mut req.into_parts().0, &())
            .await
            .expect("should resolve locale");
        assert_eq!(locale, Locale::Ja);

        // Invalid/missing header should fall back to default.
        let req = Request::builder().body(()).unwrap();
        let locale = Locale::from_request_parts(&mut req.into_parts().0, &())
            .await
            .expect("should resolve locale");
        assert_eq!(locale, Locale::default());
    }
}
