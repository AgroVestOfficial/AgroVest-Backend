//! Rate limiting middleware.
//!
//! Three tiers: strict (auth flows), write (authenticated mutations), and
//! read (everything else). Each tier defines its own per-minute quota and
//! its own keying strategy.
//!
//! Distributed limiting is the default path: a fixed one-minute bucket is
//! kept in Redis via INCR plus EXPIRE so multiple application instances
//! behind a load balancer share the same counter. If Redis becomes
//! unreachable the middleware degrades to an in-process governor keyed
//! limiter so the API stays up; a single warning is logged when this
//! happens so the degradation is observable but does not flood the logs.

use std::collections::HashMap;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use ipnet::IpNet;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use redis::AsyncCommands;
use serde_json::json;

use crate::app_state::AppState;
use crate::config::AppConfig;
use crate::middleware::auth::Claims;

/// Window length for the fixed-bucket Redis counter, in seconds.
const WINDOW_SECONDS: u64 = 60;

/// TTL applied to the Redis counter key. A little longer than the window
/// so a key that races a tick boundary still expires cleanly.
const KEY_TTL_SECONDS: u64 = 90;

const TIER_STRICT: &str = "auth";
const TIER_WRITE: &str = "write";
const TIER_READ: &str = "read";

/// One tier of the rate limit policy.
#[derive(Clone, Copy, Debug)]
pub struct Tier {
    pub name: &'static str,
    pub limit_per_minute: u32,
    pub keying: Keying,
}

impl Tier {
    pub const fn strict(limit_per_minute: u32) -> Self {
        Self {
            name: TIER_STRICT,
            limit_per_minute,
            keying: Keying::Ip,
        }
    }

    pub const fn write(limit_per_minute: u32) -> Self {
        Self {
            name: TIER_WRITE,
            limit_per_minute,
            keying: Keying::UserOrIp,
        }
    }

    pub const fn read(limit_per_minute: u32) -> Self {
        Self {
            name: TIER_READ,
            limit_per_minute,
            keying: Keying::Ip,
        }
    }
}

/// How a request is keyed against the limiter.
#[derive(Clone, Copy, Debug)]
pub enum Keying {
    /// Always key by client IP.
    Ip,
    /// Try the JWT `sub` first; fall back to client IP when no usable token is present.
    UserOrIp,
}

/// Pick the tier that applies to `(method, path)` for the given config.
///
/// The strict tier covers the auth flows that can exhaust Redis or be
/// brute-forced. The write tier covers authenticated mutations. Everything
/// else (GETs, /auth/me, OPTIONS preflight) lands on the read tier.
pub fn tier_for(method: &Method, path: &str, config: &AppConfig) -> Tier {
    let strict = matches!(
        (method, path),
        (&Method::POST, "/api/v1/auth/nonce") | (&Method::POST, "/api/v1/auth/verify")
    );
    if strict {
        return Tier::strict(config.rate_limit_auth);
    }

    if matches!(
        method,
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    ) && is_write_path(path)
    {
        return Tier::write(config.rate_limit_write);
    }

    Tier::read(config.rate_limit_global)
}

/// Paths that count as "writes" for the write tier. The strict tier above
/// captures /auth/nonce and /auth/verify before this is consulted, so the
/// auth flows do not double-count here.
fn is_write_path(path: &str) -> bool {
    const WRITE_PREFIXES: &[&str] = &[
        "/api/v1/escrows",
        "/api/v1/investments",
        "/api/v1/proposals",
        "/api/v1/disputes",
        "/api/v1/upload",
        "/api/v1/farms",
        "/api/v1/products",
        "/api/v1/reviews",
        "/api/v1/cart",
        "/api/v1/users",
    ];
    WRITE_PREFIXES.iter().any(|p| path.starts_with(p))
}

/// Lazily initialised process-wide state for the in-memory fallback path
/// and the parsed trusted-proxy list. Initialised on the first request,
/// then reused.
struct LocalState {
    trusted_proxies: Vec<IpNet>,
    jwt_decoding_key: DecodingKey,
    redis_healthy: AtomicBool,
    warned_once: AtomicBool,
    fallbacks: HashMap<&'static str, Arc<LocalLimiter>>,
}

type LocalLimiter = RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>;

static LOCAL_STATE: OnceLock<LocalState> = OnceLock::new();

