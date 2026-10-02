//! Domain Business Logic
//!
//! This module contains business logic for custom domains.
//! Note: Most domain logic is in the deployment module, this is for
//! domain-specific operations that don't involve deployments.

use std::time::Duration;

use cms_db::{domain::DomainQueries, project::ProjectQueries};
use cms_entity::{
    common::{Id, MemberRole},
    domain::{Domain, DomainResponse},
};

use crate::{AppError, BizContext};

/// Domain service
pub struct DomainService;

impl DomainService {
    /// Verify domain ownership using the persisted, unpredictable TXT challenge.
    ///
    /// The challenge must be published at `_cms-rs-verification.<hostname>` as
    /// `cms-rs-verification=<token>`. DNS is queried over HTTPS through a configured
    /// resolver endpoint, so the hostname cannot be used to make arbitrary HTTP
    /// requests from the CMS server.
    pub async fn verify_domain_ownership(
        ctx: &BizContext,
        user_id: &str,
        domain_id: &str,
        verification_token: &str,
        resolver_url: &str,
        timeout_seconds: u64,
    ) -> Result<bool, AppError> {
        let domain = DomainQueries::get_by_id(&ctx.pool, domain_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Domain not found".to_string()))?;

        let deployment =
            cms_db::deployment::DeploymentQueries::get_by_id(&ctx.pool, &domain.deployment_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Admin)
            .await?;

        if verification_token != domain.verification_token {
            return Err(AppError::InvalidToken(
                "Domain challenge does not match".to_string(),
            ));
        }

        let record_name = format!("_cms-rs-verification.{}", domain.hostname);
        let expected = format!("cms-rs-verification={}", domain.verification_token);
        let verified =
            lookup_txt_challenge(resolver_url, timeout_seconds, &record_name, &expected).await?;

        if verified {
            DomainQueries::verify(&ctx.pool, domain_id, &domain.verification_token).await?;
        } else if domain.verified_at.is_some() {
            // An explicit failed re-check revokes the routing grant. The request
            // path itself never trusts an unverified database row.
            DomainQueries::unverify(&ctx.pool, domain_id, &domain.verification_token).await?;
        }

        Ok(verified)
    }

    /// Check if a domain is available
    pub async fn is_domain_available(ctx: &BizContext, hostname: &str) -> Result<bool, AppError> {
        let hostname = cms_db::domain::normalize_hostname(hostname)?;
        let domain = DomainQueries::get_by_hostname(&ctx.pool, &hostname).await?;
        Ok(domain.is_none())
    }

    /// Get domain by hostname
    pub async fn get_domain_by_hostname(
        ctx: &BizContext,
        hostname: &str,
    ) -> Result<Option<DomainResponse>, AppError> {
        let domain = DomainQueries::get_by_hostname(&ctx.pool, hostname).await?;
        Ok(domain.map(|d| d.into()))
    }

    /// List domains
    pub async fn list_domains(
        ctx: &BizContext,
        user_id: &str,
        deployment_id: Option<&str>,
        _is_primary: Option<bool>,
        _is_verified: Option<bool>,
    ) -> Result<Vec<DomainResponse>, AppError> {
        let deployment_id = deployment_id.unwrap_or("");
        crate::deployment::DeploymentService::list_domains(ctx, user_id, deployment_id).await
    }

    /// Create domain
    pub async fn create_domain(
        ctx: &BizContext,
        user_id: &str,
        request: cms_entity::domain::CreateDomainRequest,
    ) -> Result<DomainResponse, AppError> {
        let deployment_id = request.deployment_id.clone();
        crate::deployment::DeploymentService::create_domain(ctx, user_id, &deployment_id, request)
            .await
    }

    /// Get domain
    pub async fn get_domain(
        ctx: &BizContext,
        user_id: &str,
        domain_id: &str,
    ) -> Result<DomainResponse, AppError> {
        crate::deployment::DeploymentService::get_domain(ctx, user_id, domain_id).await
    }

    /// Update domain
    pub async fn update_domain(
        ctx: &BizContext,
        user_id: &str,
        domain_id: &str,
        request: cms_entity::domain::UpdateDomainRequest,
    ) -> Result<DomainResponse, AppError> {
        crate::deployment::DeploymentService::update_domain(ctx, user_id, domain_id, request).await
    }

    /// Delete domain
    pub async fn delete_domain(
        ctx: &BizContext,
        user_id: &str,
        domain_id: &str,
    ) -> Result<bool, AppError> {
        crate::deployment::DeploymentService::delete_domain(ctx, user_id, domain_id).await
    }

    /// Verify domain
    pub async fn verify_domain(
        ctx: &BizContext,
        user_id: &str,
        domain_id: &str,
        verification_token: &str,
        resolver_url: &str,
        timeout_seconds: u64,
    ) -> Result<cms_entity::domain::DomainVerificationResult, AppError> {
        let verified = Self::verify_domain_ownership(
            ctx,
            user_id,
            domain_id,
            verification_token,
            resolver_url,
            timeout_seconds,
        )
        .await?;
        let domain = DomainQueries::get_by_id(&ctx.pool, domain_id).await?;
        let hostname = domain.map(|d| d.hostname).unwrap_or_default();
        Ok(cms_entity::domain::DomainVerificationResult {
            domain_id: domain_id.to_string(),
            hostname,
            is_verified: verified,
            verification_token: Some(verification_token.to_string()),
        })
    }

    /// Set primary domain
    pub async fn set_primary_domain(
        ctx: &BizContext,
        user_id: &str,
        deployment_id: &str,
        domain_id: &str,
    ) -> Result<DomainResponse, AppError> {
        let deployment = cms_db::deployment::DeploymentQueries::get_by_id(&ctx.pool, deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Admin)
            .await?;
        let updated =
            DomainQueries::set_primary_for_deployment(&ctx.pool, deployment_id, domain_id).await?;
        Ok(updated.into())
    }
}

#[derive(serde::Deserialize)]
struct DnsJsonAnswer {
    #[serde(rename = "type")]
    record_type: u16,
    data: String,
}

#[derive(serde::Deserialize)]
struct DnsJsonResponse {
    #[serde(rename = "Status")]
    status: u16,
    #[serde(rename = "Answer", default)]
    answers: Vec<DnsJsonAnswer>,
}

fn dns_answers_contain_challenge(response: &DnsJsonResponse, expected_challenge: &str) -> bool {
    response.status == 0
        && response.answers.iter().any(|answer| {
            answer.record_type == 16 && answer.data.trim().trim_matches('"') == expected_challenge
        })
}

async fn lookup_txt_challenge(
    resolver_url: &str,
    timeout_seconds: u64,
    record_name: &str,
    expected_challenge: &str,
) -> Result<bool, AppError> {
    let resolver = reqwest::Url::parse(resolver_url)
        .map_err(|error| AppError::InvalidInput(format!("Invalid DNS resolver URL: {error}")))?;
    if resolver.scheme() != "https" {
        return Err(AppError::InvalidInput(
            "DNS resolver URL must use HTTPS".to_string(),
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds.clamp(1, 15)))
        .build()
        .map_err(|error| AppError::Internal(error.into()))?;
    let response = client
        .get(resolver)
        .query(&[("name", record_name), ("type", "TXT")])
        .header(reqwest::header::ACCEPT, "application/dns-json")
        .send()
        .await
        .map_err(|error| AppError::ProviderError(format!("DNS TXT lookup failed: {error}")))?
        .error_for_status()
        .map_err(|error| {
            AppError::ProviderError(format!("DNS resolver returned an error: {error}"))
        })?;
    let dns_response = response.json::<DnsJsonResponse>().await.map_err(|error| {
        AppError::ProviderError(format!("Invalid DNS resolver response: {error}"))
    })?;

    Ok(dns_answers_contain_challenge(
        &dns_response,
        expected_challenge,
    ))
}

#[cfg(test)]
mod domain_verification_tests {
    use super::*;

    #[test]
    fn dns_txt_proof_requires_the_exact_txt_answer_and_success_status() {
        let response: DnsJsonResponse = serde_json::from_value(serde_json::json!({
            "Status": 0,
            "Answer": [
                { "type": 1, "data": "192.0.2.1" },
                { "type": 16, "data": "\"cms-rs-verification=expected-token\"" }
            ]
        }))
        .unwrap();
        assert!(dns_answers_contain_challenge(
            &response,
            "cms-rs-verification=expected-token"
        ));
        assert!(!dns_answers_contain_challenge(
            &response,
            "cms-rs-verification=other-token"
        ));

        let nxdomain: DnsJsonResponse = serde_json::from_value(serde_json::json!({
            "Status": 3,
            "Answer": []
        }))
        .unwrap();
        assert!(!dns_answers_contain_challenge(
            &nxdomain,
            "cms-rs-verification=expected-token"
        ));
    }
}
