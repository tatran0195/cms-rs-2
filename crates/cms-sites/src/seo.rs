//! SEO helpers
//!
//! This module provides SEO-related functionality for published sites.

use chrono::{DateTime, Utc};
use cms_entity::{common::Id, page::Page, project::Project};

/// SEO metadata
#[derive(Debug, Clone)]
pub struct SeoMetadata {
    pub title: String,
    pub description: Option<String>,
    pub canonical_url: String,
    pub og_title: String,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub og_type: String,
    pub og_url: String,
    pub twitter_card: String,
    pub twitter_title: String,
    pub twitter_description: Option<String>,
    pub twitter_image: Option<String>,
    pub robots: String,
    pub favicon: Option<String>,
}

/// SEO generator
pub struct SeoGenerator {
    base_url: String,
    default_description: String,
    default_image: Option<String>,
}

impl SeoGenerator {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            default_description: "Documentation powered by CMS".to_string(),
            default_image: None,
        }
    }

    /// Set default description
    pub fn with_default_description(mut self, description: String) -> Self {
        self.default_description = description;
        self
    }

    /// Set default image
    pub fn with_default_image(mut self, image_url: String) -> Self {
        self.default_image = Some(image_url);
        self
    }

    /// Generate SEO metadata for a page
    pub fn generate_page_metadata(&self, page: &Page, project: &Project) -> SeoMetadata {
        let canonical_url = self.build_page_url(project, page);
        let description = page
            .description
            .clone()
            .or(project.description.clone())
            .unwrap_or_else(|| self.default_description.clone());

        SeoMetadata {
            title: format!("{} - {}", page.title, project.name),
            description: Some(description.clone()),
            canonical_url: canonical_url.clone(),
            og_title: format!("{} - {}", page.title, project.name),
            og_description: Some(description.clone()),
            og_image: project.icon.clone().or(self.default_image.clone()),
            og_type: "article".to_string(),
            og_url: canonical_url,
            twitter_card: "summary_large_image".to_string(),
            twitter_title: format!("{} - {}", page.title, project.name),
            twitter_description: Some(description),
            twitter_image: project.icon.clone().or(self.default_image.clone()),
            robots: "index, follow".to_string(),
            favicon: project.icon.clone(),
        }
    }

    /// Generate SEO metadata for a project
    pub fn generate_project_metadata(&self, project: &Project) -> SeoMetadata {
        let canonical_url = self.build_project_url(project);
        let description = project
            .description
            .clone()
            .unwrap_or_else(|| self.default_description.clone());

        SeoMetadata {
            title: project.name.clone(),
            description: Some(description.clone()),
            canonical_url: canonical_url.clone(),
            og_title: project.name.clone(),
            og_description: Some(description.clone()),
            og_image: project.icon.clone().or(self.default_image.clone()),
            og_type: "website".to_string(),
            og_url: canonical_url,
            twitter_card: "summary_large_image".to_string(),
            twitter_title: project.name.clone(),
            twitter_description: Some(description),
            twitter_image: project.icon.clone().or(self.default_image.clone()),
            robots: "index, follow".to_string(),
            favicon: project.icon.clone(),
        }
    }

    /// Build page URL
    fn build_page_url(&self, _project: &Project, page: &Page) -> String {
        let base_url = self.base_url.trim_end_matches('/');
        let path = page.path.trim_matches('/');
        if path.is_empty() {
            format!("{base_url}/")
        } else {
            format!("{base_url}/{path}")
        }
    }

    /// Build the documentation site URL.
    fn build_project_url(&self, _project: &Project) -> String {
        self.base_url.trim_end_matches('/').to_string()
    }

    /// Generate HTML meta tags
    pub fn generate_meta_tags(&self, metadata: &SeoMetadata) -> String {
        let mut tags = Vec::new();

        // Basic meta tags
        tags.push("<meta charset=\"utf-8\">".to_string());
        tags.push(
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
                .to_string(),
        );

        // Description
        if let Some(description) = &metadata.description {
            tags.push(format!(
                "<meta name=\"description\" content=\"{}\">",
                self.escape_html(description)
            ));
        }

        // Canonical URL
        tags.push(format!(
            "<link rel=\"canonical\" href=\"{}\">",
            self.escape_html(&metadata.canonical_url)
        ));

        // Robots
        tags.push(format!(
            "<meta name=\"robots\" content=\"{}\">",
            metadata.robots
        ));

        // Open Graph / Facebook
        tags.push(format!(
            "<meta property=\"og:type\" content=\"{}\">",
            metadata.og_type
        ));
        tags.push(format!(
            "<meta property=\"og:url\" content=\"{}\">",
            self.escape_html(&metadata.og_url)
        ));
        tags.push(format!(
            "<meta property=\"og:title\" content=\"{}\">",
            self.escape_html(&metadata.og_title)
        ));

        if let Some(description) = &metadata.og_description {
            tags.push(format!(
                "<meta property=\"og:description\" content=\"{}\">",
                self.escape_html(description)
            ));
        }

        if let Some(image) = &metadata.og_image {
            tags.push(format!(
                "<meta property=\"og:image\" content=\"{}\">",
                self.escape_html(image)
            ));
            tags.push("<meta property=\"og:image:width\" content=\"1200\">".to_string());
            tags.push("<meta property=\"og:image:height\" content=\"630\">".to_string());
        }

        // Twitter
        tags.push(format!(
            "<meta name=\"twitter:card\" content=\"{}\">",
            metadata.twitter_card
        ));
        tags.push(format!(
            "<meta name=\"twitter:title\" content=\"{}\">",
            self.escape_html(&metadata.twitter_title)
        ));

        if let Some(description) = &metadata.twitter_description {
            tags.push(format!(
                "<meta name=\"twitter:description\" content=\"{}\">",
                self.escape_html(description)
            ));
        }

        if let Some(image) = &metadata.twitter_image {
            tags.push(format!(
                "<meta name=\"twitter:image\" content=\"{}\">",
                self.escape_html(image)
            ));
        }

        // Favicon
        if let Some(favicon) = &metadata.favicon {
            tags.push(format!(
                "<link rel=\"icon\" type=\"image/x-icon\" href=\"{}\">",
                self.escape_html(favicon)
            ));
            tags.push(format!(
                "<link rel=\"apple-touch-icon\" sizes=\"180x180\" href=\"{}\">",
                self.escape_html(favicon)
            ));
            tags.push(format!(
                "<link rel=\"icon\" type=\"image/png\" sizes=\"32x32\" href=\"{}\">",
                self.escape_html(favicon)
            ));
            tags.push(format!(
                "<link rel=\"icon\" type=\"image/png\" sizes=\"16x16\" href=\"{}\">",
                self.escape_html(favicon)
            ));
        } else {
            // Default favicon
            tags.push(
                "<link rel=\"icon\" type=\"image/x-icon\" href=\"/favicon.ico\">".to_string(),
            );
        }

        // Manifest
        tags.push("<link rel=\"manifest\" href=\"/site.webmanifest\">".to_string());

        // Apple touch icon
        tags.push("<meta name=\"apple-mobile-web-app-capable\" content=\"yes\">".to_string());
        tags.push(
            "<meta name=\"apple-mobile-web-app-status-bar-style\" content=\"default\">".to_string(),
        );

        // Theme color
        tags.push("<meta name=\"theme-color\" content=\"#ffffff\">".to_string());

        tags.join("\n")
    }

    /// Generate structured data (JSON-LD) for SEO
    pub fn generate_structured_data(&self, metadata: &SeoMetadata, is_page: bool) -> String {
        let data = if is_page {
            serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": metadata.og_title,
                "description": metadata.og_description.as_deref().unwrap_or(""),
                "url": metadata.og_url,
                "image": metadata.og_image,
            })
        } else {
            serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebSite",
                "name": metadata.og_title,
                "description": metadata.og_description.as_deref().unwrap_or(""),
                "url": metadata.og_url,
                "potentialAction": {
                    "@type": "SearchAction",
                    "target": format!("{}/search?q={{search_term_string}}", metadata.og_url),
                    "query-input": "required name=search_term_string",
                },
            })
        };
        let serialized = serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string());
        // JSON is embedded in HTML; escaping '<' prevents a user-controlled
        // title or description from prematurely closing the script element.
        let html_safe_json = serialized.replace('<', "\\u003c");
        format!("<script type=\"application/ld+json\">{html_safe_json}</script>")
    }

    /// Escape HTML special characters
    fn escape_html(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace("'", "&#x27;")
    }
}

