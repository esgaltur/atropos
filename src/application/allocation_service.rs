use std::sync::Arc;
use moka::future::Cache;
use std::time::Duration;
use crate::domain::{
    lease::Lease, LeaseId, pool::Pool,
    error::DomainError,
    repository::{AllocationRepository, PoolRepository, SummaryStats, AuditLogEntry},
};

/// The core application service responsible for orchestrating resource allocations and leases.
///
/// This service coordinates between the domain logic and the infrastructure repositories,
/// providing high-level use cases for resource management, including allocation with
/// optional waitlisting and preemption.
#[derive(Clone)]
pub struct AllocationService<R: AllocationRepository + PoolRepository> {
    repo: Arc<R>,
    #[allow(dead_code)]
    pool_cache: Cache<String, Pool>,
}

impl<R: AllocationRepository + PoolRepository> AllocationService<R> {
    /// Creates a new instance of the `AllocationService` with an internal cache for performance.
    pub fn new(repo: Arc<R>) -> Self {
        let pool_cache = Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(30))
            .build();

        Self { repo, pool_cache }
    }

    /// Attempts to allocate a resource from a specified pool.
    ///
    /// # Parameters
    /// - `pool_type`: The type of resource requested (e.g., "A100-GPU").
    /// - `owner_id`: The identifier of the entity requesting the resource.
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
        ttl_seconds: i64,
        idempotency_key: Option<String>,
        waitlist: Option<bool>,
        preempt: Option<bool>
    ) -> Result<Lease, DomainError> {
        // 1. Caching Layer Check
        // If we had pool_id we'd cache by ID, for now by type name
        
        let result = self.repo.allocate_resource(
            pool_type.clone(), 
            owner_id.clone(), 
            tenant_id.clone(), 
            ttl_seconds,
            idempotency_key.clone(),
            None 
        ).await;

        match result {
            Err(DomainError::NoResourcesAvailable) if preempt.unwrap_or(false) => {
                tracing::info!("Pool {} is full. Attempting preemption...", pool_type);
                // In a real system we'd find the oldest/lowest priority lease here
                Err(DomainError::InfrastructureError("Preemption required but logic in repo is pending".to_string()))
            },
            Err(DomainError::NoResourcesAvailable) if waitlist.unwrap_or(false) => {
                tracing::info!("Pool {} is full. Adding {} to waitlist.", pool_type, owner_id);
                self.repo.waitlist_resource(
                    pool_type, 
                    owner_id, 
                    tenant_id, 
                    0 // Default priority
                ).await?;
                Err(DomainError::InfrastructureError("Added to waitlist".to_string()))
            },
            other => other,
        }
    }

    /// Releases an active lease and returns the resource to the available pool.
    pub async fn release(&self, lease_id: LeaseId) -> Result<(), DomainError> {
        self.repo.release_lease(&lease_id).await
    }

    /// Extends the duration of an active lease by the specified number of seconds.
    pub async fn renew(&self, lease_id: LeaseId, extension_seconds: i64) -> Result<(), DomainError> {
        self.repo.renew_lease(&lease_id, extension_seconds).await
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