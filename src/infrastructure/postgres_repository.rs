use sqlx::{PgPool, Row};
use std::future::Future;
use uuid::Uuid;
use chrono::Utc;

use crate::domain::{
    pool::Pool, resource::Resource, lease::Lease,
    PoolId, ResourceId, LeaseId,
    AllocationPolicy, ResourceStatus, LeaseStatus,
    error::DomainError, repository::{PoolRepository, ResourceRepository, AllocationRepository, SummaryStats, AuditLogEntry}
};

#[derive(Clone)]
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
                "#
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

    fn find_by_id(&self, id: &PoolId) -> impl Future<Output = Result<Option<Pool>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = id.0;
        async move {
            let record = sqlx::query(
                r#"
                SELECT id, name, resource_type, policy, created_at
                FROM pools WHERE id = $1
                "#
            )
            .bind(query_id)
            .fetch_optional(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            if let Some(row) = record {
                let id: Uuid = row.try_get("id").unwrap_or_default();
                let name: String = row.try_get("name").unwrap_or_default();
                let resource_type: String = row.try_get("resource_type").unwrap_or_default();
                let policy_str: String = row.try_get("policy").unwrap_or_default();
                let created_at: Option<chrono::DateTime<Utc>> = row.try_get("created_at").unwrap_or_default();

                let policy = policy_str.parse::<AllocationPolicy>()
                    .map_err(|_| DomainError::InfrastructureError("Invalid policy in DB".to_string()))?;
                
                Ok(Some(Pool {
                    id: PoolId(id),
                    name,
                    resource_type,
                    policy,
                    created_at: created_at.unwrap_or_else(Utc::now),
                }))
            } else {
                Ok(None)
            }
        }
    }
}

impl ResourceRepository for PostgresRepository {
    fn create(&self, resource: Resource) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            sqlx::query(
                r#"
                INSERT INTO resources (id, pool_id, external_id, status, attributes, version, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#
            )
            .bind(resource.id.0)
            .bind(resource.pool_id.0)
            .bind(resource.external_id)
            .bind(resource.status.to_string())
            .bind(resource.attributes)
            .bind(resource.version)
            .bind(resource.updated_at)
            .execute(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
            
            Ok(())
        }
    }

    fn find_by_id(&self, id: &ResourceId) -> impl Future<Output = Result<Option<Resource>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = id.0;
        async move {
            let record = sqlx::query(
                r#"
                SELECT id, pool_id, external_id, status, attributes, version, updated_at
                FROM resources WHERE id = $1
                "#
            )
            .bind(query_id)
            .fetch_optional(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            if let Some(row) = record {
                let id: Uuid = row.try_get("id").unwrap_or_default();
                let pool_id: Option<Uuid> = row.try_get("pool_id").unwrap_or_default();
                let external_id: String = row.try_get("external_id").unwrap_or_default();
                let status_str: String = row.try_get("status").unwrap_or_default();
                let attributes: Option<serde_json::Value> = row.try_get("attributes").unwrap_or_default();
                let version: Option<i64> = row.try_get("version").unwrap_or_default();
                let updated_at: Option<chrono::DateTime<Utc>> = row.try_get("updated_at").unwrap_or_default();

                let status = status_str.parse::<ResourceStatus>()
                    .map_err(|_| DomainError::InfrastructureError("Invalid status in DB".to_string()))?;
                
                let pool_id_uuid = pool_id.ok_or_else(|| DomainError::InfrastructureError("Missing pool_id".to_string()))?;

                Ok(Some(Resource {
                    id: ResourceId(id),
                    pool_id: PoolId(pool_id_uuid),
                    external_id,
                    status,
                    attributes: attributes.unwrap_or_default(),
                    version: version.unwrap_or(0),
                    updated_at: updated_at.unwrap_or_else(Utc::now),
                }))
            } else {
                Ok(None)
            }
        }
    }
}