fn local_state(app: &AppState) -> &'static LocalState {
    LOCAL_STATE.get_or_init(|| {
        let trusted_proxies = parse_trusted_proxies(&app.config.trusted_proxies);
        let jwt_decoding_key = DecodingKey::from_secret(app.config.jwt_secret.as_bytes());

        let mut fallbacks: HashMap<&'static str, Arc<LocalLimiter>> = HashMap::new();
        for (name, limit) in [
            (TIER_STRICT, app.config.rate_limit_auth),
            (TIER_WRITE, app.config.rate_limit_write),
            (TIER_READ, app.config.rate_limit_global),
        ] {
            fallbacks.insert(name, Arc::new(RateLimiter::keyed(build_quota(limit))));
        }

        LocalState {
            trusted_proxies,
            jwt_decoding_key,
            redis_healthy: AtomicBool::new(true),
            warned_once: AtomicBool::new(false),
            fallbacks,
        }
    })
}

fn build_quota(per_minute: u32) -> Quota {
    let n = NonZeroU32::new(per_minute.max(1)).expect("max(1) guarantees non-zero");
    Quota::per_minute(n)
}

/// Axum middleware entry point. Wired in `routes::build_router` with
/// `axum::middleware::from_fn_with_state`.
pub async fn apply(
    State(app): State<AppState>,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Response {
    let state = local_state(&app);
    let tier = tier_for(request.method(), request.uri().path(), &app.config);

    let direct_peer = connect_info.as_ref().map(|c| c.0.ip());
    let client_ip = resolve_client_ip(request.headers(), direct_peer, &state.trusted_proxies);

    let key = build_key(tier, request.headers(), &state.jwt_decoding_key, client_ip);

    let outcome = check(&app, state, tier, &key).await;

    match outcome {
        Decision::Allow {
            remaining,
            reset_at,
        } => {
            let mut response = next.run(request).await;
            attach_headers(
                response.headers_mut(),
                tier.limit_per_minute,
                remaining,
                reset_at,
                None,
            );
            response
        }
        Decision::Reject {
            reset_at,
            retry_after,
        } => too_many_requests(tier.limit_per_minute, 0, reset_at, retry_after),
    }
}

#[derive(Debug)]
enum Decision {
    Allow { remaining: u32, reset_at: u64 },
    Reject { reset_at: u64, retry_after: u64 },
}

async fn check(app: &AppState, state: &'static LocalState, tier: Tier, key: &str) -> Decision {
    // Distributed path first. Only consult the local fallback when Redis
    // has previously failed in this process.
    if state.redis_healthy.load(Ordering::Acquire) {
        match redis_check(app, tier, key).await {
            Ok(decision) => return decision,
            Err(err) => {
                if !state.warned_once.swap(true, Ordering::AcqRel) {
                    tracing::warn!(
                        error = %err,
                        tier = tier.name,
                        "rate limit Redis backend failed, degrading to in-process limiter",
                    );
                }
                state.redis_healthy.store(false, Ordering::Release);
            }
        }
    }

    local_check(state, tier, key)
}

async fn redis_check(app: &AppState, tier: Tier, key: &str) -> redis::RedisResult<Decision> {
    let mut conn = app.redis.clone();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let window = now / WINDOW_SECONDS;
    let reset_at = (window + 1) * WINDOW_SECONDS;
    let redis_key = format!("ratelimit:{}:{}:{}", tier.name, key, window);

    let count: u64 = conn.incr(&redis_key, 1u64).await?;
    if count == 1 {
        let _: () = conn.expire(&redis_key, KEY_TTL_SECONDS as i64).await?;
    }

    if count > tier.limit_per_minute as u64 {
        Ok(Decision::Reject {
            reset_at,
            retry_after: reset_at.saturating_sub(now),
        })
    } else {
        let remaining = (tier.limit_per_minute as u64).saturating_sub(count) as u32;
        Ok(Decision::Allow {
            remaining,
            reset_at,
        })
    }
}

fn local_check(state: &'static LocalState, tier: Tier, key: &str) -> Decision {
    let limiter = state
        .fallbacks
        .get(tier.name)
        .expect("local limiter initialised for every tier in local_state()");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let window = now / WINDOW_SECONDS;
    let reset_at = (window + 1) * WINDOW_SECONDS;

    match limiter.check_key(&key.to_string()) {
        Ok(_snapshot) => Decision::Allow {
            // governor does not expose a stable remaining count across
            // versions in the form a header consumer expects; report the
            // tier limit minus one for the just-consumed request.
            remaining: tier.limit_per_minute.saturating_sub(1),
            reset_at,
        },
        Err(_not_until) => Decision::Reject {
            reset_at,
            retry_after: reset_at.saturating_sub(now),
        },
    }
}

fn build_key(
    tier: Tier,
    headers: &HeaderMap,
    decoding_key: &DecodingKey,
    ip: Option<IpAddr>,
) -> String {
    match tier.keying {
        Keying::Ip => ip_key(ip),
        Keying::UserOrIp => {
            if let Some(sub) = extract_jwt_subject(headers, decoding_key) {
                format!("user:{}", sub)
            } else {
                ip_key(ip)
            }
        }
    }
}

fn ip_key(ip: Option<IpAddr>) -> String {
    match ip {
        Some(addr) => format!("ip:{}", addr),
        None => "ip:unknown".to_string(),
    }
}

fn extract_jwt_subject(headers: &HeaderMap, key: &DecodingKey) -> Option<String> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))?;

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<Claims>(token, key, &validation)
        .ok()
        .map(|data| data.claims.sub)
}

