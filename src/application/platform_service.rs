use crate::domain::{
    error::DomainError,
    lease::Lease,
    repository::{
        CostGroupBy, CostRow, LeaseFilter, PlatformRepository, PoolRepository, PoolUtilization,
        QuotaRecord, WaitlistPosition,
    },
    reservation::Reservation,
    LeaseId, PoolId, ResourceId, ResourceStatus,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

/// Synchronous, request-scoped platform operations: lease querying, labelling,
/// quota administration, resource status changes, utilization/reporting and
/// reservation management.
pub struct PlatformService<R: PlatformRepository + PoolRepository> {
    repo: Arc<R>,
}

impl<R: PlatformRepository + PoolRepository> PlatformService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn list_leases(&self, filter: LeaseFilter) -> Result<Vec<Lease>, DomainError> {
        self.repo.list_leases(filter).await
    }

    pub async fn get_lease(&self, id: LeaseId) -> Result<Lease, DomainError> {
        self.repo
            .find_lease_by_id(&id)
            .await?
            .ok_or(DomainError::LeaseNotFound)
    }

    pub async fn set_lease_labels(
        &self,
        id: LeaseId,
        labels: serde_json::Value,
    ) -> Result<(), DomainError> {
        self.repo.set_lease_labels(&id, labels).await
    }

    pub async fn set_quota(
        &self,
        tenant_id: String,
        pool_type: String,
        max_active_leases: i32,
        soft_limit: Option<i32>,
        weight: Option<i32>,
    ) -> Result<QuotaRecord, DomainError> {
        let quota = QuotaRecord {
            tenant_id,
            pool_type,
            max_active_leases,
            soft_limit,
            weight: weight.unwrap_or(1),
        };
        self.repo.upsert_quota(quota.clone()).await?;
        Ok(quota)
    }

    pub async fn get_quota(
        &self,
        tenant_id: &str,
        pool_type: &str,
    ) -> Result<Option<QuotaRecord>, DomainError> {
        self.repo.get_quota(tenant_id, pool_type).await
    }

    pub async fn update_resource_status(
        &self,
        id: ResourceId,
        status: ResourceStatus,
    ) -> Result<(), DomainError> {
        self.repo.update_resource_status(&id, status).await
    }

    pub async fn get_pool_utilization(
        &self,
        pool_type: &str,
    ) -> Result<PoolUtilization, DomainError> {
        self.repo.get_pool_utilization(pool_type).await
    }

    pub async fn get_waitlist_position(&self, id: Uuid) -> Result<WaitlistPosition, DomainError> {
        self.repo
            .get_waitlist_position(&id)
            .await?
            .ok_or(DomainError::ResourceNotFound)
    }

    pub async fn cost_report(&self, group_by: CostGroupBy) -> Result<Vec<CostRow>, DomainError> {
        self.repo.cost_report(group_by).await
    }

    /// Enforces a pool's optional `max_capacity` before a new resource is
    /// registered. Returns `Err(QuotaExceeded)` when the pool is already full.
    pub async fn ensure_capacity_for_new_resource(
        &self,
        pool_id: PoolId,
    ) -> Result<(), DomainError> {
        let pool = self
            .repo
            .find_by_id(&pool_id)
            .await?
            .ok_or(DomainError::PoolNotFound)?;

        if let Some(max) = pool.max_capacity {
            let current = self.repo.count_resources_in_pool(&pool_id).await?;
            if current >= max as i64 {
                return Err(DomainError::QuotaExceeded(format!(
                    "pool {} is at capacity ({}/{})",
                    pool.name, current, max
                )));
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_reservation(
        &self,
        pool_type: String,
        owner_id: String,
        tenant_id: String,
        priority: i32,
        ttl_seconds: i64,
        constraints: Option<serde_json::Value>,
        start_at: DateTime<Utc>,
    ) -> Result<Reservation, DomainError> {
        let reservation = Reservation {
            id: Uuid::new_v4(),
            pool_type,
            owner_id,
            tenant_id,
            priority,
            ttl_seconds,
            constraints,
            start_at,
            status: "PENDING".to_string(),
            lease_id: None,
            created_at: Utc::now(),
        };
        self.repo.create_reservation(reservation.clone()).await?;
        Ok(reservation)
    }

    pub async fn get_reservation(&self, id: Uuid) -> Result<Reservation, DomainError> {
        self.repo
            .get_reservation(&id)
            .await?
            .ok_or(DomainError::ResourceNotFound)
    }
}
