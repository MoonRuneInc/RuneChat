//! Keyed rate limiters for auth and invite surfaces.
//!
//! Uses governor's in-memory token buckets. No global limiter — each key gets
//! its own independent bucket, so one noisy client cannot starve others.

use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::http::HeaderMap;
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};
use uuid::Uuid;

/// Shorthand for a keyed rate limiter backed by governor's default state store.
pub type KeyedLimiter<K> = Arc<RateLimiter<K, DefaultKeyedStateStore<K>, DefaultClock>>;

/// All rate limiters used by the application.
#[derive(Clone)]
pub struct RateLimiters {
    /// Login attempts per client IP.
    pub login_ip: KeyedLimiter<String>,
    /// Login attempts per identifier (username or email).
    pub login_identifier: KeyedLimiter<String>,
    /// TOTP verification attempts per authenticated user.
    pub totp_user: KeyedLimiter<Uuid>,
    /// Unauthenticated invite preview lookups per IP.
    pub invite_preview_ip: KeyedLimiter<String>,
    /// Authenticated invite joins per IP.
    pub invite_join_ip: KeyedLimiter<String>,
    /// Registration attempts per client IP.
    pub register_ip: KeyedLimiter<String>,
}

impl RateLimiters {
    pub fn new() -> Self {
        // Login IP: generous bucket for NAT/shared offices and test suites.
        // Burst 100, then 10 per minute.
        let login_ip = Arc::new(RateLimiter::keyed(
            Quota::per_minute(NonZeroU32::new(10).unwrap())
                .allow_burst(NonZeroU32::new(100).unwrap()),
        ));

        // Login identifier: tight — brute-force protection.
        // Burst 4, then 1 per minute.
        let login_identifier = Arc::new(RateLimiter::keyed(
            Quota::per_minute(NonZeroU32::new(1).unwrap()).allow_burst(NonZeroU32::new(4).unwrap()),
        ));

        // TOTP per user: burst 5, then 1 per minute.
        let totp_user = Arc::new(RateLimiter::keyed(
            Quota::per_minute(NonZeroU32::new(1).unwrap()).allow_burst(NonZeroU32::new(5).unwrap()),
        ));

        // Invite preview (unauthenticated): tight — easy enumeration target.
        // Burst 3, then 1 per minute.
        let invite_preview_ip = Arc::new(RateLimiter::keyed(
            Quota::per_minute(NonZeroU32::new(1).unwrap()).allow_burst(NonZeroU32::new(3).unwrap()),
        ));

        // Invite join (authenticated): moderate — legitimate users behind NAT.
        // Burst 15, then 5 per minute.
        let invite_join_ip = Arc::new(RateLimiter::keyed(
            Quota::per_minute(NonZeroU32::new(5).unwrap())
                .allow_burst(NonZeroU32::new(15).unwrap()),
        ));

        // Registration (unauthenticated): tight — prevents bulk account creation
        // and protects the external HIBP API from abuse.
        // Burst 3, then 1 per minute.
        let register_ip = Arc::new(RateLimiter::keyed(
            Quota::per_minute(NonZeroU32::new(1).unwrap()).allow_burst(NonZeroU32::new(3).unwrap()),
        ));

        Self {
            login_ip,
            login_identifier,
            totp_user,
            invite_preview_ip,
            invite_join_ip,
            register_ip,
        }
    }
}

/// Extract a client IP string from request headers and optional socket address.
///
/// Priority:
/// 1. `X-Real-Ip` header (set by nginx proxy)
/// 2. `X-Forwarded-For` header (first entry)
/// 3. Direct connection socket address
/// 4. `"unknown"` fallback
pub fn extract_client_ip(headers: &HeaderMap, addr: Option<SocketAddr>) -> String {
    if let Some(val) = headers.get("X-Real-Ip") {
        if let Ok(s) = val.to_str() {
            return s.to_string();
        }
    }
    if let Some(val) = headers.get("X-Forwarded-For") {
        if let Ok(s) = val.to_str() {
            if let Some(first) = s.split(',').next() {
                return first.trim().to_string();
            }
        }
    }
    addr.map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
