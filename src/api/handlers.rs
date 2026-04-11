use crate::api::routes::{AppRepository, AppState};
use crate::domain::error::DomainError;
use crate::domain::{AllocationPolicy, LeaseId, PoolId};
use atropos_contracts::{
    AllocateRequest, AllocationPolicy as ContractAllocationPolicy, CreatePoolRequest,
    LeaseResponse, LeaseStatus as ContractLeaseStatus, PoolResponse, RegisterResourceRequest,
    RenewRequest, ResourceResponse, ResourceStatus as ContractResourceStatus,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    Json,
};
use uuid::Uuid;

fn map_allocation_policy(policy: ContractAllocationPolicy) -> AllocationPolicy {
    match policy {
        ContractAllocationPolicy::FIFO => AllocationPolicy::FIFO,
        ContractAllocationPolicy::LRU => AllocationPolicy::LRU,
        ContractAllocationPolicy::Random => AllocationPolicy::Random,
    }
}

fn map_contract_allocation_policy(policy: AllocationPolicy) -> ContractAllocationPolicy {
    match policy {
        AllocationPolicy::FIFO => ContractAllocationPolicy::FIFO,
        AllocationPolicy::LRU => ContractAllocationPolicy::LRU,
        AllocationPolicy::Random => ContractAllocationPolicy::Random,
    }
}

fn map_resource_status(status: crate::domain::ResourceStatus) -> ContractResourceStatus {
    match status {
        crate::domain::ResourceStatus::Healthy => ContractResourceStatus::Healthy,
        crate::domain::ResourceStatus::Unhealthy => ContractResourceStatus::Unhealthy,
        crate::domain::ResourceStatus::Draining => ContractResourceStatus::Draining,
        crate::domain::ResourceStatus::Disabled => ContractResourceStatus::Disabled,
        crate::domain::ResourceStatus::Cooldown => ContractResourceStatus::Cooldown,
    }
}

fn map_lease_status(status: crate::domain::LeaseStatus) -> ContractLeaseStatus {
    match status {
        crate::domain::LeaseStatus::Pending => ContractLeaseStatus::Pending,
        crate::domain::LeaseStatus::Active => ContractLeaseStatus::Active,
        crate::domain::LeaseStatus::Expiring => ContractLeaseStatus::Expiring,
        crate::domain::LeaseStatus::Released => ContractLeaseStatus::Released,
        crate::domain::LeaseStatus::Expired => ContractLeaseStatus::Expired,
        crate::domain::LeaseStatus::Revoked => ContractLeaseStatus::Revoked,
        crate::domain::LeaseStatus::Waiting => ContractLeaseStatus::Waiting,
    }
}

fn map_pool(pool: crate::domain::pool::Pool) -> PoolResponse {
    PoolResponse {
        id: pool.id.0,
        name: pool.name,
        resource_type: pool.resource_type,
        policy: map_contract_allocation_policy(pool.policy),
        created_at: pool.created_at,
    }
}

fn map_resource(resource: crate::domain::resource::Resource) -> ResourceResponse {
    ResourceResponse {
        id: resource.id.0,
        pool_id: resource.pool_id.0,
        external_id: resource.external_id,
        status: map_resource_status(resource.status),
        attributes: resource.attributes,
        version: resource.version,
        updated_at: resource.updated_at,
    }
}

fn map_lease(lease: crate::domain::lease::Lease) -> LeaseResponse {
    LeaseResponse {
        id: lease.id.0,
        resource_id: lease.resource_id.0,
        owner_id: lease.owner_id,
        tenant_id: lease.tenant_id,
        status: map_lease_status(lease.status),
        created_at: lease.created_at,
        expires_at: lease.expires_at,
        idempotency_key: lease.idempotency_key,
        cost_center: lease.cost_center,
    }
}

pub async fn health_check() -> Html<&'static str> {
    Html(
        r#"
        <div class="text-center animate-pulse">
            <div class="w-16 h-16 bg-emerald-500/20 rounded-full flex items-center justify-center mb-4 mx-auto text-emerald-500 border border-emerald-500/30">
                <i class="fas fa-heartbeat text-3xl"></i>
            </div>
            <p class="text-emerald-400 font-bold tracking-wider">SYSTEM OPERATIONAL</p>
            <p class="text-slate-500 text-xs mt-1">Connectivity Verified</p>
        </div>
    "#,
    )
}

pub async fn create_pool(
    State(state): State<AppState<impl AppRepository>>,
    Json(payload): Json<CreatePoolRequest>,
) -> Result<Json<PoolResponse>, (StatusCode, String)> {
    let result = state
        .pool_service
        .create_pool(
            payload.name,
            payload.resource_type,
            map_allocation_policy(payload.policy),
        )
        .await;

    match result {
        Ok(pool) => Ok(Json(map_pool(pool))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn get_pool_by_name(
    State(_state): State<AppState<impl AppRepository>>,
    Path(_name): Path<String>,
) -> Result<Json<PoolResponse>, (StatusCode, String)> {
    // We'll need to add find_by_name to the service and repo, but for now
    // let's assume we can at least return a 404 if not found
    Err((
        StatusCode::NOT_IMPLEMENTED,
        "Search by name pending".to_string(),
    ))
}

pub async fn register_resource(
    State(state): State<AppState<impl AppRepository>>,
    Json(payload): Json<RegisterResourceRequest>,
) -> Result<Json<ResourceResponse>, (StatusCode, String)> {
    let result = state
        .resource_service
        .register_resource(
            PoolId(payload.pool_id),
            payload.external_id,
            payload.attributes,
        )
        .await;

    match result {
        Ok(resource) => Ok(Json(map_resource(resource))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn allocate_lease(
    State(state): State<AppState<impl AppRepository>>,
    Json(payload): Json<AllocateRequest>,
) -> Result<Json<LeaseResponse>, (StatusCode, String)> {
    let result = state
        .allocation_service
        .allocate(
            payload.pool_type,
            payload.owner_id,
            payload.tenant_id,
            payload.priority.unwrap_or(0),
            payload.ttl_seconds,
            payload.constraints,
            payload.spread_by,
            payload.idempotency_key,
            payload.waitlist,
            payload.preempt,
        )
        .await;

    match result {
        Ok(lease) => Ok(Json(map_lease(lease))),
        Err(DomainError::NoResourcesAvailable) => {
            Err((StatusCode::CONFLICT, "No resources available".to_string()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn renew_lease(
    State(state): State<AppState<impl AppRepository>>,
    Path(id): Path<uuid::Uuid>,
    Json(payload): Json<RenewRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state
        .allocation_service
        .renew(LeaseId(id), payload.extension_seconds)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(DomainError::LeaseNotFound) => {
            Err((StatusCode::NOT_FOUND, "Lease not found".to_string()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn heartbeat_lease(
    State(state): State<AppState<impl AppRepository>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state.allocation_service.heartbeat(LeaseId(id)).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(DomainError::LeaseNotFound) => {
            Err((StatusCode::NOT_FOUND, "Lease not found".to_string()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn release_lease(
    State(state): State<AppState<impl AppRepository>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let result = state.allocation_service.release(LeaseId(id)).await;

    match result {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(DomainError::LeaseNotFound) => {
            Err((StatusCode::NOT_FOUND, "Lease not found".to_string()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
