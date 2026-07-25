use crate::domain::{
    error::DomainError, lease::Lease, pool::Pool, resource::Resource, LeaseId, PoolId, ResourceId,
};
use std::future::Future;

pub trait PoolRepository: Send + Sync {
    fn create(&self, pool: Pool) -> impl Future<Output = Result<(), DomainError>> + Send;
    fn find_by_id(
        &self,
        id: &PoolId,
    ) -> impl Future<Output = Result<Option<Pool>, DomainError>> + Send;
    fn find_by_name(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<Pool>, DomainError>> + Send;
}

pub trait ResourceRepository: Send + Sync {
    fn create(&self, resource: Resource) -> impl Future<Output = Result<(), DomainError>> + Send;
    fn find_by_id(
        &self,
        id: &ResourceId,
    ) -> impl Future<Output = Result<Option<Resource>, DomainError>> + Send;
}

pub trait AllocationRepository: Send + Sync {
    /// Atomic operation to find an available resource matching the criteria
    /// and create a lease for it in a single transaction.
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
    ) -> impl Future<Output = Result<Lease, DomainError>> + Send;

    /// Lightweight connectivity check used by readiness/health probes.
    fn ping(&self) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn release_lease(
        &self,
        lease_id: &LeaseId,
    ) -> impl Future<Output = Result<Option<String>, DomainError>> + Send;

    fn renew_lease(
        &self,
        lease_id: &LeaseId,
        extension_seconds: i64,
    ) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn waitlist_resource(
        &self,
        pool_type: String,
        owner_id: String,
        tenant_id: String,
        priority: i32,
    ) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn heartbeat_lease(
        &self,
        lease_id: &LeaseId,
    ) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn get_summary_stats(&self) -> impl Future<Output = Result<SummaryStats, DomainError>> + Send;

    fn get_recent_audit_logs(
        &self,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<AuditLogEntry>, DomainError>> + Send;

    /// Attempts to fulfill the next pending waitlist entry for a given pool type.
    /// If a resource is available, it creates a lease and returns it.
    fn fulfill_next_waitlist_entry(
        &self,
        pool_type: String,
    ) -> impl Future<Output = Result<Option<Lease>, DomainError>> + Send;
}

#[derive(serde::Serialize, Clone, Debug, Default)]
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

/// Optional filters for listing leases.
#[derive(Clone, Debug, Default)]
pub struct LeaseFilter {
    pub tenant_id: Option<String>,
    pub owner_id: Option<String>,
    pub status: Option<String>,
    pub limit: i64,
}

/// A persisted tenant quota, including the fair-share fields added in the
/// platform-features migration.
#[derive(Clone, Debug)]
pub struct QuotaRecord {
    pub tenant_id: String,
    pub pool_type: String,
    pub max_active_leases: i32,
    pub soft_limit: Option<i32>,
    pub weight: i32,
}

/// Point-in-time utilization snapshot for a single pool type.
#[derive(Clone, Debug, Default)]
pub struct PoolUtilization {
    pub pool_type: String,
    pub total_resources: i64,
    pub healthy_resources: i64,
    pub active_leases: i64,
    pub available: i64,
    pub utilization_pct: f64,
}

/// A caller's position within a pool's waitlist.
#[derive(Clone, Debug)]
pub struct WaitlistPosition {
    pub id: uuid::Uuid,
    pub pool_type: String,
    pub priority: i32,
    pub position: i64,
    pub total_waiting: i64,
}

/// A single grouped row of the active-lease cost report.
#[derive(Clone, Debug)]
pub struct CostRow {
    pub group: String,
    pub active_leases: i64,
}

/// The dimension by which the cost report aggregates active leases.
#[derive(Clone, Copy, Debug)]
pub enum CostGroupBy {
    Tenant,
    CostCenter,
}

impl CostGroupBy {
    /// Returns the (validated) SQL column this dimension maps to. Using an enum
    /// here keeps the column name off the request path and avoids SQL injection.
    pub fn column(&self) -> &'static str {
        match self {
            CostGroupBy::Tenant => "tenant_id",
            CostGroupBy::CostCenter => "cost_center",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CostGroupBy::Tenant => "tenant",
            CostGroupBy::CostCenter => "cost_center",
        }
    }
}

/// Read/administration operations that support the higher-level platform APIs
/// (lease querying, quota management, reservations, utilization and reporting).
///
/// This is deliberately a separate trait from the core allocation traits so that
/// adding platform features does not churn the existing `AllocationService`
/// repository bounds or its test mocks.
pub trait PlatformRepository: Send + Sync {
    fn list_leases(
        &self,
        filter: LeaseFilter,
    ) -> impl Future<Output = Result<Vec<Lease>, DomainError>> + Send;

    fn find_lease_by_id(
        &self,
        id: &LeaseId,
    ) -> impl Future<Output = Result<Option<Lease>, DomainError>> + Send;

    fn set_lease_labels(
        &self,
        id: &LeaseId,
        labels: serde_json::Value,
    ) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn upsert_quota(
        &self,
        quota: QuotaRecord,
    ) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn get_quota(
        &self,
        tenant_id: &str,
        pool_type: &str,
    ) -> impl Future<Output = Result<Option<QuotaRecord>, DomainError>> + Send;

    fn update_resource_status(
        &self,
        id: &ResourceId,
        status: crate::domain::ResourceStatus,
    ) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn count_resources_in_pool(
        &self,
        pool_id: &PoolId,
    ) -> impl Future<Output = Result<i64, DomainError>> + Send;

    fn get_pool_utilization(
        &self,
        pool_type: &str,
    ) -> impl Future<Output = Result<PoolUtilization, DomainError>> + Send;

    fn get_waitlist_position(
        &self,
        id: &uuid::Uuid,
    ) -> impl Future<Output = Result<Option<WaitlistPosition>, DomainError>> + Send;

    fn cost_report(
        &self,
        group_by: CostGroupBy,
    ) -> impl Future<Output = Result<Vec<CostRow>, DomainError>> + Send;

    fn create_reservation(
        &self,
        reservation: crate::domain::reservation::Reservation,
    ) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn get_reservation(
        &self,
        id: &uuid::Uuid,
    ) -> impl Future<Output = Result<Option<crate::domain::reservation::Reservation>, DomainError>> + Send;

    fn list_due_reservations(
        &self,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<crate::domain::reservation::Reservation>, DomainError>> + Send;

    fn complete_reservation(
        &self,
        id: &uuid::Uuid,
        lease_id: &LeaseId,
    ) -> impl Future<Output = Result<(), DomainError>> + Send;

    fn fail_reservation(
        &self,
        id: &uuid::Uuid,
        error: &str,
    ) -> impl Future<Output = Result<(), DomainError>> + Send;
}
