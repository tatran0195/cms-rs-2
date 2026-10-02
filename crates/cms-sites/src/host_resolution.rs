//! Host resolution
//!
//! This module handles resolving request hosts to projects and deployments.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::{Duration, Instant},
};

use axum::http::{header, HeaderMap};
use cms_db::{deployment::DeploymentQueries, domain::DomainQueries, project::ProjectQueries};
use cms_entity::common::Id;
use cms_error::AppError;
use sqlx::PgPool;

const HOST_CACHE_MAX_ENTRIES: usize = 4096;

/// Host resolution result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostResolutionResult {
    pub project_id: Id,
    pub deployment_id: Option<Id>,
    pub domain_id: Option<Id>,
    pub is_custom_domain: bool,
    pub hostname: String,
}

/// Cache entry for host resolution
#[derive(Debug, Clone)]
struct HostCacheEntry {
    pub result: Option<HostResolutionResult>,
    pub expires_at: Instant,
    pub generation: u64,
}

/// Host resolver with caching
pub struct HostResolver {
    pool: PgPool,
    default_host: String,
    cache: RwLock<HashMap<String, HostCacheEntry>>,
    cache_ttl: Duration,
    generation: Arc<AtomicU64>,
}

impl HostResolver {
    pub fn new(pool: PgPool, default_host: String) -> Self {
        Self::with_generation(pool, default_host, Arc::new(AtomicU64::new(0)))
    }

    /// Construct a resolver whose cache is invalidated by the shared app-state
    /// generation counter whenever a domain is created, changed, or verified.
    pub fn with_generation(pool: PgPool, default_host: String, generation: Arc<AtomicU64>) -> Self {
        Self {
            pool,
            default_host,
            cache: RwLock::new(HashMap::new()),
            // Keep externally written (out-of-band DB) changes bounded while API
            // writes invalidate immediately through the shared generation counter.
            cache_ttl: Duration::from_secs(30),
            generation,
        }
    }

    /// Resolve host to project
    pub async fn resolve(
        &self,
        headers: &HeaderMap,
    ) -> Result<Option<HostResolutionResult>, AppError> {
        let host = self.get_host(headers)?;
        let generation = self.generation.load(Ordering::Acquire);

        if let Some(cached) = self.cached_result(&host, generation) {
            // If an update raced the read, don't return a result from the old
            // generation; the next lookup will query current database state.
            if self.generation.load(Ordering::Acquire) == generation {
                return Ok(cached);
            }
        }

        let result = self.resolve_from_database(&host).await?;

        // Don't publish a cache entry if a domain mutation happened during the
        // database read. Old entries are generation-tagged and will not be served.
        if self.generation.load(Ordering::Acquire) == generation {
            let now = Instant::now();
            let mut cache = self.cache.write().unwrap();
            cache.retain(|_, entry| entry.generation == generation && entry.expires_at > now);
            if cache.len() >= HOST_CACHE_MAX_ENTRIES {
                if let Some(oldest_host) = cache
                    .iter()
                    .min_by_key(|(_, entry)| entry.expires_at)
                    .map(|(host, _)| host.clone())
                {
                    cache.remove(&oldest_host);
                }
            }
            cache.insert(
                host,
                HostCacheEntry {
                    result: result.clone(),
                    expires_at: now + self.cache_ttl,
                    generation,
                },
            );
        }

        Ok(result)
    }

    fn cached_result(&self, host: &str, generation: u64) -> Option<Option<HostResolutionResult>> {
        self.cache
            .read()
            .unwrap()
            .get(host)
            .filter(|entry| entry.generation == generation && entry.expires_at > Instant::now())
            .map(|entry| entry.result.clone())
    }

    /// Get host from headers
    fn get_host(&self, headers: &HeaderMap) -> Result<String, AppError> {
        // Get from X-Forwarded-Host (for reverse proxy)
        if let Some(forwarded_host) = headers.get("X-Forwarded-Host") {
            if let Ok(host) = forwarded_host.to_str() {
                return Ok(host.to_string());
            }
        }

        // Get from Host header
        if let Some(host) = headers.get(header::HOST) {
            if let Ok(host_str) = host.to_str() {
                // Remove port if present
                let host_without_port = host_str.split(':').next().unwrap_or(host_str);
                return Ok(host_without_port.to_string());
            }
        }

        // Fall back to default host
        Ok(self.default_host.clone())
    }

