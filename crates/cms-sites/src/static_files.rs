//! Static files serving
//!
//! This module handles serving static files for published sites.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::Path,
    http::{header, StatusCode},
    response::Response,
};
use cms_storage::Storage;

/// Static file server alias for routes
pub type StaticFileServer = StaticFilesHandler;

/// Static files handler
pub struct StaticFilesHandler {
    storage: Arc<dyn Storage>,
    cache_control: String,
    max_age: u64,
}

impl StaticFilesHandler {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            storage,
            cache_control: "public, max-age=300".to_string(),
            max_age: 300,
        }
    }

    pub fn with_cache(storage: Arc<dyn Storage>, cache_control: String, max_age: u64) -> Self {
        Self {
            storage,
            cache_control,
            max_age,
        }
    }

    /// Serve a static file
    pub async fn serve_file(&self, path: &str) -> Result<Response, StatusCode> {
        // Sanitize the path to prevent directory traversal
        let sanitized_path = self.sanitize_path(path);

        // Get file from storage
        let content = match self.storage.get(&sanitized_path).await {
            Ok(content) => content,
            Err(_) => return Err(StatusCode::NOT_FOUND),
        };

        // Determine content type
        let content_type = self.get_content_type(&sanitized_path);

        // Build response
        let mut response = Response::new(Body::from(content));

        // Set headers
        let headers = response.headers_mut();
        if let Ok(val) = header::HeaderValue::from_str(&content_type) {
            headers.insert(header::CONTENT_TYPE, val);
        }
        if let Ok(val) = header::HeaderValue::from_str(&format!("public, max-age={}", self.max_age))
        {
            headers.insert(header::CACHE_CONTROL, val);
        }

        // Add security headers
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            header::HeaderValue::from_static("nosniff"),
        );

        Ok(response)
    }

    /// Sanitize path to prevent directory traversal
    fn sanitize_path(&self, path: &str) -> String {
        // Remove leading slashes
        let path = path.trim_start_matches('/');

        // Normalize path (remove ./ and ../)
        let normalized = std::path::Path::new(path)
            .components()
            .filter(|c| {
                !matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
            .collect::<std::path::PathBuf>();

        // Convert to string
        normalized.to_string_lossy().into_owned()
    }

    /// Get content type for file extension
    fn get_content_type(&self, path: &str) -> String {
        if let Some(ext) = path.rsplit('.').next() {
            match ext.to_lowercase().as_str() {
                // HTML
                "html" => "text/html; charset=utf-8".to_string(),
                // CSS
                "css" => "text/css; charset=utf-8".to_string(),
                // JavaScript
                "js" | "mjs" | "cjs" => "application/javascript; charset=utf-8".to_string(),
                "ts" => "text/typescript; charset=utf-8".to_string(),
                // JSON
                "json" => "application/json; charset=utf-8".to_string(),
                // Images
                "png" => "image/png".to_string(),
                "jpg" | "jpeg" => "image/jpeg".to_string(),
                "gif" => "image/gif".to_string(),
                "svg" | "svgz" => "image/svg+xml; charset=utf-8".to_string(),
                "webp" => "image/webp".to_string(),
                "ico" => "image/x-icon".to_string(),
                "bmp" => "image/bmp".to_string(),
                // Fonts
                "woff" => "font/woff".to_string(),
                "woff2" => "font/woff2".to_string(),
                "ttf" => "font/ttf".to_string(),
                "otf" => "font/otf".to_string(),
                "eot" => "application/vnd.ms-fontobject".to_string(),
                // Documents
                "pdf" => "application/pdf".to_string(),
                "xml" => "application/xml; charset=utf-8".to_string(),
                "txt" | "md" | "markdown" => "text/plain; charset=utf-8".to_string(),
                "csv" => "text/csv; charset=utf-8".to_string(),
                // Archives
                "zip" => "application/zip".to_string(),
                "tar" => "application/x-tar".to_string(),
                "gz" | "gzip" => "application/gzip".to_string(),
                // Video
                "mp4" => "video/mp4".to_string(),
                "webm" => "video/webm".to_string(),
                "ogg" | "ogv" => "video/ogg".to_string(),
                // Audio
                "mp3" => "audio/mpeg".to_string(),
                "wav" => "audio/wav".to_string(),
                "oga" => "audio/ogg".to_string(),
                // Default
                _ => "application/octet-stream".to_string(),
            }
        } else {
            "application/octet-stream".to_string()
        }
    }

    /// Serve CSS file
    pub async fn serve_css(&self, path: Path<String>) -> Result<Response, StatusCode> {
        let file_path = format!("css/{}", path.0);
        self.serve_file(&file_path).await
    }

    /// Serve JavaScript file
    pub async fn serve_js(&self, path: Path<String>) -> Result<Response, StatusCode> {
        let file_path = format!("js/{}", path.0);
        self.serve_file(&file_path).await
    }

    /// Serve image file
    pub async fn serve_image(&self, path: Path<String>) -> Result<Response, StatusCode> {
        let file_path = format!("images/{}", path.0);
        self.serve_file(&file_path).await
    }

    /// Serve font file
    pub async fn serve_font(&self, path: Path<String>) -> Result<Response, StatusCode> {
        let file_path = format!("fonts/{}", path.0);
        self.serve_file(&file_path).await
    }

    /// Serve asset file (from project assets)
    pub async fn serve_asset(&self, path: Path<String>) -> Result<Response, StatusCode> {
        let file_path = format!("assets/{}", path.0);
        self.serve_file(&file_path).await
    }
}

/// Static file server for development
pub struct DevStaticFileServer {
    base_path: String,
}

impl DevStaticFileServer {
    pub fn new(base_path: String) -> Self {
        Self { base_path }
    }

    /// Serve a file from the filesystem (for development)
    pub async fn serve_dev_file(&self, path: &str) -> Result<Response, StatusCode> {
        use std::{fs, path::PathBuf};

        // Sanitize path
        let sanitized = path.trim_start_matches('/');
        let file_path = PathBuf::from(&self.base_path).join(sanitized);

        // Check if path is within base directory
        if !file_path.starts_with(&self.base_path) {
            return Err(StatusCode::FORBIDDEN);
        }

        // Read file
        let content = match fs::read(&file_path) {
            Ok(content) => content,
            Err(_) => return Err(StatusCode::NOT_FOUND),
        };

        // Determine content type
        let content_type = if let Some(ext) = file_path.extension() {
            match ext.to_string_lossy().to_lowercase().as_str() {
                "html" => "text/html; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "js" => "application/javascript; charset=utf-8",
                "json" => "application/json; charset=utf-8",
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "svg" => "image/svg+xml; charset=utf-8",
                "ico" => "image/x-icon",
                _ => "application/octet-stream",
            }
        } else {
            "application/octet-stream"
        };

        // Build response
        let mut response = Response::new(Body::from(content));

        let headers = response.headers_mut();
        if let Ok(val) = header::HeaderValue::from_str(content_type) {
            headers.insert(header::CONTENT_TYPE, val);
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path() {
        let handler = StaticFilesHandler::new(Arc::new(cms_storage::LocalFsStorage::new(
            "/tmp".to_string(),
        )));

        assert_eq!(
            handler.sanitize_path("path/to/file.txt").replace('\\', "/"),
            "path/to/file.txt"
        );
        assert_eq!(
            handler
                .sanitize_path("/path/to/file.txt")
                .replace('\\', "/"),
            "path/to/file.txt"
        );
        assert_eq!(
            handler.sanitize_path("path/../file.txt").replace('\\', "/"),
            "path/file.txt"
        );
        assert_eq!(
            handler.sanitize_path("path/./file.txt").replace('\\', "/"),
            "path/file.txt"
        );
        assert_eq!(
            handler
                .sanitize_path("../../../etc/passwd")
                .replace('\\', "/"),
            "etc/passwd"
        );
    }

    #[test]
    fn test_get_content_type() {
        let handler = StaticFilesHandler::new(Arc::new(cms_storage::LocalFsStorage::new(
            "/tmp".to_string(),
        )));

        assert_eq!(
            handler.get_content_type("file.html"),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            handler.get_content_type("file.css"),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            handler.get_content_type("file.js"),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(handler.get_content_type("file.png"), "image/png");
        assert_eq!(
            handler.get_content_type("file.json"),
            "application/json; charset=utf-8"
        );
        assert_eq!(
            handler.get_content_type("file.unknown"),
            "application/octet-stream"
        );
    }
}
