//! CMS Storage
//!
//! This crate provides a Storage trait with two implementations:
//! - LocalFsStorage (default): stores files on the local filesystem
//! - S3Storage (optional): stores files on S3-compatible storage
//!
//! The trait is designed so that cms-biz can be tested against a fake
//! implementation without a live storage backend.

use std::{path::Path, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use cms_config::StorageConfig;
use cms_error::AppError;

/// Upload target for file uploads
#[derive(Debug, Clone)]
pub enum UploadTarget {
    /// S3 backend: the browser PUTs directly to this presigned URL
    Presigned(String),
    /// Local backend: the browser PUTs to this server endpoint instead
    ServerMediated(String),
}

/// Download target for file downloads
#[derive(Debug, Clone)]
pub enum DownloadTarget {
    /// S3 backend: direct download URL
    Presigned(String),
    /// Local backend: download through the server
    ServerMediated(String),
}

/// The Storage trait defines the interface for object storage
#[async_trait]
pub trait Storage: Send + Sync {
    /// Put an object into storage
    async fn put(&self, key: &str, body: Bytes, content_type: &str) -> Result<(), AppError>;

    /// Get an object from storage
    async fn get(&self, key: &str) -> Result<Bytes, AppError>;

    /// Delete an object from storage
    async fn delete(&self, key: &str) -> Result<(), AppError>;

    /// Get an upload target (presigned URL or server endpoint)
    async fn upload_target(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<UploadTarget, AppError>;

    /// Get a download target (presigned URL or server endpoint)
    async fn download_target(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<DownloadTarget, AppError>;

    /// Ensure the storage backend is ready (create buckets/directories)
    async fn ensure_ready(&self) -> Result<(), AppError>;

    /// Check if an object exists
    async fn exists(&self, key: &str) -> Result<bool, AppError>;

    /// List objects with a given prefix
    async fn list(&self, prefix: &str) -> Result<Vec<String>, AppError>;
}

/// Create a Storage implementation based on configuration
pub async fn create_storage(config: &StorageConfig) -> Result<Box<dyn Storage>, AppError> {
    match config.backend.as_str() {
        "local" => {
            let root_dir = config.local_root.as_deref().unwrap_or("./storage");
            Ok(Box::new(LocalFsStorage::new(root_dir.to_string())))
        }
        "s3" => {
            #[cfg(feature = "s3")]
            {
                if let (Some(endpoint), Some(bucket), Some(access_key), Some(secret_key)) = (
                    &config.s3_endpoint,
                    &config.s3_bucket,
                    &config.s3_access_key,
                    &config.s3_secret_key,
                ) {
                    Ok(Box::new(
                        S3Storage::new(
                            endpoint.clone(),
                            bucket.clone(),
                            access_key.clone(),
                            secret_key.clone(),
                            config.s3_region.clone().unwrap_or_default(),
                            config.s3_path_style,
                        )
                        .await,
                    ))
                } else {
                    Err(AppError::StorageNotConfigured)
                }
            }
            #[cfg(not(feature = "s3"))]
            {
                Err(AppError::Storage(
                    "S3 backend requires the 's3' feature to be enabled".to_string(),
                ))
            }
        }
        _ => Err(AppError::Storage(format!(
            "Unknown storage backend: {}",
            config.backend
        ))),
    }
}

/// Local filesystem storage implementation
pub struct LocalFsStorage {
    root_dir: String,
}

impl LocalFsStorage {
    /// Create a new LocalFsStorage instance
    pub fn new(root_dir: String) -> Self {
        Self { root_dir }
    }

    /// Get the safe full path for a key, preventing path traversal attacks
    fn get_path(&self, key: &str) -> Result<std::path::PathBuf, AppError> {
        let normalized = key.replace('\\', "/");
        let path = Path::new(&normalized);
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    return Err(AppError::InvalidInput(
                        "Path traversal detected".to_string(),
                    ));
                }
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    return Err(AppError::InvalidInput(
                        "Absolute paths not allowed in storage keys".to_string(),
                    ));
                }
                _ => {}
            }
        }

        let root = Path::new(&self.root_dir);
        Ok(root.join(path))
    }

    /// Ensure parent directory exists
    async fn ensure_parent_exists(&self, key: &str) -> Result<(), AppError> {
        let path = self.get_path(key)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Storage(format!("Failed to create directory: {}", e)))?;
        }
        Ok(())
    }
}

#[async_trait]
impl Storage for LocalFsStorage {
    async fn put(&self, key: &str, body: Bytes, _content_type: &str) -> Result<(), AppError> {
        self.ensure_parent_exists(key).await?;

        let path = self.get_path(key)?;
        tokio::fs::write(&path, &body)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to write file: {}", e)))?;

        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Bytes, AppError> {
        let path = self.get_path(key)?;
        let bytes = tokio::fs::read(&path).await.map_err(|e| {
            if e.to_string().contains("No such file") {
                AppError::ObjectNotFound(key.to_string())
            } else {
                AppError::Storage(format!("Failed to read file: {}", e))
            }
        })?;

        Ok(Bytes::from(bytes))
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let path = self.get_path(key)?;
        match tokio::fs::remove_file(&path).await {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.to_string().contains("No such file") {
                    // It's okay if the file doesn't exist
                    Ok(())
                } else {
                    Err(AppError::Storage(format!("Failed to delete file: {}", e)))
                }
            }
        }
    }

