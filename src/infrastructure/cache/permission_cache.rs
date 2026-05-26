// src/infrastructure/cache/permission_cache.rs
//
// Loads the permission strings for a user (joined from custom_roles) and
// caches them in a DashMap<user_id, HashSet<String>>.
//
// A permission string uses the pattern  "<resource>:<action>", e.g.
//   "agreements:read"
//   "payments:write"
//   "staff:manage"
//   "*"   →  super-admin wildcard
//
// Cache is per-user. Invalidate on role reassignment or role mutation.

use std::collections::HashSet;

use dashmap::DashMap;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::application::errors::AppError;

pub struct PermissionCache {
    pool: PgPool,
    inner: DashMap<Uuid, HashSet<String>>,
}

impl PermissionCache {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            inner: DashMap::new(),
        }
    }

    // ── Loading ───────────────────────────────────────────────────────────────

    async fn load(&self, user_id: Uuid) -> anyhow::Result<()> {
        let row = sqlx::query(
            r#"
            SELECT cr.permissions
            FROM users u
            JOIN custom_roles cr ON cr.id = u.role_id
            WHERE u.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("user not found"))?;

        let perms: HashSet<String> =
            serde_json::from_value(row.get("permissions")).unwrap_or_default();

        self.inner.insert(user_id, perms);
        Ok(())
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Returns `true` if the user holds `perm` or the wildcard `"*"`.
    /// Loads from Postgres on first call for this user.
    pub async fn has(&self, user_id: Uuid, perm: &str) -> bool {
        if !self.inner.contains_key(&user_id) {
            if let Err(e) = self.load(user_id).await {
                tracing::error!("permission_cache: load failed for {user_id}: {e}");
                return false;
            }
        }
        self.inner
            .get(&user_id)
            .map(|p| p.contains(perm) || p.contains("*"))
            .unwrap_or(false)
    }

    /// Returns `Ok(())` if the user has `perm`, otherwise `Err(AppError::Forbidden)`.
    pub async fn require(&self, user_id: Uuid, perm: &str) -> Result<(), AppError> {
        if self.has(user_id, perm).await {
            Ok(())
        } else {
            Err(AppError::Forbidden(perm.to_string()))
        }
    }

    /// Drop cached permissions for a user. Call after role change or role mutation.
    pub fn invalidate(&self, user_id: Uuid) {
        self.inner.remove(&user_id);
    }

    /// Return the full permission set for a user (for debug / admin display).
    pub async fn all_for(&self, user_id: Uuid) -> anyhow::Result<HashSet<String>> {
        if !self.inner.contains_key(&user_id) {
            self.load(user_id).await?;
        }
        Ok(self
            .inner
            .get(&user_id)
            .map(|p| p.clone())
            .unwrap_or_default())
    }
}

// ── IP allow-list helper ──────────────────────────────────────────────────────
//
// Used by the auth middleware after it loads AgencySecurityPolicy.

/// Returns `true` if `peer_ip` is permitted.
/// An empty allow-list means no restriction (all IPs are allowed).
pub fn ip_in_allowlist(peer_ip: std::net::IpAddr, allowlist_json: &serde_json::Value) -> bool {
    let list: Vec<String> = serde_json::from_value(allowlist_json.clone()).unwrap_or_default();
    if list.is_empty() {
        return true;
    }
    list.iter().any(|cidr| cidr_contains(peer_ip, cidr))
}

/// Minimal CIDR membership test (IPv4 only for the initial version).
/// Replace with the `ipnetwork` crate for full IPv6 support.
fn cidr_contains(ip: std::net::IpAddr, cidr: &str) -> bool {
    let parts: Vec<&str> = cidr.splitn(2, '/').collect();
    if parts.len() == 1 {
        // Plain IP match
        return cidr
            .parse::<std::net::IpAddr>()
            .map(|a| a == ip)
            .unwrap_or(false);
    }
    let Ok(base) = parts[0].parse::<std::net::Ipv4Addr>() else {
        return false;
    };
    let Ok(prefix_len) = parts[1].parse::<u32>() else {
        return false;
    };
    let std::net::IpAddr::V4(v4) = ip else {
        return false;
    };

    let mask = if prefix_len == 0 {
        0u32
    } else {
        !0u32 << (32 - prefix_len)
    };
    (u32::from(base) & mask) == (u32::from(v4) & mask)
}
