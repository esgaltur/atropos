use chrono::Utc;
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::future::Future;
use uuid::Uuid;

use crate::domain::{
    error::DomainError,
    lease::Lease,
    pool::Pool,
    repository::{
        AllocationRepository, AuditLogEntry, CostGroupBy, CostRow, LeaseFilter, PlatformRepository,
        PoolRepository, PoolUtilization, QuotaRecord, ResourceRepository, SummaryStats,
        WaitlistPosition,
    },
    reservation::Reservation,
    resource::Resource,
    LeaseId, LeaseStatus, PoolId, ResourceId, ResourceStatus,
};

pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Maps the UPPERCASE lease status text stored in the database to the domain
/// [`LeaseStatus`] enum (whose `FromStr` expects PascalCase). Unknown values
/// fall back to `Active` to keep listing resilient.
fn parse_lease_status(raw: &str) -> LeaseStatus {
    match raw.to_ascii_uppercase().as_str() {
        "PENDING" => LeaseStatus::Pending,
        "ACTIVE" => LeaseStatus::Active,
        "EXPIRING" => LeaseStatus::Expiring,
        "RELEASED" => LeaseStatus::Released,
        "EXPIRED" => LeaseStatus::Expired,
        "REVOKED" => LeaseStatus::Revoked,
        "WAITING" => LeaseStatus::Waiting,
        _ => LeaseStatus::Active,
    }
}

/// Looks up an existing `ACTIVE` lease matching the supplied idempotency key.
///
/// Used to make allocation retries safe: if a caller re-sends a request with the
/// same key, the previously granted lease is returned instead of creating a new one.
async fn fetch_active_lease_by_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    key: &str,
) -> Result<Option<Lease>, DomainError> {
    let row = sqlx::query(
        r#"
        SELECT id, resource_id, owner_id, tenant_id, priority, created_at, expires_at, idempotency_key, cost_center
        FROM leases
        WHERE idempotency_key = $1 AND status = 'ACTIVE'
        LIMIT 1
        "#,
    )
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

    Ok(row.map(|r| Lease {
        id: LeaseId(r.get("id")),
        resource_id: ResourceId(r.get("resource_id")),
        owner_id: r.get("owner_id"),
        tenant_id: r.get("tenant_id"),
        priority: r.get("priority"),
        status: LeaseStatus::Active,
        created_at: r.get("created_at"),
        expires_at: r.get("expires_at"),
        idempotency_key: r.get("idempotency_key"),
        cost_center: r.get("cost_center"),
    }))
}

