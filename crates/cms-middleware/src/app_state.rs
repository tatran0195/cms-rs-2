//! Application state
//!
//! This module contains the shared application state that is passed to all handlers.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use cms_biz::BizContext;
use cms_config::Config;
use cms_error::AppError;
use cms_queue::JobQueue;
use cms_storage::Storage;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// Application configuration
    pub config: Arc<Config>,
    /// Business context with database pool and access control
    pub biz_context: BizContext,
    /// Storage backend
    pub storage: Arc<dyn Storage>,
    /// Job queue
    pub job_queue: Arc<dyn JobQueue>,
    /// Search engine
    pub search_engine: Arc<dyn cms_search::SearchEngine>,
    /// Outbound email delivery used by auth and workflow handlers
    pub mailer: Arc<dyn cms_biz::email::Mailer>,
    /// Generation counter used to invalidate cached host-to-project resolutions.
    pub host_resolution_generation: Arc<AtomicU64>,
}

impl AppState {
    /// Create AppState from full configuration
    pub async fn from_config(config: &Config) -> Result<Self, AppError> {
        let pool = cms_db::create_pool(&config.database.url).await?;
        cms_db::run_migrations(&pool).await?;
        let storage_box = cms_storage::create_storage(&config.storage).await?;
        let storage: Arc<dyn Storage> = Arc::from(storage_box);
        let job_queue = cms_queue::create_job_queue_with_pool(&config.queue, pool.clone()).await?;
        let search_engine =
            cms_search::create_search_engine_with_pool(&config.search, pool.clone()).await?;
        let authz = Arc::new(cms_authz::ProductionAuthz::new_with_admin_emails(
            pool.clone(),
            config.auth.system_admin_emails.clone(),
        ));
        let biz_context = BizContext::new(pool, authz);
        let mailer = cms_biz::email::create_mailer(config.mailer.as_ref())?;

        Ok(Self::new_with_mailer(
            config.clone(),
            biz_context,
            storage,
            job_queue,
            search_engine,
            mailer,
        ))
    }

    /// Create a new AppState with provided dependencies
    pub fn new(
        config: Config,
        biz_context: BizContext,
        storage: Arc<dyn Storage>,
        job_queue: Arc<dyn JobQueue>,
        search_engine: Arc<dyn cms_search::SearchEngine>,
    ) -> Self {
        Self::new_with_mailer(
            config,
            biz_context,
            storage,
            job_queue,
            search_engine,
            Arc::new(cms_biz::email::UnconfiguredMailer),
        )
    }

    /// Create state with an explicitly configured mail provider.
    pub fn new_with_mailer(
        config: Config,
        biz_context: BizContext,
        storage: Arc<dyn Storage>,
        job_queue: Arc<dyn JobQueue>,
        search_engine: Arc<dyn cms_search::SearchEngine>,
        mailer: Arc<dyn cms_biz::email::Mailer>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            biz_context,
            storage,
            job_queue,
            search_engine,
            mailer,
            host_resolution_generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Invalidate cached host-to-project resolutions after a domain change.
    pub fn invalidate_host_resolution_cache(&self) {
        self.host_resolution_generation
            .fetch_add(1, Ordering::AcqRel);
    }

    /// Validate middleware configurations from config
    pub fn validate_config(_config: &Config) -> Result<(), AppError> {
        Ok(())
    }
}