    async fn upload_target(
        &self,
        key: &str,
        _expires_in: Duration,
    ) -> Result<UploadTarget, AppError> {
        // Validate key before returning target
        let _ = self.get_path(key)?;
        // For local storage, uploads are server-mediated
        Ok(UploadTarget::ServerMediated(format!(
            "/api/app/assets/{}/upload",
            key
        )))
    }

    async fn download_target(
        &self,
        key: &str,
        _expires_in: Duration,
    ) -> Result<DownloadTarget, AppError> {
        // Validate key before returning target
        let _ = self.get_path(key)?;
        // For local storage, downloads are server-mediated
        Ok(DownloadTarget::ServerMediated(format!(
            "/api/app/assets/{}",
            key
        )))
    }

    async fn ensure_ready(&self) -> Result<(), AppError> {
        tokio::fs::create_dir_all(&self.root_dir)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to create root directory: {}", e)))?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, AppError> {
        let path = self.get_path(key)?;
        Ok(tokio::fs::try_exists(&path)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to check file existence: {}", e)))?)
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, AppError> {
        let root_path = Path::new(&self.root_dir);
        let prefix_path = if prefix.is_empty() {
            root_path.to_path_buf()
        } else {
            let normalized = prefix.replace('\\', "/");
            let path = Path::new(&normalized);
            for component in path.components() {
                if matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                ) {
                    return Err(AppError::InvalidInput(
                        "Path traversal detected".to_string(),
                    ));
                }
            }
            root_path.join(path)
        };

        let mut keys = Vec::new();

        if prefix_path.exists() {
            let mut dir = tokio::fs::read_dir(&prefix_path)
                .await
                .map_err(|e| AppError::Storage(format!("Failed to read directory: {}", e)))?;

            while let Some(entry) = dir
                .next_entry()
                .await
                .map_err(|e| AppError::Storage(format!("Failed to read directory entry: {}", e)))?
            {
                let path = entry.path();
                let key = path
                    .strip_prefix(root_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");

                if entry
                    .file_type()
                    .await
                    .map_err(|e| AppError::Storage(format!("Failed to get file type: {}", e)))?
                    .is_file()
                {
                    keys.push(key);
                }
            }
        }

        Ok(keys)
    }
}

/// S3 storage implementation
#[cfg(feature = "s3")]
pub struct S3Storage {
    client: aws_sdk_s3::Client,
    bucket: String,
    use_path_style: bool,
    endpoint: String,
}

#[cfg(feature = "s3")]
impl S3Storage {
    /// Create a new S3Storage instance
    pub async fn new(
        endpoint: String,
        bucket: String,
        access_key: String,
        secret_key: String,
        region: String,
        use_path_style: bool,
    ) -> Self {
        use aws_config::BehaviorVersion;

        let config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(endpoint.clone())
            .region(aws_config::Region::new(region.clone()))
            .load()
            .await;

        // Override credentials if provided
        let config = if !access_key.is_empty() && !secret_key.is_empty() {
            aws_config::defaults(BehaviorVersion::latest())
                .endpoint_url(endpoint.clone())
                .region(aws_config::Region::new(region))
                .credentials_provider(aws_credential_types::Credentials::new(
                    &access_key,
                    &secret_key,
                    None,
                    None,
                    "loaded-from-config",
                ))
                .load()
                .await
        } else {
            config
        };

        let client = aws_sdk_s3::Client::new(&config);
        let host = endpoint.replace("http://", "").replace("https://", "");

        Self {
            client,
            bucket,
            use_path_style,
            endpoint: host,
        }
    }

    /// Get the S3 key from a storage key
    fn get_s3_key(&self, key: &str) -> String {
        // Remove leading slash if present
        key.trim_start_matches('/').to_string()
    }

    /// Get the public URL for an object
    fn get_public_url(&self, key: &str) -> String {
        let s3_key = self.get_s3_key(key);
        if self.use_path_style {
            format!("https://{}/{}/{}", self.endpoint, self.bucket, s3_key)
        } else {
            format!("https://{}.{}/{}", self.bucket, self.endpoint, s3_key)
        }
    }
}

#[cfg(feature = "s3")]
#[async_trait]
impl Storage for S3Storage {
    async fn put(&self, key: &str, body: Bytes, content_type: &str) -> Result<(), AppError> {
        let s3_key = self.get_s3_key(key);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .body(body.into())
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| AppError::Storage(format!("S3 put failed: {}", e)))?;

        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Bytes, AppError> {
        let s3_key = self.get_s3_key(key);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NoSuchKey") {
                    AppError::ObjectNotFound(key.to_string())
                } else {
                    AppError::Storage(format!("S3 get failed: {}", e))
                }
            })?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| AppError::Storage(format!("Failed to collect S3 response: {}", e)))?
            .into_bytes();

        Ok(bytes)
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let s3_key = self.get_s3_key(key);

        match self
            .client
            .delete_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.to_string().contains("NoSuchKey") {
                    Ok(())
                } else {
                    Err(AppError::Storage(format!("S3 delete failed: {}", e)))
                }
            }
        }
    }

    async fn upload_target(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<UploadTarget, AppError> {
        let s3_key = self.get_s3_key(key);

        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
            .map_err(|e| AppError::Storage(format!("Invalid expires_in: {}", e)))?;

        let response = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to create presigned URL: {}", e)))?;

        Ok(UploadTarget::Presigned(response.uri().to_string()))
    }

    async fn download_target(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<DownloadTarget, AppError> {
        let s3_key = self.get_s3_key(key);

        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
            .map_err(|e| AppError::Storage(format!("Invalid expires_in: {}", e)))?;

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to create presigned URL: {}", e)))?;

        Ok(DownloadTarget::Presigned(response.uri().to_string()))
    }

    async fn ensure_ready(&self) -> Result<(), AppError> {
        // Check if bucket exists, create if not
        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.to_string().contains("NotFound") || e.to_string().contains("NoSuchBucket") {
                    self.client
                        .create_bucket()
                        .bucket(&self.bucket)
                        .send()
                        .await
                        .map_err(|e| {
                            AppError::Storage(format!("Failed to create bucket: {}", e))
                        })?;
                    Ok(())
                } else {
                    Err(AppError::Storage(format!("Bucket check failed: {}", e)))
                }
            }
        }
    }

    async fn exists(&self, key: &str) -> Result<bool, AppError> {
        let s3_key = self.get_s3_key(key);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) if e.to_string().contains("NotFound") || e.to_string().contains("NoSuchKey") => {
                Ok(false)
            }
            Err(e) => Err(AppError::Storage(format!("Head object failed: {}", e))),
        }
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, AppError> {
        let response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| AppError::Storage(format!("List objects failed: {}", e)))?;

        let keys = response
            .contents()
            .iter()
            .map(|obj| obj.key().unwrap_or_default().to_string())
            .collect();

        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn test_local_storage_put_get() {
        let dir = tempdir().unwrap();
        let storage = LocalFsStorage::new(dir.path().to_string_lossy().into_owned());

        storage.ensure_ready().await.unwrap();

        let key = "test/file.txt";
        let content = Bytes::from("Hello, World!");

        storage
            .put(key, content.clone(), "text/plain")
            .await
            .unwrap();

        let retrieved = storage.get(key).await.unwrap();
        assert_eq!(retrieved, content);
    }

    #[tokio::test]
    async fn test_local_storage_delete() {
        let dir = tempdir().unwrap();
        let storage = LocalFsStorage::new(dir.path().to_string_lossy().into_owned());

        storage.ensure_ready().await.unwrap();

        let key = "test/file.txt";
        let content = Bytes::from("Hello, World!");

        storage.put(key, content, "text/plain").await.unwrap();
        assert!(storage.exists(key).await.unwrap());

        storage.delete(key).await.unwrap();
        assert!(!storage.exists(key).await.unwrap());
    }

    #[tokio::test]
    async fn test_local_storage_list() {
        let dir = tempdir().unwrap();
        let storage = LocalFsStorage::new(dir.path().to_string_lossy().into_owned());

        storage.ensure_ready().await.unwrap();

        storage
            .put("test/file1.txt", Bytes::from("content1"), "text/plain")
            .await
            .unwrap();
        storage
            .put("test/file2.txt", Bytes::from("content2"), "text/plain")
            .await
            .unwrap();
        storage
            .put("other/file.txt", Bytes::from("content3"), "text/plain")
            .await
            .unwrap();

        let keys = storage.list("test/").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().any(|k| k.contains("file1.txt")));
        assert!(keys.iter().any(|k| k.contains("file2.txt")));
    }

    #[tokio::test]
    async fn test_local_storage_upload_target() {
        let dir = tempdir().unwrap();
        let storage = LocalFsStorage::new(dir.path().to_string_lossy().into_owned());

        let target = storage
            .upload_target("test/file.txt", Duration::from_secs(3600))
            .await
            .unwrap();

        match target {
            UploadTarget::ServerMediated(url) => {
                assert!(url.contains("/api/app/assets/test/file.txt/upload"));
            }
            UploadTarget::Presigned(_) => panic!("Expected ServerMediated for local storage"),
        }
    }

    #[tokio::test]
    async fn test_local_storage_path_traversal_prevention() {
        let dir = tempdir().unwrap();
        let storage = LocalFsStorage::new(dir.path().to_string_lossy().into_owned());

        // Relative parent traversal
        assert!(storage
            .put("../evil.txt", Bytes::from("payload"), "text/plain")
            .await
            .is_err());
        assert!(storage.get("../evil.txt").await.is_err());
        assert!(storage.delete("foo/../../evil.txt").await.is_err());
        assert!(storage.exists("nested/../../../etc/passwd").await.is_err());
        assert!(storage.list("../").await.is_err());

        // Absolute path attempts
        assert!(storage
            .put("/root/file.txt", Bytes::from("payload"), "text/plain")
            .await
            .is_err());
    }
}
