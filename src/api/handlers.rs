use axum::{
    extract::{State, Path},
    http::StatusCode,
    Json,
};
use serde::{Deserialize};
use crate::api::routes::AppState;
use crate::domain::error::DomainError;
use crate::domain::{LeaseId, PoolId, AllocationPolicy};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreatePoolRequest {
    pub name: String,
    pub resource_type: String,
    pub policy: AllocationPolicy,
}

#[derive(Deserialize)]
pub struct RegisterResourceRequest {
    pub pool_id: Uuid,
    pub external_id: String,
    pub attributes: serde_json::Value,
}

#[derive(Deserialize)]
pub struct AllocateRequest {
    pub pool_type: String,
    pub constraints: Option<serde_json::Value>, 
    pub ttl_seconds: i64,
    pub idempotency_key: Option<String>,
    pub waitlist: Option<bool>, 
    pub preempt: Option<bool>,
    pub owner_id: String,
    pub tenant_id: String,
}

use axum::response::Html;

pub async fn health_check() -> Html<&'static str> {
    Html(r#"
        <div class="text-center animate-pulse">
            <div class="w-16 h-16 bg-emerald-500/20 rounded-full flex items-center justify-center mb-4 mx-auto text-emerald-500 border border-emerald-500/30">
                <i class="fas fa-heartbeat text-3xl"></i>
            </div>
            <p class="text-emerald-400 font-bold tracking-wider">SYSTEM OPERATIONAL</p>
            <p class="text-slate-500 text-xs mt-1">Connectivity Verified</p>
        </div>
    "#)
}

pub async fn create_pool(
    State(state): State<AppState>,
    Json(payload): Json<CreatePoolRequest>,
) -> Result<Json<crate::domain::pool::Pool>, (StatusCode, String)> {
    let result = state.pool_service.create_pool(
        payload.name,
        payload.resource_type,
        payload.policy,
    ).await;

    match result {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn get_pool_by_name(
    State(_state): State<AppState>,
    Path(_name): Path<String>,
) -> Result<Json<crate::domain::pool::Pool>, (StatusCode, String)> {
    // We'll need to add find_by_name to the service and repo, but for now
    // let's assume we can at least return a 404 if not found
    Err((StatusCode::NOT_IMPLEMENTED, "Search by name pending".to_string()))
}

pub async fn register_resource(
    State(state): State<AppState>,
    Json(payload): Json<RegisterResourceRequest>,
) -> Result<Json<crate::domain::resource::Resource>, (StatusCode, String)> {
    let result = state.resource_service.register_resource(
        PoolId(payload.pool_id),
        payload.external_id,
        payload.attributes,
    ).await;

    match result {
        Ok(resource) => Ok(Json(resource)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn allocate_lease(
    State(state): State<AppState>,
    Json(payload): Json<AllocateRequest>,
) -> Result<Json<crate::domain::lease::Lease>, (StatusCode, String)> {
    
    let result = state.allocation_service.allocate(
        payload.pool_type,
        payload.owner_id,
        payload.tenant_id,
        payload.ttl_seconds,
        payload.idempotency_key,
        payload.waitlist,
        payload.preempt,
    ).await;

    match result {
        Ok(lease) => Ok(Json(lease)),
        Err(DomainError::NoResourcesAvailable) => Err((StatusCode::CONFLICT, "No resources available".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[derive(Deserialize)]
pub struct RenewRequest {
    pub extension_seconds: i64,
}

pub async fn renew_lease(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<RenewRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    
    let result = state.allocation_service.renew(LeaseId(id), payload.extension_seconds).await;

    match result {
        Ok(_) => Ok(StatusCode::OK),
        Err(DomainError::LeaseNotFound) => Err((StatusCode::NOT_FOUND, "Lease not found or not active".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn release_lease(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    
    let result = state.allocation_service.release(LeaseId(id)).await;

    match result {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(DomainError::LeaseNotFound) => Err((StatusCode::NOT_FOUND, "Lease not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