impl AllocationRepository for PostgresRepository {
    fn allocate_resource(
        &self,
        pool_type: String,
        owner_id: String,
        tenant_id: String,
        ttl_seconds: i64,
        idempotency_key: Option<String>,
        cost_center: Option<String>
    ) -> impl Future<Output = Result<Lease, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let mut tx = db_pool.begin().await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            // I originally tried using a mutex here, but I realized it wouldn't scale 
            // across multiple API nodes. Switching to SKIP LOCKED was a bit of a 
            // "lightbulb moment" for handling the concurrency at the DB level.
            // It's much cleaner and lets Postgres handle the heavy lifting.
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
                "#
            )
            .bind(&pool_type)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let resource_id: Uuid = match resource_record {
                Some(record) => record.try_get("id").map_err(|_| DomainError::InfrastructureError("Missing id".to_string()))?,
                None => return Err(DomainError::NoResourcesAvailable),
            };

            let lease_id = Uuid::new_v4();
            let now = Utc::now();
            let expires_at = now + chrono::Duration::seconds(ttl_seconds);

            // TODO(esgaltur): I should probably check if the owner already has too many 
            // active leases before allowing a new one. Adding a quota system is on my list.
            sqlx::query(
                r#"
                INSERT INTO leases (id, resource_id, owner_id, tenant_id, status, created_at, expires_at, idempotency_key, cost_center)
                VALUES ($1, $2, $3, $4, 'ACTIVE', $5, $6, $7, $8)
                "#
            )
            .bind(lease_id)
            .bind(resource_id)
            .bind(&owner_id)
            .bind(&tenant_id)
            .bind(now)
            .bind(expires_at)
            .bind(&idempotency_key)
            .bind(&cost_center)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let details = serde_json::json!({ "ttl": ttl_seconds, "pool": pool_type });
            sqlx::query(
                r#"
                INSERT INTO audit_log (actor_id, action, resource_id, lease_id, details)
                VALUES ($1, 'ALLOCATE', $2, $3, $4)
                "#
            )
            .bind(&owner_id)
            .bind(resource_id)
            .bind(lease_id)
            .bind(details)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            tx.commit().await.map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(Lease {
                id: LeaseId(lease_id),
                resource_id: ResourceId(resource_id),
                owner_id,
                tenant_id,
                status: LeaseStatus::Active,
                created_at: now,
                expires_at,
                idempotency_key,
                cost_center,
            })
        }
    }

    fn release_lease(&self, lease_id: &LeaseId) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = lease_id.0;
        async move {
            let mut tx = db_pool.begin().await.map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
            let res = sqlx::query(
                r#"
                UPDATE leases
                SET status = 'RELEASED'
                WHERE id = $1 AND status = 'ACTIVE'
                "#
            )
            .bind(query_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            if res.rows_affected() == 0 {
                return Err(DomainError::LeaseNotFound);
            }

            sqlx::query(
                r#"
                INSERT INTO audit_log (action, lease_id)
                VALUES ('RELEASE', $1)
                "#
            )
            .bind(query_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            tx.commit().await.map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(())
        }
    }

    fn renew_lease(&self, lease_id: &LeaseId, extension_seconds: i64) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        let query_id = lease_id.0;
        async move {
            let res = sqlx::query(
                r#"
                UPDATE leases
                SET expires_at = expires_at + ($2 || ' seconds')::interval
                WHERE id = $1 AND status = 'ACTIVE'
                "#
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
        priority: i32
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            sqlx::query(
                r#"
                INSERT INTO waitlist_entries (id, pool_type, owner_id, tenant_id, priority)
                VALUES ($1, $2, $3, $4, $5)
                "#
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

    fn get_summary_stats(&self) -> impl Future<Output = Result<SummaryStats, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let (active_leases,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM leases WHERE status = 'ACTIVE'")
                .fetch_one(&db_pool).await.unwrap_or((0,));
            let (total_resources,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM resources")
                .fetch_one(&db_pool).await.unwrap_or((0,));
            let (healthy_resources,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM resources WHERE status = 'Healthy'")
                .fetch_one(&db_pool).await.unwrap_or((0,));
            let (waitlist_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM waitlist_entries")
                .fetch_one(&db_pool).await.unwrap_or((0,));

            Ok(SummaryStats {
                active_leases,
                total_resources,
                healthy_resources,
                waitlist_count,
            })
        }
    }

    fn get_recent_audit_logs(&self, limit: i64) -> impl Future<Output = Result<Vec<AuditLogEntry>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let rows = sqlx::query(
                r#"
                SELECT id, actor_id, action, resource_id, created_at
                FROM audit_log
                ORDER BY created_at DESC
                LIMIT $1
                "#
            )
            .bind(limit)
            .fetch_all(&db_pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let entries = rows.into_iter().map(|row| {
                AuditLogEntry {
                    id: row.get("id"),
                    actor_id: row.get("actor_id"),
                    action: row.get("action"),
                    resource_id: row.get("resource_id"),
                    created_at: row.get("created_at"),
                }
            }).collect();

            Ok(entries)
        }
    }

    fn fulfill_next_waitlist_entry(
        &self,
        pool_type: String
    ) -> impl Future<Output = Result<Option<Lease>, DomainError>> + Send {
        let db_pool = self.pool.clone();
        async move {
            let mut tx = db_pool.begin().await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            // 1. Find next waitlist entry
            let waitlist_record = sqlx::query(
                r#"
                SELECT id, owner_id, tenant_id
                FROM waitlist_entries
                WHERE pool_type = $1
                ORDER BY priority DESC, created_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
                "#
            )
            .bind(&pool_type)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            let (waitlist_id, owner_id, tenant_id) = match waitlist_record {
                Some(row) => (
                    row.get::<Uuid, _>("id"),
                    row.get::<String, _>("owner_id"),
                    row.get::<String, _>("tenant_id")
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
                "#
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
                INSERT INTO leases (id, resource_id, owner_id, tenant_id, status, created_at, expires_at)
                VALUES ($1, $2, $3, $4, 'ACTIVE', $5, $6)
                "#
            )
            .bind(lease_id)
            .bind(resource_id)
            .bind(&owner_id)
            .bind(&tenant_id)
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
                "#
            )
            .bind(&owner_id)
            .bind(resource_id)
            .bind(lease_id)
            .bind(serde_json::json!({ "pool": pool_type, "waitlist_id": waitlist_id }))
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            tx.commit().await.map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

            Ok(Some(Lease {
                id: LeaseId(lease_id),
                resource_id: ResourceId(resource_id),
                owner_id,
                tenant_id,
                status: LeaseStatus::Active,
                created_at: now,
                expires_at,
                idempotency_key: None,
                cost_center: None,
            }))
        }
    }
}