fn resolve_client_ip(
    headers: &HeaderMap,
    direct_peer: Option<IpAddr>,
    trusted_proxies: &[IpNet],
) -> Option<IpAddr> {
    let direct = direct_peer?;

    let proxy_trusted = trusted_proxies.iter().any(|net| net.contains(&direct));
    if !proxy_trusted {
        return Some(direct);
    }

    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(leftmost) = forwarded.split(',').next() {
            if let Ok(parsed) = leftmost.trim().parse::<IpAddr>() {
                return Some(parsed);
            }
        }
    }

    if let Some(real) = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
    {
        return Some(real);
    }

    Some(direct)
}

fn parse_trusted_proxies(raw: &[String]) -> Vec<IpNet> {
    raw.iter()
        .filter_map(|s| {
            if let Ok(net) = s.parse::<IpNet>() {
                Some(net)
            } else if let Ok(addr) = s.parse::<IpAddr>() {
                Some(IpNet::from(addr))
            } else {
                tracing::warn!(value = %s, "TRUSTED_PROXIES entry is not a valid IP or CIDR, ignoring");
                None
            }
        })
        .collect()
}

fn attach_headers(
    headers: &mut HeaderMap,
    limit: u32,
    remaining: u32,
    reset_at: u64,
    retry_after: Option<u64>,
) {
    let _ =
        HeaderValue::from_str(&limit.to_string()).map(|v| headers.insert("x-ratelimit-limit", v));
    let _ = HeaderValue::from_str(&remaining.to_string())
        .map(|v| headers.insert("x-ratelimit-remaining", v));
    let _ = HeaderValue::from_str(&reset_at.to_string())
        .map(|v| headers.insert("x-ratelimit-reset", v));
    if let Some(secs) = retry_after {
        let _ = HeaderValue::from_str(&secs.to_string()).map(|v| headers.insert("retry-after", v));
    }
}

