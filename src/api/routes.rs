use crate::application::{
    allocation_service::AllocationService, pool_service::PoolService,
    resource_service::ResourceService,
};
use crate::domain::repository::{AllocationRepository, PoolRepository, ResourceRepository};
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

use super::{handlers, ui};

pub trait AppRepository:
    AllocationRepository + PoolRepository + ResourceRepository + Send + Sync + 'static
{
}

impl<T> AppRepository for T where
    T: AllocationRepository + PoolRepository + ResourceRepository + Send + Sync + 'static
{
}

pub struct AppState<R: AppRepository> {
    pub pool_service: Arc<PoolService<R>>,
    pub resource_service: Arc<ResourceService<R>>,
    pub allocation_service: Arc<AllocationService<R>>,
}

impl<R: AppRepository> Clone for AppState<R> {
    fn clone(&self) -> Self {
        Self {
            pool_service: self.pool_service.clone(),
            resource_service: self.resource_service.clone(),
            allocation_service: self.allocation_service.clone(),
        }
    }
}

pub fn create_router<R: AppRepository>(state: AppState<R>) -> Router {
    Router::new()
        .route("/", get(ui::admin_dashboard))
        .route("/admin", get(ui::admin_dashboard))
        .route("/admin/audit-log", get(ui::audit_log_stream))
        .route("/health", get(handlers::health_check))
        .route("/pools", post(handlers::create_pool))
        .route("/resources", post(handlers::register_resource))
        .route("/leases", post(handlers::allocate_lease))
        .route("/leases/:id", delete(handlers::release_lease))
        .route("/leases/:id/renew", post(handlers::renew_lease))
        .route("/leases/:id/heartbeat", post(handlers::heartbeat_lease))
        .with_state(state)
}
