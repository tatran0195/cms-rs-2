//! CMS Worker Library
//!
//! Background job processing worker implementation.
//! It consumes jobs from the configured queue backend (Redis or in-memory).

use std::sync::Arc;

pub use app_state::WorkerState;
use cms_error::AppError;
use cms_queue::JobQueue;
use tracing::{debug, error, info};

/// Start job consumers
pub async fn start_consumers(
    job_queue: Arc<dyn JobQueue>,
    state: Arc<WorkerState>,
) -> Result<(), AppError> {
    let (_tx, rx) = tokio::sync::watch::channel(false);
    start_consumers_with_shutdown(job_queue, state, rx).await
}

/// Start job consumers with graceful shutdown support
pub async fn start_consumers_with_shutdown(
    job_queue: Arc<dyn JobQueue>,
    state: Arc<WorkerState>,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<(), AppError> {
    let config = &state.config.queue;
    let num_workers = config.workers;

    info!("Starting {} job consumer(s)", num_workers);

    for i in 0..num_workers {
        let queue = job_queue.clone();
        let state = state.clone();
        let mut worker_shutdown = shutdown_rx.clone();

        tokio::spawn(async move {
            let consumer_name = format!("worker-{}", i);
            if let Err(e) = run_consumer(queue, state, consumer_name, &mut worker_shutdown).await {
                error!("Consumer {} failed: {}", i, e);
            }
        });
    }

    Ok(())
}

/// Run a single job consumer with graceful shutdown handling
pub async fn run_consumer(
    job_queue: Arc<dyn JobQueue>,
    state: Arc<WorkerState>,
    consumer_name: String,
    shutdown_rx: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<(), AppError> {
    info!("Consumer {} started", consumer_name);

    loop {
        if *shutdown_rx.borrow() {
            info!("Consumer {} shutting down gracefully", consumer_name);
            break;
        }

        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Consumer {} received shutdown signal", consumer_name);
                    break;
                }
            }
            res = job_queue.consume(&consumer_name) => {
                match res {
                    Ok(job) => {
                        debug!(
                            "Consumer {} processing job: {:?}",
                            consumer_name, job.job_type
                        );

                        // Process the job — nack on failure so it can be retried
                        match process_job(&job, state.clone()).await {
                            Ok(()) => {
                                if let Err(e) = job_queue.ack(&job.id).await {
                                    error!("Failed to acknowledge job {}: {}", job.id.0, e);
                                }
                            }
                            Err(e) => {
                                error!(
                                    "Failed to process job {} (type={:?}): {}",
                                    job.id.0, job.job_type, e
                                );
                                if let Err(nack_err) = job_queue.nack(&job.id, &e.to_string()).await {
                                    error!(
                                        "Failed to nack job {} after error: {}",
                                        job.id.0, nack_err
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Consumer {} error: {}", consumer_name, e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Process a single job
pub async fn process_job(
    job: &cms_queue::JobEnvelope,
    state: Arc<WorkerState>,
) -> Result<(), AppError> {
    use cms_queue::JobType;

    match job.job_type {
        JobType::Analytics => {
            cms_biz::analytics::process_analytics_job(&state.db, &job.payload).await
        }
        JobType::Email => {
            cms_biz::email::process_email_job(state.mailer.clone(), &job.payload).await
        }
        JobType::Export => {
            cms_biz::export::process_export_job(&state.db, state.storage.clone(), &job.payload)
                .await
        }
        JobType::Git => cms_biz::git::process_git_job(&state.db, &job.payload).await,
        JobType::Publish => {
            cms_biz::deployment::process_deployment_job(
                &state.db,
                state.storage.clone(),
                &job.payload,
            )
            .await
        }
        JobType::Search => {
            cms_biz::search::process_search_job(&state.db, state.search.clone(), &job.payload).await
        }
        JobType::Usage => cms_biz::usage::process_usage_job(&state.db, &job.payload).await,
        JobType::Reaper => {
            cms_biz::queue::process_reaper_job(&state.db, state.job_queue.clone(), &job.payload)
                .await
        }
    }
}

/// Worker application state
pub mod app_state {
    use std::sync::Arc;

    use cms_analytics::AnalyticsStore;
    use cms_biz::email::Mailer;
    use cms_config::Config;
    use cms_db::PgPool;
    use cms_error::AppError;
    use cms_queue::JobQueue;
    use cms_search::SearchEngine;
    use cms_storage::Storage;

    #[derive(Clone)]
    pub struct WorkerState {
        pub config: Arc<Config>,
        pub db: PgPool,
        pub storage: Arc<dyn Storage>,
        pub search: Arc<dyn SearchEngine>,
        pub analytics: Arc<dyn AnalyticsStore>,
        pub mailer: Arc<dyn Mailer>,
        pub job_queue: Arc<dyn JobQueue>,
    }

    impl WorkerState {
        pub async fn new(config: &Config, job_queue: Arc<dyn JobQueue>) -> Result<Self, AppError> {
            let db = cms_db::create_pool(&config.database.url).await?;
            Self::with_pool(config, job_queue, db).await
        }

        pub async fn with_pool(
            config: &Config,
            job_queue: Arc<dyn JobQueue>,
            db: PgPool,
        ) -> Result<Self, AppError> {
            let storage: Arc<dyn Storage> =
                cms_storage::create_storage(&config.storage).await?.into();
            let search =
                cms_search::create_search_engine_with_pool(&config.search, db.clone()).await?;
            let analytics =
                cms_analytics::create_analytics_store(&config.analytics, db.clone()).await?;
            let mailer = cms_biz::email::create_mailer(config.mailer.as_ref())?;

            Ok(Self {
                config: Arc::new(config.clone()),
                db,
                storage,
                search,
                analytics,
                mailer,
                job_queue,
            })
        }

        /// Build a worker over the API server's shared pools and adapters so
        /// single-binary deployments execute jobs rather than marking them done.
        pub async fn from_app_state(
            app: &cms_middleware::app_state::AppState,
        ) -> Result<Self, AppError> {
            let analytics = cms_analytics::create_analytics_store(
                &app.config.analytics,
                app.biz_context.pool.clone(),
            )
            .await?;
            Ok(Self {
                config: app.config.clone(),
                db: app.biz_context.pool.clone(),
                storage: app.storage.clone(),
                search: app.search_engine.clone(),
                analytics,
                mailer: app.mailer.clone(),
                job_queue: app.job_queue.clone(),
            })
        }
    }

    /// Create a mailer based on configuration.
    ///
    /// Returns a real SMTP mailer when `smtp_host` is configured.
    /// Falls back to a no-op mailer (with a warning) when SMTP is not configured —
    /// acceptable for local dev / environments without email.
    pub async fn create_mailer(
        config: &Option<cms_config::MailerConfig>,
    ) -> Result<Arc<dyn Mailer>, AppError> {
        if let Some(cfg) = config {
            if let Some(ref host) = cfg.smtp_host {
                tracing::info!("Using SMTP mailer: {}:{}", host, cfg.smtp_port);
                return Ok(Arc::new(SmtpMailer::new(cfg.clone())?));
            }
        }

        tracing::warn!(
            "No SMTP host configured — emails will be silently discarded. Set \
             CMS_MAILER__SMTP_HOST to enable email delivery."
        );
        Ok(Arc::new(NoopMailer))
    }

    // -----------------------------------------------------------------------
    // SMTP mailer — production path
    // -----------------------------------------------------------------------

    use lettre::{
        message::{header::ContentType, Mailbox},
        transport::smtp::authentication::Credentials,
        AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    };

    /// Real SMTP mailer using `lettre`.
    pub struct SmtpMailer {
        transport: AsyncSmtpTransport<Tokio1Executor>,
        from: Mailbox,
    }

    impl SmtpMailer {
        pub fn new(config: cms_config::MailerConfig) -> Result<Self, AppError> {
            let host = config
                .smtp_host
                .as_deref()
                .ok_or_else(|| AppError::InvalidInput("smtp_host is required".to_string()))?;

            let from_email = config
                .from_email
                .as_deref()
                .unwrap_or("noreply@example.com");
            let from_name = config.from_name.as_deref().unwrap_or("CMS");

            let from: Mailbox = format!("{} <{}>", from_name, from_email)
                .parse()
                .map_err(|e| {
                    AppError::InvalidInput(format!("Invalid from_email address: {}", e))
                })?;

            // Use TLS (port 465) or STARTTLS (port 587) based on config
            let builder = if config.smtp_use_tls {
                AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("SMTP relay error: {}", e)))?
            } else {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("SMTP relay error: {}", e)))?
            };

            let builder =
                if let (Some(user), Some(pass)) = (&config.smtp_username, &config.smtp_password) {
                    builder.credentials(Credentials::new(user.clone(), pass.clone()))
                } else {
                    builder
                };

            let transport = builder.port(config.smtp_port).build();

            Ok(Self { transport, from })
        }
    }

    #[async_trait::async_trait]
    impl Mailer for SmtpMailer {
        async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
            let to_mailbox: Mailbox = to.parse().map_err(|e| {
                AppError::InvalidInput(format!("Invalid recipient address '{}': {}", to, e))
            })?;

            let email = Message::builder()
                .from(self.from.clone())
                .to(to_mailbox)
                .subject(subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body.to_string())
                .map_err(|e| AppError::Internal(e.into()))?;

            self.transport.send(email).await.map_err(|e| {
                tracing::error!("SMTP send failed to={} subject={}: {}", to, subject, e);
                AppError::Internal(e.into())
            })?;

            tracing::debug!("Email sent to={} subject={}", to, subject);
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // No-op mailer — fallback when SMTP is unconfigured
    // -----------------------------------------------------------------------

    /// No-op mailer: drops all emails with a debug log.
    /// Used when no SMTP host is configured.
    pub struct NoopMailer;

    #[async_trait::async_trait]
    impl Mailer for NoopMailer {
        async fn send_email(&self, to: &str, subject: &str, _body: &str) -> Result<(), AppError> {
            tracing::debug!(
                "[NoopMailer] Email discarded to={} subject={} (no SMTP configured)",
                to,
                subject
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use cms_biz::email::Mailer;
    use cms_config::MailerConfig;

    use crate::app_state::*;

    #[tokio::test]
    async fn test_noop_mailer_send() {
        let mailer = NoopMailer;
        let result = mailer
            .send_email("test@example.com", "Test Subject", "Test Body")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_smtp_mailer_validation() {
        // Missing smtp_host should error
        let config = MailerConfig {
            smtp_host: None,
            smtp_port: 587,
            smtp_username: None,
            smtp_password: None,
            smtp_use_tls: false,
            from_email: Some("admin@example.com".to_string()),
            from_name: Some("CMS Admin".to_string()),
        };

        let result = SmtpMailer::new(config);
        assert!(result.is_err());

        // Valid configuration should construct correctly
        let valid_config = MailerConfig {
            smtp_host: Some("smtp.example.com".to_string()),
            smtp_port: 587,
            smtp_username: Some("user".to_string()),
            smtp_password: Some("pass".to_string()),
            smtp_use_tls: false,
            from_email: Some("admin@example.com".to_string()),
            from_name: Some("CMS Admin".to_string()),
        };

        let valid_result = SmtpMailer::new(valid_config);
        assert!(valid_result.is_ok());
    }
}
