use crate::domain::{
    pool::Pool, resource::Resource, lease::Lease,
    PoolId, ResourceId, LeaseId,
    error::DomainError
};
use std::future::Future;

pub trait PoolRepository: Send + Sync {
    fn create(&self, pool: Pool) -> impl Future<Output = Result<(), DomainError>> + Send;
    fn find_by_id(&self, id: &PoolId) -> impl Future<Output = Result<Option<Pool>, DomainError>> + Send;
}

pub trait ResourceRepository: Send + Sync {
    fn create(&self, resource: Resource) -> impl Future<Output = Result<(), DomainError>> + Send;
    fn find_by_id(&self, id: &ResourceId) -> impl Future<Output = Result<Option<Resource>, DomainError>> + Send;
}

pub trait AllocationRepository: Send + Sync {
    /// Atomic operation to find an available resource matching the criteria
    /// and create a lease for it in a single transaction.
    fn allocate_resource(
        &self, 
        pool_type: String, 
        owner_id: String, 
        tenant_id: String, 
        ttl_seconds: i64,
        idempotency_key: Option<String>,
        cost_center: Option<String>
    ) -> impl Future<Output = Result<Lease, DomainError>> + Send;
    
    fn release_lease(&self, lease_id: &LeaseId) -> impl Future<Output = Result<Option<String>, DomainError>> + Send;
    
    fn renew_lease(&self, lease_id: &LeaseId, extension_seconds: i64) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn waitlist_resource(
        &self,
        pool_type: String,
        owner_id: String,
        tenant_id: String,
        priority: i32
    ) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn get_summary_stats(&self) -> impl Future<Output = Result<SummaryStats, DomainError>> + Send;

    fn get_recent_audit_logs(&self, limit: i64) -> impl Future<Output = Result<Vec<AuditLogEntry>, DomainError>> + Send;

    /// Attempts to fulfill the next pending waitlist entry for a given pool type.
    /// If a resource is available, it creates a lease and returns it.
    fn fulfill_next_waitlist_entry(
        &self,
        pool_type: String
    ) -> impl Future<Output = Result<Option<Lease>, DomainError>> + Send;
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct SummaryStats {
    pub active_leases: i64,
    pub total_resources: i64,
    pub waitlist_count: i64,
    pub healthy_resources: i64,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct AuditLogEntry {
    pub id: i64,
    pub actor_id: Option<String>,
    pub action: Option<String>,
    pub resource_id: Option<uuid::Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