/// Page view analytics
#[derive(Debug, Clone)]
pub struct PageViewAnalytics {
    pub page_id: Id,
    pub project_id: Id,
    pub view_count: i64,
    pub unique_visitors: i64,
    pub last_viewed_at: Option<DateTime<Utc>>,
}

/// Sitemap URL entry
#[derive(Debug, Clone)]
pub struct SitemapUrl {
    pub loc: String,
    pub lastmod: Option<String>,
    pub changefreq: Option<String>,
    pub priority: Option<f32>,
}

/// The page metadata required for a sitemap without retaining the Markdown body.
#[derive(Debug, Clone)]
pub struct SitemapPageMetadata {
    pub path: String,
    pub updated_at: DateTime<Utc>,
}

/// Sitemap generator
pub struct SitemapGenerator {
    base_url: String,
}

impl SitemapGenerator {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Generate sitemap XML for a project from full pages (compatibility API).
    pub fn generate_project_sitemap(&self, project: &Project, pages: &[Page]) -> String {
        let metadata = pages
            .iter()
            .map(|page| SitemapPageMetadata {
                path: page.path.clone(),
                updated_at: page.updated_at,
            })
            .collect::<Vec<_>>();
        self.generate_project_sitemap_from_metadata(project, &metadata)
    }

    /// Generate sitemap XML without loading page bodies.
    pub fn generate_project_sitemap_from_metadata(
        &self,
        project: &Project,
        pages: &[SitemapPageMetadata],
    ) -> String {
        let mut urls = Vec::with_capacity(pages.len() + 1);
        let base_url = self.base_url.trim_end_matches('/');

        // The site is hosted at its own subdomain or custom domain, so it has
        // no extra project-slug path prefix.
        urls.push(SitemapUrl {
            loc: format!("{base_url}/"),
            lastmod: Some(project.updated_at.format("%Y-%m-%d").to_string()),
            changefreq: Some("weekly".to_string()),
            priority: Some(1.0),
        });

        // Preserve the complete nested path captured in the release snapshot.
        for page in pages {
            let path = page.path.trim_matches('/');
            let loc = if path.is_empty() {
                format!("{base_url}/")
            } else {
                format!("{base_url}/{path}")
            };
            urls.push(SitemapUrl {
                loc,
                lastmod: Some(page.updated_at.format("%Y-%m-%d").to_string()),
                changefreq: Some("weekly".to_string()),
                priority: Some(0.8),
            });
        }

        self.generate_sitemap_xml(&urls)
    }