    /// Resolve host from database
    async fn resolve_from_database(
        &self,
        host: &str,
    ) -> Result<Option<HostResolutionResult>, AppError> {
        // Localhost and loopback IPs always serve the SPA dashboard
        if host == "localhost"
            || host == "127.0.0.1"
            || host == "[::1]"
            || host == "::1"
            || host == "0.0.0.0"
        {
            return Ok(None);
        }

        // Try to find domain by hostname
        if let Some(domain) = DomainQueries::get_verified_by_hostname(&self.pool, host).await? {
            // Get deployment for this domain
            if let Some(deployment) =
                DeploymentQueries::get_by_id(&self.pool, &domain.deployment_id).await?
            {
                // Get project for this deployment
                if let Some(project) =
                    ProjectQueries::get_by_id(&self.pool, &deployment.project_id).await?
                {
                    return Ok(Some(HostResolutionResult {
                        project_id: project.id,
                        deployment_id: Some(deployment.id),
                        domain_id: Some(domain.id),
                        is_custom_domain: true,
                        hostname: domain.hostname,
                    }));
                }
            }
        }

        // If no custom domain found, check if it's the default app domain.
        let bare_default = self
            .default_host
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split(':')
            .next()
            .unwrap_or(&self.default_host)
            .to_string();
        if host == self.default_host
            || host == bare_default
            || host.ends_with(".cms.app")
            || host.ends_with(".cms.com")
            || host.ends_with(".cms.dev")
            || (bare_default.contains('.') && host.ends_with(&format!(".{}", bare_default)))
        {
            // Extract project from subdomain
            if let Some(project_slug) = self.extract_subdomain(host) {
                if let Some(project) =
                    ProjectQueries::get_by_slug_global(&self.pool, &project_slug).await?
                {
                    return Ok(Some(HostResolutionResult {
                        project_id: project.id,
                        deployment_id: None,
                        domain_id: None,
                        is_custom_domain: false,
                        hostname: host.to_string(),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Extract subdomain from host
    fn extract_subdomain(&self, host: &str) -> Option<String> {
        let host = host.strip_suffix(':').unwrap_or(host);

        // Normalize default_host to its bare host part (strip port & scheme).
        let default_host = self
            .default_host
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let bare_default = default_host.split(':').next().unwrap_or(default_host);

        if host == default_host || host == bare_default {
            return None;
        }

        // Split by dots
        let parts: Vec<&str> = host.split('.').collect();

        // If we have at least 3 parts (e.g., project.cms.app), the first part is the subdomain
        if parts.len() >= 3 {
            // Check if the last two parts match our default host
            let last_two = format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1]);
            if last_two == bare_default {
                return Some(parts[0].to_string());
            }

            // Check for .cms.app or .cms.com (legacy app-brand domains).
            if (parts[parts.len() - 1] == "app" || parts[parts.len() - 1] == "com")
                && parts.len() >= 2
                && parts[parts.len() - 2] == "cms"
            {
                return Some(parts[0].to_string());
            }
        }

        // If only 2 parts, the first part might be the subdomain
        if parts.len() == 2 && parts[1] == bare_default {
            return Some(parts[0].to_string());
        }

        None
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.write().unwrap().clear();
    }

    /// Get the number of unexpired entries in the current cache generation.
    pub fn cache_size(&self) -> usize {
        let generation = self.generation.load(Ordering::Acquire);
        let now = Instant::now();
        self.cache
            .read()
            .unwrap()
            .values()
            .filter(|entry| entry.generation == generation && entry.expires_at > now)
            .count()
    }
}

/// Default host resolver
pub fn create_host_resolver(pool: PgPool) -> HostResolver {
    HostResolver::new(pool, "cms.app".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_subdomain() {
        let resolver = HostResolver::new(
            sqlx::PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap(),
            "cms.app".to_string(),
        );

        assert_eq!(
            resolver.extract_subdomain("myproject.cms.app"),
            Some("myproject".to_string())
        );
        assert_eq!(resolver.extract_subdomain("cms.app"), None);
        assert_eq!(resolver.extract_subdomain("localhost:3000"), None);
        assert_eq!(
            resolver.extract_subdomain("myproject.cms.com"),
            Some("myproject".to_string())
        );
    }

    #[tokio::test]
    async fn shared_generation_invalidates_positive_and_negative_cache_entries() {
        let generation = Arc::new(AtomicU64::new(7));
        let resolver = HostResolver::with_generation(
            sqlx::PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap(),
            "cms.app".to_string(),
            generation.clone(),
        );
        let resolved = HostResolutionResult {
            project_id: "project-1".to_string(),
            deployment_id: Some("deployment-1".to_string()),
            domain_id: Some("domain-1".to_string()),
            is_custom_domain: true,
            hostname: "docs.example.com".to_string(),
        };
        let now = Instant::now();
        {
            let mut cache = resolver.cache.write().unwrap();
            cache.insert(
                "docs.example.com".to_string(),
                HostCacheEntry {
                    result: Some(resolved.clone()),
                    expires_at: now + Duration::from_secs(10),
                    generation: 7,
                },
            );
            cache.insert(
                "missing.example.com".to_string(),
                HostCacheEntry {
                    result: None,
                    expires_at: now + Duration::from_secs(10),
                    generation: 7,
                },
            );
        }

        assert_eq!(
            resolver.cached_result("docs.example.com", 7),
            Some(Some(resolved))
        );
        assert_eq!(resolver.cached_result("missing.example.com", 7), Some(None));
        assert_eq!(resolver.cache_size(), 2);

        generation.fetch_add(1, Ordering::AcqRel);
        assert_eq!(resolver.cached_result("docs.example.com", 8), None);
        assert_eq!(resolver.cached_result("missing.example.com", 8), None);
        assert_eq!(resolver.cache_size(), 0);
    }
}
