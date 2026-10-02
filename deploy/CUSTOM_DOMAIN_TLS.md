# Custom-domain TLS (Caddy)

CMS-RS owns domain verification and authorization; Caddy owns ACME certificate issuance, private-key storage, and renewal. CMS-RS never receives a private key. `deploy/Caddyfile` is an example edge configuration for Caddy v2 on-demand TLS.

## Setup

1. Put Caddy in front of the CMS API/site server and make both reachable on the same private network (`cms-rs:3000` in the example). Publish ports 80 and 443 and point customer domains to the Caddy edge.
2. Generate a high-entropy secret and configure **the exact same value** as `domain_tls.proxy_secret` in CMS-RS and `CMS_DOMAIN_TLS__PROXY_SECRET` in Caddy. Do not expose the secret to browsers or logs. Caddy removes any client-provided `X-CMS-Proxy-Token` and injects its own value upstream.
3. Persist Caddy's data directory across restarts. Caddy uses its ACME automation to renew certificates; loss of that storage can cause avoidable re-issuance and CA rate limiting.
4. Apply the CMS migrations before enabling the edge, then reload Caddy and CMS-RS.

The Caddy `ask` callback is `/api/public/domains/tls-authorize?domain={host}` (Caddy appends `domain`). It returns success only when the hostname is DNS-verified and bound to an active release of a public, non-taken-down project. Unknown, unverified, unpublished, private, and taken-down hosts fail closed. Database or application failures also prevent issuance.

## Status semantics

- `PENDING`: no successful trusted HTTPS observation has been recorded for the current verified hostname.
- `ACTIVE`: CMS-RS has observed an HTTPS request carrying the configured proxy secret. When Caddy is used, this means its TLS handshake succeeded; the actual certificate and renewal remain in Caddy.
- `ssl_checked_at` is the last such observation. Since Caddy keeps private certificate state, `ssl_certificate` and `ssl_certificate_expires_at` remain empty unless an operator uses a different certificate integration. Admin health counts treat a Caddy-managed observation as recent for seven days; the certificate authority/expiry is not independently polled by CMS-RS.
- Changing a hostname atomically rotates its DNS token, revokes `verified_at`, and clears certificate metadata, ACME order, status, prior error, and last-check time. An unrelated partial update leaves that hostname-bound state unchanged.

This design intentionally avoids storing or exporting private key material. For installations that need precise expiry/error reporting in CMS-RS, add a secured Caddy event/status integration or use a certificate automation provider that reports lifecycle events; do not infer certificate health from DNS verification alone.

## Operational checks

- Confirm the CMS authorization endpoint returns 404 for an unverified domain and 204 for a verified domain with a published, public release.
- Confirm Caddy logs an authorization rejection for an unverified host and obtains/renews a certificate only for authorized hosts.
- After the first successful HTTPS request through Caddy, confirm the domain reports `sslStatus: ACTIVE` and a recent `lastCheckedAt`.
- Run the PostgreSQL-backed product test with `cargo xtask e2e`; its custom-domain checks cover the authorization gate, spoofed-vs-trusted proxy headers, TLS observation, partial-update preservation, and hostname-change invalidation.
