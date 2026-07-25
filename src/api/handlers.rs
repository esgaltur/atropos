use crate::api::routes::{AppRepository, AppState};
use crate::application::allocation_service::AllocationOutcome;
use crate::domain::error::DomainError;
use crate::domain::repository::{CostGroupBy, LeaseFilter};
use crate::domain::{AllocationPolicy, LeaseId, PoolId, ResourceId};
use atropos_contracts::{
    AllocateRequest, AllocationPolicy as ContractAllocationPolicy, CostReport, CostReportRow,
    CreatePoolRequest, CreateReservationRequest, LeaseQuery, LeaseResponse,
    LeaseStatus as ContractLeaseStatus, PoolResponse, PoolUtilizationResponse, QuotaResponse,
    RegisterResourceRequest, RenewRequest, ReservationResponse, ResourceResponse,
    ResourceStatus as ContractResourceStatus, SetLeaseLabelsRequest, SetQuotaRequest,
    UpdateResourceStatusRequest, WaitlistPositionResponse,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
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

fn map_contract_resource_status(status: ContractResourceStatus) -> crate::domain::ResourceStatus {
    match status {
        ContractResourceStatus::Healthy => crate::domain::ResourceStatus::Healthy,
        ContractResourceStatus::Unhealthy => crate::domain::ResourceStatus::Unhealthy,
        ContractResourceStatus::Draining => crate::domain::ResourceStatus::Draining,
        ContractResourceStatus::Disabled => crate::domain::ResourceStatus::Disabled,
        ContractResourceStatus::Cooldown => crate::domain::ResourceStatus::Cooldown,
    }
}

fn map_pool(pool: crate::domain::pool::Pool) -> PoolResponse {
    PoolResponse {
        id: pool.id.0,
        name: pool.name,
        resource_type: pool.resource_type,
        policy: map_contract_allocation_policy(pool.policy),
        max_capacity: pool.max_capacity,
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

pub async fn health_check(State(state): State<AppState<impl AppRepository>>) -> Response {
    match state.allocation_service.health().await {
        Ok(_) => (
            StatusCode::OK,
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
            ),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Health check failed: database unreachable: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(
                    r#"
        <div class="text-center">
            <div class="w-16 h-16 bg-red-500/20 rounded-full flex items-center justify-center mb-4 mx-auto text-red-500 border border-red-500/30">
                <i class="fas fa-heart-broken text-3xl"></i>
            </div>
            <p class="text-red-400 font-bold tracking-wider">SYSTEM DEGRADED</p>
            <p class="text-slate-500 text-xs mt-1">Database Unreachable</p>
        </div>
    "#,
                ),
            )
                .into_response()
        }
    }
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
            payload.max_capacity,
        )
        .await;

    match result {
        Ok(pool) => Ok(Json(map_pool(pool))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn get_pool_by_name(
    State(state): State<AppState<impl AppRepository>>,
    Path(name): Path<String>,
) -> Result<Json<PoolResponse>, (StatusCode, String)> {
    match state.pool_service.find_pool_by_name(&name).await {
        Ok(Some(pool)) => Ok(Json(map_pool(pool))),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Pool not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn register_resource(
    State(state): State<AppState<impl AppRepository>>,
    Json(payload): Json<RegisterResourceRequest>,
) -> Result<Json<ResourceResponse>, (StatusCode, String)> {
    // Enforce the pool's optional capacity cap before creating a new resource.
    if let Err(e) = state
        .platform_service
        .ensure_capacity_for_new_resource(PoolId(payload.pool_id))
        .await
    {
        return Err(match e {
            DomainError::PoolNotFound => (StatusCode::NOT_FOUND, e.to_string()),
            DomainError::QuotaExceeded(msg) => (StatusCode::CONFLICT, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        });
    }

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
) -> Response {
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
        Ok(AllocationOutcome::Leased(lease)) => {
            (StatusCode::OK, Json(map_lease(lease))).into_response()
        }
        Ok(AllocationOutcome::Waitlisted) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "status": "WAITLISTED",
                "message": "No resources available; request has been queued on the waitlist."
            })),
        )
            .into_response(),
        Err(DomainError::NoResourcesAvailable) => {
            (StatusCode::CONFLICT, "No resources available".to_string()).into_response()
        }
        Err(DomainError::QuotaExceeded(msg)) => {
            (StatusCode::TOO_MANY_REQUESTS, msg).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
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

pub async fn list_leases(
    State(state): State<AppState<impl AppRepository>>,
    Query(query): Query<LeaseQuery>,
) -> Result<Json<Vec<LeaseResponse>>, (StatusCode, String)> {
    let filter = LeaseFilter {
        tenant_id: query.tenant_id,
        owner_id: query.owner_id,
        status: query.status,
        limit: query.limit.unwrap_or(100),
    };
    match state.platform_service.list_leases(filter).await {
        Ok(leases) => Ok(Json(leases.into_iter().map(map_lease).collect())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn get_lease(
    State(state): State<AppState<impl AppRepository>>,
    Path(id): Path<Uuid>,
) -> Result<Json<LeaseResponse>, (StatusCode, String)> {
    match state.platform_service.get_lease(LeaseId(id)).await {
        Ok(lease) => Ok(Json(map_lease(lease))),
        Err(DomainError::LeaseNotFound) => {
            Err((StatusCode::NOT_FOUND, "Lease not found".to_string()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn set_lease_labels(
    State(state): State<AppState<impl AppRepository>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<SetLeaseLabelsRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state
        .platform_service
        .set_lease_labels(LeaseId(id), payload.labels)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(DomainError::LeaseNotFound) => {
            Err((StatusCode::NOT_FOUND, "Lease not found".to_string()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn set_quota(
    State(state): State<AppState<impl AppRepository>>,
    Json(payload): Json<SetQuotaRequest>,
) -> Result<Json<QuotaResponse>, (StatusCode, String)> {
    match state
        .platform_service
        .set_quota(
            payload.tenant_id,
            payload.pool_type,
            payload.max_active_leases,
            payload.soft_limit,
            payload.weight,
        )
        .await
    {
        Ok(quota) => Ok(Json(QuotaResponse {
            tenant_id: quota.tenant_id,
            pool_type: quota.pool_type,
            max_active_leases: quota.max_active_leases,
            soft_limit: quota.soft_limit,
            weight: quota.weight,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn get_quota(
    State(state): State<AppState<impl AppRepository>>,
    Path((tenant_id, pool_type)): Path<(String, String)>,
) -> Result<Json<QuotaResponse>, (StatusCode, String)> {
    match state
        .platform_service
        .get_quota(&tenant_id, &pool_type)
        .await
    {
        Ok(Some(quota)) => Ok(Json(QuotaResponse {
            tenant_id: quota.tenant_id,
            pool_type: quota.pool_type,
            max_active_leases: quota.max_active_leases,
            soft_limit: quota.soft_limit,
            weight: quota.weight,
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Quota not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn update_resource_status(
    State(state): State<AppState<impl AppRepository>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateResourceStatusRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state
        .platform_service
        .update_resource_status(ResourceId(id), map_contract_resource_status(payload.status))
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(DomainError::ResourceNotFound) => {
            Err((StatusCode::NOT_FOUND, "Resource not found".to_string()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn get_pool_utilization(
    State(state): State<AppState<impl AppRepository>>,
    Path(name): Path<String>,
) -> Result<Json<PoolUtilizationResponse>, (StatusCode, String)> {
    // The pool is addressed by name; its resource_type drives utilization queries.
    let pool = match state.pool_service.find_pool_by_name(&name).await {
        Ok(Some(pool)) => pool,
        Ok(None) => return Err((StatusCode::NOT_FOUND, "Pool not found".to_string())),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    match state
        .platform_service
        .get_pool_utilization(&pool.resource_type)
        .await
    {
        Ok(u) => Ok(Json(PoolUtilizationResponse {
            pool_type: u.pool_type,
            total_resources: u.total_resources,
            healthy_resources: u.healthy_resources,
            active_leases: u.active_leases,
            available: u.available,
            utilization_pct: u.utilization_pct,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn get_waitlist_position(
    State(state): State<AppState<impl AppRepository>>,
    Path(id): Path<Uuid>,
) -> Result<Json<WaitlistPositionResponse>, (StatusCode, String)> {
    match state.platform_service.get_waitlist_position(id).await {
        Ok(p) => Ok(Json(WaitlistPositionResponse {
            id: p.id,
            pool_type: p.pool_type,
            priority: p.priority,
            position: p.position,
            total_waiting: p.total_waiting,
        })),
        Err(DomainError::ResourceNotFound) => Err((
            StatusCode::NOT_FOUND,
            "Waitlist entry not found".to_string(),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[derive(serde::Deserialize)]
pub struct CostReportQuery {
    pub group_by: Option<String>,
}

pub async fn cost_report(
    State(state): State<AppState<impl AppRepository>>,
    Query(query): Query<CostReportQuery>,
) -> Result<Json<CostReport>, (StatusCode, String)> {
    let group_by = match query.group_by.as_deref() {
        Some("cost_center") => CostGroupBy::CostCenter,
        None | Some("tenant") => CostGroupBy::Tenant,
        Some(other) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unsupported group_by '{other}' (expected 'tenant' or 'cost_center')"),
            ))
        }
    };

    match state.platform_service.cost_report(group_by).await {
        Ok(rows) => Ok(Json(CostReport {
            group_by: group_by.as_str().to_string(),
            rows: rows
                .into_iter()
                .map(|r| CostReportRow {
                    group: r.group,
                    active_leases: r.active_leases,
                })
                .collect(),
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

fn map_reservation(r: crate::domain::reservation::Reservation) -> ReservationResponse {
    ReservationResponse {
        id: r.id,
        pool_type: r.pool_type,
        owner_id: r.owner_id,
        tenant_id: r.tenant_id,
        priority: r.priority,
        ttl_seconds: r.ttl_seconds,
        start_at: r.start_at,
        status: r.status,
        lease_id: r.lease_id.map(|l| l.0),
        created_at: r.created_at,
    }
}

pub async fn create_reservation(
    State(state): State<AppState<impl AppRepository>>,
    Json(payload): Json<CreateReservationRequest>,
) -> Result<(StatusCode, Json<ReservationResponse>), (StatusCode, String)> {
    match state
        .platform_service
        .create_reservation(
            payload.pool_type,
            payload.owner_id,
            payload.tenant_id,
            payload.priority.unwrap_or(0),
            payload.ttl_seconds,
            payload.constraints,
            payload.start_at,
        )
        .await
    {
        Ok(reservation) => Ok((StatusCode::CREATED, Json(map_reservation(reservation)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn get_reservation(
    State(state): State<AppState<impl AppRepository>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ReservationResponse>, (StatusCode, String)> {
    match state.platform_service.get_reservation(id).await {
        Ok(reservation) => Ok(Json(map_reservation(reservation))),
        Err(DomainError::ResourceNotFound) => {
            Err((StatusCode::NOT_FOUND, "Reservation not found".to_string()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