fn too_many_requests(limit: u32, remaining: u32, reset_at: u64, retry_after: u64) -> Response {
    let body = Json(json!({
        "error": {
            "code": StatusCode::TOO_MANY_REQUESTS.as_u16(),
            "message": format!("Rate limit exceeded. Try again in {} seconds.", retry_after),
        }
    }));

    let mut response = (StatusCode::TOO_MANY_REQUESTS, body).into_response();
    attach_headers(
        response.headers_mut(),
        limit,
        remaining,
        reset_at,
        Some(retry_after),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn headers_with(name: &'static str, value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(name, HeaderValue::from_str(value).unwrap());
        h
    }

    fn test_config() -> AppConfig {
        AppConfig {
            database_url: String::new(),
            redis_url: String::new(),
            jwt_secret: "test-secret".to_string(),
            jwt_expiration_hours: 24,
            pinata_api_key: String::new(),
            pinata_secret_key: String::new(),
            ipfs_backend: String::new(),
            ipfs_gateway_url: String::new(),
            soroban_rpc_url: String::new(),
            farm_contract_address: String::new(),
            investment_contract_address: String::new(),
            escrow_contract_address: String::new(),
            dao_contract_address: String::new(),
            indexer_poll_interval_secs: 0,
            server_host: String::new(),
            server_port: 0,
            cors_origins: vec![],
            max_upload_size_mb: 10,
            rate_limit_global: 100,
            rate_limit_auth: 10,
            rate_limit_write: 30,
            trusted_proxies: vec![],
        }
    }

    #[test]
    fn auth_nonce_lands_on_strict_tier() {
        let cfg = test_config();
        let tier = tier_for(&Method::POST, "/api/v1/auth/nonce", &cfg);
        assert_eq!(tier.name, TIER_STRICT);
        assert_eq!(tier.limit_per_minute, 10);
    }

    #[test]
    fn auth_verify_lands_on_strict_tier() {
        let cfg = test_config();
        let tier = tier_for(&Method::POST, "/api/v1/auth/verify", &cfg);
        assert_eq!(tier.name, TIER_STRICT);
    }

    #[test]
    fn auth_me_get_lands_on_read_tier() {
        let cfg = test_config();
        let tier = tier_for(&Method::GET, "/api/v1/auth/me", &cfg);
        assert_eq!(tier.name, TIER_READ);
    }

    #[test]
    fn escrows_post_lands_on_write_tier() {
        let cfg = test_config();
        let tier = tier_for(&Method::POST, "/api/v1/escrows", &cfg);
        assert_eq!(tier.name, TIER_WRITE);
        assert_eq!(tier.limit_per_minute, 30);
    }

    #[test]
    fn investments_invest_post_lands_on_write_tier() {
        let cfg = test_config();
        let tier = tier_for(&Method::POST, "/api/v1/investments/123/invest", &cfg);
        assert_eq!(tier.name, TIER_WRITE);
    }

    #[test]
    fn farms_get_lands_on_read_tier() {
        let cfg = test_config();
        let tier = tier_for(&Method::GET, "/api/v1/farms", &cfg);
        assert_eq!(tier.name, TIER_READ);
        assert_eq!(tier.limit_per_minute, 100);
    }

    #[test]
    fn upload_post_lands_on_write_tier() {
        let cfg = test_config();
        let tier = tier_for(&Method::POST, "/api/v1/upload", &cfg);
        assert_eq!(tier.name, TIER_WRITE);
    }

    #[test]
    fn put_on_write_path_lands_on_write_tier() {
        let cfg = test_config();
        let tier = tier_for(&Method::PUT, "/api/v1/escrows/abc/approve", &cfg);
        assert_eq!(tier.name, TIER_WRITE);
    }

    #[test]
    fn trusted_proxy_cidr_parses_ipv4_and_ipv6_ranges() {
        let raw = vec![
            "10.0.0.0/8".to_string(),
            "172.16.0.0/12".to_string(),
            "2001:db8::/32".to_string(),
        ];
        let nets = parse_trusted_proxies(&raw);
        assert_eq!(nets.len(), 3);
        assert!(nets[0].contains(&"10.1.2.3".parse::<IpAddr>().unwrap()));
        assert!(nets[1].contains(&"172.16.0.5".parse::<IpAddr>().unwrap()));
        assert!(nets[2].contains(&"2001:db8::1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn trusted_proxy_bare_ip_is_treated_as_single_host() {
        let raw = vec!["192.168.1.1".to_string()];
        let nets = parse_trusted_proxies(&raw);
        assert_eq!(nets.len(), 1);
        assert!(nets[0].contains(&"192.168.1.1".parse::<IpAddr>().unwrap()));
        assert!(!nets[0].contains(&"192.168.1.2".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn trusted_proxy_invalid_entry_is_filtered_out() {
        let raw = vec!["not-an-ip".to_string(), "10.0.0.0/8".to_string()];
        let nets = parse_trusted_proxies(&raw);
        assert_eq!(nets.len(), 1);
    }

    #[test]
    fn client_ip_falls_back_to_direct_peer_when_no_proxy_trusted() {
        let headers = headers_with("x-forwarded-for", "1.2.3.4");
        let direct = Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)));
        let trusted: Vec<IpNet> = vec![];
        let resolved = resolve_client_ip(&headers, direct, &trusted);
        assert_eq!(resolved, direct);
    }

    #[test]
    fn client_ip_honours_x_forwarded_for_when_direct_peer_is_in_trusted_range() {
        let headers = headers_with("x-forwarded-for", "1.2.3.4, 10.0.0.5");
        let direct = Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)));
        let trusted = vec!["10.0.0.0/8".parse::<IpNet>().unwrap()];
        let resolved = resolve_client_ip(&headers, direct, &trusted);
        assert_eq!(resolved, Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))));
    }

    #[test]
    fn client_ip_ignores_x_forwarded_for_when_direct_peer_is_not_trusted() {
        let headers = headers_with("x-forwarded-for", "1.2.3.4");
        let direct = Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9)));
        let trusted = vec!["10.0.0.0/8".parse::<IpNet>().unwrap()];
        let resolved = resolve_client_ip(&headers, direct, &trusted);
        assert_eq!(resolved, direct);
    }

    #[test]
    fn client_ip_falls_back_to_x_real_ip_when_forwarded_absent() {
        let headers = headers_with("x-real-ip", "1.2.3.4");
        let direct = Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9)));
        let trusted = vec!["10.0.0.0/8".parse::<IpNet>().unwrap()];
        let resolved = resolve_client_ip(&headers, direct, &trusted);
        assert_eq!(resolved, Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))));
    }

    #[test]
    fn client_ip_supports_ipv6_forwarded_address() {
        let headers = headers_with("x-forwarded-for", "2001:db8::1");
        let direct = Some(IpAddr::V6(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1)));
        let trusted = vec!["fd00::/8".parse::<IpNet>().unwrap()];
        let resolved = resolve_client_ip(&headers, direct, &trusted);
        assert_eq!(
            resolved,
            Some(IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)))
        );
    }

    #[test]
    fn ip_key_falls_back_to_unknown_when_peer_is_absent() {
        assert_eq!(ip_key(None), "ip:unknown");
        assert_eq!(
            ip_key(Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)))),
            "ip:1.2.3.4"
        );
    }

    #[test]
    fn build_key_prefers_jwt_subject_in_write_tier_when_token_decodes() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        let secret = b"test-secret";
        let claims = Claims {
            sub: "GFUNDER".to_string(),
            exp: (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600) as usize,
        };
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();

        let headers = headers_with("authorization", &format!("Bearer {}", token));
        let decoding_key = DecodingKey::from_secret(secret);
        let key = build_key(
            Tier::write(30),
            &headers,
            &decoding_key,
            Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))),
        );
        assert_eq!(key, "user:GFUNDER");
    }

    #[test]
    fn build_key_falls_back_to_ip_in_write_tier_when_token_missing() {
        let headers = HeaderMap::new();
        let decoding_key = DecodingKey::from_secret(b"test-secret");
        let key = build_key(
            Tier::write(30),
            &headers,
            &decoding_key,
            Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))),
        );
        assert_eq!(key, "ip:1.2.3.4");
    }

    #[test]
    fn build_key_falls_back_to_ip_in_write_tier_when_token_invalid() {
        let headers = headers_with("authorization", "Bearer not-a-jwt");
        let decoding_key = DecodingKey::from_secret(b"test-secret");
        let key = build_key(
            Tier::write(30),
            &headers,
            &decoding_key,
            Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))),
        );
        assert_eq!(key, "ip:1.2.3.4");
    }

    #[test]
    fn build_key_always_uses_ip_in_strict_tier_even_with_valid_token() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        let secret = b"test-secret";
        let claims = Claims {
            sub: "GFUNDER".to_string(),
            exp: (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600) as usize,
        };
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();

        let headers = headers_with("authorization", &format!("Bearer {}", token));
        let decoding_key = DecodingKey::from_secret(secret);
        let key = build_key(
            Tier::strict(10),
            &headers,
            &decoding_key,
            Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))),
        );
        assert_eq!(key, "ip:1.2.3.4");
    }

    #[test]
    fn too_many_requests_body_matches_api_error_shape() {
        let response = too_many_requests(10, 0, 1700000060, 30);
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        let headers = response.headers();
        assert_eq!(headers.get("x-ratelimit-limit").unwrap(), "10");
        assert_eq!(headers.get("x-ratelimit-remaining").unwrap(), "0");
        assert_eq!(headers.get("x-ratelimit-reset").unwrap(), "1700000060");
        assert_eq!(headers.get("retry-after").unwrap(), "30");
    }
}

