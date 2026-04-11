use chrono::Utc;
use sqlx::{PgPool, Row};
use std::future::Future;
use uuid::Uuid;

use crate::domain::{
    error::DomainError,
    lease::Lease,
    pool::Pool,
    repository::{
        AllocationRepository, AuditLogEntry, PoolRepository, ResourceRepository, SummaryStats,
    },
    resource::Resource,
    LeaseId, LeaseStatus, PoolId, ResourceId,
};

pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PoolRepository for PostgresRepository {
    fn create(&self, pool: Pool) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            sqlx::query(
                r#"
                INSERT INTO pools (id, name, resource_type, policy, created_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(pool.id.0)
            .bind(pool.name)
            .bind(pool.resource_type)
            .bind(pool.policy.to_string())
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
                SELECT id, name, resource_type, policy, created_at
                FROM pools
                WHERE id = $1
                "#,
            )
            .bind(query_id)
            .fetch_optional(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let pool = row.map(|r| Pool {
                id: PoolId(r.get("id")),
                name: r.get("name"),
                resource_type: r.get("resource_type"),
                policy: r.get::<String, _>("policy").parse().unwrap(),
                created_at: r.get("created_at"),
            });

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

            let res = row.map(|r| Resource {
                id: ResourceId(r.get("id")),
                pool_id: PoolId(r.get("pool_id")),
                external_id: r.get("external_id"),
                status: r.get::<String, _>("status").parse().unwrap(),
                attributes: r.get("attributes"),
                version: r.get("version"),
                updated_at: r.get("updated_at"),
            });

            Ok(res)
        }
    }
}

impl AllocationRepository for PostgresRepository {
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
    ) -> impl Future<Output = Result<Lease, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let mut tx = db_pool
                .begin()
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            // 0. Quota Check
            let quota_record = sqlx::query(
                r#"
                SELECT max_active_leases 
                FROM tenant_quotas 
                WHERE tenant_id = $1 AND pool_type = $2
                "#
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
                    "#
                )
                .bind(&tenant_id)
                .bind(&pool_type)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

                if current_leases >= max_leases as i64 {
                    return Err(DomainError::InfrastructureError(format!("Tenant quota of {} exceeded for pool {}", max_leases, pool_type)));
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
                Some(record) => (
                    record.get::<Uuid, _>("id"),
                    false
                ),
                None => {
                    // 2. Preemption: Find active lease in the pool with lower priority + Attribute Match
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
                        "#
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

            let action = if preemption_occured { "PREEMPT_ALLOCATE" } else { "ALLOCATE" };
            let details = serde_json::json!({ "ttl": ttl_seconds, "pool": pool_type, "priority": priority });
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
                "UPDATE leases SET last_heartbeat_at = NOW() WHERE id = $1 AND status = 'ACTIVE'"
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
}