impl PoolRepository for PostgresRepository {
    fn create(&self, pool: Pool) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            sqlx::query(
                r#"
                INSERT INTO pools (id, name, resource_type, policy, max_capacity, created_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(pool.id.0)
            .bind(pool.name)
            .bind(pool.resource_type)
            .bind(pool.policy.to_string())
            .bind(pool.max_capacity)
            .bind(pool.created_at)
            .execute(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(())
        }
    }

    fn find_by_id(
        &self,
        id: &PoolId,
    ) -> impl Future<Output = Result<Option<Pool>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = id.0;
        async move {
            let row = sqlx::query(
                r#"
                SELECT id, name, resource_type, policy, max_capacity, created_at
                FROM pools
                WHERE id = $1
                "#,
            )
            .bind(query_id)
            .fetch_optional(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let pool = match row {
                Some(r) => Some(Pool {
                    id: PoolId(r.get("id")),
                    name: r.get("name"),
                    resource_type: r.get("resource_type"),
                    policy: r.get::<String, _>("policy").parse().map_err(|e| {
                        DomainError::InfrastructureError(format!("invalid policy value: {e}"))
                    })?,
                    max_capacity: r.get("max_capacity"),
                    created_at: r.get("created_at"),
                }),
                None => None,
            };

            Ok(pool)
        }
    }

    fn find_by_name(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<Pool>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_name = name.to_string();
        async move {
            let row = sqlx::query(
                r#"
                SELECT id, name, resource_type, policy, max_capacity, created_at
                FROM pools
                WHERE name = $1
                "#,
            )
            .bind(query_name)
            .fetch_optional(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let pool = match row {
                Some(r) => Some(Pool {
                    id: PoolId(r.get("id")),
                    name: r.get("name"),
                    resource_type: r.get("resource_type"),
                    policy: r.get::<String, _>("policy").parse().map_err(|e| {
                        DomainError::InfrastructureError(format!("invalid policy value: {e}"))
                    })?,
                    max_capacity: r.get("max_capacity"),
                    created_at: r.get("created_at"),
                }),
                None => None,
            };

            Ok(pool)
        }
    }
}

impl ResourceRepository for PostgresRepository {
    fn create(&self, res: Resource) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            sqlx::query(
                r#"
                INSERT INTO resources (id, pool_id, external_id, status, attributes, version, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(res.id.0)
            .bind(res.pool_id.0)
            .bind(res.external_id)
            .bind(res.status.to_string())
            .bind(res.attributes)
            .bind(res.version)
            .bind(res.updated_at)
            .execute(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(())
        }
    }

    fn find_by_id(
        &self,
        id: &ResourceId,
    ) -> impl Future<Output = Result<Option<Resource>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = id.0;
        async move {
            let row = sqlx::query(
                r#"
                SELECT id, pool_id, external_id, status, attributes, version, updated_at
                FROM resources
                WHERE id = $1
                "#,
            )
            .bind(query_id)
            .fetch_optional(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let res = match row {
                Some(r) => Some(Resource {
                    id: ResourceId(r.get("id")),
                    pool_id: PoolId(r.get("pool_id")),
                    external_id: r.get("external_id"),
                    status: r.get::<String, _>("status").parse().map_err(|e| {
                        DomainError::InfrastructureError(format!("invalid status value: {e}"))
                    })?,
                    attributes: r.get("attributes"),
                    version: r.get("version"),
                    updated_at: r.get("updated_at"),
                }),
                None => None,
            };

            Ok(res)
        }
    }
}

impl AllocationRepository for PostgresRepository {
    #[allow(clippy::too_many_arguments)]
    fn allocate_resource(
        &self,
        pool_type: String,
        owner_id: String,
        tenant_id: String,
        priority: i32,
        ttl_seconds: i64,
        constraints: Option<serde_json::Value>,
        spread_by: Option<String>,
        idempotency_key: Option<String>,
        cost_center: Option<String>,
        preempt: bool,
    ) -> impl Future<Output = Result<Lease, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let mut tx = db_pool
                .begin()
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            // -1. Idempotency: if a live lease already exists for this key, return it as-is.
            if let Some(key) = idempotency_key.as_ref() {
                if let Some(existing) = fetch_active_lease_by_idempotency_key(&mut tx, key).await? {
                    return Ok(existing);
                }
            }

            // 0. Quota Check (lock the quota row to avoid TOCTOU races between concurrent allocations)
            let quota_record = sqlx::query(
                r#"
                SELECT max_active_leases 
                FROM tenant_quotas 
                WHERE tenant_id = $1 AND pool_type = $2
                FOR UPDATE
                "#,
            )
            .bind(&tenant_id)
            .bind(&pool_type)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            if let Some(row) = quota_record {
                let max_leases: i32 = row.try_get("max_active_leases").unwrap_or(0);

                let (current_leases,): (i64,) = sqlx::query_as(
                    r#"
                    SELECT COUNT(*) 
                    FROM leases l
                    JOIN resources r ON l.resource_id = r.id
                    JOIN pools p ON r.pool_id = p.id
                    WHERE l.tenant_id = $1 AND p.resource_type = $2 AND l.status = 'ACTIVE'
                    "#,
                )
                .bind(&tenant_id)
                .bind(&pool_type)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

                if current_leases >= max_leases as i64 {
                    return Err(DomainError::QuotaExceeded(format!(
                        "tenant {} reached limit of {} active leases for pool {}",
                        tenant_id, max_leases, pool_type
                    )));
                }
            }

            // 1. Standard Allocation with Rack Anti-Affinity
            let resource_record = sqlx::query(
                r#"
                SELECT r.id
                FROM resources r
                JOIN pools p ON r.pool_id = p.id
                LEFT JOIN (
                    SELECT res.rack_id, COUNT(*) as tenant_rack_count
                    FROM leases l
                    JOIN resources res ON l.resource_id = res.id
                    WHERE l.tenant_id = $3 AND l.status = 'ACTIVE'
                    GROUP BY res.rack_id
                ) rack_usage ON r.rack_id = rack_usage.rack_id
                WHERE p.resource_type = $1 
                  AND r.status = 'Healthy'
                  AND ($2::jsonb IS NULL OR r.attributes @> $2::jsonb)
                  AND NOT EXISTS (
                    SELECT 1 FROM leases l
                    WHERE l.resource_id = r.id AND l.status = 'ACTIVE' AND l.expires_at > NOW()
                )
                ORDER BY (CASE WHEN $4 = 'rack_id' THEN tenant_rack_count ELSE 0 END) ASC NULLS FIRST
                LIMIT 1
                FOR UPDATE OF r SKIP LOCKED
                "#,
            )
            .bind(&pool_type)
            .bind(&constraints)
            .bind(&tenant_id)
            .bind(&spread_by)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let (resource_id, preemption_occured) = match resource_record {
                Some(record) => (record.get::<Uuid, _>("id"), false),
                None => {
                    // 2. Preemption: only attempted when the caller explicitly opted in.
                    if !preempt {
                        return Err(DomainError::NoResourcesAvailable);
                    }

                    // Find active lease in the pool with lower priority + Attribute Match
                    let preempt_record = sqlx::query(
                        r#"
                        SELECT l.id as lease_id, l.resource_id
                        FROM leases l
                        JOIN resources r ON l.resource_id = r.id
                        JOIN pools p ON r.pool_id = p.id
                        WHERE p.resource_type = $1 
                          AND l.status = 'ACTIVE' 
                          AND l.expires_at > NOW()
                          AND l.priority < $2
                          AND ($3::jsonb IS NULL OR r.attributes @> $3::jsonb)
                        ORDER BY l.priority ASC, l.expires_at ASC
                        LIMIT 1
                        FOR UPDATE SKIP LOCKED
                        "#,
                    )
                    .bind(&pool_type)
                    .bind(priority)
                    .bind(&constraints)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

                    if let Some(row) = preempt_record {
                        let old_lease_id: Uuid = row.get("lease_id");
                        let res_id: Uuid = row.get("resource_id");

                        // Revoke old lease
                        sqlx::query("UPDATE leases SET status = 'REVOKED' WHERE id = $1")
                            .bind(old_lease_id)
                            .execute(&mut *tx)
                            .await
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

                        sqlx::query(
                            "INSERT INTO audit_log (action, lease_id, details) VALUES ('PREEMPT_REVOKE', $1, $2)"
                        )
                        .bind(old_lease_id)
                        .bind(serde_json::json!({ "new_owner": owner_id, "new_priority": priority }))
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

                        // Outbox Event for Preempt
                        sqlx::query("INSERT INTO outbox_events (id, event_type, payload) VALUES ($1, 'LEASE_REVOKED', $2)")
                            .bind(Uuid::new_v4())
                            .bind(serde_json::json!({ "lease_id": old_lease_id, "resource_id": res_id, "reason": "PREEMPTED" }))
                            .execute(&mut *tx).await.ok();

                        (res_id, true)
                    } else {
                        return Err(DomainError::NoResourcesAvailable);
                    }
                }
            };

            let lease_id = Uuid::new_v4();
            let now = Utc::now();
            let expires_at = now + chrono::Duration::seconds(ttl_seconds);

            sqlx::query(
                r#"
                INSERT INTO leases (id, resource_id, owner_id, tenant_id, priority, status, created_at, expires_at, idempotency_key, cost_center, last_heartbeat_at)
                VALUES ($1, $2, $3, $4, $5, 'ACTIVE', $6, $7, $8, $9, $6)
                "#
            )
            .bind(lease_id)
            .bind(resource_id)
            .bind(&owner_id)
            .bind(&tenant_id)
            .bind(priority)
            .bind(now)
            .bind(expires_at)
            .bind(&idempotency_key)
            .bind(&cost_center)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let action = if preemption_occured {
                "PREEMPT_ALLOCATE"
            } else {
                "ALLOCATE"
            };
            let details =
                serde_json::json!({ "ttl": ttl_seconds, "pool": pool_type, "priority": priority });
            sqlx::query(
                r#"
                INSERT INTO audit_log (actor_id, action, resource_id, lease_id, details)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(&owner_id)
            .bind(action)
            .bind(resource_id)
            .bind(lease_id)
            .bind(details)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            // Outbox Event for Granted Lease
            sqlx::query("INSERT INTO outbox_events (id, event_type, payload) VALUES ($1, 'LEASE_GRANTED', $2)")
                .bind(Uuid::new_v4())
                .bind(serde_json::json!({ "lease_id": lease_id, "resource_id": resource_id, "owner_id": owner_id, "tenant_id": tenant_id }))
                .execute(&mut *tx)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            tx.commit()
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(Lease {
                id: LeaseId(lease_id),
                resource_id: ResourceId(resource_id),
                owner_id,
                tenant_id,
                priority,
                status: LeaseStatus::Active,
                created_at: now,
                expires_at,
                idempotency_key,
                cost_center,
            })
        }
    }

    fn release_lease(
        &self,
        lease_id: &LeaseId,
    ) -> impl Future<Output = Result<Option<String>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = lease_id.0;
        async move {
            let mut tx = db_pool
                .begin()
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            // Perform the update and return the pool type
            let row = sqlx::query(
                r#"
                UPDATE leases
                SET status = 'RELEASED'
                FROM resources r, pools p
                WHERE leases.id = $1 AND leases.status = 'ACTIVE'
                  AND leases.resource_id = r.id
                  AND r.pool_id = p.id
                RETURNING p.resource_type
                "#,
            )
            .bind(query_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let pool_type: String = match row {
                Some(r) => r.get("resource_type"),
                None => return Err(DomainError::LeaseNotFound),
            };

            sqlx::query(
                r#"
                INSERT INTO audit_log (action, lease_id)
                VALUES ('RELEASE', $1)
                "#,
            )
            .bind(query_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            tx.commit()
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(Some(pool_type))
        }
    }

    fn renew_lease(
        &self,
        lease_id: &LeaseId,
        extension_seconds: i64,
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = lease_id.0;
        async move {
            let res = sqlx::query(
                r#"
                UPDATE leases
                SET expires_at = expires_at + ($2 || ' seconds')::interval
                WHERE id = $1 AND status = 'ACTIVE'
                "#,
            )
            .bind(query_id)
            .bind(extension_seconds.to_string())
            .execute(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            if res.rows_affected() == 0 {
                return Err(DomainError::LeaseNotFound);
            }
            Ok(())
        }
    }

    fn waitlist_resource(
        &self,
        pool_type: String,
        owner_id: String,
        tenant_id: String,
        priority: i32,
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            sqlx::query(
                r#"
                INSERT INTO waitlist_entries (id, pool_type, owner_id, tenant_id, priority)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(pool_type)
            .bind(owner_id)
            .bind(tenant_id)
            .bind(priority)
            .execute(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(())
        }
    }

    fn heartbeat_lease(
        &self,
        lease_id: &LeaseId,
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = lease_id.0;
        async move {
            let res = sqlx::query(
                "UPDATE leases SET last_heartbeat_at = NOW() WHERE id = $1 AND status = 'ACTIVE'",
            )
            .bind(query_id)
            .execute(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            if res.rows_affected() == 0 {
                return Err(DomainError::LeaseNotFound);
            }
            Ok(())
        }
    }

    fn get_summary_stats(&self) -> impl Future<Output = Result<SummaryStats, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let (active_leases,): (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM leases WHERE status = 'ACTIVE'")
                    .fetch_one(&db_pool)
                    .await
                    .unwrap_or((0,));
            let (total_resources,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM resources")
                .fetch_one(&db_pool)
                .await
                .unwrap_or((0,));
            let (healthy_resources,): (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM resources WHERE status = 'Healthy'")
                    .fetch_one(&db_pool)
                    .await
                    .unwrap_or((0,));
            let (waitlist_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM waitlist_entries")
                .fetch_one(&db_pool)
                .await
                .unwrap_or((0,));

            Ok(SummaryStats {
                active_leases,
                total_resources,
                healthy_resources,
                waitlist_count,
            })
        }
    }

    fn get_recent_audit_logs(
        &self,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<AuditLogEntry>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let rows = sqlx::query(
                r#"
                SELECT id, actor_id, action, resource_id, created_at
                FROM audit_log
                ORDER BY created_at DESC
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let entries = rows
                .into_iter()
                .map(|row| AuditLogEntry {
                    id: row.get("id"),
                    actor_id: row.get("actor_id"),
                    action: row.get("action"),
                    resource_id: row.get("resource_id"),
                    created_at: row.get("created_at"),
                })
                .collect();

            Ok(entries)
        }
    }

    fn fulfill_next_waitlist_entry(
        &self,
        pool_type: String,
    ) -> impl Future<Output = Result<Option<Lease>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let mut tx = db_pool
                .begin()
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            // 1. Find next waitlist entry with Priority Aging
            let waitlist_record = sqlx::query(
                r#"
                SELECT id, owner_id, tenant_id, priority
                FROM waitlist_entries
                WHERE pool_type = $1
                ORDER BY (priority + (EXTRACT(EPOCH FROM (NOW() - created_at)) / 3600)::int * 10) DESC, created_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
                "#,
            )
            .bind(&pool_type)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let (waitlist_id, owner_id, tenant_id, priority) = match waitlist_record {
                Some(row) => (
                    row.get::<Uuid, _>("id"),
                    row.get::<String, _>("owner_id"),
                    row.get::<String, _>("tenant_id"),
                    row.get::<i32, _>("priority"),
                ),
                None => return Ok(None),
            };

            // 2. Find an available resource
            let resource_record = sqlx::query(
                r#"
                SELECT r.id
                FROM resources r
                JOIN pools p ON r.pool_id = p.id
                WHERE p.resource_type = $1 AND r.status = 'Healthy'
                AND NOT EXISTS (
                    SELECT 1 FROM leases l
                    WHERE l.resource_id = r.id AND l.status = 'ACTIVE' AND l.expires_at > NOW()
                )
                LIMIT 1
                FOR UPDATE SKIP LOCKED
                "#,
            )
            .bind(&pool_type)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let resource_id: Uuid = match resource_record {
                Some(record) => record.get("id"),
                None => return Ok(None),
            };

            // 3. Fulfill: Create Lease & Remove Waitlist Entry
            let lease_id = Uuid::new_v4();
            let now = Utc::now();
            let expires_at = now + chrono::Duration::hours(1); // Default TTL for auto-fulfillment

            sqlx::query(
                r#"
                INSERT INTO leases (id, resource_id, owner_id, tenant_id, priority, status, created_at, expires_at, last_heartbeat_at)
                VALUES ($1, $2, $3, $4, $5, 'ACTIVE', $6, $7, $6)
                "#
            )
            .bind(lease_id)
            .bind(resource_id)
            .bind(&owner_id)
            .bind(&tenant_id)
            .bind(priority)
            .bind(now)
            .bind(expires_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            sqlx::query("DELETE FROM waitlist_entries WHERE id = $1")
                .bind(waitlist_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            sqlx::query(
                r#"
                INSERT INTO audit_log (actor_id, action, resource_id, lease_id, details)
                VALUES ($1, 'WAITLIST_FULFILL', $2, $3, $4)
                "#,
            )
            .bind(&owner_id)
            .bind(resource_id)
            .bind(lease_id)
            .bind(serde_json::json!({ "pool": pool_type, "waitlist_id": waitlist_id, "priority": priority }))
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            tx.commit()
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(Some(Lease {
                id: LeaseId(lease_id),
                resource_id: ResourceId(resource_id),
                owner_id,
                tenant_id,
                priority,
                status: LeaseStatus::Active,
                created_at: now,
                expires_at,
                idempotency_key: None,
                cost_center: None,
            }))
        }
    }

    fn ping(&self) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            sqlx::query("SELECT 1")
                .execute(&db_pool)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
            Ok(())
        }
    }
}

impl PlatformRepository for PostgresRepository {
    fn list_leases(
        &self,
        filter: LeaseFilter,
    ) -> impl Future<Output = Result<Vec<Lease>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let limit = if filter.limit <= 0 || filter.limit > 500 {
                100
            } else {
                filter.limit
            };
            let rows = sqlx::query(
                r#"
                SELECT id, resource_id, owner_id, tenant_id, priority, status, created_at, expires_at, idempotency_key, cost_center
                FROM leases
                WHERE ($1::text IS NULL OR tenant_id = $1)
                  AND ($2::text IS NULL OR owner_id = $2)
                  AND ($3::text IS NULL OR status = $3)
                ORDER BY created_at DESC
                LIMIT $4
                "#,
            )
            .bind(&filter.tenant_id)
            .bind(&filter.owner_id)
            .bind(filter.status.as_ref().map(|s| s.to_ascii_uppercase()))
            .bind(limit)
            .fetch_all(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let leases = rows
                .into_iter()
                .map(|r| Lease {
                    id: LeaseId(r.get("id")),
                    resource_id: ResourceId(r.get("resource_id")),
                    owner_id: r.get("owner_id"),
                    tenant_id: r.get("tenant_id"),
                    priority: r.get("priority"),
                    status: parse_lease_status(&r.get::<String, _>("status")),
                    created_at: r.get("created_at"),
                    expires_at: r.get("expires_at"),
                    idempotency_key: r.get("idempotency_key"),
                    cost_center: r.get("cost_center"),
                })
                .collect();

            Ok(leases)
        }
    }

    fn find_lease_by_id(
        &self,
        id: &LeaseId,
    ) -> impl Future<Output = Result<Option<Lease>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = id.0;
        async move {
            let row = sqlx::query(
                r#"
                SELECT id, resource_id, owner_id, tenant_id, priority, status, created_at, expires_at, idempotency_key, cost_center
                FROM leases
                WHERE id = $1
                "#,
            )
            .bind(query_id)
            .fetch_optional(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(row.map(|r| Lease {
                id: LeaseId(r.get("id")),
                resource_id: ResourceId(r.get("resource_id")),
                owner_id: r.get("owner_id"),
                tenant_id: r.get("tenant_id"),
                priority: r.get("priority"),
                status: parse_lease_status(&r.get::<String, _>("status")),
                created_at: r.get("created_at"),
                expires_at: r.get("expires_at"),
                idempotency_key: r.get("idempotency_key"),
                cost_center: r.get("cost_center"),
            }))
        }
    }

    fn set_lease_labels(
        &self,
        id: &LeaseId,
        labels: serde_json::Value,
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = id.0;
        async move {
            let res = sqlx::query("UPDATE leases SET labels = $2 WHERE id = $1")
                .bind(query_id)
                .bind(labels)
                .execute(&db_pool)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            if res.rows_affected() == 0 {
                return Err(DomainError::LeaseNotFound);
            }
            Ok(())
        }
    }

    fn upsert_quota(
        &self,
        quota: QuotaRecord,
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            sqlx::query(
                r#"
                INSERT INTO tenant_quotas (tenant_id, pool_type, max_active_leases, soft_limit, weight)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (tenant_id, pool_type)
                DO UPDATE SET max_active_leases = EXCLUDED.max_active_leases,
                              soft_limit = EXCLUDED.soft_limit,
                              weight = EXCLUDED.weight
                "#,
            )
            .bind(&quota.tenant_id)
            .bind(&quota.pool_type)
            .bind(quota.max_active_leases)
            .bind(quota.soft_limit)
            .bind(quota.weight)
            .execute(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(())
        }
    }

    fn get_quota(
        &self,
        tenant_id: &str,
        pool_type: &str,
    ) -> impl Future<Output = Result<Option<QuotaRecord>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let tenant_id = tenant_id.to_string();
        let pool_type = pool_type.to_string();
        async move {
            let row = sqlx::query(
                r#"
                SELECT tenant_id, pool_type, max_active_leases, soft_limit, weight
                FROM tenant_quotas
                WHERE tenant_id = $1 AND pool_type = $2
                "#,
            )
            .bind(&tenant_id)
            .bind(&pool_type)
            .fetch_optional(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(row.map(|r| QuotaRecord {
                tenant_id: r.get("tenant_id"),
                pool_type: r.get("pool_type"),
                max_active_leases: r.get("max_active_leases"),
                soft_limit: r.get("soft_limit"),
                weight: r.get("weight"),
            }))
        }
    }

    fn update_resource_status(
        &self,
        id: &ResourceId,
        status: ResourceStatus,
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = id.0;
        async move {
            let res = sqlx::query(
                r#"
                UPDATE resources
                SET status = $2, version = version + 1, updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(query_id)
            .bind(status.to_string())
            .execute(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            if res.rows_affected() == 0 {
                return Err(DomainError::ResourceNotFound);
            }
            Ok(())
        }
    }

    fn count_resources_in_pool(
        &self,
        pool_id: &PoolId,
    ) -> impl Future<Output = Result<i64, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = pool_id.0;
        async move {
            let (count,): (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM resources WHERE pool_id = $1")
                    .bind(query_id)
                    .fetch_one(&db_pool)
                    .await
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
            Ok(count)
        }
    }

    fn get_pool_utilization(
        &self,
        pool_type: &str,
    ) -> impl Future<Output = Result<PoolUtilization, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let pool_type = pool_type.to_string();
        async move {
            let (total_resources,): (i64,) = sqlx::query_as(
                r#"
                SELECT COUNT(*)
                FROM resources r JOIN pools p ON r.pool_id = p.id
                WHERE p.resource_type = $1
                "#,
            )
            .bind(&pool_type)
            .fetch_one(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let (healthy_resources,): (i64,) = sqlx::query_as(
                r#"
                SELECT COUNT(*)
                FROM resources r JOIN pools p ON r.pool_id = p.id
                WHERE p.resource_type = $1 AND r.status = 'Healthy'
                "#,
            )
            .bind(&pool_type)
            .fetch_one(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let (active_leases,): (i64,) = sqlx::query_as(
                r#"
                SELECT COUNT(*)
                FROM leases l
                JOIN resources r ON l.resource_id = r.id
                JOIN pools p ON r.pool_id = p.id
                WHERE p.resource_type = $1 AND l.status = 'ACTIVE' AND l.expires_at > NOW()
                "#,
            )
            .bind(&pool_type)
            .fetch_one(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let available = (healthy_resources - active_leases).max(0);
            let utilization_pct = if healthy_resources > 0 {
                (active_leases as f64 / healthy_resources as f64) * 100.0
            } else {
                0.0
            };

            Ok(PoolUtilization {
                pool_type,
                total_resources,
                healthy_resources,
                active_leases,
                available,
                utilization_pct,
            })
        }
    }

    fn get_waitlist_position(
        &self,
        id: &Uuid,
    ) -> impl Future<Output = Result<Option<WaitlistPosition>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = *id;
        async move {
            let entry = sqlx::query(
                "SELECT id, pool_type, priority, created_at FROM waitlist_entries WHERE id = $1",
            )
            .bind(query_id)
            .fetch_optional(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let Some(entry) = entry else {
                return Ok(None);
            };

            let pool_type: String = entry.get("pool_type");
            let priority: i32 = entry.get("priority");

            // Rank uses the same priority-aging ordering as fulfillment so the
            // reported position reflects the order entries will actually be served.
            let (position,): (i64,) = sqlx::query_as(
                r#"
                SELECT COUNT(*) + 1
                FROM waitlist_entries other
                WHERE other.pool_type = $1
                  AND (other.priority + (EXTRACT(EPOCH FROM (NOW() - other.created_at)) / 3600)::int * 10)
                    > (SELECT (w.priority + (EXTRACT(EPOCH FROM (NOW() - w.created_at)) / 3600)::int * 10)
                       FROM waitlist_entries w WHERE w.id = $2)
                "#,
            )
            .bind(&pool_type)
            .bind(query_id)
            .fetch_one(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let (total_waiting,): (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM waitlist_entries WHERE pool_type = $1")
                    .bind(&pool_type)
                    .fetch_one(&db_pool)
                    .await
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(Some(WaitlistPosition {
                id: query_id,
                pool_type,
                priority,
                position,
                total_waiting,
            }))
        }
    }

    fn cost_report(
        &self,
        group_by: CostGroupBy,
    ) -> impl Future<Output = Result<Vec<CostRow>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            // `group_by.column()` returns a fixed, validated identifier (never
            // user input), so interpolating it into the query is safe.
            let column = group_by.column();
            let query = format!(
                r#"
                SELECT COALESCE({column}, '(none)') AS grp, COUNT(*) AS cnt
                FROM leases
                WHERE status = 'ACTIVE'
                GROUP BY grp
                ORDER BY cnt DESC
                "#
            );

            let rows = sqlx::query(&query)
                .fetch_all(&db_pool)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let rows = rows
                .into_iter()
                .map(|r| CostRow {
                    group: r.get("grp"),
                    active_leases: r.get("cnt"),
                })
                .collect();

            Ok(rows)
        }
    }

    fn create_reservation(
        &self,
        reservation: Reservation,
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            sqlx::query(
                r#"
                INSERT INTO reservations
                    (id, pool_type, owner_id, tenant_id, priority, ttl_seconds, constraints, start_at, status, lease_id, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
            )
            .bind(reservation.id)
            .bind(&reservation.pool_type)
            .bind(&reservation.owner_id)
            .bind(&reservation.tenant_id)
            .bind(reservation.priority)
            .bind(reservation.ttl_seconds)
            .bind(&reservation.constraints)
            .bind(reservation.start_at)
            .bind(&reservation.status)
            .bind(reservation.lease_id.map(|l| l.0))
            .bind(reservation.created_at)
            .execute(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(())
        }
    }

    fn get_reservation(
        &self,
        id: &Uuid,
    ) -> impl Future<Output = Result<Option<Reservation>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = *id;
        async move {
            let row = sqlx::query(
                r#"
                SELECT id, pool_type, owner_id, tenant_id, priority, ttl_seconds, constraints, start_at, status, lease_id, created_at
                FROM reservations
                WHERE id = $1
                "#,
            )
            .bind(query_id)
            .fetch_optional(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(row.map(map_reservation))
        }
    }

    fn list_due_reservations(
        &self,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<Reservation>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let rows = sqlx::query(
                r#"
                SELECT id, pool_type, owner_id, tenant_id, priority, ttl_seconds, constraints, start_at, status, lease_id, created_at
                FROM reservations
                WHERE status = 'PENDING' AND start_at <= NOW()
                ORDER BY start_at ASC
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(rows.into_iter().map(map_reservation).collect())
        }
    }

    fn complete_reservation(
        &self,
        id: &Uuid,
        lease_id: &LeaseId,
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = *id;
        let lease_id = lease_id.0;
        async move {
            sqlx::query(
                "UPDATE reservations SET status = 'FULFILLED', lease_id = $2, last_error = NULL WHERE id = $1",
            )
            .bind(query_id)
            .bind(lease_id)
            .execute(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
            Ok(())
        }
    }

    fn fail_reservation(
        &self,
        id: &Uuid,
        error: &str,
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = *id;
        let error = error.to_string();
        async move {
            sqlx::query("UPDATE reservations SET status = 'FAILED', last_error = $2 WHERE id = $1")
                .bind(query_id)
                .bind(error)
                .execute(&db_pool)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
            Ok(())
        }
    }
}

fn map_reservation(r: sqlx::postgres::PgRow) -> Reservation {
    Reservation {
        id: r.get("id"),
        pool_type: r.get("pool_type"),
        owner_id: r.get("owner_id"),
        tenant_id: r.get("tenant_id"),
        priority: r.get("priority"),
        ttl_seconds: r.get("ttl_seconds"),
        constraints: r.get("constraints"),
        start_at: r.get("start_at"),
        status: r.get("status"),
        lease_id: r.get::<Option<Uuid>, _>("lease_id").map(LeaseId),
        created_at: r.get("created_at"),
    }
}