/// End-to-end tests that drive the full router through the middleware.
///
/// These tests require a reachable Redis (default `redis://127.0.0.1:6379`,
/// override with `REDIS_URL`) and a Postgres connection that AppState can
/// open lazily. The rate limit path never reads the database, so Postgres
/// is only opened, not exercised. Use docker-compose from the repo root to
/// bring both up before running these.
#[cfg(test)]
mod integration_tests {
    use crate::app_state::AppState;
    use crate::config::AppConfig;
    use crate::routes::build_router;
    use axum::body::{to_bytes, Body};
    use axum::extract::ConnectInfo;
    use axum::http::{Method as HttpMethod, Request, StatusCode};
    use serde_json::Value;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tower::ServiceExt;

    const JWT_SECRET: &str = "rate-limit-integration-secret";

    /// Each test gets a unique IP so concurrent runs and process-wide
    /// limiter state never collide on the same key.
    static IP_COUNTER: AtomicU32 = AtomicU32::new(1);

    fn next_unique_ip() -> SocketAddr {
        let n = IP_COUNTER.fetch_add(1, Ordering::AcqRel);
        let octet_a = ((n >> 8) & 0xff) as u8;
        let octet_b = (n & 0xff) as u8;
        SocketAddr::from(([10, 200, octet_a, octet_b], 54321))
    }