    /// Generate sitemap XML
    fn generate_sitemap_xml(&self, urls: &[SitemapUrl]) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        for url in urls {
            xml.push_str("  <url>\n");
            xml.push_str(&format!("    <loc>{}</loc>\n", url.loc));
            if let Some(lastmod) = &url.lastmod {
                xml.push_str(&format!("    <lastmod>{}</lastmod>\n", lastmod));
            }
            if let Some(changefreq) = &url.changefreq {
                xml.push_str(&format!("    <changefreq>{}</changefreq>\n", changefreq));
            }
            if let Some(priority) = &url.priority {
                xml.push_str(&format!("    <priority>{:.1}</priority>\n", priority));
            }
            xml.push_str("  </url>\n");
        }

        xml.push_str("</urlset>\n");
        xml
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_seo_metadata_generation() {
        let generator = SeoGenerator::new("https://example.com".to_string());

        let project = Project {
            id: "proj-1".to_string(),
            organization_id: "org-1".to_string(),
            name: "Test Project".to_string(),
            slug: "test-project".to_string(),
            description: Some("A test project".to_string()),
            icon: Some("https://example.com/icon.png".to_string()),
            is_public: true,
            config: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let page = Page {
            id: "page-1".to_string(),
            project_id: "proj-1".to_string(),
            branch_id: "branch-1".to_string(),
            language_id: None,
            parent_id: None,
            kind: "PAGE".to_string(),
            path: "/test".to_string(),
            slug: "test".to_string(),
            title: "Test Page".to_string(),
            description: Some("A test page".to_string()),
            content: "# Test".to_string(),
            icon: None,
            config: None,
            translation_key: None,
            position: 0,
            is_published: true,
            is_indexed: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let sitemap = SitemapGenerator::new("https://docs.example.com".to_string())
            .generate_project_sitemap(&project, std::slice::from_ref(&page));
        assert!(sitemap.contains("https://docs.example.com/test"));
        assert!(!sitemap.contains("/test-project/test"));

        let metadata = generator.generate_page_metadata(&page, &project);

        assert_eq!(metadata.title, "Test Page - Test Project");
        assert_eq!(metadata.canonical_url, "https://example.com/test");
        assert_eq!(metadata.og_type, "article");
        let project_metadata = generator.generate_project_metadata(&project);
        assert_eq!(project_metadata.canonical_url, "https://example.com");

        let structured = generator.generate_structured_data(&metadata, true);
        let json_text = structured
            .strip_prefix("<script type=\"application/ld+json\">")
            .and_then(|value| value.strip_suffix("</script>"))
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(json_text).unwrap();
        assert_eq!(parsed["url"], "https://example.com/test");
        assert_eq!(parsed["image"], "https://example.com/icon.png");
    }

    #[test]
    fn test_meta_tags_generation() {
        let generator = SeoGenerator::new("https://example.com".to_string());

        let metadata = SeoMetadata {
            title: "Test Page".to_string(),
            description: Some("A test page".to_string()),
            canonical_url: "https://example.com/test".to_string(),
            og_title: "Test Page".to_string(),
            og_description: Some("A test page".to_string()),
            og_image: Some("https://example.com/image.png".to_string()),
            og_type: "article".to_string(),
            og_url: "https://example.com/test".to_string(),
            twitter_card: "summary_large_image".to_string(),
            twitter_title: "Test Page".to_string(),
            twitter_description: Some("A test page".to_string()),
            twitter_image: Some("https://example.com/image.png".to_string()),
            robots: "index, follow".to_string(),
            favicon: Some("https://example.com/favicon.ico".to_string()),
        };

        let tags = generator.generate_meta_tags(&metadata);

        assert!(tags.contains("<meta charset=\"utf-8\">"));
        assert!(tags.contains("<meta name=\"description\" content=\"A test page\">"));
        assert!(tags.contains("<link rel=\"canonical\" href=\"https://example.com/test\">"));
        assert!(tags.contains("<meta property=\"og:title\" content=\"Test Page\">"));
    }
}
