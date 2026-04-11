use crate::domain::{
    error::DomainError,
    lease::Lease,
    repository::{AllocationRepository, AuditLogEntry, PoolRepository, SummaryStats},
    LeaseId,
};
use moka::future::Cache;
use std::sync::Arc;

pub struct AllocationService<R: AllocationRepository + PoolRepository> {
    repo: Arc<R>,
    // Optional caching layer for fast capacity checks
    _cache: Cache<String, i64>,
}

impl<R: AllocationRepository + PoolRepository> AllocationService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            _cache: Cache::new(1000),
        }
    }

    /// Primary entry point for allocating a resource from a pool.
    ///
    /// This method manages the high-level flow:
    /// 1. Attempt database-native atomic allocation (SKIP LOCKED).
    /// 2. If pool is full and waitlist is enabled, queue the request.
    /// 3. Returns a `Lease` on success or an Error on failure/waitlist.
    ///
    /// # Arguments
    /// - `pool_type`: The type of resource requested (e.g., "GPU").
    /// - `owner_id`: Unique identifier for the user/service requesting the resource.
    /// - `tenant_id`: The project or tenant context for the allocation.
    /// - `ttl_seconds`: Duration in seconds for which the resource is leased.
    /// - `idempotency_key`: Optional key to prevent double-allocations on retry.
    /// - `waitlist`: If true, the request will be added to a waitlist if no resources are available.
    /// - `preempt`: If true, the service may attempt to reclaim resources from lower-priority leases (Experimental).
    #[allow(clippy::too_many_arguments)]
    pub async fn allocate(
        &self,
        pool_type: String,
        owner_id: String,
        tenant_id: String,
        priority: i32,
        ttl_seconds: i64,
        constraints: Option<serde_json::Value>,
        spread_by: Option<String>,
        idempotency_key: Option<String>,
        waitlist: Option<bool>,
        preempt: Option<bool>,
    ) -> Result<Lease, DomainError> {
        // 1. Attempt allocation
        let result = self
            .repo
            .allocate_resource(
                pool_type.clone(),
                owner_id.clone(),
                tenant_id.clone(),
                priority,
                ttl_seconds,
                constraints,
                spread_by,
                idempotency_key.clone(),
                None,
            )
            .await;

        match result {
            Err(DomainError::NoResourcesAvailable) if waitlist.unwrap_or(false) => {
                tracing::info!(
                    "Pool {} is full and no preemption possible. Adding {} to waitlist.",
                    pool_type,
                    owner_id
                );
                self.repo
                    .waitlist_resource(
                        pool_type, owner_id, tenant_id, priority,
                    )
                    .await?;
                Err(DomainError::InfrastructureError(
                    "Added to waitlist".to_string(),
                ))
            }
            Err(DomainError::NoResourcesAvailable) if preempt.unwrap_or(false) => {
                Err(DomainError::InfrastructureError(
                    "Preemption required but logic in repo is pending".to_string(),
                ))
            }
            other => other,
        }
    }

    /// Releases an active lease and returns the resource to the available pool.
    pub async fn release(&self, lease_id: LeaseId) -> Result<(), DomainError> {
        if let Some(pool_type) = self.repo.release_lease(&lease_id).await? {
            // Automatically fulfill the next waitlist entry for this pool type
            if let Ok(Some(lease)) = self
                .repo
                .fulfill_next_waitlist_entry(pool_type.clone())
                .await
            {
                tracing::info!("Automatically fulfilled waitlist entry after manual release for pool {} (Lease: {})", pool_type, lease.id);
            }
        }
        Ok(())
    }

    /// Extends the duration of an active lease by the specified number of seconds.
    pub async fn renew(
        &self,
        lease_id: LeaseId,
        extension_seconds: i64,
    ) -> Result<(), DomainError> {
        self.repo.renew_lease(&lease_id, extension_seconds).await
    }

    /// Reports that the client holding the lease is still active.
    pub async fn heartbeat(&self, lease_id: LeaseId) -> Result<(), DomainError> {
        self.repo.heartbeat_lease(&lease_id).await
    }

    /// Retrieves aggregate statistics for all resource pools and active leases.
    pub async fn get_stats(&self) -> Result<SummaryStats, DomainError> {
        self.repo.get_summary_stats().await
    }

    /// Retrieves a list of recent audit log entries for resource lifecycle events.
    pub async fn get_recent_logs(&self, limit: i64) -> Result<Vec<AuditLogEntry>, DomainError> {
        self.repo.get_recent_audit_logs(limit).await
    }
}