    fn env_or(key: &str, default: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| default.to_string())
    }

    fn test_config() -> AppConfig {
        AppConfig {
            database_url: env_or(
                "DATABASE_URL",
                "postgres://postgres:postgres@localhost:5432/agrovest",
            ),
            redis_url: env_or("REDIS_URL", "redis://127.0.0.1:6379"),
            jwt_secret: JWT_SECRET.to_string(),
            jwt_expiration_hours: 24,
            pinata_api_key: String::new(),
            pinata_secret_key: String::new(),
            ipfs_backend: "pinata".to_string(),
            ipfs_gateway_url: "https://gateway.pinata.cloud/ipfs".to_string(),
            soroban_rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            farm_contract_address: String::new(),
            investment_contract_address: String::new(),
            escrow_contract_address: String::new(),
            dao_contract_address: String::new(),
            indexer_poll_interval_secs: 5,
            server_host: "0.0.0.0".to_string(),
            server_port: 8080,
            cors_origins: vec!["http://localhost:3000".to_string()],
            max_upload_size_mb: 10,
            rate_limit_global: 100,
            rate_limit_auth: 10,
            rate_limit_write: 30,
            trusted_proxies: vec![],
        }
    }

    async fn setup() -> AppState {
        let state = AppState::new(test_config())
            .await
            .expect("connect to test Postgres/Redis (see AGENTS.md)");
        let mut conn = state.redis.clone();
        let _: redis::RedisResult<()> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
        state
    }

    fn request_with_peer(
        method: HttpMethod,
        path: &str,
        peer: SocketAddr,
        body: Body,
    ) -> Request<Body> {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(body)
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(peer));
        req
    }

    #[tokio::test]
    #[ignore] // Integration test requires live Redis/database
    async fn strict_tier_blocks_the_eleventh_auth_nonce_call_from_the_same_ip() {
        let state = setup().await;
        let app = build_router(state);
        let peer = next_unique_ip();
        let body = serde_json::to_string(&serde_json::json!({
            "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        }))
        .unwrap();

        for i in 1..=10 {
            let response = app
                .clone()
                .oneshot(request_with_peer(
                    HttpMethod::POST,
                    "/api/v1/auth/nonce",
                    peer,
                    Body::from(body.clone()),
                ))
                .await
                .unwrap();
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {} should not be rate limited",
                i
            );
        }

        let blocked = app
            .clone()
            .oneshot(request_with_peer(
                HttpMethod::POST,
                "/api/v1/auth/nonce",
                peer,
                Body::from(body),
            ))
            .await
            .unwrap();
        assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    #[ignore] // Integration test requires live Redis/database
    async fn write_tier_blocks_the_thirty_first_escrows_post_from_the_same_ip() {
        let state = setup().await;
        let app = build_router(state);
        let peer = next_unique_ip();

        for i in 1..=30 {
            let response = app
                .clone()
                .oneshot(request_with_peer(
                    HttpMethod::POST,
                    "/api/v1/escrows",
                    peer,
                    Body::from("{}"),
                ))
                .await
                .unwrap();
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {} should not be rate limited",
                i
            );
        }

        let blocked = app
            .clone()
            .oneshot(request_with_peer(
                HttpMethod::POST,
                "/api/v1/escrows",
                peer,
                Body::from("{}"),
            ))
            .await
            .unwrap();
        assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    #[ignore] // Integration test requires live Redis/database
    async fn read_tier_blocks_the_hundred_and_first_farms_get_from_the_same_ip() {
        let state = setup().await;
        let app = build_router(state);
        let peer = next_unique_ip();

        for i in 1..=100 {
            let response = app
                .clone()
                .oneshot(request_with_peer(
                    HttpMethod::GET,
                    "/api/v1/farms",
                    peer,
                    Body::empty(),
                ))
                .await
                .unwrap();
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {} should not be rate limited",
                i
            );
        }

        let blocked = app
            .clone()
            .oneshot(request_with_peer(
                HttpMethod::GET,
                "/api/v1/farms",
                peer,
                Body::empty(),
            ))
            .await
            .unwrap();
        assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    #[ignore] // Integration test requires live Redis/database
    async fn rate_limited_response_matches_api_error_json_shape() {
        let state = setup().await;
        let app = build_router(state);
        let peer = next_unique_ip();

        for _ in 1..=10 {
            let _ = app
                .clone()
                .oneshot(request_with_peer(
                    HttpMethod::POST,
                    "/api/v1/auth/nonce",
                    peer,
                    Body::from(
                        r#"{"address":"GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#,
                    ),
                ))
                .await
                .unwrap();
        }

        let blocked = app
            .clone()
            .oneshot(request_with_peer(
                HttpMethod::POST,
                "/api/v1/auth/nonce",
                peer,
                Body::from(
                    r#"{"address":"GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#,
                ),
            ))
            .await
            .unwrap();

        assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
        let headers = blocked.headers().clone();
        assert_eq!(headers.get("x-ratelimit-limit").unwrap(), "10");
        assert_eq!(headers.get("x-ratelimit-remaining").unwrap(), "0");
        assert!(headers.get("x-ratelimit-reset").is_some());
        assert!(headers.get("retry-after").is_some());

        let bytes = to_bytes(blocked.into_body(), usize::MAX).await.unwrap();
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["error"]["code"], 429);
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Rate limit exceeded"));
    }

    #[tokio::test]
    #[ignore] // Integration test requires live Redis/database
    async fn x_ratelimit_remaining_decrements_across_successive_calls() {
        let state = setup().await;
        let app = build_router(state);
        let peer = next_unique_ip();

        let mut remaining_values: Vec<u32> = Vec::new();
        for _ in 0..3 {
            let response = app
                .clone()
                .oneshot(request_with_peer(
                    HttpMethod::GET,
                    "/api/v1/farms",
                    peer,
                    Body::empty(),
                ))
                .await
                .unwrap();
            let header_value = response
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u32>().ok())
                .expect("x-ratelimit-remaining must be present and parseable");
            remaining_values.push(header_value);
        }

        // Each successive call should report a remaining count strictly less
        // than the previous, regardless of which backend (Redis or local
        // fallback) handled the request.
        assert!(
            remaining_values.windows(2).all(|pair| pair[1] < pair[0]),
            "remaining counts did not strictly decrement: {:?}",
            remaining_values
        );
    }

    #[tokio::test]
    #[ignore] // Integration test requires live Redis/database
    async fn x_forwarded_for_is_ignored_when_direct_peer_is_not_a_trusted_proxy() {
        let state = setup().await;
        let app = build_router(state);
        // Two requests from the same TCP peer but spoofing different
        // X-Forwarded-For values must share a key, because the direct peer
        // is not in TRUSTED_PROXIES. We assert this by burning the strict
        // tier under the shared peer regardless of the spoofed header.
        let peer = next_unique_ip();
        let body = r#"{"address":"GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#;

        for i in 0..10 {
            let spoofed = format!("198.51.100.{}", i + 1);
            let mut req = Request::builder()
                .method(HttpMethod::POST)
                .uri("/api/v1/auth/nonce")
                .header("content-type", "application/json")
                .header("x-forwarded-for", spoofed)
                .body(Body::from(body))
                .unwrap();
            req.extensions_mut().insert(ConnectInfo(peer));
            let response = app.clone().oneshot(req).await.unwrap();
            assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }

        let mut req = Request::builder()
            .method(HttpMethod::POST)
            .uri("/api/v1/auth/nonce")
            .header("content-type", "application/json")
            .header("x-forwarded-for", "203.0.113.99")
            .body(Body::from(body))
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(peer));
        let blocked = app.clone().oneshot(req).await.unwrap();
        assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
